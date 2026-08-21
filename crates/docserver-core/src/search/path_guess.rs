//! ファイルパス推測: `sample-docs/data/データ移行概要設計.html` のような文字列から該当ファイルを探す。

use super::{name_search, strip_ext, SearchQuery, SearchResult};
use crate::index::FileEntry;

/// このスコア以上の単独トップは「確信あり」として UI が即オープンしてよい
pub const CONFIDENT_SCORE: i64 = 80;

pub fn search(entries: &[FileEntry], q: &SearchQuery) -> Vec<SearchResult> {
    let segs = &q.segments;
    if segs.is_empty() {
        return vec![];
    }
    let last = segs.last().unwrap();
    let last_stem = strip_ext(last);
    let mut out = vec![];

    for e in entries {
        if let Some((score, reason)) = score_entry(e, segs, last, &last_stem) {
            let hl = if e.norm_stem == last_stem { vec![(0, e.stem.chars().count())] } else { vec![] };
            out.push(SearchResult { entry: e.clone(), score, reason: reason.into(), highlights: hl });
        }
    }

    if out.is_empty() {
        // 最終セグメントを fuzzy にファイル名検索（スコア上限 40）
        let fq = super::parse_query(&last_stem, super::SearchMode::Name);
        for mut r in name_search::search(entries, &fq) {
            r.score = r.score.min(40);
            r.reason = format!("fuzzy:{}", r.reason);
            out.push(r);
        }
    }
    out
}

fn score_entry(e: &FileEntry, segs: &[String], last: &str, last_stem: &str) -> Option<(i64, &'static str)> {
    let root_norm = crate::index::normalize(&e.root);
    let es = &e.norm_segments;

    // 1. root名/rel_path 完全一致
    if segs.len() == es.len() + 1 && segs[0] == root_norm && segs[1..] == es[..] {
        return Some((100, "exact:root+path"));
    }
    // 2. rel_path 完全一致
    if segs[..] == es[..] {
        return Some((90, "exact:path"));
    }
    // 3. セグメント suffix 一致（ディレクトリ境界）
    if segs.len() <= es.len() && es[es.len() - segs.len()..] == segs[..] {
        return Some((80, "suffix:path"));
    }
    // 4. 先頭セグメントが root 名で、残りが suffix 一致（ローカルの実ディレクトリ名が違う場合）
    if segs.len() >= 2 && segs[0] == root_norm {
        let rest = &segs[1..];
        if rest.len() <= es.len() && es[es.len() - rest.len()..] == rest[..] {
            return Some((85, "suffix:root+path"));
        }
    }
    // 4b. 途中のセグメントから先が suffix 一致（上位ディレクトリが余分に付いている: ~/src/sample-docs/data/x.html）
    for start in 1..segs.len() {
        let rest = &segs[start..];
        if rest.len() <= es.len() && es[es.len() - rest.len()..] == rest[..] {
            return Some((78, "suffix:partial"));
        }
        if rest.len() >= 2 && rest[0] == root_norm {
            let r2 = &rest[1..];
            if r2.len() <= es.len() && es[es.len() - r2.len()..] == r2[..] {
                return Some((79, "suffix:partial+root"));
            }
        }
    }
    // 5. 拡張子違いで suffix 一致
    {
        let n = segs.len();
        if n <= es.len() {
            let dirs_match = es[es.len() - n..es.len() - 1] == segs[..n - 1];
            if dirs_match && e.norm_stem == last_stem && es.last().map(|l| l != last).unwrap_or(false) {
                return Some((60, "suffix:ext-mismatch"));
            }
        }
    }
    // 6. ファイル名完全一致
    if es.last().map(|l| l == last).unwrap_or(false) {
        return Some((50, "exact:filename"));
    }
    if e.norm_stem == last_stem {
        return Some((45, "exact:stem"));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::{parse_query, SearchMode};
    use crate::test_util::fixture_entries;

    fn run(q: &str) -> Vec<(String, i64)> {
        let entries = fixture_entries();
        let pq = parse_query(q, SearchMode::Auto);
        assert_eq!(pq.mode, SearchMode::Path);
        let mut r = search(&entries, &pq);
        r.sort_by(|a, b| b.score.cmp(&a.score));
        r.into_iter().map(|r| (r.entry.full_key(), r.score)).collect()
    }

    #[test]
    fn root_plus_path() {
        let r = run("file:sample-docs/data/データ移行概要設計.html");
        assert_eq!(r[0], ("sample-docs/data/データ移行概要設計.html".into(), 100));
    }

    #[test]
    fn rel_path_only() {
        let r = run("data/データ移行概要設計.html");
        assert_eq!(r[0].0, "sample-docs/data/データ移行概要設計.html");
        assert_eq!(r[0].1, 90);
    }

    #[test]
    fn extra_parent_dirs() {
        let r = run("/home/me/src/sample-docs/data/データ移行概要設計.html");
        assert_eq!(r[0].0, "sample-docs/data/データ移行概要設計.html");
        assert!(r[0].1 >= 78);
    }

    #[test]
    fn different_root_name_falls_to_suffix() {
        let r = run("sample-docs-main/data/データ移行概要設計.html");
        assert_eq!(r[0].0, "sample-docs/data/データ移行概要設計.html");
        assert_eq!(r[0].1, 78);
    }

    #[test]
    fn ext_mismatch() {
        let r = run("data/データ移行概要設計.md");
        assert_eq!(r[0].0, "sample-docs/data/データ移行概要設計.html");
        assert_eq!(r[0].1, 60);
    }

    #[test]
    fn same_filename_in_two_roots() {
        let r = run("README.md");
        assert_eq!(r.len(), 2);
        assert!(r.iter().all(|(_, s)| *s == 90));
    }

    #[test]
    fn nothing_matches_falls_to_fuzzy() {
        let r = run("x/y/データ移行.html");
        assert!(!r.is_empty());
        assert!(r[0].1 <= 40);
    }
}
