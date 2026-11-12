mod db;
mod llm;
mod models;
mod prompt;

use db::DbResult;
use models::*;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio_util::sync::CancellationToken;

pub struct AppState {
    pub db: Mutex<rusqlite::Connection>,
    pub generations: Mutex<HashMap<String, CancellationToken>>,
    /// Session-only secret. Never persisted to SQLite or logs.
    pub api_key: Mutex<Option<String>>,
}

fn lock_db(state: &AppState) -> Result<std::sync::MutexGuard<'_, rusqlite::Connection>, String> {
    state
        .db
        .lock()
        .map_err(|_| "データベースのロックに失敗しました".to_string())
}

#[tauri::command]
fn list_characters(state: State<'_, AppState>) -> DbResult<Vec<Character>> {
    let conn = lock_db(state.inner())?;
    db::list_characters(&conn)
}

#[tauri::command]
fn create_character(state: State<'_, AppState>, input: CharacterInput) -> DbResult<Character> {
    let conn = lock_db(state.inner())?;
    db::create_character(&conn, &input)
}

#[tauri::command]
fn update_character(
    state: State<'_, AppState>,
    id: String,
    input: CharacterInput,
) -> DbResult<Character> {
    let conn = lock_db(state.inner())?;
    db::update_character(&conn, &id, &input)
}

#[tauri::command]
fn delete_character(state: State<'_, AppState>, id: String) -> DbResult<()> {
    let conn = lock_db(state.inner())?;
    db::delete_character(&conn, &id)
}

#[tauri::command]
fn list_conversations(state: State<'_, AppState>) -> DbResult<Vec<Conversation>> {
    let conn = lock_db(state.inner())?;
    db::list_conversations(&conn)
}

#[tauri::command]
fn create_conversation(
    state: State<'_, AppState>,
    input: ConversationInput,
) -> DbResult<Conversation> {
    let conn = lock_db(state.inner())?;
    db::create_conversation(&conn, &input)
}

#[tauri::command]
fn update_conversation(
    state: State<'_, AppState>,
    id: String,
    patch: ConversationUpdate,
) -> DbResult<Conversation> {
    let conn = lock_db(state.inner())?;
    db::update_conversation(&conn, &id, &patch)
}

#[tauri::command]
fn delete_conversation(state: State<'_, AppState>, id: String) -> DbResult<()> {
    let conn = lock_db(state.inner())?;
    db::delete_conversation(&conn, &id)
}

#[tauri::command]
fn list_messages(state: State<'_, AppState>, conversation_id: String) -> DbResult<Vec<Message>> {
    let conn = lock_db(state.inner())?;
    db::list_messages(&conn, &conversation_id)
}

#[tauri::command]
fn send_message(
    state: State<'_, AppState>,
    conversation_id: String,
    content: String,
) -> DbResult<Message> {
    let content = content.trim();
    if content.is_empty() {
        return Err("メッセージが空です".into());
    }
    let conn = lock_db(state.inner())?;
    db::get_conversation(&conn, &conversation_id)?;
    db::insert_message(&conn, &conversation_id, "user", content, None)
}

#[tauri::command]
fn get_settings(state: State<'_, AppState>) -> DbResult<Settings> {
    let conn = lock_db(state.inner())?;
    db::load_settings(&conn)
}

fn clamp_settings(settings: &mut Settings) {
    settings.base_url = llm::normalize_base_url(&settings.base_url);
    settings.model = settings.model.trim().to_string();
    settings.temperature = settings.temperature.clamp(0.0, 2.0);
    settings.top_p = settings.top_p.clamp(0.0, 1.0);
    settings.max_tokens = settings.max_tokens.clamp(32, 4096);
    settings.context_budget = settings.context_budget.clamp(1024, 32000);
    settings.timeout_secs = settings.timeout_secs.clamp(10, 600);
}

#[tauri::command]
fn save_settings(state: State<'_, AppState>, patch: SettingsPatch) -> DbResult<Settings> {
    let conn = lock_db(state.inner())?;
    let mut settings = db::load_settings(&conn)?;
    if let Some(v) = patch.base_url {
        settings.base_url = v;
    }
    if let Some(v) = patch.model {
        settings.model = v;
    }
    if let Some(v) = patch.temperature {
        settings.temperature = v;
    }
    if let Some(v) = patch.top_p {
        settings.top_p = v;
    }
    if let Some(v) = patch.max_tokens {
        settings.max_tokens = v;
    }
    if let Some(v) = patch.context_budget {
        settings.context_budget = v;
    }
    if let Some(v) = patch.timeout_secs {
        settings.timeout_secs = v;
    }
    if let Some(v) = patch.show_metrics {
        settings.show_metrics = v;
    }
    clamp_settings(&mut settings);
    db::save_settings(&conn, &settings)?;
    Ok(settings)
}

