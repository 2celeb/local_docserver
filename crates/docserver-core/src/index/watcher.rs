use crate::app::App;
use crate::config::RootSettings;
use notify::RecursiveMode;
use notify_debouncer_mini::{new_debouncer, DebounceEventResult, Debouncer};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info, warn};

/// 各ルートを監視し、変更があれば該当ルートのみ再スキャンする。
/// 戻り値の Debouncer を drop すると監視が止まる。
pub fn start(app: Arc<App>) -> Option<Debouncer<notify::RecommendedWatcher>> {
    let settings = app.settings();
    if !settings.watch || settings.roots.is_empty() {
        return None;
    }
    let roots = settings.roots.clone();
    let exts = settings.include_extensions.clone();
    let app2 = app.clone();
    let mut debouncer = match new_debouncer(Duration::from_millis(700), move |res: DebounceEventResult| {
        match res {
            Ok(events) => {
                let mut dirty: Vec<String> = vec![];
                for ev in events {
                    debug!(path = %ev.path.display(), kind = ?ev.kind, "fs event");
                    for r in &roots {
                        if dirty.contains(&r.name) || !ev.path.starts_with(&r.path) {
                            continue;
                        }
                        if is_relevant(&ev.path, r, &exts) {
                            dirty.push(r.name.clone());
                        }
                    }
                }
                for name in dirty {
                    info!(root = %name, "変更検知 → 再スキャン");
                    app2.rescan_root(&name);
                }
            }
            Err(e) => warn!(?e, "watch error"),
        }
    }) {
        Ok(d) => d,
        Err(e) => {
            warn!(?e, "watcher 起動失敗");
            return None;
        }
    };
    for r in &settings.roots {
        if r.path.is_dir() {
            if let Err(e) = debouncer.watcher().watch(&r.path, RecursiveMode::Recursive) {
                warn!(?e, root = %r.name, "watch 登録失敗");
            }
        }
    }
    Some(debouncer)
}

/// 再スキャンに値するイベントか。
/// - 除外ディレクトリ配下・.git 配下は無視
/// - 対象拡張子のファイル、または消失したパス（削除・リネーム元）のみ対象
/// - 既存ディレクトリへのイベント（走査時の atime 更新等）は無視 → 自己ループ防止
fn is_relevant(path: &Path, root: &RootSettings, exts: &[String]) -> bool {
    if let Ok(rel) = path.strip_prefix(&root.path) {
        for c in rel.components() {
            let name = c.as_os_str().to_string_lossy();
            if name == ".git" || root.exclude.iter().any(|x| x == name.as_ref()) {
                return false;
            }
        }
    }
    if !path.exists() {
        return true;
    }
    if path.is_dir() {
        return false;
    }
    path.extension()
        .map(|e| {
            let e = e.to_string_lossy();
            exts.iter().any(|x| x.eq_ignore_ascii_case(&e))
        })
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relevance_rules() {
        let tmp = tempfile::tempdir().unwrap();
        let root = RootSettings { name: "r".into(), path: tmp.path().to_path_buf(), exclude: vec!["node_modules".into()] };
        let exts = vec!["html".to_string(), "md".to_string()];
        std::fs::create_dir_all(tmp.path().join("d")).unwrap();
        std::fs::create_dir_all(tmp.path().join("node_modules/x")).unwrap();
        std::fs::write(tmp.path().join("d/a.html"), "").unwrap();
        std::fs::write(tmp.path().join("d/a.png"), "").unwrap();
        std::fs::write(tmp.path().join("node_modules/x/b.html"), "").unwrap();

        assert!(!is_relevant(&tmp.path().join("d"), &root, &exts), "既存ディレクトリは無視");
        assert!(is_relevant(&tmp.path().join("d/a.html"), &root, &exts));
        assert!(!is_relevant(&tmp.path().join("d/a.png"), &root, &exts));
        assert!(!is_relevant(&tmp.path().join("node_modules/x/b.html"), &root, &exts), "除外配下");
        assert!(is_relevant(&tmp.path().join("d/gone.html"), &root, &exts), "消失パス");
        assert!(is_relevant(&tmp.path().join("gone_dir"), &root, &exts), "消失ディレクトリ");
    }
}
