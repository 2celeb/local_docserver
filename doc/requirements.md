# local_docserver 要件定義書

- 版: 1.0（2026-08-21、実装済み機能に基づく確定版）
- 対象: `local_docserver` v0.1.0
- 関連文書: [design.md](design.md)（設計）、[tasks.md](tasks.md)（タスク一覧）、[plan.md](plan.md)（初期計画）

---

## 1. 背景と目的

複数のリポジトリ／ディレクトリに散在する仕様書（HTML / Markdown / Mermaid）を、
ローカル PC 上で **1 つの検索窓から横断検索し、その場で閲覧できる** デスクトップアプリを提供する。

主対象の例は `sample-docs` リポジトリ（HTML 仕様書 約 96 本、`ディレクトリ/画面名_種別.html` のフラット構成、
`../assets/style.css` / `../assets/app.js` を相対参照）だが、設定で任意のリポジトリを複数登録できる汎用ツールとする。

### 解決したい課題

| 課題 | 要求 |
|---|---|
| 「`file:sample-docs/data/データ移行概要設計.html` を見て」のような参照を受け取っても、探してブラウザで開くのが手間 | パス文字列を貼り付けるだけで該当ファイルが即座に開く |
| ファイル名の一部しか覚えていない／全角半角の揺れがある | 部分一致・AND・fuzzy・NFKC 正規化で見つかる |
| HTML 仕様書が相対パスで CSS/JS を参照しており、単体で開くと崩れる | ディレクトリ構造を保ったまま配信し、相対参照をそのまま解決する |
| Markdown / Mermaid はレンダリングしないと読みにくい | アプリ内で GFM・Mermaid を描画する |
| 他のブラウザやツールからも同じ文書を参照したい | ローカル HTTP サーバとして公開し、URL を共有できる |

---

## 2. スコープ

### 2.1 対象

- 対象 OS: Windows / macOS / Linux（Tauri v2）
- 対象ファイル種別（既定）: `html, htm, md, markdown, mmd, mermaid`（設定で変更可）
- 利用形態
  - **デスクトップアプリ**（Tauri + React + MUI）
  - **ヘッドレス CLI**（`docserver` バイナリ。HTTP サーバのみ起動し、任意ブラウザで利用）
- 利用者: ローカル PC 上の本人（信頼できる文書を閲覧する前提）

### 2.2 対象外（v0.1.0）

- 全文検索（本文の語句検索）・Markdown 見出し検索
- 認証・LAN 公開を前提とした多人数利用
- 文書の編集機能
- Tauri の 3 OS 実機インストーラ配布（CI 未整備。Linux 上の `tauri dev` で動作確認済み）

---

## 3. 用語

| 用語 | 意味 |
|---|---|
| ルート (root) | 公開対象のリポジトリ／ディレクトリ 1 つ。`name`（表示名・URL プレフィックス）と `path` を持つ |
| `rel_path` | ルートからの相対パス（`/` 区切り、NFC 正規化済み） |
| `file:` 参照 | `file:<root名>/<rel_path>` 形式の文書参照文字列。アプリからコピーでき、検索窓に貼り付けて開ける |
| パス推測モード | 入力をパスとして解釈し、ルート名・ディレクトリ・拡張子の一致度でスコアリングする検索 |
| ファイル名モード | 入力をファイル名の一部として解釈し、完全／前方／部分／AND／fuzzy で検索する |
| stem | 拡張子を除いたファイル名 |

---

## 4. 機能要件

### FR-1 設定