#[tauri::command]
fn set_api_key(state: State<'_, AppState>, key: Option<String>) -> Result<bool, String> {
    let mut stored = state
        .api_key
        .lock()
        .map_err(|_| "APIキーの保存に失敗しました".to_string())?;
    *stored = key.filter(|k| !k.trim().is_empty()).map(|k| k.trim().to_string());
    Ok(stored.is_some())
}

#[tauri::command]
fn has_api_key(state: State<'_, AppState>) -> bool {
    state
        .api_key
        .lock()
        .map(|k| k.is_some())
        .unwrap_or(false)
}

#[tauri::command]
async fn list_models(
    state: State<'_, AppState>,
    base_url: String,
    api_key: Option<String>,
) -> Result<Vec<String>, String> {
    let stored_key = state.api_key.lock().ok().and_then(|k| k.clone());
    let key = api_key
        .filter(|k| !k.trim().is_empty())
        .or(stored_key);
    let client = llm::build_client(20)?;
    llm::list_models(&client, &base_url, key.as_deref()).await
}

#[tauri::command]
async fn test_connection(
    state: State<'_, AppState>,
    base_url: String,
    api_key: Option<String>,
) -> Result<ConnectionInfo, String> {
    let stored_key = state.api_key.lock().ok().and_then(|k| k.clone());
    let key = api_key
        .filter(|k| !k.trim().is_empty())
        .or(stored_key);
    let client = llm::build_client(20)?;
    let result = llm::list_models(&client, &base_url, key.as_deref()).await;
    match result {
        Ok(models) => {
            let message = if models.is_empty() {
                "接続できました。モデル一覧は空です。モデル名を手入力してください。".to_string()
            } else {
                format!("接続できました。{} 個のモデルが見つかりました。", models.len())
            };
            Ok(ConnectionInfo {
                ok: true,
                models,
                message,
            })
        }
        Err(message) => Ok(ConnectionInfo {
            ok: false,
            models: vec![],
            message,
        }),
    }
}

#[tauri::command]
fn cancel_generation(state: State<'_, AppState>, request_id: String) -> bool {
    if let Ok(map) = state.generations.lock() {
        if let Some(token) = map.get(&request_id) {
            token.cancel();
            return true;
        }
    }
    false
}

fn emit_error(app: &AppHandle, request_id: &str, kind: &str, message: &str, partial: &str) {
    let _ = app.emit(
        "llm:error",
        ErrorPayload {
            request_id: request_id.to_string(),
            message: message.to_string(),
            kind: kind.to_string(),
            partial: partial.to_string(),
        },
    );
}

