use std::sync::{Arc, Mutex};
use std::time::Duration;

use homer_core::{CharacterInput, ConversationInput, Settings};
use homer_storage::Storage;
use homer_ui::services::{Services, UiEvent};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;

async fn spawn_sse_server(chunks: Vec<(u64, String)>) -> (String, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = [0u8; 8192];
        let _ = socket.read(&mut buf).await;
        socket
            .write_all(
                b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n",
            )
            .await
            .unwrap();
        for (delay_ms, chunk) in chunks {
            if delay_ms > 0 {
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
            }
            if socket.write_all(chunk.as_bytes()).await.is_err() {
                return;
            }
            let _ = socket.flush().await;
        }
    });
    (format!("http://{addr}/v1"), handle)
}

fn prepare(storage: &Storage, base_url: &str) -> (String, String) {
    let character = storage
        .create_character(&CharacterInput {
            name: "七海".into(),
            description: "図書室で会う同級生".into(),
            speech_style: "落ち着いた丁寧語".into(),
            ..Default::default()
        })
        .unwrap();
    let conversation = storage
        .create_conversation(&ConversationInput {
            character_id: Some(character.id.clone()),
            scenario: "雨の日の図書室".into(),
            ..Default::default()
        })
        .unwrap();
    let settings = Settings {
        base_url: base_url.to_string(),
        model: "test-model".into(),
        max_tokens: 120,
        ..Default::default()
    };
    storage.save_settings(&settings).unwrap();
    (character.id, conversation.id)
}

fn sse(content: &str) -> String {
    format!(
        "data: {{\"choices\":[{{\"delta\":{{\"content\":{}}}}}]}}\n\n",
        serde_json::to_string(content).unwrap()
    )
}

async fn next_event(rx: &async_channel::Receiver<UiEvent>) -> UiEvent {
    tokio::time::timeout(Duration::from_secs(8), rx.recv())
        .await
        .expect("timed out waiting for UI event")
        .expect("channel closed")
}

#[tokio::test]
async fn send_generates_streams_and_persists_reply() {
    let (base, _server) = spawn_sse_server(vec![
        (0, sse("「また来たんだ。")),
        (10, sse("……別に、嫌じゃないけど」") + "data: [DONE]\n\n"),
    ])
    .await;
    let storage = Storage::open_in_memory().unwrap();
    let (_character_id, conversation_id) = prepare(&storage, &base);
    let (tx, rx) = Services::channel();
    let services = Services {
        storage: Arc::new(storage),
        runtime: tokio::runtime::Handle::current(),
        tx,
        api_key: Arc::new(Mutex::new(None)),
    };

    services.send_and_generate(
        conversation_id.clone(),
        "req-1".into(),
        "こんにちは".into(),
        CancellationToken::new(),
    );

    let mut streamed = String::new();
    let mut user_saved = false;
    let done = loop {
        match next_event(&rx).await {
            UiEvent::UserMessageSaved(message) => {
                user_saved = true;
                assert_eq!(message.content, "こんにちは");
            }
            UiEvent::Delta { delta, .. } => streamed.push_str(&delta),
            UiEvent::GenerationDone { message, .. } => break message,
            UiEvent::GenerationError { message, .. } => panic!("generation failed: {message}"),
            _ => {}
        }
    };

    assert!(user_saved);
    assert_eq!(streamed, "「また来たんだ。……別に、嫌じゃないけど」");
    assert_eq!(done.content, streamed);
    assert_eq!(done.role, "assistant");

    let messages = services.storage.list_messages(&conversation_id).unwrap();
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].role, "user");
    assert_eq!(messages[1].content, done.content);
    let metrics = messages[1]
        .meta
        .as_ref()
        .and_then(|meta| meta.metrics.as_ref())
        .expect("metrics stored");
    assert!(metrics.chars > 0);
    assert!(metrics.context_tokens > 0);
}

#[tokio::test]
async fn connection_failure_is_reported_and_user_message_is_kept() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    drop(listener);
    let base = format!("http://{addr}/v1");
    let storage = Storage::open_in_memory().unwrap();
    let (_character_id, conversation_id) = prepare(&storage, &base);
    let (tx, rx) = Services::channel();
    let services = Services {
        storage: Arc::new(storage),
        runtime: tokio::runtime::Handle::current(),
        tx,
        api_key: Arc::new(Mutex::new(None)),
    };
    services.send_and_generate(
        conversation_id.clone(),
        "req-2".into(),
        "こんにちは".into(),
        CancellationToken::new(),
    );

    let error = loop {
        match next_event(&rx).await {
            UiEvent::GenerationError { message, kind, .. } => break (message, kind),
            UiEvent::GenerationDone { .. } => panic!("unexpected success"),
            _ => {}
        }
    };
    assert_eq!(error.1, "network");
    assert!(error.0.contains("接続できません"));
    let messages = services.storage.list_messages(&conversation_id).unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].role, "user");
}

#[tokio::test]
async fn cancellation_keeps_partial_reply() {
    let (base, _server) = spawn_sse_server(vec![
        (0, sse("途中まで")),
        (2000, sse("の応答") + "data: [DONE]\n\n"),
    ])
    .await;
    let storage = Storage::open_in_memory().unwrap();
    let (_character_id, conversation_id) = prepare(&storage, &base);
    let (tx, rx) = Services::channel();
    let services = Services {
        storage: Arc::new(storage),
        runtime: tokio::runtime::Handle::current(),
        tx,
        api_key: Arc::new(Mutex::new(None)),
    };
    let cancel = CancellationToken::new();
    services.send_and_generate(
        conversation_id.clone(),
        "req-3".into(),
        "話して".into(),
        cancel.clone(),
    );

    loop {
        match next_event(&rx).await {
            UiEvent::Delta { .. } => {
                cancel.cancel();
            }
            UiEvent::GenerationDone {
                message, stopped, ..
            } => {
                assert!(stopped);
                assert_eq!(message.content, "途中まで");
                assert!(message
                    .meta
                    .as_ref()
                    .map(|meta| meta.stopped)
                    .unwrap_or(false));
                break;
            }
            UiEvent::GenerationError { message, .. } => panic!("unexpected error: {message}"),
            _ => {}
        }
    }
    let messages = services.storage.list_messages(&conversation_id).unwrap();
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[1].content, "途中まで");
    assert!(messages[1].meta.as_ref().unwrap().stopped);
}
