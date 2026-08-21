# local_docserver タスク一覧

- 版: 1.0（2026-08-21）
- 状態: **v0.1.0 の全タスク完了**（✅）
- 関連文書: [requirements.md](requirements.md)、[design.md](design.md)
- 検証: `cargo test -p docserver-core` 22 件 / `vitest` 2 件 / `npm run build` / `cargo check -p local-docserver` / sample-docs（96 ファイル）での実動作確認、すべて成功（2026-08-21）

凡例: 要件 ID は requirements.md、成果物は主なファイル。

---

## Phase 0: プロジェクト雛形 ✅

| # | タスク | 要件 | 成果物 | 状態 |
|---|---|---|---|---|
| 0-1 | Cargo ワークスペース作成（`crates/docserver-core` + `src-tauri`、`default-members` = core、release プロファイル lto/strip） | NFR-5 | `Cargo.toml` | ✅ |
| 0-2 | Vite + React 18 + TypeScript + MUI v6 のフロント雛形、`tsconfig`（strict・unused 検出） | FR-6 | `package.json`, `vite.config.ts`, `tsconfig.json`, `index.html`, `src/main.tsx` | ✅ |
| 0-3 | Tauri v2 シェル（`tauri.conf.json`、`build.rs`、`capabilities/default.json`、アイコン一式） | NFR-5, NFR-6 | `src-tauri/` | ✅ |
| 0-4 | 設定サンプル `config/setting.yaml`、`.gitignore`、README（構成・設定・検索構文・開発手順・API） | FR-1 | `config/setting.yaml`, `README.md` | ✅ |
| 0-5 | npm scripts 整備（`dev` / `build` / `test` / `tauri` / `core:test` / `core:serve`） | – | `package.json` | ✅ |
| 0-6 | Vite の開発プロキシ（`/api`, `/r` → `DOCSERVER_URL`）と `manualChunks`（mermaid / mui 分離） | FR-6.14 | `vite.config.ts` | ✅ |

## Phase 1: コア — 設定・インデックス・サーバ ✅

| # | タスク | 要件 | 成果物 | 状態 |
|---|---|---|---|---|
| 1-1 | `Settings` / `ServerSettings` / `RootSettings` 定義、`serde(default)` 既定値 | FR-1.1 | `config.rs` | ✅ |
| 1-2 | 設定ファイル探索（明示 → cwd → exe 隣 → OS 設定ディレクトリ）、OS 別ディレクトリ解決 | FR-1.2 | `config.rs::locate`, `dirs_config_dir` | ✅ |
| 1-3 | 読込・相対パス解決（設定ファイル基準）・canonicalize・バリデーション・保存 | FR-1.4, FR-1.5 | `config.rs::load/resolve_paths/validate/save` | ✅ |
| 1-4 | 未検出時のデフォルト起動 | FR-1.6 | `load_or_default` | ✅ |
| 1-5 | `FileKind` / `FileEntry`（NFC 正規化、NFKC 前処理、epoch ms） | FR-2.5, FR-2.6 | `index/mod.rs` | ✅ |
| 1-6 | `FileIndex`（RwLock + Arc スナップショット、generation、`replace` / `replace_root`） | FR-2.8, NFR-2 | `index/mod.rs` | ✅ |
| 1-7 | 走査（`ignore::WalkBuilder`、隠し・gitignore・exclude・拡張子・深さ・symlink 不追従） | FR-2.1〜2.4, 2.7 | `index/scanner.rs` | ✅ |
| 1-8 | `resolve_under_root`（`..`／非 Normal セグメント拒否） | FR-5.3 | `index/scanner.rs` | ✅ |
| 1-9 | `App`（設定 + インデックス、`rescan_all` / `rescan_root` / `update_settings` / `resolve`(NFD フォールバック)） | FR-5.4 | `app.rs` | ✅ |
| 1-10 | axum Router、`bind`（port 0 対応）/ `serve` | FR-5.1 | `server/mod.rs` | ✅ |
| 1-11 | `/r/{root}/{*path}` 静的配信（canonicalize + prefix 再検証、MIME/charset、no-cache） | FR-5.2, 5.3, 5.5 | `server/mod.rs::serve_file` | ✅ |
| 1-12 | JSON API: `/api/health` `/api/roots` `/api/files` `/api/raw` `/api/config`(GET/PUT) `/api/reload` | FR-5.6 | `server/mod.rs` | ✅ |
| 1-13 | `dist/` の `rust-embed` 埋め込み、SPA フォールバック、未ビルド時の案内 | FR-5.7 | `server/mod.rs::frontend` | ✅ |
| 1-14 | CORS permissive | FR-5.8 | `server/mod.rs` | ✅ |
| 1-15 | ヘッドレス CLI `docserver`（`--config` / `--port` / `--open` / `--search`） | FR-7 | `bin/docserver.rs` | ✅ |
| 1-16 | `lib.rs` 公開 API（`App`, `Settings`, `SearchMode`, `server`, `index::watcher`） | NFR-5 | `lib.rs` | ✅ |