#[tauri::command]
async fn stream_reply(
    app: AppHandle,
    state: State<'_, AppState>,
    conversation_id: String,
    request_id: String,
) -> Result<(), String> {
    let (conversation, character, history, settings) = {
        let conn = lock_db(state.inner())?;
        let conversation = db::get_conversation(&conn, &conversation_id)?;
        let character = match conversation.character_id.as_deref() {
            Some(id) => db::get_character(&conn, id).ok(),
            None => None,
        };
        let history = db::list_messages(&conn, &conversation_id)?;
        let settings = db::load_settings(&conn)?;
        (conversation, character, history, settings)
    };

    let compiled = prompt::compile(&prompt::PromptInput {
        character: character.as_ref(),
        conversation: &conversation,
        history: &history,
        context_budget: settings.context_budget,
        max_tokens: settings.max_tokens,
    });

    let client = llm::build_client(settings.timeout_secs)?;
    let api_key = state.api_key.lock().ok().and_then(|k| k.clone());
    let token = CancellationToken::new();
    {
        let mut map = state
            .generations
            .lock()
            .map_err(|_| "生成状態を管理できません".to_string())?;
        map.insert(request_id.clone(), token.clone());
    }

    let start = Instant::now();
    let mut ttft_ms: Option<u64> = None;
    let app_for_delta = app.clone();
    let rid = request_id.clone();
    let outcome = llm::stream_chat(
        &client,
        llm::ChatRequest {
            base_url: &settings.base_url,
            api_key: api_key.as_deref(),
            model: &settings.model,
            messages: &compiled.messages,
            temperature: settings.temperature,
            top_p: settings.top_p,
            max_tokens: settings.max_tokens,
        },
        token.clone(),
        |delta| {
            if ttft_ms.is_none() {
                ttft_ms = Some(start.elapsed().as_millis() as u64);
            }
            let _ = app_for_delta.emit(
                "llm:delta",
                DeltaPayload {
                    request_id: rid.clone(),
                    delta: delta.to_string(),
                },
            );
        },
    )
    .await;

    if let Ok(mut map) = state.generations.lock() {
        map.remove(&request_id);
    }

    match outcome {
        Ok(result) => {
            let text = result.text.trim_end().to_string();
            if text.trim().is_empty() {
                let message = "モデルから空の応答が返りました。モデル名やプロンプト設定を確認してください。";
                emit_error(&app, &request_id, "empty", message, "");
                return Err(message.into());
            }
            let stopped = result.end == llm::StreamEnd::Canceled;
            let duration_ms = start.elapsed().as_millis() as u64;
            let estimated_tokens = prompt::estimate_tokens(&text);
            let metrics = GenMetrics {
                ttft_ms: ttft_ms.unwrap_or(duration_ms),
                duration_ms,
                chars: text.chars().count(),
                estimated_tokens,
                tokens_per_sec: if duration_ms > 0 {
                    (estimated_tokens as f64) * 1000.0 / (duration_ms as f64)
                } else {
                    0.0
                },
                context_tokens: compiled.context_tokens,
                trimmed_messages: compiled.trimmed_messages,
            };
            let meta = serde_json::json!({
                "metrics": metrics,
                "stopped": stopped,
                "incomplete": false,
            });
            let saved = {
                let conn = lock_db(state.inner())?;
                match db::insert_message(&conn, &conversation_id, "assistant", &text, Some(&meta)) {
                    Ok(message) => Some(message),
                    Err(e) => {
                        // The model output is still delivered to the UI even if saving fails.
                        eprintln!("assistant message save failed: {e}");
                        None
                    }
                }
            };
            if let Some(message) = saved {
                let _ = app.emit(
                    "llm:done",
                    DonePayload {
                        request_id: request_id.clone(),
                        message,
                        metrics,
                        stopped,
                    },
                );
            } else {
                emit_error(
                    &app,
                    &request_id,
                    "persistence",
                    "応答は生成されましたが、会話の保存に失敗しました。",
                    &text,
                );
                return Err("応答の保存に失敗しました".into());
            }
            Ok(())
        }
        Err(failure) => {
            let partial = failure.partial.trim_end().to_string();
            if !partial.is_empty() {
                let meta = serde_json::json!({
                    "incomplete": true,
                    "stopped": false,
                });
                if let Ok(conn) = lock_db(state.inner()) {
                    let _ = db::insert_message(
                        &conn,
                        &conversation_id,
                        "assistant",
                        &partial,
                        Some(&meta),
                    );
                }
            }
            eprintln!("generation failed ({}): {}", failure.kind, failure.message);
            emit_error(
                &app,
                &request_id,
                &failure.kind,
                &failure.message,
                &partial,
            );
            Err(failure.message)
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let data_dir = app
                .path()
                .app_data_dir()
                .map_err(|e| format!("アプリデータフォルダを取得できません: {e}"))?;
            let db_path = data_dir.join("homer.db");
            let connection = db::open_db(&db_path)
                .map_err(|e| format!("データベースを初期化できません: {e}"))?;
            app.manage(AppState {
                db: Mutex::new(connection),
                generations: Mutex::new(HashMap::new()),
                api_key: Mutex::new(None),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_characters,
            create_character,
            update_character,
            delete_character,
            list_conversations,
            create_conversation,
            update_conversation,
            delete_conversation,
            list_messages,
            send_message,
            get_settings,
            save_settings,
            set_api_key,
            has_api_key,
            list_models,
            test_connection,
            cancel_generation,
            stream_reply,
        ])
        .run(tauri::generate_context!())
        .expect("Homer の起動に失敗しました");
}
