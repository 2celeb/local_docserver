use super::FileEntry;
use crate::config::{RootSettings, Settings};
use ignore::WalkBuilder;
use std::path::Path;
use tracing::{debug, warn};

/// 1 ルートを走査して FileEntry を返す
pub fn scan_root(settings: &Settings, root: &RootSettings) -> Vec<FileEntry> {
    if !root.path.is_dir() {
        warn!(root = %root.name, path = %root.path.display(), "ルートが存在しません");
        return vec![];
    }
    let mut builder = WalkBuilder::new(&root.path);
    builder
        .hidden(true)
        .git_ignore(settings.respect_gitignore)
        .git_global(settings.respect_gitignore)
        .git_exclude(settings.respect_gitignore)
        .follow_links(false)
        .max_depth(Some(settings.max_depth));

    let excludes: Vec<String> = root.exclude.clone();
    builder.filter_entry(move |e| {
        let name = e.file_name().to_string_lossy();
        !excludes.iter().any(|x| x == name.as_ref())
    });

    let mut out = vec![];
    for entry in builder.build() {
        let entry = match entry {
            Ok(e) => e,
            Err(err) => {
                debug!(%err, "walk error");
                continue;
            }
        };
        if !entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
            continue;
        }
        let path = entry.path();
        let ext = match path.extension() {
            Some(e) => e.to_string_lossy().to_string(),
            None => continue,
        };
        if !settings.ext_allowed(&ext) {
            continue;
        }
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        if let Some(fe) = FileEntry::new(&root.name, &root.path, path, &meta) {
            out.push(fe);
        }
    }
    out
}

pub fn scan_all(settings: &Settings) -> Vec<FileEntry> {
    settings.roots.iter().flat_map(|r| scan_root(settings, r)).collect()
}

/// パスが root 配下かどうか（パストラバーサル防止）。`rel` は '/' 区切り。
pub fn resolve_under_root(root_path: &Path, rel: &str) -> Option<std::path::PathBuf> {
    use std::path::Component;
    let mut p = root_path.to_path_buf();
    for seg in rel.split('/') {
        if seg.is_empty() || seg == "." {
            continue;
        }
        if seg == ".." {
            return None;
        }
        let c = Path::new(seg);
        if c.components().any(|c| !matches!(c, Component::Normal(_))) {
            return None;
        }
        p.push(seg);
    }
    Some(p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn mk(settings: &mut Settings, dir: &Path) {
        fs::create_dir_all(dir.join("data")).unwrap();
        fs::create_dir_all(dir.join("node_modules/x")).unwrap();
        fs::write(dir.join("data/データ移行概要設計.html"), "<html>").unwrap();
        fs::write(dir.join("data/読んで.md"), "# hi").unwrap();
        fs::write(dir.join("flow.mmd"), "graph TD; a-->b").unwrap();
        fs::write(dir.join("img.png"), "x").unwrap();
        fs::write(dir.join("node_modules/x/a.html"), "x").unwrap();
        settings.roots.push(RootSettings {
            name: "t".into(),
            path: dir.to_path_buf(),
            exclude: vec!["node_modules".into()],
        });
    }

    #[test]
    fn scans_only_allowed_and_excludes() {
        let tmp = tempfile::tempdir().unwrap();
        let mut s = Settings::default();
        mk(&mut s, tmp.path());
        let mut entries = scan_all(&s);
        entries.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
        let rels: Vec<_> = entries.iter().map(|e| e.rel_path.as_str()).collect();
        assert_eq!(rels, vec!["data/データ移行概要設計.html", "data/読んで.md", "flow.mmd"]);
        let e = &entries[0];
        assert_eq!(e.stem, "データ移行概要設計");
        assert_eq!(e.ext, "html");
        assert_eq!(e.kind, super::super::FileKind::Html);
        assert_eq!(e.norm_segments, vec!["data", "データ移行概要設計.html"]);
    }

    #[test]
    fn traversal_blocked() {
        let root = Path::new("/r");
        assert!(resolve_under_root(root, "../etc/passwd").is_none());
        assert!(resolve_under_root(root, "a/../../b").is_none());
        assert_eq!(resolve_under_root(root, "a/b.html").unwrap(), Path::new("/r/a/b.html"));
        assert_eq!(resolve_under_root(root, "/a//b.html").unwrap(), Path::new("/r/a/b.html"));
    }
}