| ID | 要件 | 実装 |
|---|---|---|
| FR-1.1 | 設定は YAML ファイル `setting.yaml` で管理する。項目: `server.{host,port,open_browser}`, `roots[].{name,path,exclude[]}`, `include_extensions[]`, `respect_gitignore`, `watch`, `max_depth` | `config.rs` |
| FR-1.2 | 設定ファイルの探索順: `--config` 明示 → `./config/setting.yaml` → `./setting.yaml` → 実行ファイル隣 → OS 設定ディレクトリ（Linux `~/.config/local_docserver/`、macOS `~/Library/Application Support/local_docserver/`、Windows `%APPDATA%\local_docserver\`） | `Settings::locate` |
| FR-1.3 | Tauri アプリは `tauri dev` 時に cwd が `src-tauri/` になるため、`cwd/config`、`cwd/../config`、（デバッグ時）`CARGO_MANIFEST_DIR/../config` も探索する | `lib.rs::project_config_path` |
| FR-1.4 | `roots[].path` が相対パスの場合、**設定ファイルの位置基準**で絶対パス化し canonicalize する | `Settings::resolve_paths` |
| FR-1.5 | `roots[].name` は必須・`/` `\` 禁止・重複禁止。違反時は起動エラーまたは保存拒否 | `Settings::validate` |
| FR-1.6 | 設定ファイルが見つからない場合はデフォルト設定（roots 空）で起動し、警告を出す | `load_or_default` |
| FR-1.7 | GUI の設定ダイアログからルート追加／削除／ディレクトリ選択、拡張子、深さ、gitignore、watch、host/port を編集し、保存すると YAML に書き戻して全再スキャンする | `SettingsDialog.tsx`, `PUT /api/config`, `update_settings` |
| FR-1.8 | 保存先は読み込んだ設定ファイル。未検出で起動した場合は `./config/setting.yaml`（Tauri はプロジェクト設定優先） | `put_config`, `update_settings` |

### FR-2 インデックス（走査）

| ID | 要件 | 実装 |
|---|---|---|
| FR-2.1 | 各ルートを再帰走査し、`include_extensions` に含まれる拡張子のファイルのみインデックスする（大文字小文字無視） | `scanner.rs` |
| FR-2.2 | 隠しファイル／ディレクトリは除外。`respect_gitignore=true` なら `.gitignore` / global gitignore / `.git/info/exclude` を尊重 | `ignore::WalkBuilder` |
| FR-2.3 | `roots[].exclude` に列挙したディレクトリ名（またはファイル名）は、その配下ごと除外する | `filter_entry` |
| FR-2.4 | シンボリックリンクは辿らない。`max_depth` で深さ制限 | 同上 |
| FR-2.5 | 各エントリは `root, rel_path, file_name, stem, ext, kind(html/markdown/mermaid/other), size, modified(ms)` を持つ。`rel_path` と `file_name` は NFC 正規化 | `FileEntry::new` |
| FR-2.6 | 検索用に `stem` と各パスセグメントを NFKC + lowercase で前処理して保持する | `norm_stem`, `norm_segments` |
| FR-2.7 | ルートが存在しない場合は警告を出し、そのルートは 0 件として扱う（起動は継続） | `scan_root` |
| FR-2.8 | インデックスは世代番号（generation）を持ち、入れ替えごとに増加する | `FileIndex` |

### FR-3 変更監視

| ID | 要件 | 実装 |
|---|---|---|
| FR-3.1 | `watch=true` のとき各ルートを再帰監視し、変更を 700ms デバウンスして**該当ルートのみ**再スキャンする | `watcher.rs` |
| FR-3.2 | 反応する変更: 対象拡張子のファイルの作成／更新、または消失したパス（削除・リネーム元）。`.git` 配下・`exclude` 配下は無視 | `is_relevant` |
| FR-3.3 | 既存ディレクトリ自身へのイベント（走査時の atime 更新等）は無視し、再スキャン→イベント→再スキャンの自己ループを防ぐ | 同上 |
| FR-3.4 | watch 設定の変更は再起動後に反映される | UI ラベルで明示 |

### FR-4 検索

| ID | 要件 | 実装 |
|---|---|---|
| FR-4.1 | 検索モードは `auto` / `path` / `name`。`auto` は入力形状から自動判定する | `query.rs` |
| FR-4.2 | クエリ前処理: `file:` / `path:` / `file://` プレフィックス除去（→ path 強制）、前後の空白・引用符・`<>` 除去、`\`→`/`、URL デコード、本サーバの URL（`http://host/r/...`）貼り付けの吸収、先頭 `./` と前後 `/` 除去、NFKC + lowercase | `parse_query` |
| FR-4.3 | auto 判定: `/` を含む、または対象拡張子で終わる → **path**、それ以外 → **name** | 同上 |
| FR-4.4 | **パス推測**のスコア: `root名/rel_path` 完全一致 100 / `rel_path` 完全一致 90 / 先頭が root 名＋残りが suffix 一致 85 / セグメント suffix 一致 80 / 途中セグメント以降が suffix 一致 78–79 / ディレクトリ一致かつ拡張子違い 60 / ファイル名一致 50 / stem 一致 45。0 件なら最終セグメントで fuzzy ファイル名検索（上限 40） | `path_guess.rs` |
| FR-4.5 | **ファイル名検索**のスコア: stem 完全一致 100 / 前方一致 90 / 部分一致 80−開始位置 / 空白区切り全トークンが stem に含まれる(AND) 61–70 / 全トークンがパスに含まれる 60 / パスに部分一致 50 / fuzzy（SkimMatcherV2、2 文字以上）1–40 | `name_search.rs` |
| FR-4.6 | path モードで 0 件の場合、最終セグメントの stem でファイル名検索にフォールバックし、`mode=name` として返す | `search/mod.rs` |
| FR-4.7 | 並び順: スコア降順 → 更新日時降順 → `rel_path` 昇順。`limit` で打ち切り（既定 50、UI は 100） | 同上 |
| FR-4.8 | 結果には `score`、ヒット理由 `reason`、stem 内ハイライト範囲 `highlights` を含める | `SearchResult` |
| FR-4.9 | path モードで先頭候補のスコアが 80 以上かつ 2 位より高い場合、`best` に `root/rel_path` を返し、UI はそれを即オープンする | `CONFIDENT_SCORE`, `App.tsx` |
| FR-4.10 | 入力は 180ms デバウンスし、最後に発行したリクエストの結果のみ採用する | `useSearch.ts` |

