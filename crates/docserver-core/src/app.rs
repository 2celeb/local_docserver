//! アプリケーション状態: 設定 + インデックス + 再スキャン。Tauri / CLI 双方から使う。

use crate::config::Settings;
use crate::index::{scanner, FileEntry, FileIndex};
use crate::search::{self, SearchMode, SearchResponse};
use parking_lot::RwLock;
use std::path::Path;
use std::sync::Arc;
use tracing::info;

pub struct App {
    settings: RwLock<Settings>,
    pub index: FileIndex,
}

impl App {
    pub fn new(settings: Settings) -> Arc<Self> {
        Arc::new(Self { settings: RwLock::new(settings), index: FileIndex::new() })
    }

    pub fn settings(&self) -> Settings {
        self.settings.read().clone()
    }

    /// 設定を差し替えて全再スキャン
    pub fn update_settings(&self, s: Settings) -> anyhow::Result<()> {
        s.validate()?;
        if let Some(p) = &s.config_path {
            s.save(p)?;
        }
        *self.settings.write() = s;
        self.rescan_all();
        Ok(())
    }

    pub fn rescan_all(&self) {
        let s = self.settings();
        let t = std::time::Instant::now();
        let entries = scanner::scan_all(&s);
        info!(count = entries.len(), ms = t.elapsed().as_millis(), "全スキャン完了");
        self.index.replace(entries);
    }

    pub fn rescan_root(&self, name: &str) {
        let s = self.settings();
        if let Some(r) = s.roots.iter().find(|r| r.name == name) {
            let entries = scanner::scan_root(&s, r);
            self.index.replace_root(name, entries);
        }
    }

    pub fn search(&self, q: &str, mode: SearchMode, limit: usize) -> SearchResponse {
        let snap = self.index.snapshot();
        search::search(&snap, q, mode, limit)
    }

    pub fn files(&self, root: Option<&str>) -> Vec<FileEntry> {
        self.index
            .snapshot()
            .iter()
            .filter(|e| root.map(|r| e.root == r).unwrap_or(true))
            .cloned()
            .collect()
    }

    /// `root` と '/' 区切りの相対パスから実ファイルパスを安全に解決する
    pub fn resolve(&self, root: &str, rel: &str) -> Option<std::path::PathBuf> {
        let s = self.settings.read();
        let r = s.roots.iter().find(|r| r.name == root)?;
        let p = scanner::resolve_under_root(&r.path, rel)?;
        // NFC/NFD 差異対策: そのまま無ければ index から探す
        if p.exists() {
            return Some(p);
        }
        let rel_nfc: String = {
            use unicode_normalization::UnicodeNormalization;
            rel.trim_matches('/').nfc().collect()
        };
        self.index.snapshot().iter().find(|e| e.root == root && e.rel_path == rel_nfc).map(|e| e.abs_path.clone())
    }

    pub fn root_path(&self, root: &str) -> Option<std::path::PathBuf> {
        self.settings.read().roots.iter().find(|r| r.name == root).map(|r| r.path.clone())
    }

    pub fn is_under_any_root(&self, p: &Path) -> bool {
        self.settings.read().roots.iter().any(|r| p.starts_with(&r.path))
    }
}
