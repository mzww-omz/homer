//! OpenAI-compatible inference adapter (LM Studio, Ollama, and custom servers).
//!
//! Public API is intentionally independent of GPUI so it can be used from a CLI
//! or tests. Streaming updates are delivered through a callback, cancellation is
//! cooperative through `tokio_util::sync::CancellationToken`.

mod llm;

pub use llm::{
    build_client, extract_delta, extract_stream_error, http_error_message, list_models,
    network_error_message, stream_chat, ChatRequest, SseBuffer, StreamEnd, StreamFailure,
    StreamResult,
};
