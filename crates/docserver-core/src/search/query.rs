use crate::index::normalize;
use percent_encoding::percent_decode_str;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SearchMode {
    #[default]
    Auto,
    Path,
    Name,
}

#[derive(Debug, Clone)]
pub struct SearchQuery {
    pub raw: String,
    pub mode: SearchMode,
    /// 正規化済み全文（'/' 区切り）
    pub text: String,
    /// '/' 区切りの正規化済みセグメント（Path モード）
    pub segments: Vec<String>,
    /// 空白区切りの正規化済みトークン（Name モード）
    pub tokens: Vec<String>,
}

const PREFIXES: &[&str] = &["file:", "path:", "file://"];

/// クエリ文字列を解釈する。
/// - `file:` / `path:` プレフィックス除去
/// - 引用符・空白除去、`\` → `/`、URL デコード、NFKC 正規化
/// - Auto の場合: `/` を含む or 対応拡張子で終わる → Path、それ以外 → Name
pub fn parse_query(raw: &str, mode: SearchMode) -> SearchQuery {
    let mut s = raw.trim().to_string();
    let mut forced_path = false;
    for p in PREFIXES {
        if s.get(..p.len()).map(|h| h.eq_ignore_ascii_case(p)).unwrap_or(false) {
            s = s[p.len()..].to_string();
            forced_path = true;
            break;
        }
    }
    let s = s.trim().trim_matches(|c| c == '"' || c == '\'' || c == '`' || c == '<' || c == '>').trim();
    let s = s.replace('\\', "/");
    let s = percent_decode_str(&s).decode_utf8().map(|c| c.to_string()).unwrap_or(s);
    // "http://host:port/r/<root>/..." 形式（本サーバの URL 貼り付け）を許容
    let s = strip_server_url(&s);
    let s = s.trim_start_matches("./").trim_matches('/').to_string();
    let text = normalize(&s);

    let looks_path = forced_path || text.contains('/') || has_doc_ext(&text);
    let resolved = match mode {
        SearchMode::Auto => {
            if looks_path {
                SearchMode::Path
            } else {
                SearchMode::Name
            }
        }
        m => m,
    };
    let segments: Vec<String> = text.split('/').filter(|s| !s.is_empty() && *s != ".").map(String::from).collect();
    let tokens: Vec<String> = text
        .split(|c: char| c.is_whitespace() || c == '　' || c == '/')
        .filter(|t| !t.is_empty())
        .map(String::from)
        .collect();
    SearchQuery { raw: raw.to_string(), mode: resolved, text, segments, tokens }
}

fn has_doc_ext(s: &str) -> bool {
    ["html", "htm", "md", "markdown", "mmd", "mermaid"]
        .iter()
        .any(|e| s.ends_with(&format!(".{e}")))
}

fn strip_server_url(s: &str) -> String {
    if let Some(rest) = s.strip_prefix("http://").or_else(|| s.strip_prefix("https://")) {
        if let Some(i) = rest.find("/r/") {
            return rest[i + 3..].to_string();
        }
        if let Some(i) = rest.find('/') {
            return rest[i + 1..].to_string();
        }
    }
    s.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_prefix_forces_path() {
        let q = parse_query("file:sample-docs/data/データ移行概要設計.html", SearchMode::Auto);
        assert_eq!(q.mode, SearchMode::Path);
        assert_eq!(q.segments, vec!["sample-docs", "data", "データ移行概要設計.html"]);
    }

    #[test]
    fn plain_name_is_name_mode() {
        let q = parse_query("データ移行概要設計", SearchMode::Auto);
        assert_eq!(q.mode, SearchMode::Name);
        assert_eq!(q.tokens, vec!["データ移行概要設計"]);
    }

    #[test]
    fn ext_only_is_path_mode() {
        let q = parse_query("データ移行概要設計.html", SearchMode::Auto);
        assert_eq!(q.mode, SearchMode::Path);
    }

    #[test]
    fn windows_and_url_forms() {
        let q = parse_query(r#""sample-docs\data\a.html""#, SearchMode::Auto);
        assert_eq!(q.segments, vec!["sample-docs", "data", "a.html"]);
        let q = parse_query("http://127.0.0.1:8765/r/sample-docs/data/%E3%81%82.html", SearchMode::Auto);
        assert_eq!(q.segments, vec!["sample-docs", "data", "あ.html"]);
    }

    #[test]
    fn nfkc_normalizes() {
        let q = parse_query("ﾃﾞｰﾀ　ＡＢＣ", SearchMode::Auto);
        assert_eq!(q.tokens, vec!["データ", "abc"]);
    }
}
