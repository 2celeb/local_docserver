//! テスト用フィクスチャ
use crate::index::FileEntry;
use std::path::Path;

pub fn fixture_entries() -> Vec<FileEntry> {
    let tmp = std::env::temp_dir().join("docserver-core-fixture");
    let files: &[(&str, &str)] = &[
        ("sample-docs", "data/データ移行概要設計.html"),
        ("sample-docs", "data/データ移行概要設計_詳細.html"),
        ("sample-docs", "frontend/出走表_GraphQL設計.html"),
        ("sample-docs", "frontend/出走表_仕様書.html"),
        ("sample-docs", "README.md"),
        ("sample-docs", "index.html"),
        ("other", "README.md"),
        ("other", "docs/flow.mmd"),
    ];
    let mut out = vec![];
    for (root, rel) in files {
        let root_path = tmp.join(root);
        let abs = root_path.join(rel);
        std::fs::create_dir_all(abs.parent().unwrap()).unwrap();
        if !abs.exists() {
            std::fs::write(&abs, "x").unwrap();
        }
        let meta = std::fs::metadata(&abs).unwrap();
        out.push(FileEntry::new(root, Path::new(&root_path), &abs, &meta).unwrap());
    }
    out
}
