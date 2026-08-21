pub mod name_search;
pub mod path_guess;
pub mod query;

use crate::index::FileEntry;
use serde::Serialize;

pub use query::{parse_query, SearchMode, SearchQuery};

#[derive(Debug, Clone, Serialize)]
pub struct SearchResult {
    #[serde(flatten)]
    pub entry: FileEntry,
    pub score: i64,
    /// どのルールでヒットしたか（UI 表示・デバッグ用）
    pub reason: String,
    /// stem 内のハイライト範囲（char index, [start, end)）
    pub highlights: Vec<(usize, usize)>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchResponse {
    pub query: String,
    pub mode: SearchMode,
    pub total: usize,
    pub results: Vec<SearchResult>,
    /// スコアが十分高い単一候補（UI が即オープンしてよい）
    pub best: Option<String>,
}

/// エントリ集合に対して検索を行う。mode=Auto の場合はクエリ形状から判定し、
/// パス推測が 0 件ならファイル名検索にフォールバックする。
pub fn search(entries: &[FileEntry], raw: &str, mode: SearchMode, limit: usize) -> SearchResponse {
    let q = parse_query(raw, mode);
    let mut results = match q.mode {
        SearchMode::Path => path_guess::search(entries, &q),
        SearchMode::Name => name_search::search(entries, &q),
        SearchMode::Auto => unreachable!("parse_query resolves Auto"),
    };
    let mut mode_used = q.mode;
    if results.is_empty() && q.mode == SearchMode::Path {
        // 最終セグメントでファイル名検索にフォールバック
        let last = q.segments.last().cloned().unwrap_or_default();
        let stem = strip_ext(&last);
        let fq = parse_query(&stem, SearchMode::Name);
        results = name_search::search(entries, &fq);
        mode_used = SearchMode::Name;
    }
    results.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then_with(|| b.entry.modified.cmp(&a.entry.modified))
            .then_with(|| a.entry.rel_path.cmp(&b.entry.rel_path))
    });
    let total = results.len();
    results.truncate(limit);
    let best = match results.as_slice() {
        [first, ..] if mode_used == SearchMode::Path && first.score >= path_guess::CONFIDENT_SCORE
            && results.get(1).map(|s| s.score < first.score).unwrap_or(true) =>
        {
            Some(first.entry.full_key())
        }
        _ => None,
    };
    SearchResponse { query: raw.to_string(), mode: mode_used, total, results, best }
}

pub(crate) fn strip_ext(name: &str) -> String {
    match name.rfind('.') {
        Some(i) if i > 0 && name.len() - i <= 10 => name[..i].to_string(),
        _ => name.to_string(),
    }
}
