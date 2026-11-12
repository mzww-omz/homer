use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use gpui::{
    point, px, size, App, AppContext, Application, Bounds, TitlebarOptions, WindowBounds,
    WindowOptions,
};
use homer_ui::app::Startup;
use homer_ui::{HomerApp, Services};

fn data_dir() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        #[cfg(target_os = "macos")]
        {
            return PathBuf::from(home).join("Library/Application Support/Homer");
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = home;
        }
    }
    std::env::temp_dir().join("homer")
}

fn main() {
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("Homer: tokio runtime を起動できません: {error}");
            std::process::exit(1);
        }
    };

    let dir = data_dir();
    if let Err(error) = std::fs::create_dir_all(&dir) {
        eprintln!(
            "Homer: データフォルダを作成できません ({}): {error}",
            dir.display()
        );
        std::process::exit(1);
    }
    let db_path = dir.join("homer.db");
    let storage = match homer_storage::Storage::open(&db_path) {
        Ok(storage) => Arc::new(storage),
        Err(error) => {
            eprintln!(
                "Homer: データベースを開けません ({}): {error}",
                db_path.display()
            );
            std::process::exit(1);
        }
    };

    let characters = storage.list_characters().unwrap_or_else(|error| {
        eprintln!("Homer: キャラクターを読み込めません: {error}");
        Vec::new()
    });
    let conversations = storage.list_conversations().unwrap_or_else(|error| {
        eprintln!("Homer: 会話を読み込めません: {error}");
        Vec::new()
    });
    let settings = storage.load_settings().unwrap_or_default();
    let messages = conversations
        .first()
        .and_then(|conversation| storage.list_messages(&conversation.id).ok())
        .unwrap_or_default();

    let (tx, rx) = Services::channel();
    let services = Services {
        storage,
        runtime: runtime.handle().clone(),
        tx,
        api_key: Arc::new(Mutex::new(None)),
    };
    let startup = Startup {
        characters,
        conversations,
        messages,
        settings,
    };

    let (width, height) = homer_ui::window_size();
    let (min_width, min_height) = homer_ui::minimum_window_size();

    Application::new().run(move |cx: &mut App| {
        homer_ui::bind_keys(cx);
        let bounds = Bounds::centered(None, size(px(width), px(height)), cx);
        let window = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    window_min_size: Some(size(px(min_width), px(min_height))),
                    titlebar: Some(TitlebarOptions {
                        title: Some("Homer".into()),
                        appears_transparent: false,
                        traffic_light_position: Some(point(px(18.), px(18.))),
                    }),
                    ..Default::default()
                },
                |_window, cx| cx.new(|cx| HomerApp::new(services.clone(), startup, rx.clone(), cx)),
            )
            .expect("Homer: ウィンドウを開けません");
        let debug_view = std::env::var("HOMER_DEBUG_VIEW").unwrap_or_default();
        window
            .update(cx, |view, window, cx| {
                window.focus(&view.composer_focus(cx));
                window.activate_window();
                if !debug_view.is_empty() {
                    view.debug_open_view(&debug_view, window, cx);
                }
            })
            .expect("Homer: 初期フォーカスを設定できません");
        cx.activate(true);
    });

    drop(runtime);
}