## Phase 2: 検索 ✅

| # | タスク | 要件 | 成果物 | 状態 |
|---|---|---|---|---|
| 2-1 | `SearchMode` / `SearchQuery`、クエリ前処理（プレフィックス・引用符・`\`・URL デコード・サーバ URL・NFKC）、auto 判定、segments / tokens 生成 | FR-4.1〜4.3 | `search/query.rs` | ✅ |
| 2-2 | パス推測スコアリング（100 / 90 / 85 / 80 / 79 / 78 / 60 / 50 / 45）、fuzzy フォールバック（上限 40）、`CONFIDENT_SCORE` | FR-4.4 | `search/path_guess.rs` | ✅ |
| 2-3 | ファイル名検索（完全 / 前方 / 部分 / AND:stem / AND:path / path 部分 / fuzzy）、ハイライト範囲 | FR-4.5, 4.8 | `search/name_search.rs` | ✅ |
| 2-4 | 検索統合（path→name フォールバック、ソート、limit、`best` 判定）、`SearchResult` / `SearchResponse` | FR-4.6, 4.7, 4.9 | `search/mod.rs` | ✅ |
| 2-5 | テストフィクスチャ（2 ルート・8 エントリ・日本語 stem・同名 README） | NFR-7 | `test_util.rs` | ✅ |
| 2-6 | 受け入れケースの実リポジトリ確認（`file:sample-docs/data/データ移行概要設計.html` → exact:root+path 100、`best` あり） | FR-4 受け入れ | 手動確認 | ✅ |

## Phase 3: フロントエンド ✅

| # | タスク | 要件 | 成果物 | 状態 |
|---|---|---|---|---|
| 3-1 | 型定義（`FileEntry` / `SearchResult` / `SearchResponse` / `RootInfo` / `Settings`）、`fileKey` / `encodePath` | – | `api/types.ts` | ✅ |
| 3-2 | 通信層（Tauri invoke ⇄ HTTP フォールバック、`getServerBase`、`fileUrl`、`openExternal`、`revealInFolder`、`pickDirectory`） | FR-6.14, 6.15 | `api/client.ts` | ✅ |
| 3-3 | `useSearch`（180ms debounce、最新リクエストのみ採用）、`useLocalStorage` | FR-4.10, FR-6.11 | `hooks/` | ✅ |
| 3-4 | MUI テーマ（ライト／ダーク、日本語フォントスタック） | FR-6.11 | `theme.ts` | ✅ |
| 3-5 | `App`: AppBar・サイドバー（タブ自動切替・リサイズ・表示切替）・`/view/:root/*` ルーティング・`Ctrl/Cmd+K`・`best` 即オープン・Snackbar | FR-6.1〜6.3, 6.6, 6.12, 6.13 | `App.tsx`, `main.tsx` | ✅ |
| 3-6 | `SearchBar`（モード選択、Enter/Esc、クリア） | FR-6.2 | `components/SearchBar.tsx` | ✅ |
| 3-7 | `SearchResults` + `Highlight` + `KindIcon`（モードチップ・ハイライト・root チップ・スコア/理由） | FR-6.4 | `components/` | ✅ |
| 3-8 | `FileTree`（root→dir→file、日本語ソート、root 展開・件数） | FR-6.5 | `components/FileTree.tsx` | ✅ |
| 3-9 | `Viewer`（種別判定、ヘッダ操作: `file:` コピー／再読み込み／フォルダ／ブラウザ） | FR-6.7 | `components/viewers/Viewer.tsx` | ✅ |
| 3-10 | `HtmlViewer`（iframe + sandbox、相対参照解決） | FR-6.8 | `components/viewers/HtmlViewer.tsx` | ✅ |
| 3-11 | `MarkdownViewer`（GFM / raw HTML / slug / highlight、mermaid フェンス、相対リンクのアプリ内遷移、相対画像解決、ダーク対応スタイル） | FR-6.9 | `components/viewers/MarkdownViewer.tsx` | ✅ |
| 3-12 | `MermaidBlock`（動的 import、テーマ連動 initialize、エラー表示）/ `MermaidViewer`（図／ソース、SVG 保存） | FR-6.10 | `components/viewers/Mermaid*.tsx` | ✅ |
| 3-13 | `SettingsDialog`（roots 編集・ディレクトリ選択・拡張子・深さ・gitignore・watch・host/port → 保存して再スキャン） | FR-1.7 | `components/SettingsDialog.tsx` | ✅ |
| 3-14 | Vitest 環境（jsdom、jest-dom）と `resolveRelative` / `kindOf` テスト | NFR-7 | `src/test/` | ✅ |

## Phase 4: Tauri 統合・監視 ✅

| # | タスク | 要件 | 成果物 | 状態 |
|---|---|---|---|---|
| 4-1 | 変更監視（700ms debounce、ルート単位再スキャン、`is_relevant` による自己ループ防止） | FR-3 | `index/watcher.rs` | ✅ |
| 4-2 | Tauri シェル: 専用 tokio ランタイムでサーバ起動、`AppState`（app / server_url / watcher） | FR-5.1 | `src-tauri/src/lib.rs` | ✅ |
| 4-3 | IPC コマンド `server_url` / `search` / `list_files` / `get_settings` / `update_settings` / `reload` / `reveal` | FR-6.14 | `src-tauri/src/lib.rs` | ✅ |
| 4-4 | Tauri プラグイン（opener / dialog）と capabilities 設定 | FR-6.7, 6.15 | `capabilities/default.json` | ✅ |
| 4-5 | `tauri dev` 時の設定ファイル解決（cwd=`src-tauri/` 対策: `../config`・`CARGO_MANIFEST_DIR` 候補） | FR-1.3 | `lib.rs::project_config_path` | ✅ |
| 4-6 | `config/setting.yaml` をバンドルリソースに同梱、CSP 無効化（iframe 表示のため） | NFR-6, NFR-3 | `tauri.conf.json` | ✅ |
| 4-7 | Linux（WSL2）で Tauri 前提パッケージ導入 → `cargo check -p local-docserver` 成功 → `npm run tauri dev` で GUI 動作確認 | – | – | ✅ |

## Phase 5: ディレクトリ切り出し・整備 ✅（2026-08-21）

| # | タスク | 内容 | 状態 |
|---|---|---|---|
| 5-1 | 文書リポジトリ配下から独立したプロジェクトディレクトリへ切り出し | 元リポジトリに残骸なしを確認 | ✅ |
| 5-2 | `config/setting.yaml` / README の `roots[].path` を絶対パス記述に変更、不要な `exclude: ["local_docserver"]` を削除 | 移動後に相対パスが親ディレクトリ全体を指してしまう問題の解消 | ✅ |
| 5-3 | OS 設定ディレクトリ側の `setting.yaml` も絶対パス化 | OS 設定ディレクトリ基準で解決される相対パスの罠を回避 | ✅ |
| 5-4 | 切り出し後の全検証（core test 22 / vitest 2 / `npm run build` / `cargo check` Tauri / ヘッドレス起動で sample-docs 96 件・検索・配信・埋め込み UI を確認） | – | ✅ |
| 5-5 | 要件定義書・設計書・タスク一覧を `doc/` に整備 | `requirements.md`, `design.md`, `tasks.md` | ✅ |

---

## 完了条件と確認結果

| 確認項目 | 結果（2026-08-21） |
|---|---|
| `cargo test -p docserver-core` | 22 passed / 0 failed |
| `npx vitest run` | 2 passed |
| `npm run build`（tsc -b + vite build） | 成功（`dist/` 生成） |
| `cargo check -p local-docserver` | 成功 |
| ヘッドレス `docserver`: `/api/roots` | `sample-docs` → 絶対パス, exists=true, count=96 |
| `/api/search?q=file:sample-docs/data/データ移行概要設計.html` | 1 件, score=100, reason=`exact:root+path`, `best` あり |
| `/api/search?q=データ移行` | 3 件（prefix 90 ほか） |
| `/` `/assets/*` `/r/sample-docs/data/….html` | すべて 200 |
| `npm run tauri dev` | 起動・設定解決（`config/setting.yaml`）・検索・表示を確認 |

## 今後の候補（v0.1.0 スコープ外・未着手）

- 全文検索（tantivy + 日本語トークナイザ）／Markdown 見出し検索
- GitHub Actions による 3 OS ビルド・インストーラ生成（`tauri-action`）
- Windows（`\` パス・長いパス）／macOS（NFD ファイル名）の実機確認
- `watch` / `server` 設定変更のホットリロード
- Mermaid 図のズーム操作