受け入れケース（すべて自動テストで検証済み）:

- `file:sample-docs/data/データ移行概要設計.html` → 該当 1 件がスコア 100 で最上位、即オープン
- `data/データ移行概要設計.html` → スコア 90
- `/home/me/src/sample-docs/data/データ移行概要設計.html`（上位ディレクトリ余分）→ 78 以上
- `sample-docs-main/data/…`（ローカルのディレクトリ名違い）→ 78
- `data/データ移行概要設計.md`（拡張子違い）→ 60
- `README.md` が 2 ルートにある → 両方 90 で返す（`best` なし）
- `データ移行概要設計` → 完全一致 100、`データ移行概要設計_詳細` が前方一致 90
- `出走表 GraphQL` → AND 一致で `出走表_GraphQL設計`
- `ﾃﾞｰﾀ移行`（半角カナ）→ NFKC 正規化で一致
- `grphql` → fuzzy で `GraphQL` を含むものがヒット
- `"sample-docs\data\a.html"`（Windows パス・引用符）／`http://127.0.0.1:8765/r/sample-docs/data/%E3%81%82.html`（URL）→ 同じセグメントに正規化

### FR-5 配信（HTTP サーバ）

| ID | 要件 | 実装 |
|---|---|---|
| FR-5.1 | `server.host:port` で HTTP サーバを起動する。`port=0` で自動割当し、実アドレスを UI に表示する | `server::bind` |
| FR-5.2 | `GET /r/{root}/{path}` でルート配下の**全ファイル**（対象拡張子以外の CSS/JS/画像も含む）を静的配信し、HTML の相対参照を解決する | `serve_file` |
| FR-5.3 | パストラバーサル防止: `..` セグメント拒否 → 解決後 canonicalize → ルート実パス配下であることを再検証（symlink 越え防止）。違反は 403/404 | `resolve_under_root`, `serve_file` |
| FR-5.4 | macOS の NFD ファイル名対策: 解決パスが存在しない場合、NFC 化した `rel_path` でインデックスから実パスを引く | `App::resolve` |
| FR-5.5 | MIME は拡張子から推定し、text 系には `charset=utf-8` を付与。`Cache-Control: no-cache` | `serve_file` |
| FR-5.6 | JSON API: `/api/health`, `/api/roots`, `/api/files?root=`, `/api/search?q=&mode=&limit=`, `/api/raw?root=&path=`, `GET/PUT /api/config`, `POST /api/reload`（詳細は design.md） | `server/mod.rs` |
| FR-5.7 | ビルド済みフロントエンド `dist/` をバイナリに埋め込み、`/` と `/assets/*` で配信する。未知パスは SPA 用に `index.html` を返す。未ビルド時は 404 と案内文 | `rust-embed`, `frontend` |
| FR-5.8 | CORS は permissive（ローカル利用前提） | `CorsLayer` |
| FR-5.9 | `open_browser=true` または CLI `--open` で起動時に既定ブラウザを開く | `docserver.rs` |

### FR-6 UI（デスクトップ／ブラウザ共通）

