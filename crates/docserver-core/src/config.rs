use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub server: ServerSettings,
    pub roots: Vec<RootSettings>,
    pub include_extensions: Vec<String>,
    pub respect_gitignore: bool,
    pub watch: bool,
    pub max_depth: usize,
    /// 設定ファイルの実パス（保存時に使用。シリアライズ対象外）
    #[serde(skip)]
    pub config_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerSettings {
    pub host: String,
    pub port: u16,
    pub open_browser: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootSettings {
    pub name: String,
    pub path: PathBuf,
    #[serde(default)]
    pub exclude: Vec<String>,
}

impl Default for ServerSettings {
    fn default() -> Self {
        Self { host: "127.0.0.1".into(), port: 8765, open_browser: false }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            server: ServerSettings::default(),
            roots: vec![],
            include_extensions: ["html", "htm", "md", "markdown", "mmd", "mermaid"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            respect_gitignore: true,
            watch: true,
            max_depth: 20,
            config_path: None,
        }
    }
}

impl Settings {
    /// 設定ファイルを探索する。優先順: 明示パス → ./config/setting.yaml → 実行ファイル隣 → OS 設定ディレクトリ
    pub fn locate(explicit: Option<&Path>) -> Option<PathBuf> {
        let mut candidates: Vec<PathBuf> = vec![];
        if let Some(p) = explicit {
            candidates.push(p.to_path_buf());
        }
        if let Ok(cwd) = std::env::current_dir() {
            candidates.push(cwd.join("config/setting.yaml"));
            candidates.push(cwd.join("setting.yaml"));
        }
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                candidates.push(dir.join("config/setting.yaml"));
                candidates.push(dir.join("setting.yaml"));
            }
        }
        if let Some(cfg) = dirs_config_dir() {
            candidates.push(cfg.join("local_docserver/setting.yaml"));
        }
        candidates.into_iter().find(|p| p.is_file())
    }

    pub fn load(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("設定ファイルを読めません: {}", path.display()))?;
        let mut s: Settings =
            serde_yaml::from_str(&text).with_context(|| format!("YAML 解析エラー: {}", path.display()))?;
        let abs = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        s.config_path = Some(abs.clone());
        s.resolve_paths(abs.parent().unwrap_or(Path::new(".")));
        s.validate()?;
        Ok(s)
    }

    pub fn load_or_default(explicit: Option<&Path>) -> Result<Self> {
        match Self::locate(explicit) {
            Some(p) => Self::load(&p),
            None => Ok(Self::default()),
        }
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let text = serde_yaml::to_string(self)?;
        std::fs::write(path, text).with_context(|| format!("設定を書けません: {}", path.display()))
    }

    /// roots[].path を設定ファイルの位置基準で絶対パス化する
    pub fn resolve_paths(&mut self, base: &Path) {
        for r in &mut self.roots {
            if r.path.is_relative() {
                r.path = base.join(&r.path);
            }
            if let Ok(c) = r.path.canonicalize() {
                r.path = c;
            }
        }
    }

    pub fn validate(&self) -> Result<()> {
        let mut names = std::collections::HashSet::new();
        for r in &self.roots {
            anyhow::ensure!(!r.name.is_empty(), "roots[].name は必須です");
            anyhow::ensure!(
                !r.name.contains('/') && !r.name.contains('\\'),
                "roots[].name に / は使えません: {}",
                r.name
            );
            anyhow::ensure!(names.insert(r.name.clone()), "roots[].name が重複: {}", r.name);
        }
        Ok(())
    }

    pub fn ext_allowed(&self, ext: &str) -> bool {
        let e = ext.to_ascii_lowercase();
        self.include_extensions.iter().any(|x| x.eq_ignore_ascii_case(&e))
    }
}

fn dirs_config_dir() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("APPDATA").map(PathBuf::from)
    }
    #[cfg(target_os = "macos")]
    {
        std::env::var_os("HOME").map(|h| PathBuf::from(h).join("Library/Application Support"))
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_sample() {
        let y = r#"
server: { host: 127.0.0.1, port: 0 }
roots:
  - name: a
    path: ./x
  - name: b
    path: /abs
    exclude: [node_modules]
"#;
        let s: Settings = serde_yaml::from_str(y).unwrap();
        assert_eq!(s.roots.len(), 2);
        assert_eq!(s.server.port, 0);
        assert!(s.respect_gitignore);
        assert!(s.ext_allowed("HTML"));
        assert!(!s.ext_allowed("png"));
    }

    #[test]
    fn duplicate_names_rejected() {
        let y = "roots: [{name: a, path: x}, {name: a, path: y}]";
        let s: Settings = serde_yaml::from_str(y).unwrap();
        assert!(s.validate().is_err());
    }
}
