use std::sync::{Arc, Mutex};
use std::time::Instant;

use async_channel::{Receiver, Sender};
use homer_core::{
    Character, CharacterInput, Conversation, GenMetrics, Message, MessageMeta, Settings,
};
use homer_inference::{build_client, list_models, stream_chat, ChatRequest, StreamEnd};
use homer_storage::Storage;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone)]
pub enum UiEvent {
    ConversationLoaded {
        conversation_id: String,
        messages: Vec<Message>,
    },
    ConversationCreated(Conversation),
    ConversationUpdated(Conversation),
    ConversationDeleted(String),
    CharacterSaved(Character),
    CharacterDeleted(String),
    UserMessageSaved(Message),
    Delta {
        request_id: String,
        delta: String,
    },
    GenerationDone {
        request_id: String,
        message: Message,
        stopped: bool,
    },
    GenerationError {
        request_id: String,
        message: String,
        kind: String,
        restore_input: Option<String>,
        retryable: bool,
    },
    SettingsSaved(Settings),
    ModelsFetched {
        models: Vec<String>,
        error: Option<String>,
    },
    Error(String),
}

#[derive(Clone)]
pub struct Services {
    pub storage: Arc<Storage>,
    pub runtime: tokio::runtime::Handle,
    pub tx: Sender<UiEvent>,
    pub api_key: Arc<Mutex<Option<String>>>,
}

impl Services {
    pub fn channel() -> (Sender<UiEvent>, Receiver<UiEvent>) {
        async_channel::unbounded()
    }

    pub fn api_key(&self) -> Option<String> {
        self.api_key.lock().ok().and_then(|key| key.clone())
    }

    pub fn set_api_key(&self, key: Option<String>) {
        if let Ok(mut slot) = self.api_key.lock() {
            *slot = key.filter(|value| !value.trim().is_empty());
        }
    }

    pub fn load_messages(&self, conversation_id: String) {
        let storage = self.storage.clone();
        let tx = self.tx.clone();
        self.runtime.spawn(async move {
            match storage.list_messages(&conversation_id) {
                Ok(messages) => {
                    let _ = tx
                        .send(UiEvent::ConversationLoaded {
                            conversation_id,
                            messages,
                        })
                        .await;
                }
                Err(error) => {
                    let _ = tx.send(UiEvent::Error(error)).await;
                }
            }
        });
    }

    pub fn save_character(&self, id: Option<String>, input: CharacterInput) {
        let storage = self.storage.clone();
        let tx = self.tx.clone();
        self.runtime.spawn(async move {
            let result = match &id {
                Some(id) => storage.update_character(id, &input),
                None => storage.create_character(&input),
            };
            match result {
                Ok(character) => {
                    let _ = tx.send(UiEvent::CharacterSaved(character)).await;
                }
                Err(error) => {
                    let _ = tx.send(UiEvent::Error(error)).await;
                }
            }
        });
    }

    pub fn delete_character(&self, id: String) {
        let storage = self.storage.clone();
        let tx = self.tx.clone();
        self.runtime.spawn(async move {
            match storage.delete_character(&id) {
                Ok(()) => {
                    let _ = tx.send(UiEvent::CharacterDeleted(id)).await;
                }
                Err(error) => {
                    let _ = tx.send(UiEvent::Error(error)).await;
                }
            }
        });
    }

    pub fn create_conversation(&self, input: homer_core::ConversationInput) {
        let storage = self.storage.clone();
        let tx = self.tx.clone();
        self.runtime.spawn(async move {
            match storage.create_conversation(&input) {
                Ok(conversation) => {
                    let _ = tx.send(UiEvent::ConversationCreated(conversation)).await;
                }
                Err(error) => {
                    let _ = tx.send(UiEvent::Error(error)).await;
                }
            }
        });
    }

    pub fn update_conversation(&self, id: String, patch: homer_core::ConversationUpdate) {
        let storage = self.storage.clone();
        let tx = self.tx.clone();
        self.runtime.spawn(async move {
            match storage.update_conversation(&id, &patch) {
                Ok(conversation) => {
                    let _ = tx.send(UiEvent::ConversationUpdated(conversation)).await;
                }
                Err(error) => {
                    let _ = tx.send(UiEvent::Error(error)).await;
                }
            }
        });
    }

    pub fn delete_conversation(&self, id: String) {
        let storage = self.storage.clone();
        let tx = self.tx.clone();
        self.runtime.spawn(async move {
            match storage.delete_conversation(&id) {
                Ok(()) => {
                    let _ = tx.send(UiEvent::ConversationDeleted(id)).await;
                }
                Err(error) => {
                    let _ = tx.send(UiEvent::Error(error)).await;
                }
            }
        });
    }

