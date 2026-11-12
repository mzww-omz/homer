use futures_util::StreamExt;
use serde_json::{json, Value};
use std::time::Duration;
use tokio_util::sync::CancellationToken;

use homer_core::normalize_base_url;

fn error_body_snippet(body: &str) -> String {
    let trimmed = body.trim();
    let snippet: String = trimmed.chars().take(400).collect();
    if snippet.is_empty() {
        "（本文なし）".into()
    } else {
        snippet
    }
}

pub fn http_error_message(status: u16, body: &str) -> String {
    let detail = error_body_snippet(body);
    match status {
        401 | 403 => format!(
            "認証エラー（HTTP {status}）。APIキーが必要なサーバーかもしれません。詳細: {detail}"
        ),
        404 => format!(
            "接続先が見つかりません（HTTP 404）。ベースURLに /v1 が含まれているか、モデル名が正しいか確認してください。詳細: {detail}"
        ),
        400 => format!(
            "リクエストが拒否されました（HTTP 400）。モデル名やコンテキスト長を確認してください。詳細: {detail}"
        ),
        429 => format!("サーバーが混み合っています（HTTP 429）。少し待って再試行してください。詳細: {detail}"),
        500..=599 => format!(
            "推論サーバーでエラーが発生しました（HTTP {status}）。モデルが読み込まれているか確認してください。詳細: {detail}"
        ),
        _ => format!("推論サーバーがエラーを返しました（HTTP {status}）。詳細: {detail}"),
    }
}

pub fn network_error_message(err: &reqwest::Error) -> String {
    if err.is_timeout() {
        "接続がタイムアウトしました。サーバーの状態、モデルの読み込み、設定のタイムアウト値を確認してください。".into()
    } else if err.is_connect() {
        "推論サーバーに接続できません。LM Studio または Ollama が起動しているか、設定のエンドポイント（例: http://localhost:1234/v1）を確認してください。".into()
    } else {
        format!("通信に失敗しました: {err}")
    }
}

pub fn build_client(timeout_secs: u64) -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout_secs.max(5)))
        .connect_timeout(Duration::from_secs(8))
        .build()
        .map_err(|e| format!("HTTPクライアントを作成できません: {e}"))
}

fn request_builder(
    client: &reqwest::Client,
    url: &str,
    api_key: Option<&str>,
) -> reqwest::RequestBuilder {
    let mut req = client.get(url);
    if let Some(key) = api_key {
        if !key.trim().is_empty() {
            req = req.bearer_auth(key.trim());
        }
    }
    req
}

pub async fn list_models(
    client: &reqwest::Client,
    base_url: &str,
    api_key: Option<&str>,
) -> Result<Vec<String>, String> {
    let url = format!("{}/models", normalize_base_url(base_url));
    let req = request_builder(client, &url, api_key);
    let resp = req.send().await.map_err(|e| network_error_message(&e))?;
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(http_error_message(status.as_u16(), &body));
    }
    let value: Value = resp
        .json()
        .await
        .map_err(|e| format!("モデル一覧を解析できません: {e}"))?;
    let mut models: Vec<String> = value
        .get("data")
        .and_then(|v| v.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.get("id").and_then(|id| id.as_str()))
                .map(|s| s.to_string())
                .collect()
        })
        .unwrap_or_default();
    models.sort_by_key(|a| a.to_lowercase());
    models.dedup();
    Ok(models)
}

#[derive(Debug, Clone, PartialEq)]
pub enum StreamEnd {
    Complete,
    Canceled,
}

#[derive(Debug, Clone)]
pub struct StreamResult {
    pub text: String,
    pub end: StreamEnd,
}

#[derive(Debug, Clone)]
pub struct StreamFailure {
    pub message: String,
    pub kind: String,
    pub partial: String,
}

#[derive(Debug, Clone, PartialEq)]
enum FailureKind {
    Network,
    Http,
    Protocol,
}

impl FailureKind {
    fn as_str(&self) -> &'static str {
        match self {
            FailureKind::Network => "network",
            FailureKind::Http => "http",
            FailureKind::Protocol => "protocol",
        }
    }
}

fn failure(kind: FailureKind, message: impl Into<String>, partial: &str) -> StreamFailure {
    StreamFailure {
        message: message.into(),
        kind: kind.as_str().into(),
        partial: partial.to_string(),
    }
}

/// Incremental SSE / data-line decoder. Keeps incomplete lines between chunks.
#[derive(Default)]
pub struct SseBuffer {
    buffer: String,
    done: bool,
}

