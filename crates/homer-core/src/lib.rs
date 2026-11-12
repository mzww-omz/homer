//! Shared domain model, prompt compiler, and display segmentation for Homer.
//!
//! This crate intentionally has no UI or storage dependencies so it can be reused
//! by a future CLI or test harness.

pub mod models;
pub mod prompt;
pub mod segments;

pub use models::*;
pub use prompt::{compile, estimate_tokens, ChatMessage, CompiledPrompt, PromptInput};
pub use segments::{segment_text, visible_segments, Segment, SegmentType};

/// Normalize an OpenAI-compatible base URL.
pub fn normalize_base_url(raw: &str) -> String {
    let mut url = raw.trim().trim_end_matches('/').to_string();
    if url.is_empty() {
        url = "http://localhost:1234/v1".into();
    }
    if !url.contains("://") {
        url = format!("http://{url}");
    }
    let after_scheme = url.split("://").nth(1).unwrap_or("");
    if !after_scheme.contains('/') {
        url.push_str("/v1");
    }
    url
}

#[cfg(test)]
mod tests {
    use super::normalize_base_url;

    #[test]
    fn normalizes_base_urls() {
        assert_eq!(
            normalize_base_url("http://localhost:1234"),
            "http://localhost:1234/v1"
        );
        assert_eq!(
            normalize_base_url("http://localhost:1234/v1/"),
            "http://localhost:1234/v1"
        );
        assert_eq!(normalize_base_url(""), "http://localhost:1234/v1");
    }
}
