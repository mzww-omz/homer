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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationUpdate {
    pub title: Option<String>,
    pub scenario: Option<String>,
    pub scene: Option<String>,
    pub mode: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub conversation_id: String,
    pub role: String,
    pub content: String,
    pub created_at: i64,
    pub meta: Option<serde_json::Value>,
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

#[derive(Debug, Clone, Serialize)]
pub struct ConnectionInfo {
    pub ok: bool,
    pub models: Vec<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct GenMetrics {
    pub ttft_ms: u64,
    pub duration_ms: u64,
    pub chars: usize,
    pub estimated_tokens: u32,
    pub tokens_per_sec: f64,
    pub context_tokens: u32,
    pub trimmed_messages: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeltaPayload {
    pub request_id: String,
    pub delta: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DonePayload {
    pub request_id: String,
    pub message: Message,
    pub metrics: GenMetrics,
    pub stopped: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ErrorPayload {
    pub request_id: String,
    pub message: String,
    pub kind: String,
    pub partial: String,
}