impl SseBuffer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn feed(&mut self, chunk: &str) -> Vec<String> {
        self.buffer.push_str(chunk);
        let mut events = Vec::new();
        while let Some(pos) = self.buffer.find('\n') {
            let mut line = self.buffer[..pos].to_string();
            self.buffer.drain(..=pos);
            if line.ends_with('\r') {
                line.pop();
            }
            let Some(payload) = line.strip_prefix("data:") else {
                continue;
            };
            let payload = payload.trim();
            if payload.is_empty() {
                continue;
            }
            if payload == "[DONE]" {
                self.done = true;
                break;
            }
            events.push(payload.to_string());
        }
        events
    }

    /// Flush a trailing line when the server ends the stream without a newline.
    pub fn finish(&mut self) -> Vec<String> {
        let mut events = Vec::new();
        if !self.buffer.trim().is_empty() {
            let line = std::mem::take(&mut self.buffer);
            if let Some(payload) = line.trim().strip_prefix("data:") {
                let payload = payload.trim();
                if !payload.is_empty() && payload != "[DONE]" {
                    events.push(payload.to_string());
                }
            }
        }
        events
    }

    pub fn is_done(&self) -> bool {
        self.done
    }
}

pub fn extract_delta(payload: &str) -> Option<String> {
    let value: Value = serde_json::from_str(payload).ok()?;
    if value.get("error").is_some() {
        return None;
    }
    let choice = value.get("choices")?.get(0)?;
    let delta = choice.get("delta")?;
    let content = delta.get("content")?;
    match content {
        Value::String(s) if !s.is_empty() => Some(s.clone()),
        _ => None,
    }
}

pub fn extract_stream_error(payload: &str) -> Option<String> {
    let value: Value = serde_json::from_str(payload).ok()?;
    let err = value.get("error")?;
    Some(
        err.get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("推論サーバーがストリーム中にエラーを返しました")
            .to_string(),
    )
}

/// Non-streaming fallback: pull the assistant text out of a normal completion body.
fn extract_full_completion(payload: &str) -> Option<String> {
    let value: Value = serde_json::from_str(payload).ok()?;
    let choice = value.get("choices")?.get(0)?;
    choice
        .get("message")
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .or_else(|| choice.get("text").and_then(|t| t.as_str()))
        .map(|s| s.to_string())
}

pub struct ChatRequest<'a> {
    pub base_url: &'a str,
    pub api_key: Option<&'a str>,
    pub model: &'a str,
    pub messages: &'a [homer_core::ChatMessage],
    pub temperature: f64,
    pub top_p: f64,
    pub max_tokens: u32,
}

