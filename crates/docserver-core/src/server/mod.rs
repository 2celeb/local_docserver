//! axum による HTTP サーバ。
//! - `/r/{root}/{path..}` : ルート配下を静的配信（相対参照の解決のため全ファイル）
//! - `/api/*`             : JSON API
//! - `/` `/assets/*`      : 埋め込み済みフロントエンド（ビルド済みなら）

use crate::app::App;
use crate::search::SearchMode;
use axum::{
    body::Body,
    extract::{Path as AxPath, Query, State},
    http::{header, HeaderValue, StatusCode, Uri},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;
use tracing::info;

#[derive(rust_embed::Embed)]
#[folder = "$CARGO_MANIFEST_DIR/../../dist/"]
struct FrontendAssets;

pub fn router(app: Arc<App>) -> Router {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/roots", get(roots))
        .route("/api/files", get(files))
        .route("/api/search", get(search))
        .route("/api/raw", get(raw))
        .route("/api/config", get(get_config).put(put_config))
        .route("/api/reload", post(reload))
        .route("/r/{root}/{*path}", get(serve_file))
        .fallback(frontend)
        .layer(CorsLayer::permissive())
        .with_state(app)
}

/// サーバを起動し、実際に bind したアドレスを返す。
pub async fn bind(app: Arc<App>) -> anyhow::Result<(SocketAddr, TcpListener)> {
    let s = app.settings();
    let addr: SocketAddr = format!("{}:{}", s.server.host, s.server.port).parse()?;
    let listener = TcpListener::bind(addr).await?;
    let local = listener.local_addr()?;
    info!(%local, "HTTP サーバ起動");
    Ok((local, listener))
}

pub async fn serve(app: Arc<App>, listener: TcpListener) -> anyhow::Result<()> {
    axum::serve(listener, router(app)).await?;
    Ok(())
}

async fn health() -> impl IntoResponse {
    Json(serde_json::json!({ "ok": true, "version": env!("CARGO_PKG_VERSION") }))
}

async fn roots(State(app): State<Arc<App>>) -> impl IntoResponse {
    let s = app.settings();
    let snap = app.index.snapshot();
    let v: Vec<_> = s
        .roots
        .iter()
        .map(|r| {
            serde_json::json!({
                "name": r.name,
                "path": r.path,
                "exists": r.path.is_dir(),
                "count": snap.iter().filter(|e| e.root == r.name).count(),
            })
        })
        .collect();
    Json(serde_json::json!({ "roots": v, "generation": app.index.generation() }))
}

#[derive(Deserialize)]
struct FilesQ {
    root: Option<String>,
}

async fn files(State(app): State<Arc<App>>, Query(q): Query<FilesQ>) -> impl IntoResponse {
    Json(serde_json::json!({ "files": app.files(q.root.as_deref()), "generation": app.index.generation() }))
}

#[derive(Deserialize)]
struct SearchQ {
    q: String,
    #[serde(default)]
    mode: SearchMode,
    limit: Option<usize>,
}

async fn search(State(app): State<Arc<App>>, Query(q): Query<SearchQ>) -> impl IntoResponse {
    Json(app.search(&q.q, q.mode, q.limit.unwrap_or(50)))
}

#[derive(Deserialize)]
struct RawQ {
    root: String,
    path: String,
}

async fn raw(State(app): State<Arc<App>>, Query(q): Query<RawQ>) -> Response {
    match app.resolve(&q.root, &q.path) {
        Some(p) if p.is_file() => match tokio::fs::read_to_string(&p).await {
            Ok(text) => ([(header::CONTENT_TYPE, "text/plain; charset=utf-8")], text).into_response(),
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
        },
        _ => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn get_config(State(app): State<Arc<App>>) -> impl IntoResponse {
    let s = app.settings();
    Json(serde_json::json!({ "config_path": s.config_path, "settings": s }))
}

async fn put_config(State(app): State<Arc<App>>, Json(mut s): Json<crate::config::Settings>) -> Response {
    let cur = app.settings();
    s.config_path = cur.config_path.clone();
    let base = s.config_path.as_ref().and_then(|p| p.parent()).map(|p| p.to_path_buf());
    if let Some(base) = base {
        s.resolve_paths(&base);
    }
    match app.update_settings(s) {
        Ok(()) => Json(serde_json::json!({ "ok": true, "count": app.index.len() })).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

async fn reload(State(app): State<Arc<App>>) -> impl IntoResponse {
    let app2 = app.clone();
    tokio::task::spawn_blocking(move || app2.rescan_all()).await.ok();
    Json(serde_json::json!({ "ok": true, "count": app.index.len(), "generation": app.index.generation() }))
}

async fn serve_file(State(app): State<Arc<App>>, AxPath((root, path)): AxPath<(String, String)>) -> Response {
    let Some(p) = app.resolve(&root, &path) else {
        return (StatusCode::NOT_FOUND, "not found").into_response();
    };
    // 念のため実体が root 配下か再検証（symlink 越え防止）
    let real = match p.canonicalize() {
        Ok(r) => r,
        Err(_) => return (StatusCode::NOT_FOUND, "not found").into_response(),
    };
    let Some(root_path) = app.root_path(&root).and_then(|r| r.canonicalize().ok()) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    if !real.starts_with(&root_path) || !real.is_file() {
        return (StatusCode::FORBIDDEN, "forbidden").into_response();
    }
    let mime = mime_guess::from_path(&real).first_or_octet_stream();
    let mut ct = mime.to_string();
    if mime.type_() == mime_guess::mime::TEXT && !ct.contains("charset") {
        ct.push_str("; charset=utf-8");
    }
    match tokio::fs::read(&real).await {
        Ok(bytes) => Response::builder()
            .header(header::CONTENT_TYPE, HeaderValue::from_str(&ct).unwrap_or(HeaderValue::from_static("application/octet-stream")))
            .header(header::CACHE_CONTROL, "no-cache")
            .body(Body::from(bytes))
            .unwrap(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// ビルド済みフロントエンド (dist/) を埋め込み配信。SPA のため未知パスは index.html。
async fn frontend(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };
    // 静的アセット (assets/...) 以外の未知パスは SPA ルーティング用に index.html を返す
    let file = FrontendAssets::get(path).or_else(|| {
        if path.starts_with("assets/") { None } else { FrontendAssets::get("index.html") }
    });
    match file {
        Some(f) => {
            let is_index = !FrontendAssets::iter().any(|p| p == path);
            let mime = if is_index { mime_guess::mime::TEXT_HTML_UTF_8 } else { mime_guess::from_path(path).first_or_else(|| mime_guess::mime::TEXT_HTML) };
            ([(header::CONTENT_TYPE, mime.to_string())], f.data.into_owned()).into_response()
        }
        None => (
            StatusCode::NOT_FOUND,
            "フロントエンドが未ビルドです。`npm run build` 後に再ビルドしてください。/api/* は利用可能です。",
        )
            .into_response(),
    }
}
