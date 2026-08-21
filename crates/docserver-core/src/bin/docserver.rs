//! ヘッドレス CLI: Tauri なしでサーバだけ起動する（CI/サーバ用途・開発用）。
use clap::Parser;
use docserver_core::{index::watcher, server, App, Settings};
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(name = "docserver", about = "local_docserver headless HTTP server")]
struct Cli {
    /// 設定ファイル (default: ./config/setting.yaml)
    #[arg(short, long)]
    config: Option<PathBuf>,
    /// ポート上書き
    #[arg(short, long)]
    port: Option<u16>,
    /// 起動後にブラウザを開く
    #[arg(long)]
    open: bool,
    /// 検索して終了（デバッグ用）
    #[arg(long)]
    search: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();
    let cli = Cli::parse();
    let mut settings = Settings::load_or_default(cli.config.as_deref())?;
    if let Some(p) = cli.port {
        settings.server.port = p;
    }
    match &settings.config_path {
        Some(p) => tracing::info!(path = %p.display(), "設定読込"),
        None => tracing::warn!("設定ファイルが見つかりません。デフォルト設定で起動します"),
    }
    let app = App::new(settings);
    app.rescan_all();

    if let Some(q) = cli.search {
        let r = app.search(&q, docserver_core::SearchMode::Auto, 20);
        println!("{}", serde_json::to_string_pretty(&r)?);
        return Ok(());
    }

    let _watcher = watcher::start(app.clone());
    let (addr, listener) = server::bind(app.clone()).await?;
    let url = format!("http://{addr}/");
    println!("listening on {url}");
    if cli.open || app.settings().server.open_browser {
        let _ = open::that(&url);
    }
    server::serve(app, listener).await
}