    pub fn save_settings(&self, settings: Settings) {
        let storage = self.storage.clone();
        let tx = self.tx.clone();
        self.runtime.spawn(async move {
            match storage.save_settings(&settings) {
                Ok(()) => {
                    let _ = tx.send(UiEvent::SettingsSaved(settings)).await;
                }
                Err(error) => {
                    let _ = tx.send(UiEvent::Error(error)).await;
                }
            }
        });
    }

    pub fn fetch_models(&self, base_url: String, api_key: Option<String>) {
        let tx = self.tx.clone();
        let key = api_key.or_else(|| self.api_key());
        self.runtime.spawn(async move {
            let client = match build_client(20) {
                Ok(client) => client,
                Err(error) => {
                    let _ = tx
                        .send(UiEvent::ModelsFetched {
                            models: Vec::new(),
                            error: Some(error),
                        })
                        .await;
                    return;
                }
            };
            match list_models(&client, &base_url, key.as_deref()).await {
                Ok(models) => {
                    let _ = tx
                        .send(UiEvent::ModelsFetched {
                            models,
                            error: None,
                        })
                        .await;
                }
                Err(error) => {
                    let _ = tx
                        .send(UiEvent::ModelsFetched {
                            models: Vec::new(),
                            error: Some(error),
                        })
                        .await;
                }
            }
        });
    }

