use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Character {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub personality: String,
    #[serde(default)]
    pub speech_style: String,
    #[serde(default)]
    pub first_person: String,
    #[serde(default)]
    pub relationship: String,
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub example_dialogue: String,
    #[serde(default)]
    pub extra: String,
    #[serde(default)]
    pub accent: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CharacterInput {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub personality: String,
    #[serde(default)]
    pub speech_style: String,
    #[serde(default)]
    pub first_person: String,
    #[serde(default)]
    pub relationship: String,
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub example_dialogue: String,
    #[serde(default)]
    pub extra: String,
    #[serde(default)]
    pub accent: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: String,
    pub character_id: Option<String>,
    pub title: String,
    pub scenario: String,
    pub scene: String,
    pub mode: String,
    pub notes: String,
    pub created_at: i64,
    pub updated_at: i64,
}

impl Conversation {
    pub fn is_messenger(&self) -> bool {
        self.mode == "messenger"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConversationInput {
    #[serde(default)]
    pub character_id: Option<String>,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub scenario: String,
    #[serde(default)]
    pub scene: String,
    #[serde(default)]
    pub mode: Option<String>,
    #[serde(default)]
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConversationUpdate {
    pub title: Option<String>,
    pub scenario: Option<String>,
    pub scene: Option<String>,
    pub mode: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MessageMeta {
    #[serde(default)]
    pub metrics: Option<GenMetrics>,
    #[serde(default)]
    pub stopped: bool,
    #[serde(default)]
    pub incomplete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub conversation_id: String,
    pub role: String,
    pub content: String,
    pub created_at: i64,
    pub meta: Option<MessageMeta>,
}

impl Message {
    pub fn is_user(&self) -> bool {
        self.role == "user"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub base_url: String,
    pub model: String,
    pub temperature: f64,
    pub top_p: f64,
    pub max_tokens: u32,
    pub context_budget: u32,
    pub timeout_secs: u64,
    pub show_metrics: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:1234/v1".into(),
            model: String::new(),
            temperature: 0.85,
            top_p: 0.95,
            max_tokens: 400,
            context_budget: 6000,
            timeout_secs: 120,
            show_metrics: false,
        }
    }
}

impl Settings {
    pub fn normalize(&mut self) {
        self.base_url = crate::normalize_base_url(&self.base_url);
        self.model = self.model.trim().to_string();
        self.temperature = self.temperature.clamp(0.0, 2.0);
        self.top_p = self.top_p.clamp(0.0, 1.0);
        self.max_tokens = self.max_tokens.clamp(32, 4096);
        self.context_budget = self.context_budget.clamp(1024, 32000);
        self.timeout_secs = self.timeout_secs.clamp(10, 600);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SettingsPatch {
    pub base_url: Option<String>,
    pub model: Option<String>,
    pub temperature: Option<f64>,
    pub top_p: Option<f64>,
    pub max_tokens: Option<u32>,
    pub context_budget: Option<u32>,
    pub timeout_secs: Option<u64>,
    pub show_metrics: Option<bool>,
}

impl Settings {
    pub fn apply(&mut self, patch: SettingsPatch) {
        if let Some(v) = patch.base_url {
            self.base_url = v;
        }
        if let Some(v) = patch.model {
            self.model = v;
        }
        if let Some(v) = patch.temperature {
            self.temperature = v;
        }
        if let Some(v) = patch.top_p {
            self.top_p = v;
        }
        if let Some(v) = patch.max_tokens {
            self.max_tokens = v;
        }
        if let Some(v) = patch.context_budget {
            self.context_budget = v;
        }
        if let Some(v) = patch.timeout_secs {
            self.timeout_secs = v;
        }
        if let Some(v) = patch.show_metrics {
            self.show_metrics = v;
        }
        self.normalize();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct GenMetrics {
    pub ttft_ms: u64,
    pub duration_ms: u64,
    pub chars: usize,
    pub estimated_tokens: u32,
    pub tokens_per_sec: f64,
    pub context_tokens: u32,
    pub trimmed_messages: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionInfo {
    pub ok: bool,
    pub models: Vec<String>,
    pub message: String,
}
