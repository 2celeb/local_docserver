//! Tauri シェル: コアの HTTP サーバを起動し、WebView と IPC で橋渡しする。

use docserver_core::{index::watcher, server, App, SearchMode, Settings};
use std::sync::{Arc, Mutex};
use tauri::{Manager, State};

pub struct AppState {
    pub app: Arc<App>,
    pub server_url: Mutex<Option<String>>,
    _watcher: Mutex<Option<notify_debouncer_mini::Debouncer<notify::RecommendedWatcher>>>,
}

#[tauri::command]
fn server_url(state: State<AppState>) -> Option<String> {
    state.server_url.lock().unwrap().clone()
}

#[tauri::command]
fn search(state: State<AppState>, q: String, mode: Option<SearchMode>, limit: Option<usize>) -> serde_json::Value {
    serde_json::to_value(state.app.search(&q, mode.unwrap_or_default(), limit.unwrap_or(50))).unwrap()
}

#[tauri::command]
fn list_files(state: State<AppState>, root: Option<String>) -> serde_json::Value {
    serde_json::to_value(state.app.files(root.as_deref())).unwrap()
}

#[tauri::command]
fn get_settings(state: State<AppState>) -> Settings {
    state.app.settings()
}

#[tauri::command]
fn update_settings(state: State<AppState>, settings: Settings) -> Result<usize, String> {
    let mut s = settings;
    let cur = state.app.settings();
    s.config_path = cur.config_path.clone().or_else(default_config_path);
    let base = s.config_path.as_ref().and_then(|p| p.parent()).map(|p| p.to_path_buf());
    if let Some(base) = base {
        s.resolve_paths(&base);
    }
    state.app.update_settings(s).map_err(|e| e.to_string())?;
    Ok(state.app.index.len())
}

#[tauri::command]
fn reload(state: State<AppState>) -> usize {
    state.app.rescan_all();
    state.app.index.len()
}

#[tauri::command]
fn reveal(state: State<AppState>, root: String, path: String) -> Result<(), String> {
    let p = state.app.resolve(&root, &path).ok_or("not found")?;
    let dir = p.parent().ok_or("no parent")?;
    open::that(dir).map_err(|e| e.to_string())
}

/// プロジェクト直下の config/setting.yaml。
/// `tauri dev` は cwd が src-tauri/ になるため、cwd だけでなく親ディレクトリ・
/// (デバッグビルド時は) Cargo マニフェストの親も候補にする。
fn project_config_path() -> Option<std::path::PathBuf> {
    let mut candidates: Vec<std::path::PathBuf> = vec![];
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join("config/setting.yaml"));
        candidates.push(cwd.join("../config/setting.yaml"));
    }
    if cfg!(debug_assertions) {
        candidates.push(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../config/setting.yaml"));
    }
    candidates.into_iter().find(|p| p.is_file())
}

fn default_config_path() -> Option<std::path::PathBuf> {
    project_config_path().or_else(|| std::env::current_dir().ok().map(|d| d.join("config/setting.yaml")))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let explicit = project_config_path();
    let settings = Settings::load_or_default(explicit.as_deref()).unwrap_or_else(|e| {
        tracing::error!(%e, "設定読込失敗。デフォルトで起動");
        Settings::default()
    });
    let app = App::new(settings);
    app.rescan_all();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(move |tauri_app| {
            let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build()?;
            let (addr, listener) = rt.block_on(server::bind(app.clone()))?;
            let url = format!("http://{addr}/");
            let app2 = app.clone();
            std::thread::spawn(move || {
                rt.block_on(async move {
                    if let Err(e) = server::serve(app2, listener).await {
                        tracing::error!(%e, "HTTP サーバ停止");
                    }
                });
            });
            let w = watcher::start(app.clone());
            tauri_app.manage(AppState { app: app.clone(), server_url: Mutex::new(Some(url)), _watcher: Mutex::new(w) });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            server_url,
            search,
            list_files,
            get_settings,
            update_settings,
            reload,
            reveal
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