    /// Save the user's message, build a prompt, and stream one assistant reply.
    pub fn send_and_generate(
        &self,
        conversation_id: String,
        request_id: String,
        text: String,
        cancel: CancellationToken,
    ) {
        let services = self.clone();
        let tx = self.tx.clone();
        self.runtime.spawn(async move {
            let storage = services.storage.clone();
            let convo = conversation_id.clone();
            let user_text = text.clone();
            let saved = tokio::task::spawn_blocking(move || {
                storage.insert_message(&convo, "user", &user_text, None)
            })
            .await;
            let saved = match saved {
                Ok(result) => result,
                Err(error) => Err(format!("保存処理を実行できませんでした: {error}")),
            };
            let saved = match saved {
                Ok(message) => message,
                Err(error) => {
                    let _ = tx
                        .send(UiEvent::GenerationError {
                            request_id,
                            message: format!("メッセージを保存できませんでした: {error}"),
                            kind: "persistence".into(),
                            restore_input: Some(text),
                            retryable: false,
                        })
                        .await;
                    return;
                }
            };
            if tx.send(UiEvent::UserMessageSaved(saved)).await.is_err() {
                return;
            }

            let storage = services.storage.clone();
            let load_convo = conversation_id.clone();
            let prepared = tokio::task::spawn_blocking(move || {
                let conversation = storage.conversation(&load_convo)?;
                let character = conversation
                    .character_id
                    .as_deref()
                    .and_then(|id| storage.character(id).ok());
                let mut history = storage.list_messages(&load_convo)?;
                // Interrupted partial replies are shown in the UI but should not
                // steer the next turn.
                history.retain(|message| {
                    !message
                        .meta
                        .as_ref()
                        .map(|meta| meta.incomplete)
                        .unwrap_or(false)
                });
                let settings = storage.load_settings()?;
                let compiled = homer_core::prompt::compile(&homer_core::prompt::PromptInput {
                    character: character.as_ref(),
                    conversation: &conversation,
                    history: &history,
                    context_budget: settings.context_budget,
                    max_tokens: settings.max_tokens,
                });
                Ok::<_, String>((conversation, compiled, settings))
            })
            .await;

            let prepared = match prepared {
                Ok(result) => result,
                Err(error) => Err(format!("会話を読み込めませんでした: {error}")),
            };
            let (_conversation, compiled, settings) = match prepared {
                Ok(value) => value,
                Err(error) => {
                    let _ = tx
                        .send(UiEvent::GenerationError {
                            request_id,
                            message: error,
                            kind: "persistence".into(),
                            restore_input: None,
                            retryable: true,
                        })
                        .await;
                    return;
                }
            };

            let client = match build_client(settings.timeout_secs) {
                Ok(client) => client,
                Err(error) => {
                    let _ = tx
                        .send(UiEvent::GenerationError {
                            request_id,
                            message: error,
                            kind: "config".into(),
                            restore_input: None,
                            retryable: true,
                        })
                        .await;
                    return;
                }
            };

            let started = Instant::now();
            let mut ttft_ms: Option<u64> = None;
            let tx_delta = tx.clone();
            let rid = request_id.clone();
            let api_key = services.api_key();
            let outcome = stream_chat(
                &client,
                ChatRequest {
                    base_url: &settings.base_url,
                    api_key: api_key.as_deref(),
                    model: &settings.model,
                    messages: &compiled.messages,
                    temperature: settings.temperature,
                    top_p: settings.top_p,
                    max_tokens: settings.max_tokens,
                },
                cancel,
                |delta| {
                    if ttft_ms.is_none() {
                        ttft_ms = Some(started.elapsed().as_millis() as u64);
                    }
                    let _ = tx_delta.try_send(UiEvent::Delta {
                        request_id: rid.clone(),
                        delta: delta.to_string(),
                    });
                },
            )
            .await;

            match outcome {
                Ok(result) => {
                    if result.text.trim().is_empty() {
                        let _ = tx
                            .send(UiEvent::GenerationError {
                                request_id,
                                message: "モデルから空の応答が返りました。モデル名かサーバーの状態を確認してください。".into(),
                                kind: "empty".into(),
                                restore_input: None,
                                retryable: true,
                            })
                            .await;
                        return;
                    }
                    let text = result.text.trim_end().to_string();
                    let stopped = result.end == StreamEnd::Canceled;
                    let duration_ms = started.elapsed().as_millis() as u64;
                    let estimated_tokens = homer_core::estimate_tokens(&text);
                    let metrics = GenMetrics {
                        ttft_ms: ttft_ms.unwrap_or(duration_ms),
                        duration_ms,
                        chars: text.chars().count(),
                        estimated_tokens,
                        tokens_per_sec: if duration_ms > 0 {
                            estimated_tokens as f64 * 1000.0 / duration_ms as f64
                        } else {
                            0.0
                        },
                        context_tokens: compiled.context_tokens,
                        trimmed_messages: compiled.trimmed_messages,
                    };
                    let meta = MessageMeta {
                        metrics: Some(metrics),
                        stopped,
                        incomplete: false,
                    };
                    let storage = services.storage.clone();
                    let convo = conversation_id.clone();
                    let content = text.clone();
                    let saved = tokio::task::spawn_blocking(move || {
                        storage.insert_message(&convo, "assistant", &content, Some(&meta))
                    })
                    .await;
                    let saved = match saved {
                        Ok(result) => result,
                        Err(error) => Err(format!("保存処理を実行できませんでした: {error}")),
                    };
                    match saved {
                        Ok(message) => {
                            let _ = tx
                                .send(UiEvent::GenerationDone {
                                    request_id,
                                    message,
                                    stopped,
                                })
                                .await;
                        }
                        Err(error) => {
                            let _ = tx
                                .send(UiEvent::GenerationError {
                                    request_id,
                                    message: format!(
                                        "応答は生成されましたが、保存に失敗しました: {error}"
                                    ),
                                    kind: "persistence".into(),
                                    restore_input: None,
                                    retryable: false,
                                })
                                .await;
                        }
                    }
                }
                Err(failure) => {
                    let partial = failure.partial.trim_end().to_string();
                    if !partial.is_empty() {
                        let meta = MessageMeta {
                            metrics: None,
                            stopped: false,
                            incomplete: true,
                        };
                        let storage = services.storage.clone();
                        let convo = conversation_id.clone();
                        let content = partial.clone();
                        let _ = tokio::task::spawn_blocking(move || {
                            storage.insert_message(&convo, "assistant", &content, Some(&meta))
                        })
                        .await;
                    }
                    let _ = tx
                        .send(UiEvent::GenerationError {
                            request_id,
                            message: failure.message,
                            kind: failure.kind,
                            restore_input: None,
                            retryable: true,
                        })
                        .await;
                }
            }
        });
    }

    pub fn retry_generation(
        &self,
        conversation_id: String,
        request_id: String,
        cancel: CancellationToken,
    ) {
        let services = self.clone();
        let tx = self.tx.clone();
        self.runtime.spawn(async move {
            let storage = services.storage.clone();
            let load_convo = conversation_id.clone();
            let prepared = tokio::task::spawn_blocking(move || {
                let conversation = storage.conversation(&load_convo)?;
                let character = conversation
                    .character_id
                    .as_deref()
                    .and_then(|id| storage.character(id).ok());
                let mut history = storage.list_messages(&load_convo)?;
                history.retain(|message| {
                    !message
                        .meta
                        .as_ref()
                        .map(|meta| meta.incomplete)
                        .unwrap_or(false)
                });
                let settings = storage.load_settings()?;
                let compiled = homer_core::prompt::compile(&homer_core::prompt::PromptInput {
                    character: character.as_ref(),
                    conversation: &conversation,
                    history: &history,
                    context_budget: settings.context_budget,
                    max_tokens: settings.max_tokens,
                });
                Ok::<_, String>((compiled, settings))
            })
            .await;
            let prepared = match prepared {
                Ok(result) => result,
                Err(error) => Err(format!("会話を読み込めませんでした: {error}")),
            };
            let (compiled, settings) = match prepared {
                Ok(value) => value,
                Err(error) => {
                    let _ = tx
                        .send(UiEvent::GenerationError {
                            request_id,
                            message: error,
                            kind: "persistence".into(),
                            restore_input: None,
                            retryable: false,
                        })
                        .await;
                    return;
                }
            };
            stream_reply_into_ui(
                services,
                tx,
                conversation_id,
                request_id,
                compiled,
                settings,
                cancel,
            )
            .await;
        });
    }
}

