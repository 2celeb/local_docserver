# local_docserver 設計書

- 版: 1.0（2026-08-21、実装に基づく確定版）
- 関連文書: [requirements.md](requirements.md)（要件）、[tasks.md](tasks.md)（タスク一覧）

---

## 1. 技術スタック

| レイヤ | 採用 | 備考 |
|---|---|---|
| デスクトップシェル | Tauri v2 | `tauri-plugin-opener`（外部ブラウザ／フォルダ）, `tauri-plugin-dialog`（ディレクトリ選択） |
| コア（Rust） | `docserver-core` crate | Tauri 非依存。CLI とアプリ双方から利用 |
| HTTP | axum 0.8 + tower-http 0.6 (cors) | tokio マルチスレッドランタイム |
| 走査／監視 | `ignore` 0.4, `notify` 8 + `notify-debouncer-mini` 0.6 | gitignore 尊重、700ms デバウンス |
| 検索 | 自前スコアリング + `fuzzy-matcher` (SkimMatcherV2) | `unicode-normalization` で NFC/NFKC |
| 設定 | `serde_yaml` 0.9 | YAML ⇄ `Settings` |
| 埋め込み | `rust-embed` 8 | `dist/` をバイナリへ |
| CLI | `clap` 4 | `docserver` バイナリ |
| フロント | React 18 + TypeScript 5 + Vite 8 (rolldown) | Tauri 標準構成 |
| UI | MUI v6 (`@mui/material`, `@mui/icons-material`, `@mui/x-tree-view`) | ライト／ダーク |
| Markdown | `react-markdown` 9 + `remark-gfm` + `rehype-raw` + `rehype-slug` + `rehype-highlight` | |
| Mermaid | `mermaid` 11（動的 import） | ```` ```mermaid ```` と `.mmd` |
| ルーティング | `react-router-dom` 7 | `/view/:root/*` |
| テスト | `cargo test` (+`tempfile`), Vitest 4 + jsdom + Testing Library | |

---

## 2. 全体アーキテクチャ

```
┌───────────────────────────── Tauri App (local-docserver) ─────────────────────────────┐
│  WebView: React + MUI (src/)                                                         │
│   ├ AppBar: SearchBar / URL chip / reload / theme / settings                          │
│   ├ Sidebar: SearchResults | FileTree                                                │
│   └ Viewer: HtmlViewer(iframe) / MarkdownViewer / MermaidViewer                      │
│        │ invoke (IPC)                │ fetch / iframe src (HTTP)                      │
│        ▼                             ▼                                               │
│  src-tauri/src/lib.rs            http://127.0.0.1:<port>                             │
│   commands: server_url, search,  ┌──────────── docserver-core ────────────────────┐  │
│   list_files, get_settings,      │ server/   axum Router                          │  │
│   update_settings, reload,       │   /r/{root}/{*path}  静的配信                  │  │
│   reveal                         │   /api/*             JSON API                  │  │
│        │                         │   fallback           埋め込み dist/ (SPA)     │  │
│        └────────► App ◄──────────┤ app.rs    Settings(RwLock) + FileIndex          │  │
│                                  │ index/    scanner(ignore) / watcher(notify)    │  │
│                                  │ search/   query → path_guess | name_search     │  │
│                                  │ config.rs 探索・読込・保存・パス解決            │  │
│                                  └────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────────────────────────┘

  ヘッドレス: crates/docserver-core/src/bin/docserver.rs が同じ App + server を起動
  ブラウザ:   http://127.0.0.1:<port>/ で同じ React UI（IPC の代わりに /api/* を使用）
```

### 2.1 ディレクトリ構成

```
local_docserver/
├─ Cargo.toml                 # workspace: crates/docserver-core, src-tauri（default-members = core）
├─ config/setting.yaml        # 設定（Tauri バンドル時はリソース同梱）
├─ crates/docserver-core/
│   └─ src/
│       ├─ lib.rs             # pub use App, Settings, SearchMode …
│       ├─ app.rs             # App: 設定 + インデックス + 再スキャン + パス解決
│       ├─ config.rs          # Settings / ServerSettings / RootSettings
│       ├─ index/{mod,scanner,watcher}.rs
│       ├─ search/{mod,query,path_guess,name_search}.rs
│       ├─ server/mod.rs      # axum Router + rust-embed
│       ├─ test_util.rs       # テスト用フィクスチャ（cfg(test)）
│       └─ bin/docserver.rs   # ヘッドレス CLI
├─ src-tauri/
│   ├─ Cargo.toml, build.rs, tauri.conf.json, capabilities/default.json, icons/
│   └─ src/{main,lib}.rs      # Tauri シェル・IPC コマンド
├─ src/                       # React
│   ├─ main.tsx, App.tsx, theme.ts
│   ├─ api/{client,types}.ts
│   ├─ hooks/{useSearch,useLocalStorage}.ts
│   ├─ components/{SearchBar,SearchResults,FileTree,SettingsDialog,KindIcon,Highlight}.tsx
│   ├─ components/viewers/{Viewer,HtmlViewer,MarkdownViewer,MermaidViewer,MermaidBlock}.tsx
│   └─ test/{setup.ts,markdown.test.ts}
├─ dist/                      # npm run build の出力（core に埋め込み）
├─ index.html, vite.config.ts, tsconfig.json, package.json
└─ doc/{plan,requirements,design,tasks}.md
```

---

## 3. データモデル

### 3.1 Settings（`config.rs`）

```rust
pub struct Settings {
    pub server: ServerSettings,        // host="127.0.0.1", port=8765, open_browser=false
    pub roots: Vec<RootSettings>,      // name, path(PathBuf), exclude: Vec<String>
    pub include_extensions: Vec<String>, // ["html","htm","md","markdown","mmd","mermaid"]
    pub respect_gitignore: bool,       // true
    pub watch: bool,                   // true
    pub max_depth: usize,              // 20
    #[serde(skip)] pub config_path: Option<PathBuf>, // 読み込んだ実パス（保存先）
}
```

- `#[serde(default)]` により未指定項目は既定値。
- `load()`: 読込 → `config_path` を canonicalize で確定 → `resolve_paths(config_dir)` → `validate()`。
- `resolve_paths(base)`: 相対 `path` を `base.join()` し、存在すれば canonicalize。
- `validate()`: name 必須・`/`/`\` 禁止・重複禁止。
- `dirs_config_dir()`: OS 別（`APPDATA` / `~/Library/Application Support` / `$XDG_CONFIG_HOME` or `~/.config`）。

### 3.2 FileEntry / FileIndex（`index/mod.rs`）

```rust
pub enum FileKind { Html, Markdown, Mermaid, Other }   // serde: lowercase

pub struct FileEntry {
    pub root: String,        // roots[].name
    pub rel_path: String,    // '/' 区切り, NFC
    #[serde(skip)] pub abs_path: PathBuf,
    pub file_name: String,   // NFC
    pub stem: String,        // 拡張子なし
    pub ext: String,         // lowercase
    pub kind: FileKind,
    pub size: u64,
    pub modified: u64,       // epoch millis
    #[serde(skip)] pub norm_stem: String,          // NFKC + lowercase
    #[serde(skip)] pub norm_segments: Vec<String>, // rel_path の各セグメントを NFKC + lowercase
}

pub struct FileIndex {
    entries: RwLock<Arc<Vec<FileEntry>>>,  // root, rel_path でソート済み
    generation: RwLock<u64>,
}
```

- `replace(entries)`: 全置換 + generation++。
- `replace_root(root, entries)`: 該当 root 以外を残して差し替え（watch の部分再スキャン）。
- `snapshot()`: `Arc` クローン。検索・配信はスナップショットに対して行うため、再スキャン中もロック待ちしない。
- `full_key()`: `"{root}/{rel_path}"`。UI の選択キー／`best` に使用。

### 3.3 SearchQuery / SearchResult（`search/`）

```rust
pub enum SearchMode { Auto, Path, Name }   // serde: lowercase, Default=Auto

pub struct SearchQuery {
    pub raw: String, pub mode: SearchMode,   // mode は Auto 解決済み
    pub text: String,                        // 正規化済み全文
    pub segments: Vec<String>,               // '/' 区切り（Path 用）
    pub tokens: Vec<String>,                 // 空白/全角空白/'/' 区切り（Name 用）
}

pub struct SearchResult { #[serde(flatten)] entry: FileEntry, score: i64, reason: String, highlights: Vec<(usize,usize)> }
pub struct SearchResponse { query, mode, total, results: Vec<SearchResult>, best: Option<String> }
```

---

## 4. コア処理設計

### 4.1 App（`app.rs`）

| メソッド | 処理 |
|---|---|
| `new(settings) -> Arc<App>` | 設定を `RwLock` に保持、空インデックス |
| `settings()` | 設定のクローン |
| `update_settings(s)` | `validate` → `config_path` があれば `save` → 差し替え → `rescan_all` |
| `rescan_all()` | `scanner::scan_all` → `index.replace`。所要時間を INFO ログ |
| `rescan_root(name)` | `scanner::scan_root` → `index.replace_root` |
| `search(q, mode, limit)` | スナップショットに対し `search::search` |
| `files(root?)` | スナップショットをフィルタ |
| `resolve(root, rel)` | `resolve_under_root` で安全に結合 → 存在すれば返す → 無ければ NFC 化 `rel` でインデックス照合（NFD 対策） |
| `root_path(root)` / `is_under_any_root(p)` | 配信時の再検証用 |

### 4.2 走査（`index/scanner.rs`）

```
scan_root(settings, root):
  root.path が dir でなければ warn → []
  WalkBuilder(root.path)
    .hidden(true)                                 # 隠しファイル除外
    .git_ignore/.git_global/.git_exclude(respect_gitignore)
    .follow_links(false).max_depth(max_depth)
    .filter_entry(name ∉ root.exclude)            # ディレクトリ名単位で配下ごと除外
  for entry: is_file && ext_allowed(ext) → FileEntry::new(...)
```

`resolve_under_root(root_path, rel)`: `rel` を `/` で分割し、空・`.` はスキップ、`..` は拒否、`Component::Normal` 以外（ドライブレター等）を含むセグメントは拒否してから `push`。

### 4.3 監視（`index/watcher.rs`）

```
start(app) -> Option<Debouncer>:
  watch=false or roots 空 → None
  new_debouncer(700ms, |events|
      dirty = { root | ev.path ⊂ root.path && is_relevant(ev.path, root, exts) }
      for root in dirty: app.rescan_root(root))
  各 root.path を RecursiveMode::Recursive で登録

is_relevant(path, root, exts):
  rel の途中に ".git" or exclude 名 → false
  !path.exists()        → true   (削除・リネーム元)
  path.is_dir()         → false  (自己ループ防止)
  ext ∈ exts            → true
```

戻り値の `Debouncer` を保持している間だけ監視が続く（Tauri は `AppState._watcher`、CLI は `_watcher` ローカル変数）。

### 4.4 クエリ解釈（`search/query.rs`）

```
parse_query(raw, mode):
  1. trim
  2. 先頭 "file:" | "path:" | "file://"（大文字小文字無視）を除去 → forced_path=true
  3. 前後の " ' ` < > を除去、'\' → '/'
  4. percent-decode（失敗時は元のまま）
  5. strip_server_url: "http(s)://host.../r/X" → "X"、それ以外の URL は最初の '/' 以降
  6. 先頭 "./" と前後 '/' を除去
  7. text = NFKC + lowercase
  8. looks_path = forced_path || text に '/' || 対象拡張子で終わる
  9. mode=Auto なら looks_path ? Path : Name
  10. segments = text.split('/')（空・"." 除去）、tokens = 空白/'　'/'/' で分割
```

### 4.5 パス推測（`search/path_guess.rs`）

`segs` = クエリセグメント、`es` = エントリの `norm_segments`、`root_norm` = NFKC(root 名)。最初に成立した規則のスコアを採用:

| # | 条件 | score | reason |
|---|---|---|---|
| 1 | `segs == [root_norm] + es` | 100 | `exact:root+path` |
| 2 | `segs == es` | 90 | `exact:path` |
| 3 | `es` の末尾が `segs` に一致（ディレクトリ境界） | 80 | `suffix:path` |
| 4 | `segs[0]==root_norm` かつ `es` の末尾が `segs[1..]` | 85 | `suffix:root+path` |
| 4b | `segs[start..]`（start≥1）が `es` の末尾に一致 / その先頭が root 名 | 78 / 79 | `suffix:partial` / `suffix:partial+root` |
| 5 | ディレクトリ部分一致 かつ stem 一致 かつ 最終セグメント（拡張子）不一致 | 60 | `suffix:ext-mismatch` |
| 6 | 最終セグメント（ファイル名）一致 | 50 | `exact:filename` |
| 7 | stem 一致 | 45 | `exact:stem` |

※ 規則 3 が 4 より先に評価されるため、`sample-docs/data/x.html` のように root 名を含む入力でも `es` が `[data, x.html]` なら規則 1 が先に成立する。規則 4 は `root名/余分/.../x.html` のような形で効く。

0 件の場合: 最終セグメントの stem を `name_search` に渡し、スコアを 40 に頭打ち、reason に `fuzzy:` を前置。

`CONFIDENT_SCORE = 80`。`search/mod.rs` で「Path モード・先頭が 80 以上・2 位より高スコア」のとき `best = full_key`。

### 4.6 ファイル名検索（`search/name_search.rs`）

`joined` = tokens を連結、`stem` = `norm_stem`、`full` = `norm_segments.join("/")`:

| 順 | 条件 | score | reason | highlights |
|---|---|---|---|---|
| 1 | `stem == joined` | 100 | `exact` | stem 全体 |
| 2 | `stem.starts_with(joined)` | 90 | `prefix` | 先頭〜len |
| 3 | `stem.contains(joined)` | 80 − min(開始位置, 20) | `contains` | 該当範囲 |
| 4 | tokens ≥2 かつ 全 token ⊂ stem | 70 − min(余分文字数/3, 9) | `and:stem` | 各 token |
| 5 | tokens ≥2 かつ 全 token ⊂ full | 60 | `and:path` | なし |
| 6 | `full.contains(joined)` | 50 | `contains:path` | なし |
| 7 | joined ≥2 文字、`SkimMatcherV2.fuzzy_indices(stem, joined)` > 0 | `clamp(s / (len×16) × 40, 1, 40)` | `fuzzy` | 一致文字ごと |

### 4.7 検索統合（`search/mod.rs`）

```
search(entries, raw, mode, limit):
  q = parse_query
  results = Path ? path_guess : name_search
  if results 空 && Path → name_search(strip_ext(最終セグメント)) , mode_used = Name
  sort: score desc → modified desc → rel_path asc
  total = len; truncate(limit)
  best = (mode_used==Path && top.score ≥ 80 && top.score > 2nd.score) ? top.full_key : None
```

---

## 5. HTTP サーバ設計（`server/mod.rs`）

### 5.1 ルーティング

| Method | Path | ハンドラ | 応答 |
|---|---|---|---|
| GET | `/api/health` | `health` | `{ok, version}` |
| GET | `/api/roots` | `roots` | `{roots:[{name,path,exists,count}], generation}` |
| GET | `/api/files?root=` | `files` | `{files:[FileEntry], generation}` |
| GET | `/api/search?q=&mode=auto\|path\|name&limit=` | `search` | `SearchResponse`（limit 既定 50） |
| GET | `/api/raw?root=&path=` | `raw` | `text/plain; charset=utf-8`（404 if 無し） |
| GET | `/api/config` | `get_config` | `{config_path, settings}` |
| PUT | `/api/config` | `put_config` | body=`Settings` → `config_path` を現行値で上書き → `resolve_paths(config_dir)` → `App::update_settings` → `{ok,count}` / 400 |
| POST | `/api/reload` | `reload` | `spawn_blocking(rescan_all)` → `{ok,count,generation}` |
| GET | `/r/{root}/{*path}` | `serve_file` | 静的配信 |
| * | その他 | `frontend` | 埋め込み `dist/` |

全体に `CorsLayer::permissive()`。

### 5.2 静的配信の安全性

```
serve_file(root, path):
  p = app.resolve(root, path)            # ".." 拒否・Normal 以外拒否・NFD フォールバック
  real = p.canonicalize()  (失敗→404)
  root_real = root_path.canonicalize()
  !real.starts_with(root_real) || !real.is_file() → 403
  Content-Type: mime_guess（text/* は +charset=utf-8）, Cache-Control: no-cache
```

### 5.3 フロントエンド埋め込み

- `#[derive(rust_embed::Embed)] #[folder = "$CARGO_MANIFEST_DIR/../../dist/"]`
- `frontend(uri)`: 空パス → `index.html`。存在しないパスは `assets/` 始まりなら 404、それ以外は `index.html`（SPA）。`index.html` を返すときは `text/html; charset=utf-8`。
- `dist/` が未ビルドなら 404 + 「`npm run build` 後に再ビルドしてください」。
- デバッグビルドでは rust-embed がファイルシステムを動的に読むため、`npm run build` 後に `cargo build` し直すか、バイナリが `dist/` 生成後に起動されている必要がある。

### 5.4 起動シーケンス

```
bind(app) → (SocketAddr, TcpListener)   # port=0 のとき実ポートを得る
serve(app, listener)                    # axum::serve
```

- **CLI**: `Settings::load_or_default(--config)` → `--port` 上書き → `App::new` → `rescan_all` → (`--search` なら JSON 出力して終了) → `watcher::start` → `bind` → `listening on` 出力 → (`--open`/`open_browser` でブラウザ) → `serve`
- **Tauri**: `project_config_path()` を明示パスとして `load_or_default` → `App::new` → `rescan_all` → `setup`: 専用 tokio ランタイムで `bind` → 別スレッドで `serve` → `watcher::start` → `AppState{app, server_url, _watcher}` を manage → invoke_handler 登録

---

## 6. Tauri シェル設計（`src-tauri/`）

### 6.1 IPC コマンド

| コマンド | 引数 | 戻り値 | 備考 |
|---|---|---|---|
| `server_url` | – | `Option<String>` | `http://127.0.0.1:<port>/` |
| `search` | `q, mode?, limit?` | `SearchResponse` (JSON) | limit 既定 50 |
| `list_files` | `root?` | `Vec<FileEntry>` | |
| `get_settings` | – | `Settings` | |
| `update_settings` | `settings` | `usize`（件数） | `config_path` を現行 or `default_config_path()` に、`resolve_paths` 後 `App::update_settings` |
| `reload` | – | `usize` | 同期で `rescan_all` |
| `reveal` | `root, path` | `()` | 親ディレクトリを `open::that` |

### 6.2 設定ファイル解決

`project_config_path()`: `cwd/config/setting.yaml` → `cwd/../config/setting.yaml` → （`debug_assertions` 時）`CARGO_MANIFEST_DIR/../config/setting.yaml` の最初に存在するもの。見つかれば `Settings::locate` の明示パスとして渡す（`tauri dev` の cwd が `src-tauri/` になる問題への対処）。

### 6.3 tauri.conf.json 要点

- `beforeDevCommand: npm run dev` / `devUrl: http://localhost:1420` / `beforeBuildCommand: npm run build` / `frontendDist: ../dist`
- ウィンドウ 1400×900（最小 900×600）
- `security.csp: null`（iframe で任意ローカル HTML を表示するため）
- `bundle.resources: ["../config/setting.yaml"]`、`targets: all`
- capabilities: `core:default`, `opener:default`, `dialog:default`

---

## 7. フロントエンド設計（`src/`）

### 7.1 通信層（`api/client.ts`）

- `isTauri()`: `window.__TAURI_INTERNALS__` の有無。
- `getServerBase()`: Tauri では `invoke("server_url")` をキャッシュ、ブラウザでは `""`（同一オリジン。Vite dev は `/api`,`/r` を `DOCSERVER_URL`（既定 `http://127.0.0.1:8765`）へプロキシ）。
- `search / listFiles / getSettings / updateSettings / reload`: Tauri → invoke、ブラウザ → `/api/*`。
- `listRoots / fetchRaw / fileUrl`: 常に HTTP。`fileUrl` は `${base}/r/${encodeURIComponent(root)}/${encodePath(rel)}`（セグメント単位エンコード）。
- `openExternal`: Tauri → `opener.openUrl`、ブラウザ → `window.open`。`revealInFolder`: Tauri のみ。`pickDirectory`: Tauri → `dialog.open({directory:true})`、ブラウザ → `prompt`。

### 7.2 状態とルーティング（`App.tsx`）

| 状態 | 保持先 | 用途 |
|---|---|---|
| `theme` (`light`/`dark`) | localStorage | `makeTheme(mode)` |
| `sidebarWidth` (240–700) | localStorage | ドラッグリサイズ |
| `query`, `searchMode` | state | `useSearch(query, mode)`（180ms debounce, 最新リクエストのみ採用, limit 100） |
| `files`, `roots` | state | 起動時・再スキャン後・設定保存後に `refresh()` |
| `tab` (0=検索結果, 1=ファイル) | state | `query` の有無で自動切替 |
| 現在文書 | URL `/view/<root>/<rel>` | `parseViewPath` で `{root, rel}` を復元（セグメントごとに decodeURIComponent） |

- `data.best && data.mode==="path"` で `navigate(/view/...)`（確信ヒットの即オープン）。
- `Ctrl/Cmd+K` グローバルショートカットで検索窓にフォーカス＆全選択。
- Snackbar: サーバ未接続、URL コピー、再スキャン件数、設定保存件数。

### 7.3 コンポーネント

| コンポーネント | 責務 |
|---|---|
| `SearchBar` | TextField + モード Select（auto/path/name）。Enter で `onSubmit`、Esc でクリア、クリアボタン |
| `SearchResults` | モードチップ、件数、`ListItemButton` 一覧（`KindIcon`、`Highlight` で stem 強調、root チップ、`rel_path`、score（title=reason）） |
| `Highlight` | `highlights: [start,end)[]`（char index）を `<mark>` 相当で強調 |
| `FileTree` | `FileEntry[]` → `Map` ベースのツリー構築 → `SimpleTreeView`。ディレクトリ先・`localeCompare("ja")`、root 既定展開、root に葉数表示。`selectedItems` は `fileKey` |
| `KindIcon` | `FileKind` → アイコン |
| `SettingsDialog` | `getSettings` → 編集（roots 行: 名前・パス・ディレクトリ選択・除外・削除／追加、拡張子、最大深さ、gitignore、watch、host、port）→ `updateSettings` → `onSaved(count)` |
| `Viewer` | 拡張子から `kindOf(rel)` を判定し、ヘッダ（`file:` コピー／再読み込み(nonce で再マウント)／フォルダを開く／ブラウザで開く）と種別ビューアを表示 |
| `HtmlViewer` | `fileUrl` を `iframe` で表示。`sandbox="allow-scripts allow-same-origin allow-popups allow-forms allow-modals"`、白背景 |
| `MarkdownViewer` | `fetchRaw` → `ReactMarkdown`（remark-gfm / rehype-raw / rehype-slug / rehype-highlight）。`code` の `language-mermaid` → `MermaidBlock`、`a` の相対リンク → `resolveRelative(rel, href)` でアプリ内遷移（外部・`#` は通常リンク）、`img` の相対 src → 配信 URL ベースに解決。GitHub 風スタイル（ダーク対応） |
| `MermaidViewer` | `fetchRaw` → 図／ソース切替、`#mermaid-viewer svg` を Blob で SVG 保存 |
| `MermaidBlock` | `mermaid` を動的 import、テーマ（default/dark）ごとに 1 回 `initialize`（`securityLevel: loose`）。`render(uniqueId, code)` → innerHTML + `bindFunctions`。失敗時は Alert + ソース表示 |

### 7.4 ビルド設定

- `vite.config.ts`: ポート 1420 固定（`strictPort`）、`TAURI_DEV_HOST` 対応、`/api`・`/r` をバックエンドへプロキシ、`manualChunks`（関数形式）で `mermaid` と `mui` を分離、Vitest は jsdom + `src/test/setup.ts`（jest-dom）。
- `tsconfig.json`: strict、`noUnusedLocals/Parameters`、`types: [vitest/globals, @testing-library/jest-dom]`。
- `package.json` scripts: `dev`, `build`(tsc -b && vite build), `preview`, `test`(vitest run), `tauri`, `core:test`, `core:serve`。

---

## 8. 横断的関心事

### 8.1 文字コード・正規化の方針

| 段階 | 正規化 | 理由 |
|---|---|---|
| インデックス `rel_path` / `file_name` | NFC | macOS（NFD）と他 OS の表記を統一して URL・キーを安定化 |
| インデックス `norm_stem` / `norm_segments` | NFKC + lowercase | 全角英数・半角カナ・大文字小文字の揺れを検索で吸収 |
| クエリ | `\`→`/`、percent-decode、NFKC + lowercase | Windows パス・URL 貼り付け対応 |
| 配信時のパス解決 | 実パス優先 → NFC でインデックス照合 | NFD ファイルシステム上でも NFC URL で到達 |
| URL 生成（フロント） | セグメントごとに `encodeURIComponent` | 日本語・括弧・空白を安全に運ぶ |

### 8.2 セキュリティ境界

- バインド既定 `127.0.0.1`。`host` は設定で変更可能だが LAN 公開は想定外。
- 配信はルート配下のみ。`..`／非 Normal セグメント拒否 → canonicalize 後に prefix 再検証で symlink 越えも拒否。
- WebView の CSP は無効化、iframe は sandbox 属性で最低限の隔離。Mermaid は `loose`。いずれも「ローカルの信頼済み文書」を前提とした意図的な選択。

### 8.3 ログ

`tracing` + `tracing-subscriber`（`RUST_LOG` 既定 `info`）。設定読込パス、全スキャン件数/所要時間、HTTP バインドアドレス、変更検知→再スキャン、watcher 失敗を記録。

---

## 9. テスト設計

| 種別 | 対象 | ケース |
|---|---|---|
| Rust unit (`config`) | YAML 解析・既定値・拡張子判定・重複 name 拒否 | 2 |
| Rust unit (`scanner`) | 対象拡張子のみ・exclude・NFC/セグメント、トラバーサル拒否 | 2 |
| Rust unit (`watcher`) | `is_relevant` の 6 ルール | 1 |
| Rust unit (`query`) | `file:` 強制、name 判定、拡張子で path 判定、Windows/URL 形式、NFKC | 5 |
| Rust unit (`path_guess`) | root+path / rel / 上位余分 / root 名違い / 拡張子違い / 複数 root 同名 / fuzzy フォールバック | 7 |
| Rust unit (`name_search`) | 完全→前方 / contains / AND / 半角カナ / fuzzy | 5 |
| Vitest | `resolveRelative`、`kindOf` | 2 |
| 手動 (実リポジトリ) | sample-docs 96 件で `/api/roots`・`/api/search`・`/r/...`・`/` を確認、Tauri dev で GUI 確認 | – |

フィクスチャ（`test_util.rs`）: `sample-docs` と `other` の 2 ルート、日本語 stem・`README.md` 重複を含む 8 エントリ。