pub async fn stream_chat<F>(
    client: &reqwest::Client,
    request: ChatRequest<'_>,
    cancel: CancellationToken,
    mut on_delta: F,
) -> Result<StreamResult, StreamFailure>
where
    F: FnMut(&str),
{
    let model = request.model.trim();
    if model.is_empty() {
        return Err(failure(
            FailureKind::Protocol,
            "モデルが選択されていません。設定画面でモデルを選択してください。",
            "",
        ));
    }
    let url = format!("{}/chat/completions", normalize_base_url(request.base_url));
    let body = json!({
        "model": model,
        "messages": request.messages,
        "stream": true,
        "temperature": request.temperature,
        "top_p": request.top_p,
        "max_tokens": request.max_tokens,
    });
    let mut req = client.post(&url).json(&body);
    if let Some(key) = request.api_key {
        if !key.trim().is_empty() {
            req = req.bearer_auth(key.trim());
        }
    }

    let resp = match req.send().await {
        Ok(r) => r,
        Err(e) => {
            return Err(failure(FailureKind::Network, network_error_message(&e), ""));
        }
    };

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(failure(
            FailureKind::Http,
            http_error_message(status.as_u16(), &body),
            "",
        ));
    }

    let content_type = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    if content_type.contains("application/json") {
        // Some servers ignore stream=true and answer with a single JSON body.
        let body = match resp.text().await {
            Ok(t) => t,
            Err(e) => return Err(failure(FailureKind::Network, network_error_message(&e), "")),
        };
        let text = extract_full_completion(&body).unwrap_or_default();
        if text.is_empty() {
            return Err(failure(
                FailureKind::Protocol,
                "応答を解析できませんでした。サーバーの応答形式を確認してください。".to_string(),
                "",
            ));
        }
        on_delta(&text);
        return Ok(StreamResult {
            text,
            end: StreamEnd::Complete,
        });
    }

    let mut stream = resp.bytes_stream();
    let mut decoder = SseBuffer::new();
    let mut text = String::new();

    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                return Ok(StreamResult { text, end: StreamEnd::Canceled });
            }
            chunk = tokio::time::timeout(Duration::from_secs(45), stream.next()) => {
                let chunk = match chunk {
                    Err(_) => {
                        return Err(failure(
                            FailureKind::Network,
                            "応答が途切れました（45秒間データが届きませんでした）。モデルの状態を確認してください。",
                            &text,
                        ));
                    }
                    Ok(None) => break,
                    Ok(Some(Err(e))) => {
                        return Err(failure(
                            FailureKind::Network,
                            format!("受信中に通信エラーが発生しました: {e}"),
                            &text,
                        ));
                    }
                    Ok(Some(Ok(bytes))) => bytes,
                };
                let chunk = String::from_utf8_lossy(&chunk);
                let events = decoder.feed(&chunk);
                for event in events {
                    if let Some(err) = extract_stream_error(&event) {
                        return Err(failure(FailureKind::Protocol, err, &text));
                    }
                    if let Some(delta) = extract_delta(&event) {
                        text.push_str(&delta);
                        on_delta(&delta);
                    }
                }
                if decoder.is_done() {
                    break;
                }
            }
        }
    }

    for event in decoder.finish() {
        if let Some(delta) = extract_delta(&event) {
            text.push_str(&delta);
            on_delta(&delta);
        }
    }

    Ok(StreamResult {
        text,
        end: StreamEnd::Complete,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    #[test]
    fn sse_decoder_handles_split_chunks() {
        let mut d = SseBuffer::new();
        assert!(d.feed("data: {\"a\":").is_empty());
        let events = d.feed("1}\n\ndata: {\"b\":2}\n");
        assert_eq!(events, vec!["{\"a\":1}", "{\"b\":2}"]);
        assert!(!d.is_done());
        assert!(d.feed("data: [DONE]\n").is_empty());
        assert!(d.is_done());
    }

    #[test]
    fn sse_decoder_handles_crlf_and_comments() {
        let mut d = SseBuffer::new();
        let events = d.feed(": keepalive\r\ndata: {\"x\":1}\r\n\r\n");
        assert_eq!(events, vec!["{\"x\":1}"]);
    }

    #[test]
    fn extracts_openai_delta() {
        let payload = r#"{"choices":[{"delta":{"content":"こん"}}]}"#;
        assert_eq!(extract_delta(payload), Some("こん".into()));
        assert_eq!(extract_delta(r#"{"choices":[{"delta":{}}]}"#), None);
        assert_eq!(
            extract_delta(r#"{"choices":[{"delta":{"role":"assistant"}}]}"#),
            None
        );
        assert_eq!(extract_delta("broken"), None);
    }

    #[test]
    fn extracts_stream_error() {
        let payload = r#"{"error":{"message":"model not found"}}"#;
        assert_eq!(
            extract_stream_error(payload),
            Some("model not found".into())
        );
        assert_eq!(extract_stream_error(r#"{"choices":[]}"#), None);
    }

    #[test]
    fn http_errors_are_actionable() {
        assert!(http_error_message(404, "nope").contains("/v1"));
        assert!(http_error_message(401, "").contains("APIキー"));
        assert!(http_error_message(503, "loading").contains("503"));
    }

    async fn spawn_server(
        status: &str,
        content_type: &str,
        body_chunks: Vec<(u64, String)>,
    ) -> (String, tokio::task::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let status = status.to_string();
        let content_type = content_type.to_string();
        let handle = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 4096];
            let _ = socket.read(&mut buf).await;
            let header = format!(
                "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nConnection: close\r\n\r\n"
            );
            socket.write_all(header.as_bytes()).await.unwrap();
            for (delay, chunk) in body_chunks {
                if delay > 0 {
                    tokio::time::sleep(Duration::from_millis(delay)).await;
                }
                socket.write_all(chunk.as_bytes()).await.unwrap();
                socket.flush().await.unwrap();
            }
        });
        (format!("http://{addr}/v1"), handle)
    }

    fn messages() -> Vec<homer_core::ChatMessage> {
        vec![homer_core::ChatMessage {
            role: "user".into(),
            content: "こんにちは".into(),
        }]
    }

    fn request<'a>(base: &'a str, messages: &'a [homer_core::ChatMessage]) -> ChatRequest<'a> {
        ChatRequest {
            base_url: base,
            api_key: None,
            model: "test-model",
            messages,
            temperature: 0.8,
            top_p: 0.9,
            max_tokens: 100,
        }
    }

    #[tokio::test]
    async fn streams_sse_deltas_in_order() {
        let (base, _h) = spawn_server(
            "200 OK",
            "text/event-stream",
            vec![
                (
                    0,
                    "data: {\"choices\":[{\"delta\":{\"content\":\"こん\"}}]}\n\n".into(),
                ),
                (
                    20,
                    "data: {\"choices\":[{\"delta\":{\"content\":\"にちは\"}}]}\n\ndata: [DONE]\n\n".into(),
                ),
            ],
        )
        .await;
        let msgs = messages();
        let client = build_client(10).unwrap();
        let mut collected = Vec::new();
        let result = stream_chat(
            &client,
            request(&base, &msgs),
            CancellationToken::new(),
            |d| collected.push(d.to_string()),
        )
        .await
        .unwrap();
        assert_eq!(result.text, "こんにちは");
        assert_eq!(collected, vec!["こん".to_string(), "にちは".to_string()]);
        assert_eq!(result.end, StreamEnd::Complete);
    }

    #[tokio::test]
    async fn cancellation_returns_partial_text() {
        let (base, _h) = spawn_server(
            "200 OK",
            "text/event-stream",
            vec![
                (
                    0,
                    "data: {\"choices\":[{\"delta\":{\"content\":\"途中\"}}]}\n\n".into(),
                ),
                (
                    500,
                    "data: {\"choices\":[{\"delta\":{\"content\":\"まで\"}}]}\n\n".into(),
                ),
            ],
        )
        .await;
        let token = CancellationToken::new();
        let cancel_handle = token.clone();
        let msgs = messages();
        let client = build_client(10).unwrap();
        let collected = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let collected_clone = collected.clone();
        let guard = tokio::spawn(async move {
            let cancel = cancel_handle.clone();
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(100)).await;
                cancel.cancel();
            });
            stream_chat(&client, request(&base, &msgs), cancel_handle, |d| {
                collected_clone.lock().unwrap().push(d.to_string());
            })
            .await
        });
        let result = guard.await.unwrap().unwrap();
        assert_eq!(result.end, StreamEnd::Canceled);
        assert_eq!(result.text, "途中");
        assert_eq!(collected.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn http_404_is_reported() {
        let (base, _h) = spawn_server(
            "404 Not Found",
            "application/json",
            vec![(0, r#"{"error":"not found"}"#.into())],
        )
        .await;
        let msgs = messages();
        let client = build_client(10).unwrap();
        let err = stream_chat(
            &client,
            request(&base, &msgs),
            CancellationToken::new(),
            |_| {},
        )
        .await
        .unwrap_err();
        assert_eq!(err.kind, "http");
        assert!(err.message.contains("404"));
    }

    #[tokio::test]
    async fn connection_failure_is_reported() {
        // Bind then drop to get a port that is very likely closed.
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);
        let base = format!("http://{addr}/v1");
        let msgs = messages();
        let client = build_client(5).unwrap();
        let err = stream_chat(
            &client,
            request(&base, &msgs),
            CancellationToken::new(),
            |_| {},
        )
        .await
        .unwrap_err();
        assert_eq!(err.kind, "network");
        assert!(err.message.contains("接続できません"));
    }

    #[tokio::test]
    async fn empty_model_is_rejected_before_request() {
        let msgs = messages();
        let client = build_client(5).unwrap();
        let err = stream_chat(
            &client,
            ChatRequest {
                model: " ",
                ..request("http://127.0.0.1:9/v1", &msgs)
            },
            CancellationToken::new(),
            |_| {},
        )
        .await
        .unwrap_err();
        assert!(err.message.contains("モデルが選択されていません"));
    }

    async fn spawn_models_server(status: &str, body: &str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let status = status.to_string();
        let body = body.to_string();
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 4096];
            let _ = socket.read(&mut buf).await;
            let response = format!(
                "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = socket.write_all(response.as_bytes()).await;
        });
        format!("http://{addr}/v1")
    }

    #[tokio::test]
    async fn lists_and_sorts_models() {
        let base = spawn_models_server(
            "200 OK",
            r#"{"object":"list","data":[{"id":"qwen"},{"id":"gemma"},{"id":"qwen"}]}"#,
        )
        .await;
        let client = build_client(10).unwrap();
        let models = list_models(&client, &base, None).await.unwrap();
        assert_eq!(models, vec!["gemma".to_string(), "qwen".to_string()]);
    }

    #[tokio::test]
    async fn model_list_connection_error_is_actionable() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);
        let base = format!("http://{addr}/v1");
        let client = build_client(5).unwrap();
        let error = list_models(&client, &base, None).await.unwrap_err();
        assert!(error.contains("接続できません"));
    }

    #[tokio::test]
    async fn model_list_http_error_includes_status() {
        let base = spawn_models_server("404 Not Found", r#"{"error":"not found"}"#).await;
        let client = build_client(10).unwrap();
        let error = list_models(&client, &base, None).await.unwrap_err();
        assert!(error.contains("404"));
        assert!(error.contains("/v1"));
    }

    #[tokio::test]
    async fn json_fallback_response_works() {
        let (base, _h) = spawn_server(
            "200 OK",
            "application/json",
            vec![(
                0,
                r#"{"choices":[{"message":{"content":"フォールバック"}}]}"#.into(),
            )],
        )
        .await;
        let msgs = messages();
        let client = build_client(10).unwrap();
        let result = stream_chat(
            &client,
            request(&base, &msgs),
            CancellationToken::new(),
            |_| {},
        )
        .await
        .unwrap();
        assert_eq!(result.text, "フォールバック");
    }
}