async fn stream_reply_into_ui(
    services: Services,
    tx: Sender<UiEvent>,
    conversation_id: String,
    request_id: String,
    compiled: homer_core::CompiledPrompt,
    settings: Settings,
    cancel: CancellationToken,
) {
    let client = match build_client(settings.timeout_secs) {
        Ok(client) => client,
        Err(error) => {
            let _ = tx
                .send(UiEvent::GenerationError {
                    request_id,
                    message: error,
                    kind: "config".into(),
                    restore_input: None,
                    retryable: true,
                })
                .await;
            return;
        }
    };
    let started = Instant::now();
    let mut ttft_ms: Option<u64> = None;
    let tx_delta = tx.clone();
    let rid = request_id.clone();
    let api_key = services.api_key();
    let outcome = stream_chat(
        &client,
        ChatRequest {
            base_url: &settings.base_url,
            api_key: api_key.as_deref(),
            model: &settings.model,
            messages: &compiled.messages,
            temperature: settings.temperature,
            top_p: settings.top_p,
            max_tokens: settings.max_tokens,
        },
        cancel,
        |delta| {
            if ttft_ms.is_none() {
                ttft_ms = Some(started.elapsed().as_millis() as u64);
            }
            let _ = tx_delta.try_send(UiEvent::Delta {
                request_id: rid.clone(),
                delta: delta.to_string(),
            });
        },
    )
    .await;

    match outcome {
        Ok(result) => {
            if result.text.trim().is_empty() {
                let _ = tx
                    .send(UiEvent::GenerationError {
                        request_id,
                        message: "モデルから空の応答が返りました。モデル名かサーバーの状態を確認してください。".into(),
                        kind: "empty".into(),
                        restore_input: None,
                        retryable: true,
                    })
                    .await;
                return;
            }
            let text = result.text.trim_end().to_string();
            let stopped = result.end == StreamEnd::Canceled;
            let duration_ms = started.elapsed().as_millis() as u64;
            let estimated_tokens = homer_core::estimate_tokens(&text);
            let metrics = GenMetrics {
                ttft_ms: ttft_ms.unwrap_or(duration_ms),
                duration_ms,
                chars: text.chars().count(),
                estimated_tokens,
                tokens_per_sec: if duration_ms > 0 {
                    estimated_tokens as f64 * 1000.0 / duration_ms as f64
                } else {
                    0.0
                },
                context_tokens: compiled.context_tokens,
                trimmed_messages: compiled.trimmed_messages,
            };
            let meta = MessageMeta {
                metrics: Some(metrics),
                stopped,
                incomplete: false,
            };
            let storage = services.storage.clone();
            let convo = conversation_id.clone();
            let saved = tokio::task::spawn_blocking(move || {
                storage.insert_message(&convo, "assistant", &text, Some(&meta))
            })
            .await;
            let saved = match saved {
                Ok(result) => result,
                Err(error) => Err(format!("保存処理を実行できませんでした: {error}")),
            };
            match saved {
                Ok(message) => {
                    let _ = tx
                        .send(UiEvent::GenerationDone {
                            request_id,
                            message,
                            stopped,
                        })
                        .await;
                }
                Err(error) => {
                    let _ = tx
                        .send(UiEvent::GenerationError {
                            request_id,
                            message: format!("応答の保存に失敗しました: {error}"),
                            kind: "persistence".into(),
                            restore_input: None,
                            retryable: true,
                        })
                        .await;
                }
            }
        }
        Err(failure) => {
            let partial = failure.partial.trim_end().to_string();
            if !partial.is_empty() {
                let meta = MessageMeta {
                    metrics: None,
                    stopped: false,
                    incomplete: true,
                };
                let storage = services.storage.clone();
                let convo = conversation_id.clone();
                let content = partial.clone();
                let _ = tokio::task::spawn_blocking(move || {
                    storage.insert_message(&convo, "assistant", &content, Some(&meta))
                })
                .await;
            }
            let _ = tx
                .send(UiEvent::GenerationError {
                    request_id,
                    message: failure.message,
                    kind: failure.kind,
                    restore_input: None,
                    retryable: true,
                })
                .await;
        }
    }
}