| ID | 要件 | 実装 |
|---|---|---|
| FR-6.1 | レイアウト: 上部 AppBar（検索窓・サーバ URL チップ・再スキャン・テーマ切替・設定）、左サイドバー（「検索結果」「ファイル」タブ）、右ビューア | `App.tsx` |
| FR-6.2 | 検索窓は `Ctrl/Cmd+K` でフォーカス＆全選択、`Esc` でクリア。モード選択（auto/path/name）を併設 | `SearchBar.tsx` |
| FR-6.3 | 検索入力があれば自動で「検索結果」タブ、空なら「ファイル」タブに切り替える | `App.tsx` |
| FR-6.4 | 検索結果は種別アイコン・ハイライト付き stem・root チップ・`rel_path`・スコア（tooltip に理由）を表示し、検索モード（パス推測／ファイル名）をチップで示す | `SearchResults.tsx` |
| FR-6.5 | ファイルツリーは root → ディレクトリ → ファイルの階層で表示（ディレクトリ先・日本語ロケールでソート、root は既定で展開、件数表示） | `FileTree.tsx` |
| FR-6.6 | 文書は `/view/<root>/<rel_path>` ルートで開き、ブラウザ履歴（戻る／進む）・URL 直接アクセスに対応する | `react-router-dom` |
| FR-6.7 | ビューアヘッダに `root / rel_path`、`file:` 参照コピー、再読み込み、フォルダを開く（Tauri のみ）、ブラウザで開く、を備える | `Viewer.tsx` |
| FR-6.8 | **HTML**: `/r/...` URL を iframe（`sandbox="allow-scripts allow-same-origin allow-popups allow-forms allow-modals"`）で表示し、文書内の相対参照と自前 JS（Mermaid 描画等）をそのまま動作させる。`other` 種別も同様に表示 | `HtmlViewer.tsx` |
| FR-6.9 | **Markdown**: GFM（表・タスクリスト等）、生 HTML 混在、見出し ID 付与、コードハイライト、```` ```mermaid ```` フェンスの図描画。相対リンクはアプリ内遷移（`#hash` 付き対応）、外部リンクは別タブ、相対画像は配信 URL に解決 | `MarkdownViewer.tsx` |
| FR-6.10 | **Mermaid** 単体ファイル: 図／ソース切替、SVG 保存。描画エラー時はエラー文とソースを表示 | `MermaidViewer.tsx`, `MermaidBlock.tsx` |
| FR-6.11 | ライト／ダークテーマ切替（Mermaid テーマも連動）。テーマとサイドバー幅は localStorage に保存 | `theme.ts`, `useLocalStorage.ts` |
| FR-6.12 | サイドバーはドラッグで幅変更（240–700px）、ボタンで表示／非表示 | `App.tsx` |
| FR-6.13 | サーバ URL チップをクリックするとクリップボードにコピー。再スキャン・設定保存の結果を Snackbar で通知。サーバ未接続時はエラー通知 | `App.tsx` |
| FR-6.14 | Tauri 内では IPC（invoke）で `search / list_files / get_settings / update_settings / reload / reveal / server_url` を呼び、ブラウザ直アクセス時は同等の HTTP API にフォールバックする。HTML 表示と raw 取得は常に HTTP | `api/client.ts` |
| FR-6.15 | ディレクトリ選択は Tauri ではネイティブダイアログ、ブラウザでは `prompt` 入力 | `pickDirectory` |

### FR-7 ヘッドレス CLI

| ID | 要件 | 実装 |
|---|---|---|
| FR-7.1 | `docserver [--config <path>] [--port <n>] [--open] [--search <q>]` | `bin/docserver.rs` |
| FR-7.2 | `--search` 指定時は検索結果を JSON で標準出力に出して終了する（デバッグ・スクリプト用途） | 同上 |
| FR-7.3 | 通常起動時は watcher とサーバを起動し、`listening on http://...` を出力する | 同上 |

---

## 5. 非機能要件

| ID | 要件 | 実装・根拠 |
|---|---|---|
| NFR-1 | **性能**: 数百〜数千ファイル規模でスキャンは数十 ms、検索は入力ごとにインメモリ線形走査で即応答（sample-docs 96 件: スキャン 5ms） | `ignore` crate 走査、`Arc<Vec<FileEntry>>` スナップショット |
| NFR-2 | **並行性**: インデックスは RwLock + Arc スナップショットで、再スキャン中も検索・配信をブロックしない | `FileIndex` |
| NFR-3 | **セキュリティ**: 既定バインドは `127.0.0.1`。配信はルート配下のみ（トラバーサル／symlink 越え拒否）。iframe は sandbox 付き。文書はローカル信頼済み前提のため CSP は無効（`csp: null`） | FR-5.3, FR-6.8 |
| NFR-4 | **国際化**: 日本語・全角括弧・空白を含むファイル名を扱う。NFC/NFD・全角半角・大文字小文字の揺れを吸収。UI 文言は日本語 | FR-2.5, FR-2.6, FR-4.2, FR-5.4 |
| NFR-5 | **移植性**: Rust コアは Tauri 非依存で、CLI / Tauri 双方から利用可能。Windows パス（`\`）貼り付けに対応 | `crates/docserver-core` |
| NFR-6 | **配布性**: フロントをバイナリに埋め込み、CLI 単体で UI まで配信できる。Tauri バンドルには `config/setting.yaml` をリソース同梱 | FR-5.7, `tauri.conf.json` |
| NFR-7 | **テスト容易性**: コアは `cargo test`（22 件）、フロントは Vitest（2 件）で自動検証 | tasks.md 参照 |
| NFR-8 | **運用**: `tracing` による構造化ログ（`RUST_LOG` で制御）。設定変更・再スキャン・変更検知を INFO で記録 | 各モジュール |

---

## 6. 制約・前提

- 開発環境: Node.js 20+、Rust stable、Tauri v2 の OS 別前提パッケージ
- 設定ファイルを OS 設定ディレクトリに置く場合、相対パスはそのディレクトリ基準で解決されるため絶対パス推奨
- `server.host/port`・`watch` の変更は再起動で反映
- Mermaid は `securityLevel: "loose"` で描画（ローカル文書前提）
