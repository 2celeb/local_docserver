pub mod scanner;
pub mod watcher;

use parking_lot::RwLock;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;
use unicode_normalization::UnicodeNormalization;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FileKind {
    Html,
    Markdown,
    Mermaid,
    Other,
}

impl FileKind {
    pub fn from_ext(ext: &str) -> Self {
        match ext.to_ascii_lowercase().as_str() {
            "html" | "htm" => FileKind::Html,
            "md" | "markdown" => FileKind::Markdown,
            "mmd" | "mermaid" => FileKind::Mermaid,
            _ => FileKind::Other,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct FileEntry {
    /// roots[].name
    pub root: String,
    /// ルートからの相対パス（'/' 区切り、NFC 正規化済み）
    pub rel_path: String,
    #[serde(skip)]
    pub abs_path: PathBuf,
    pub file_name: String,
    pub stem: String,
    pub ext: String,
    pub kind: FileKind,
    pub size: u64,
    /// UNIX epoch millis
    pub modified: u64,
    /// 検索用: NFKC + lowercase した stem
    #[serde(skip)]
    pub norm_stem: String,
    /// 検索用: NFKC + lowercase した rel_path の各セグメント
    #[serde(skip)]
    pub norm_segments: Vec<String>,
}

impl FileEntry {
    pub fn new(root: &str, root_path: &Path, abs: &Path, meta: &std::fs::Metadata) -> Option<Self> {
        let rel = abs.strip_prefix(root_path).ok()?;
        let rel_path: String = rel
            .components()
            .map(|c| c.as_os_str().to_string_lossy().nfc().collect::<String>())
            .collect::<Vec<_>>()
            .join("/");
        let file_name = abs.file_name()?.to_string_lossy().nfc().collect::<String>();
        let ext = abs.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
        let stem = match file_name.rfind('.') {
            Some(i) if !ext.is_empty() => file_name[..i].to_string(),
            _ => file_name.clone(),
        };
        let modified = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let norm_segments = rel_path.split('/').map(normalize).collect();
        Some(Self {
            root: root.to_string(),
            norm_stem: normalize(&stem),
            norm_segments,
            rel_path,
            abs_path: abs.to_path_buf(),
            file_name,
            stem,
            kind: FileKind::from_ext(&ext),
            ext,
            size: meta.len(),
            modified,
        })
    }

    /// `root/rel_path` 形式
    pub fn full_key(&self) -> String {
        format!("{}/{}", self.root, self.rel_path)
    }
}

/// 検索用正規化: NFKC → lowercase。全角英数・半角カナ等のゆれを吸収する。
pub fn normalize(s: &str) -> String {
    s.nfkc().collect::<String>().to_lowercase()
}

#[derive(Default)]
pub struct FileIndex {
    entries: RwLock<Arc<Vec<FileEntry>>>,
    generation: RwLock<u64>,
}

impl FileIndex {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn replace(&self, entries: Vec<FileEntry>) {
        let mut e = entries;
        e.sort_by(|a, b| a.root.cmp(&b.root).then_with(|| a.rel_path.cmp(&b.rel_path)));
        *self.entries.write() = Arc::new(e);
        *self.generation.write() += 1;
    }

    /// 特定ルートのエントリだけ入れ替える（watch による部分再スキャン用）
    pub fn replace_root(&self, root: &str, new_entries: Vec<FileEntry>) {
        let mut all: Vec<FileEntry> =
            self.entries.read().iter().filter(|e| e.root != root).cloned().collect();
        all.extend(new_entries);
        self.replace(all);
    }

    pub fn snapshot(&self) -> Arc<Vec<FileEntry>> {
        self.entries.read().clone()
    }

    pub fn len(&self) -> usize {
        self.entries.read().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn generation(&self) -> u64 {
        *self.generation.read()
    }
}
