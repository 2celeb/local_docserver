//! docserver-core: 設定読込・ファイル走査・検索・HTTP 配信のコア。
//! Tauri に依存しないため、単体バイナリ (`docserver`) としても動作する。

pub mod config;
pub mod index;
pub mod search;
pub mod server;
pub mod app;
#[cfg(test)]
pub(crate) mod test_util;

pub use app::App;
pub use config::Settings;
pub use index::{FileEntry, FileIndex, FileKind};
pub use search::{SearchMode, SearchQuery, SearchResult};
