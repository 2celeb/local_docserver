//! ファイル名検索: `データ移行概要設計` のような文字列から stem / パスを検索する。

use super::{SearchQuery, SearchResult};
use crate::index::FileEntry;
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

pub fn search(entries: &[FileEntry], q: &SearchQuery) -> Vec<SearchResult> {
    if q.tokens.is_empty() {
        return vec![];
    }
    let joined = q.tokens.join("");
    let matcher = SkimMatcherV2::default().ignore_case();
    let mut out = vec![];

    for e in entries {
        let stem = &e.norm_stem;
        let full = e.norm_segments.join("/");

        let (score, reason, hl): (i64, &str, Vec<(usize, usize)>) = if stem == &joined {
            (100, "exact", vec![(0, e.stem.chars().count())])
        } else if stem.starts_with(&joined) {
            (90, "prefix", vec![(0, joined.chars().count())])
        } else if let Some(pos) = stem.find(&joined) {
            let start = stem[..pos].chars().count();
            (80 - (start.min(20) as i64), "contains", vec![(start, start + joined.chars().count())])
        } else if q.tokens.len() > 1 && q.tokens.iter().all(|t| stem.contains(t.as_str())) {
            let mut hl = vec![];
            for t in &q.tokens {
                if let Some(p) = stem.find(t.as_str()) {
                    let s = stem[..p].chars().count();
                    hl.push((s, s + t.chars().count()));
                }
            }
            // 余分な文字が少ない（短い）名前を優先
            let extra = stem.chars().count().saturating_sub(joined.chars().count());
            (70 - (extra as i64 / 3).min(9), "and:stem", hl)
        } else if q.tokens.len() > 1 && q.tokens.iter().all(|t| full.contains(t.as_str())) {
            (60, "and:path", vec![])
        } else if let Some(p) = full.find(&joined) {
            // ディレクトリ名に含まれる
            let _ = p;
            (50, "contains:path", vec![])
        } else if joined.chars().count() >= 2 {
            match matcher.fuzzy_indices(stem, &joined) {
                Some((s, idx)) if s > 0 => {
                    // skim のスコアを 0..40 に丸める
                    let norm = ((s as f64 / (joined.chars().count() as f64 * 16.0)) * 40.0).clamp(1.0, 40.0) as i64;
                    let hl = idx.iter().map(|&i| (i, i + 1)).collect();
                    (norm, "fuzzy", hl)
                }
                _ => continue,
            }
        } else {
            continue;
        };
        out.push(SearchResult { entry: e.clone(), score, reason: reason.into(), highlights: hl });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::{parse_query, SearchMode};
    use crate::test_util::fixture_entries;

    fn run(q: &str) -> Vec<(String, i64, String)> {
        let entries = fixture_entries();
        let pq = parse_query(q, SearchMode::Auto);
        assert_eq!(pq.mode, SearchMode::Name);
        let mut r = search(&entries, &pq);
        r.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.entry.rel_path.cmp(&b.entry.rel_path)));
        r.into_iter().map(|r| (r.entry.stem.clone(), r.score, r.reason)).collect()
    }

    #[test]
    fn exact_then_prefix() {
        let r = run("データ移行概要設計");
        assert_eq!(r[0].0, "データ移行概要設計");
        assert_eq!(r[0].1, 100);
        assert!(r.iter().any(|x| x.0 == "データ移行概要設計_詳細" && x.1 == 90));
    }

    #[test]
    fn contains() {
        let r = run("概要設計");
        assert!(r.iter().any(|x| x.0 == "データ移行概要設計" && x.2 == "contains"));
    }

    #[test]
    fn and_tokens() {
        let r = run("出走表 GraphQL");
        assert_eq!(r[0].0, "出走表_GraphQL設計");
        assert_eq!(r[0].2, "and:stem");
    }

    #[test]
    fn nfkc_halfwidth_kana() {
        let r = run("ﾃﾞｰﾀ移行");
        assert_eq!(r[0].0, "データ移行概要設計");
    }

    #[test]
    fn fuzzy_latin() {
        let r = run("grphql");
        assert!(r.iter().any(|x| x.2 == "fuzzy" && x.0.contains("GraphQL")));
    }
}
