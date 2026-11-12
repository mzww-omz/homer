use std::time::{Duration, Instant};

use async_channel::Receiver;
use gpui::{
    actions, prelude::*, px, App, Context, Entity, FocusHandle, Focusable, PromptLevel,
    ScrollHandle, Timer, Window,
};
use homer_core::{
    Character, CharacterInput, Conversation, ConversationInput, ConversationUpdate, Message,
    Settings,
};
use tokio_util::sync::CancellationToken;

use crate::services::{Services, UiEvent};
use crate::text_area::TextArea;

actions!(
    homer,
    [
        NewConversationAction,
        OpenSettingsAction,
        CloseModalAction,
        SubmitMessageAction
    ]
);

pub struct Startup {
    pub characters: Vec<Character>,
    pub conversations: Vec<Conversation>,
    pub messages: Vec<Message>,
    pub settings: Settings,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SidebarTab {
    Conversations,
    Characters,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Modal {
    None,
    Character,
    NewConversation,
    Settings,
    Scene,
}

pub struct AppError {
    pub message: String,
    pub kind: String,
    pub retryable: bool,
}

pub struct Streaming {
    pub request_id: String,
    pub conversation_id: String,
    pub text: String,
    pub cancel: CancellationToken,
}

pub struct CharacterForm {
    pub id: Option<String>,
    pub name: Entity<TextArea>,
    pub description: Entity<TextArea>,
    pub personality: Entity<TextArea>,
    pub speech_style: Entity<TextArea>,
    pub first_person: Entity<TextArea>,
    pub relationship: Entity<TextArea>,
    pub role: Entity<TextArea>,
    pub example_dialogue: Entity<TextArea>,
    pub extra: Entity<TextArea>,
    pub accent: String,
    pub saving: bool,
}

pub struct NewConversationForm {
    pub character_id: Option<String>,
    pub mode: String,
    pub title: Entity<TextArea>,
    pub scenario: Entity<TextArea>,
    pub saving: bool,
}

pub struct SettingsForm {
    pub base_url: Entity<TextArea>,
    pub api_key: Entity<TextArea>,
    pub model: Entity<TextArea>,
    pub temperature: Entity<TextArea>,
    pub max_tokens: Entity<TextArea>,
    pub context_budget: Entity<TextArea>,
    pub show_metrics: bool,
    pub models: Vec<String>,
    pub status: Option<(bool, String)>,
    pub busy: bool,
    pub key_set: bool,
}

pub struct SceneForm {
    pub conversation_id: Option<String>,
    pub title: Entity<TextArea>,
    pub scenario: Entity<TextArea>,
    pub scene: Entity<TextArea>,
    pub notes: Entity<TextArea>,
    pub saving: bool,
}

fn input(
    cx: &mut Context<HomerApp>,
    placeholder: &str,
    height: f32,
    multiline: bool,
    context: &str,
) -> Entity<TextArea> {
    let placeholder = placeholder.to_string();
    let context = context.to_string();
    cx.new(move |cx| TextArea::new(placeholder, height, multiline, context, cx))
}

pub struct HomerApp {
    pub(crate) services: Services,
    pub(crate) rx: Receiver<UiEvent>,
    pub(crate) characters: Vec<Character>,
    pub(crate) conversations: Vec<Conversation>,
    pub(crate) messages: Vec<Message>,
    pub(crate) settings: Settings,
    pub(crate) active_id: Option<String>,
    pub(crate) error: Option<AppError>,
    pub(crate) streaming: Option<Streaming>,
    pub(crate) loading_messages: bool,
    pub(crate) composer: Entity<TextArea>,
    pub(crate) scroll: ScrollHandle,
    pub(crate) sidebar_scroll: ScrollHandle,
    pub(crate) tab: SidebarTab,
    pub(crate) panel_open: bool,
    pub(crate) modal: Modal,
    pub(crate) toast: Option<(String, Instant)>,
    pub(crate) char_form: CharacterForm,
    pub(crate) new_form: NewConversationForm,
    pub(crate) settings_form: SettingsForm,
    pub(crate) scene_form: SceneForm,
    pub(crate) event_loop_started: bool,
    pub(crate) auto_follow: bool,
    pub(crate) first_render: bool,
}

impl HomerApp {
    pub fn new(
        services: Services,
        startup: Startup,
        rx: Receiver<UiEvent>,
        cx: &mut Context<Self>,
    ) -> Self {
        let composer = cx.new(|cx| {
            TextArea::new("言葉を書く…", 94., true, "HomerComposer HomerInput", cx).emphasized()
        });
        let char_form = CharacterForm {
            id: None,
            name: input(cx, "例: 七海", 34., false, "HomerInput"),
            description: input(
                cx,
                "どんな人物？ ひとことで",
                68.,
                true,
                "HomerMultiline HomerInput",
            ),
            personality: input(cx, "性格や価値観", 64., true, "HomerMultiline HomerInput"),
            speech_style: input(
                cx,
                "口調、語尾、テンポなど",
                64.,
                true,
                "HomerMultiline HomerInput",
            ),
            first_person: input(cx, "私 / 僕 / あたし など", 34., false, "HomerInput"),
            relationship: input(cx, "ユーザーとの関係", 34., false, "HomerInput"),
            role: input(cx, "物語での役割", 34., false, "HomerInput"),
            example_dialogue: input(
                cx,
                "「……」\n「……別に、嫌じゃないけど」",
                74.,
                true,
                "HomerMultiline HomerInput",
            ),
            extra: input(
                cx,
                "その他の設定（任意）",
                64.,
                true,
                "HomerMultiline HomerInput",
            ),
            accent: "#7d6f5c".into(),
            saving: false,
        };
        let new_form = NewConversationForm {
            character_id: startup
                .conversations
                .first()
                .and_then(|c| c.character_id.clone()),
            mode: "story".into(),
            title: input(cx, "例: 放課後の図書室", 34., false, "HomerInput"),
            scenario: input(
                cx,
                "どんな場所で、何が起きている？",
                92.,
                true,
                "HomerMultiline HomerInput",
            ),
            saving: false,
        };
        let settings_form = SettingsForm {
            base_url: input(cx, "http://localhost:1234/v1", 34., false, "HomerInput"),
            api_key: input(cx, "空欄なら使用しない", 34., false, "HomerInput"),
            model: input(cx, "モデル名", 34., false, "HomerInput"),
            temperature: input(cx, "0.85", 34., false, "HomerInput"),
            max_tokens: input(cx, "400", 34., false, "HomerInput"),
            context_budget: input(cx, "6000", 34., false, "HomerInput"),
            show_metrics: false,
            models: Vec::new(),
            status: None,
            busy: false,
            key_set: false,
        };
        let scene_form = SceneForm {
            conversation_id: None,
            title: input(cx, "会話のタイトル", 34., false, "HomerInput"),
            scenario: input(cx, "シナリオ", 86., true, "HomerMultiline HomerInput"),
            scene: input(cx, "現在の場面", 68., true, "HomerMultiline HomerInput"),
            notes: input(
                cx,
                "覚えておくこと（任意）",
                74.,
                true,
                "HomerMultiline HomerInput",
            ),
            saving: false,
        };
        let active_id = startup.conversations.first().map(|c| c.id.clone());
        Self {
            services,
            rx,
            characters: startup.characters,
            conversations: startup.conversations,
            messages: startup.messages,
            settings: startup.settings,
            active_id,
            error: None,
            streaming: None,
            loading_messages: false,
            composer,
            scroll: ScrollHandle::new(),
            sidebar_scroll: ScrollHandle::new(),
            tab: SidebarTab::Conversations,
            panel_open: false,
            modal: Modal::None,
            toast: None,
            char_form,
            new_form,
            settings_form,
            scene_form,
            event_loop_started: false,
            auto_follow: true,
            first_render: true,
        }
    }

    pub fn composer_focus(&self, cx: &App) -> FocusHandle {
        self.composer.read(cx).focus_handle(cx)
    }

    /// Development-only hook used by screenshot/smoke runs.
    pub fn debug_open_view(&mut self, view: &str, window: &mut Window, cx: &mut Context<Self>) {
        match view {
            "settings" => self.open_settings(window, cx),
            "character" => self.open_character(None, window, cx),
            "new-conversation" => self.open_new_conversation(cx),
            "panel" => {
                self.panel_open = true;
                cx.notify();
            }
            _ => {}
        }
    }

    pub(crate) fn active_conversation(&self) -> Option<&Conversation> {
        self.active_id
            .as_deref()
            .and_then(|id| self.conversations.iter().find(|c| c.id == id))
    }

    pub(crate) fn active_character(&self) -> Option<&Character> {
        self.active_conversation()
            .and_then(|c| c.character_id.as_deref())
            .and_then(|id| self.characters.iter().find(|c| c.id == id))
    }

    fn sort_conversations(&mut self) {
        self.conversations
            .sort_by_key(|c| std::cmp::Reverse(c.updated_at));
    }

    fn touch_conversation(&mut self, id: &str, ts: i64) {
        if let Some(conversation) = self.conversations.iter_mut().find(|c| c.id == id) {
            if conversation.updated_at < ts {
                conversation.updated_at = ts;
            }
        }
        self.sort_conversations();
    }

    fn notify(&mut self, message: impl Into<String>, cx: &mut Context<Self>) {
        let message = message.into();
        self.toast = Some((message, Instant::now()));
        cx.spawn(async move |this, cx| {
            Timer::after(Duration::from_millis(2600)).await;
            let _ = this.update(cx, |app, cx| {
                if app
                    .toast
                    .as_ref()
                    .map(|(_, at)| at.elapsed() >= Duration::from_millis(2500))
                    .unwrap_or(false)
                {
                    app.toast = None;
                    cx.notify();
                }
            });
        })
        .detach();
        cx.notify();
    }

    fn near_bottom(&self) -> bool {
        let offset = self.scroll.offset().y;
        let max = self.scroll.max_offset().height;
        max + offset < px(90.)
    }

    fn scroll_to_bottom(&self) {
        self.scroll.scroll_to_bottom();
    }

    // ---------- actions ----------

    pub(crate) fn submit_message(
        &mut self,
        _: &SubmitMessageAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.send_composer(window, cx);
    }

    pub(crate) fn send_composer(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let text = self.composer.read(cx).content();
        if text.trim().is_empty() || self.streaming.is_some() {
            return;
        }
        let Some(conversation_id) = self.active_id.clone() else {
            return;
        };
        self.error = None;
        self.composer.update(cx, |input, cx| input.clear(cx));
        let request_id = uuid::Uuid::new_v4().to_string();
        let cancel = CancellationToken::new();
        self.streaming = Some(Streaming {
            request_id: request_id.clone(),
            conversation_id: conversation_id.clone(),
            text: String::new(),
            cancel: cancel.clone(),
        });
        self.auto_follow = true;
        self.scroll_to_bottom();
        self.services
            .send_and_generate(conversation_id, request_id, text, cancel);
        cx.notify();
    }

    pub(crate) fn cancel_generation(&mut self, cx: &mut Context<Self>) {
        if let Some(streaming) = &self.streaming {
            streaming.cancel.cancel();
            cx.notify();
        }
    }

    pub(crate) fn retry_generation(&mut self, cx: &mut Context<Self>) {
        let Some(conversation_id) = self.active_id.clone() else {
            return;
        };
        if self.streaming.is_some() {
            return;
        }
        let request_id = uuid::Uuid::new_v4().to_string();
        let cancel = CancellationToken::new();
        self.error = None;
        self.streaming = Some(Streaming {
            request_id: request_id.clone(),
            conversation_id: conversation_id.clone(),
            text: String::new(),
            cancel: cancel.clone(),
        });
        self.auto_follow = true;
        self.services
            .retry_generation(conversation_id, request_id, cancel);
        cx.notify();
    }

    pub(crate) fn select_conversation(&mut self, id: String, cx: &mut Context<Self>) {
        if self.active_id.as_deref() == Some(id.as_str()) {
            return;
        }
        if let Some(streaming) = self.streaming.take() {
            streaming.cancel.cancel();
        }
        self.active_id = Some(id.clone());
        self.messages.clear();
        self.loading_messages = true;
        self.error = None;
        self.auto_follow = true;
        self.services.load_messages(id);
        cx.notify();
    }

    pub(crate) fn new_conversation_action(
        &mut self,
        _: &NewConversationAction,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.modal == Modal::None {
            self.open_new_conversation(cx);
        }
    }

    pub(crate) fn open_settings_action(
        &mut self,
        _: &OpenSettingsAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.modal == Modal::None {
            self.open_settings(window, cx);
        }
    }

    pub(crate) fn close_modal_action(
        &mut self,
        _: &CloseModalAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.modal != Modal::None {
            self.modal = Modal::None;
            window.focus(&self.composer_focus(cx));
            cx.notify();
        }
    }

    pub(crate) fn open_new_conversation(&mut self, cx: &mut Context<Self>) {
        let character_id = self
            .active_character()
            .map(|c| c.id.clone())
            .or_else(|| self.characters.first().map(|c| c.id.clone()));
        self.new_form.character_id = character_id;
        self.new_form.mode = "story".into();
        self.new_form.saving = false;
        self.new_form.title.update(cx, |input, cx| input.clear(cx));
        self.new_form
            .scenario
            .update(cx, |input, cx| input.clear(cx));
        self.modal = Modal::NewConversation;
        cx.notify();
    }

    pub(crate) fn open_character(
        &mut self,
        id: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let character = id
            .as_deref()
            .and_then(|id| self.characters.iter().find(|c| c.id == id))
            .cloned();
        let get = |value: &str| value.to_string();
        let (
            name,
            description,
            personality,
            speech,
            first,
            relationship,
            role,
            example,
            extra,
            accent,
        ) = match &character {
            Some(c) => (
                get(&c.name),
                get(&c.description),
                get(&c.personality),
                get(&c.speech_style),
                get(&c.first_person),
                get(&c.relationship),
                get(&c.role),
                get(&c.example_dialogue),
                get(&c.extra),
                c.accent.clone(),
            ),
            None => (
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                "#7d6f5c".into(),
            ),
        };
        self.char_form.id = id;
        self.char_form.accent = accent;
        self.char_form.saving = false;
        let fields = [
            (&self.char_form.name, name),
            (&self.char_form.description, description),
            (&self.char_form.personality, personality),
            (&self.char_form.speech_style, speech),
            (&self.char_form.first_person, first),
            (&self.char_form.relationship, relationship),
            (&self.char_form.role, role),
            (&self.char_form.example_dialogue, example),
            (&self.char_form.extra, extra),
        ];
        for (entity, value) in fields {
            entity.update(cx, |input, cx| input.set_content(&value, cx));
        }
        self.modal = Modal::Character;
        window.focus(&self.char_form.name.read(cx).focus_handle(cx));
        cx.notify();
    }

    pub(crate) fn open_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.settings_form.saving_reset(cx);
        let s = self.settings.clone();
        self.settings_form
            .base_url
            .update(cx, |input, cx| input.set_content(&s.base_url, cx));
        self.settings_form
            .api_key
            .update(cx, |input, cx| input.clear(cx));
        self.settings_form
            .model
            .update(cx, |input, cx| input.set_content(&s.model, cx));
        self.settings_form.temperature.update(cx, |input, cx| {
            input.set_content(&format!("{:.2}", s.temperature), cx)
        });
        self.settings_form.max_tokens.update(cx, |input, cx| {
            input.set_content(&s.max_tokens.to_string(), cx)
        });
        self.settings_form.context_budget.update(cx, |input, cx| {
            input.set_content(&s.context_budget.to_string(), cx)
        });
        self.settings_form.show_metrics = s.show_metrics;
        self.settings_form.key_set = self.services.api_key().is_some();
        self.settings_form.status = None;
        self.settings_form.models.clear();
        self.modal = Modal::Settings;
        window.focus(&self.settings_form.base_url.read(cx).focus_handle(cx));
        cx.notify();
    }

    pub(crate) fn open_scene(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(conversation) = self.active_conversation() else {
            return;
        };
        let conversation = conversation.clone();
        self.scene_form.conversation_id = Some(conversation.id.clone());
        self.scene_form.saving = false;
        self.scene_form
            .title
            .update(cx, |input, cx| input.set_content(&conversation.title, cx));
        self.scene_form.scenario.update(cx, |input, cx| {
            input.set_content(&conversation.scenario, cx)
        });
        self.scene_form
            .scene
            .update(cx, |input, cx| input.set_content(&conversation.scene, cx));
        self.scene_form
            .notes
            .update(cx, |input, cx| input.set_content(&conversation.notes, cx));
        self.modal = Modal::Scene;
        window.focus(&self.scene_form.title.read(cx).focus_handle(cx));
        cx.notify();
    }

    // ---------- mutations ----------

    pub(crate) fn save_character(&mut self, cx: &mut Context<Self>) {
        if self.char_form.saving {
            return;
        }
        let name = self.char_form.name.read(cx).content();
        if name.trim().is_empty() {
            self.notify("名前を入力してください", cx);
            return;
        }
        let form = &self.char_form;
        let input = CharacterInput {
            name: name.trim().to_string(),
            description: form.description.read(cx).content(),
            personality: form.personality.read(cx).content(),
            speech_style: form.speech_style.read(cx).content(),
            first_person: form.first_person.read(cx).content(),
            relationship: form.relationship.read(cx).content(),
            role: form.role.read(cx).content(),
            example_dialogue: form.example_dialogue.read(cx).content(),
            extra: form.extra.read(cx).content(),
            accent: form.accent.clone(),
        };
        self.char_form.saving = true;
        self.services
            .save_character(self.char_form.id.clone(), input);
        cx.notify();
    }

    pub(crate) fn delete_character(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.char_form.id.clone() else {
            return;
        };
        let name = self.char_form.name.read(cx).content();
        let answer = window.prompt(
            PromptLevel::Warning,
            &format!("「{name}」を削除しますか？\nこのキャラクターとの会話は残ります。"),
            None,
            &["削除", "キャンセル"],
            cx,
        );
        let services = self.services.clone();
        cx.spawn(async move |this, cx| {
            if let Ok(0) = answer.await {
                services.delete_character(id.clone());
                let _ = this.update(cx, |app, cx| {
                    app.modal = Modal::None;
                    cx.notify();
                });
            }
        })
        .detach();
    }

    pub(crate) fn save_new_conversation(&mut self, cx: &mut Context<Self>) {
        if self.new_form.saving {
            return;
        }
        self.new_form.saving = true;
        let input = ConversationInput {
            character_id: self.new_form.character_id.clone(),
            title: self.new_form.title.read(cx).content(),
            scenario: self.new_form.scenario.read(cx).content(),
            scene: String::new(),
            mode: Some(self.new_form.mode.clone()),
            notes: String::new(),
        };
        self.services.create_conversation(input);
        cx.notify();
    }

    pub(crate) fn save_settings(&mut self, cx: &mut Context<Self>) {
        if self.settings_form.busy {
            return;
        }
        let base_url = self.settings_form.base_url.read(cx).content();
        if base_url.trim().is_empty() {
            self.notify("エンドポイントを入力してください", cx);
            return;
        }
        let model = self.settings_form.model.read(cx).content();
        let temperature = self
            .settings_form
            .temperature
            .read(cx)
            .content()
            .trim()
            .parse::<f64>()
            .unwrap_or(self.settings.temperature);
        let max_tokens = self
            .settings_form
            .max_tokens
            .read(cx)
            .content()
            .trim()
            .parse::<u32>()
            .unwrap_or(self.settings.max_tokens);
        let context_budget = self
            .settings_form
            .context_budget
            .read(cx)
            .content()
            .trim()
            .parse::<u32>()
            .unwrap_or(self.settings.context_budget);
        let mut settings = self.settings.clone();
        settings.base_url = base_url;
        settings.model = model;
        settings.temperature = temperature;
        settings.max_tokens = max_tokens;
        settings.context_budget = context_budget;
        settings.show_metrics = self.settings_form.show_metrics;
        settings.normalize();
        let api_key = self.settings_form.api_key.read(cx).content();
        if !api_key.trim().is_empty() {
            self.services.set_api_key(Some(api_key));
            self.settings_form.key_set = true;
        }
        self.settings_form.busy = true;
        self.settings_form
            .model
            .update(cx, |input, cx| input.set_content(&settings.model, cx));
        self.services.save_settings(settings);
        cx.notify();
    }

    pub(crate) fn clear_api_key(&mut self, cx: &mut Context<Self>) {
        self.services.set_api_key(None);
        self.settings_form.key_set = false;
        self.settings_form
            .api_key
            .update(cx, |input, cx| input.clear(cx));
        self.notify("APIキーをこのセッションから消去しました", cx);
    }

    pub(crate) fn fetch_models(&mut self, cx: &mut Context<Self>) {
        if self.settings_form.busy {
            return;
        }
        let base_url = self.settings_form.base_url.read(cx).content();
        let api_key = self.settings_form.api_key.read(cx).content();
        self.settings_form.busy = true;
        self.settings_form.status = Some((true, "接続を確認しています…".into()));
        self.services.fetch_models(
            base_url,
            if api_key.trim().is_empty() {
                None
            } else {
                Some(api_key)
            },
        );
        cx.notify();
    }

    pub(crate) fn save_scene(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.scene_form.conversation_id.clone() else {
            return;
        };
        if self.scene_form.saving {
            return;
        }
        self.scene_form.saving = true;
        let title = self.scene_form.title.read(cx).content();
        let patch = ConversationUpdate {
            title: if title.trim().is_empty() {
                None
            } else {
                Some(title)
            },
            scenario: Some(self.scene_form.scenario.read(cx).content()),
            scene: Some(self.scene_form.scene.read(cx).content()),
            notes: Some(self.scene_form.notes.read(cx).content()),
            mode: None,
        };
        self.services.update_conversation(id, patch);
        cx.notify();
    }

    pub(crate) fn set_mode(&mut self, mode: &str, cx: &mut Context<Self>) {
        let Some(conversation) = self.active_conversation().cloned() else {
            return;
        };
        if conversation.mode == mode {
            return;
        }
        if let Some(entry) = self
            .conversations
            .iter_mut()
            .find(|c| c.id == conversation.id)
        {
            entry.mode = mode.to_string();
        }
        self.services.update_conversation(
            conversation.id,
            ConversationUpdate {
                mode: Some(mode.to_string()),
                ..Default::default()
            },
        );
        cx.notify();
    }

    pub(crate) fn confirm_delete_conversation(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(conversation) = self.active_conversation().cloned() else {
            return;
        };
        let answer = window.prompt(
            PromptLevel::Warning,
            &format!(
                "「{}」を削除しますか？\nこの会話のメッセージも削除されます。",
                conversation.title
            ),
            None,
            &["削除", "キャンセル"],
            cx,
        );
        let services = self.services.clone();
        let id = conversation.id.clone();
        cx.spawn(async move |this, cx| {
            if let Ok(0) = answer.await {
                services.delete_conversation(id.clone());
                let _ = this.update(cx, |_, cx| cx.notify());
            }
        })
        .detach();
    }

    fn handle_deleted_conversation(&mut self, id: String, cx: &mut Context<Self>) {
        self.conversations.retain(|c| c.id != id);
        if self.active_id.as_deref() == Some(id.as_str()) {
            self.active_id = None;
            self.messages.clear();
            if let Some(next) = self.conversations.first().map(|c| c.id.clone()) {
                self.select_conversation(next, cx);
            }
        }
        if self.modal == Modal::Scene {
            self.modal = Modal::None;
        }
        cx.notify();
    }

    pub(crate) fn close_panel(&mut self, cx: &mut Context<Self>) {
        self.panel_open = false;
        cx.notify();
    }

    pub(crate) fn toggle_panel(&mut self, cx: &mut Context<Self>) {
        self.panel_open = !self.panel_open;
        cx.notify();
    }

    // ---------- events ----------

    pub(crate) fn handle_events(
        &mut self,
        events: Vec<UiEvent>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let follow = self.near_bottom() || self.auto_follow;
        for event in events {
            self.handle_event(event, window, cx);
        }
        if follow {
            self.scroll_to_bottom();
        }
        cx.notify();
    }

    fn handle_event(&mut self, event: UiEvent, window: &mut Window, cx: &mut Context<Self>) {
        match event {
            UiEvent::ConversationLoaded {
                conversation_id,
                messages,
            } => {
                if self.active_id.as_deref() == Some(conversation_id.as_str()) {
                    self.messages = messages;
                    self.loading_messages = false;
                    self.auto_follow = true;
                    self.scroll_to_bottom();
                }
            }
            UiEvent::ConversationCreated(conversation) => {
                self.new_form.saving = false;
                self.conversations.retain(|c| c.id != conversation.id);
                self.conversations.push(conversation.clone());
                self.sort_conversations();
                self.active_id = Some(conversation.id.clone());
                self.messages.clear();
                self.loading_messages = false;
                self.modal = Modal::None;
                self.error = None;
                self.auto_follow = true;
            }
            UiEvent::ConversationUpdated(conversation) => {
                if let Some(entry) = self
                    .conversations
                    .iter_mut()
                    .find(|c| c.id == conversation.id)
                {
                    *entry = conversation.clone();
                } else {
                    self.conversations.push(conversation.clone());
                }
                self.scene_form.saving = false;
                if self.modal == Modal::Scene
                    && self.scene_form.conversation_id.as_deref() == Some(conversation.id.as_str())
                {
                    self.modal = Modal::None;
                }
                self.sort_conversations();
            }
            UiEvent::ConversationDeleted(id) => {
                self.handle_deleted_conversation(id, cx);
            }
            UiEvent::CharacterSaved(character) => {
                self.char_form.saving = false;
                if let Some(entry) = self.characters.iter_mut().find(|c| c.id == character.id) {
                    *entry = character.clone();
                } else {
                    self.characters.push(character.clone());
                }
                self.characters
                    .sort_by_key(|c| std::cmp::Reverse(c.updated_at));
                if self.modal == Modal::Character {
                    self.modal = Modal::None;
                    window.focus(&self.composer_focus(cx));
                }
                if self.new_form.character_id.is_none() {
                    self.new_form.character_id = Some(character.id.clone());
                }
            }
            UiEvent::CharacterDeleted(id) => {
                self.characters.retain(|c| c.id != id);
                for conversation in &mut self.conversations {
                    if conversation.character_id.as_deref() == Some(id.as_str()) {
                        conversation.character_id = None;
                    }
                }
                if self.new_form.character_id.as_deref() == Some(id.as_str()) {
                    self.new_form.character_id = None;
                }
                if self.modal == Modal::Character {
                    self.modal = Modal::None;
                }
            }
            UiEvent::UserMessageSaved(message) => {
                let conversation_id = message.conversation_id.clone();
                self.touch_conversation(&conversation_id, message.created_at);
                if self.active_id.as_deref() == Some(conversation_id.as_str()) {
                    self.messages.push(message);
                    self.auto_follow = true;
                }
            }
            UiEvent::Delta { request_id, delta } => {
                if let Some(streaming) = &mut self.streaming {
                    if streaming.request_id == request_id {
                        streaming.text.push_str(&delta);
                        let near = self.near_bottom() || self.auto_follow;
                        self.auto_follow = near;
                    }
                }
            }
            UiEvent::GenerationDone {
                request_id,
                message,
                stopped: _,
            } => {
                if self
                    .streaming
                    .as_ref()
                    .map(|s| s.request_id == request_id)
                    .unwrap_or(false)
                {
                    self.streaming = None;
                    self.auto_follow = true;
                }
                let conversation_id = message.conversation_id.clone();
                self.touch_conversation(&conversation_id, message.created_at);
                if self.active_id.as_deref() == Some(conversation_id.as_str()) {
                    self.messages.push(message);
                }
                self.error = None;
            }
            UiEvent::GenerationError {
                request_id,
                message,
                kind,
                restore_input,
                retryable,
            } => {
                let stream_conversation = self
                    .streaming
                    .as_ref()
                    .filter(|s| s.request_id == request_id)
                    .map(|s| s.conversation_id.clone());
                if stream_conversation.is_some() {
                    self.streaming = None;
                }
                if let Some(conversation_id) = stream_conversation {
                    if self.active_id.as_deref() == Some(conversation_id.as_str()) {
                        self.services.load_messages(conversation_id);
                    }
                }
                if let Some(text) = restore_input {
                    if self.composer.read(cx).is_empty() {
                        self.composer
                            .update(cx, |input, cx| input.set_content(&text, cx));
                    }
                }
                self.error = Some(AppError {
                    message,
                    kind,
                    retryable,
                });
            }
            UiEvent::SettingsSaved(settings) => {
                self.settings = settings;
                self.settings_form.busy = false;
                self.settings_form.status = Some((true, "保存しました".into()));
                self.modal = Modal::None;
                window.focus(&self.composer_focus(cx));
            }
            UiEvent::ModelsFetched { models, error } => {
                self.settings_form.busy = false;
                match error {
                    Some(error) => {
                        self.settings_form.status = Some((false, error));
                    }
                    None => {
                        let message = if models.is_empty() {
                            "接続できました。モデル一覧は空です。名前を直接入力してください。"
                                .to_string()
                        } else {
                            format!(
                                "接続できました。{} 個のモデルが見つかりました。",
                                models.len()
                            )
                        };
                        self.settings_form.status = Some((true, message));
                        self.settings_form.models = models;
                    }
                }
            }
            UiEvent::Error(message) => {
                self.char_form.saving = false;
                self.new_form.saving = false;
                self.scene_form.saving = false;
                self.settings_form.busy = false;
                self.notify(message, cx);
            }
        }
    }
}

impl SettingsForm {
    fn saving_reset(&mut self, cx: &mut Context<HomerApp>) {
        self.busy = false;
        self.status = None;
        self.models.clear();
        self.api_key.update(cx, |input, cx| input.clear(cx));
    }
}

impl Drop for HomerApp {
    fn drop(&mut self) {
        if let Some(streaming) = &self.streaming {
            streaming.cancel.cancel();
        }
    }
}

// Re-export for homer-app key binding setup.
pub use crate::text_area::{
    InputBackspace, InputCopy, InputCut, InputDelete, InputEnd, InputHome, InputLeft, InputNewline,
    InputPaste, InputRight, InputSelectAll, InputSelectLeft, InputSelectRight,
    InputShowCharacterPalette,
};
use gpui::KeyBinding;

pub fn bind_keys(cx: &mut App) {
    let bindings = [
        KeyBinding::new("cmd-n", NewConversationAction, None),
        KeyBinding::new("cmd-,", OpenSettingsAction, None),
        KeyBinding::new("escape", CloseModalAction, None),
        KeyBinding::new("enter", SubmitMessageAction, Some("HomerComposer")),
        KeyBinding::new("cmd-enter", SubmitMessageAction, Some("HomerComposer")),
        KeyBinding::new("shift-enter", InputNewline, Some("HomerComposer")),
        KeyBinding::new("enter", InputNewline, Some("HomerMultiline")),
        KeyBinding::new("backspace", InputBackspace, Some("HomerInput")),
        KeyBinding::new("delete", InputDelete, Some("HomerInput")),
        KeyBinding::new("left", InputLeft, Some("HomerInput")),
        KeyBinding::new("right", InputRight, Some("HomerInput")),
        KeyBinding::new("shift-left", InputSelectLeft, Some("HomerInput")),
        KeyBinding::new("shift-right", InputSelectRight, Some("HomerInput")),
        KeyBinding::new("cmd-a", InputSelectAll, Some("HomerInput")),
        KeyBinding::new("home", InputHome, Some("HomerInput")),
        KeyBinding::new("end", InputEnd, Some("HomerInput")),
        KeyBinding::new("cmd-v", InputPaste, Some("HomerInput")),
        KeyBinding::new("cmd-c", InputCopy, Some("HomerInput")),
        KeyBinding::new("cmd-x", InputCut, Some("HomerInput")),
        KeyBinding::new(
            "ctrl-cmd-space",
            InputShowCharacterPalette,
            Some("HomerInput"),
        ),
    ];
    cx.bind_keys(bindings);
}

/// Kept public so the app crate can construct the window with a stable title.
pub const WINDOW_TITLE: &str = "Homer";

pub fn window_size() -> (f32, f32) {
    (1180., 800.)
}

pub fn minimum_window_size() -> (f32, f32) {
    (1000., 620.)
}
