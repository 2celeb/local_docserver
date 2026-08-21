# local_docserver 開発計画

## 1. 目的

`config/setting.yaml` に列挙した複数のリポジトリ／ディレクトリ配下の **HTML / Markdown / Mermaid** ファイルを、
ローカル Web サーバとしてまとめて公開し、横断検索・閲覧できるデスクトップアプリ（Tauri）を作る。

対象 OS: Windows / macOS / Linux

### 前提（対象リポジトリ sample-docs の調査結果）

- 仕様書は 94 本の HTML、ほぼ `ディレクトリ直下/画面名_種別.html` のフラット構成
- HTML は `../assets/style.css`, `../assets/app.js` を **相対パス参照** し、`app.js` 内で Mermaid を描画している
  → **HTML はルートごとに静的配信し、相対参照がそのまま解決される必要がある**（iframe 表示が前提）
- ファイル名は日本語・全角括弧・空白を含む → URL エンコード／Unicode 正規化（NFC/NFD。特に macOS）に注意

---

## 2. 技術スタック

| レイヤ | 採用 | 理由 |
|---|---|---|
| デスクトップ | Tauri v2 (Rust) | 3 OS 対応・軽量。ファイル走査/HTTP サーバを Rust 側で実装 |
| HTTP サーバ | axum + tower-http (Rust, Tauri 内で spawn) | 任意ブラウザからも `http://127.0.0.1:<port>/` で閲覧可能にする |
| フロント | React 18 + TypeScript + Vite | Tauri 標準構成 |
| UI | MUI (Material UI) v6 | 指定 |
| Markdown | `react-markdown` + `remark-gfm` + `rehype-raw` | GFM 表・HTML 混在に対応 |
| Mermaid | `mermaid` (npm, JS ライブラリ) | **利用可能**。`.mmd/.mermaid` 単体ファイルと Markdown 内 ```mermaid フェンスの両方を描画 |
| コードハイライト | `rehype-highlight` | Markdown 内コード |
| 設定 | `serde_yaml` (Rust) | `config/setting.yaml` 読込 |
| 走査 | `ignore` crate (walkdir + .gitignore 尊重) + `notify` (変更監視) | 大規模リポジトリでも高速 |
| 検索 | Rust 側インメモリインデックス + `fuzzy-matcher`(SkimMatcherV2) | 外部 DB 不要 |
| 日本語正規化 | `unicode-normalization` | NFC 統一、全角/半角ゆれ吸収 |

> Mermaid について: `mermaid` は純粋な JS ライブラリで、Tauri の WebView 上でそのまま動作する。
> バージョン 10 以降は ESM 対応で `mermaid.render(id, code)` が Promise を返すため React と相性が良い。
> SVG 出力なのでズーム／ダウンロードも容易。

---

## 3. 設定ファイル仕様 `config/setting.yaml`

```yaml
server:
  host: 127.0.0.1
  port: 8765          # 0 で自動割当
  open_browser: false # 起動時に外部ブラウザも開くか

roots:                # 複数リポジトリ／ディレクトリ
  - name: sample-docs    # 検索時の表示名・URL プレフィックス（/r/sample-docs/...）
    path: ../         # 絶対パス or setting.yaml からの相対パス
  - name: other-repo
    path: /home/user/src/other-repo
    exclude: ["node_modules", "dist"]

include_extensions: [html, htm, md, markdown, mmd, mermaid]
respect_gitignore: true
watch: true           # ファイル変更を監視してインデックス更新
max_depth: 20
```

設定ファイルの探索順: `--config` 引数 → カレント `config/setting.yaml` → 実行ファイル隣 → `$APPCONFIG/local_docserver/setting.yaml`。
未存在なら GUI の設定画面から作成できるようにする。

---

## 4. アーキテクチャ

```
┌──────────────── Tauri App ────────────────┐
│  WebView (React + MUI)                    │
│   ├ 検索 UI / ファイルツリー              │
│   ├ Markdown / Mermaid レンダラ            │
│   └ HTML は <iframe src="http://127.0.0.1:PORT/r/<root>/<path>">
│           │ tauri invoke (IPC)             │
│  Rust core                                │
│   ├ config: setting.yaml 読込・保存        │
│   ├ indexer: 走査 → FileEntry[] (+watch)  │
│   ├ search: パス推測 / ファイル名検索       │
│   └ http: axum 静的配信 + /api/*          │
└───────────────────────────────────────────┘
```

### 4.1 Rust モジュール構成 (`src-tauri/src/`)

```
main.rs            Tauri 起動、サーバ spawn、state 管理
config.rs          Settings 構造体、ロード/保存、パス解決
index/
  mod.rs           FileIndex (RwLock<Vec<FileEntry>>)
  scanner.rs       ignore::WalkBuilder による走査
  watcher.rs       notify による差分更新（debounce）
search/
  mod.rs           SearchQuery → SearchResult
  path_guess.rs    ファイルパス推測
  name_search.rs   ファイル名検索
server/
  mod.rs           axum Router、/r/<root>/* 静的配信、/api/*
commands.rs        Tauri command (search, list_roots, reload, get_config, ...)
```

### 4.2 FileEntry

```rust
struct FileEntry {
  root: String,          // roots[].name
  rel_path: String,      // ルートからの相対パス（'/' 区切り、NFC）
  abs_path: PathBuf,
  file_name: String,     // 拡張子込み
  stem: String,          // 拡張子なし
  kind: Html | Markdown | Mermaid,
  size: u64,
  modified: SystemTime,
  // 検索用前処理
  norm_stem: String,     // NFKC + lowercase
  path_segments: Vec<String>,
}
```

### 4.3 HTTP エンドポイント

| Method | Path | 内容 |
|---|---|---|
| GET | `/r/{root}/{path..}` | ルート配下を静的配信（html/css/js/画像など**全ファイル**。相対参照解決のため） |
| GET | `/api/roots` | ルート一覧 |
| GET | `/api/files?root=` | インデックス済みファイル一覧 |
| GET | `/api/search?q=&mode=auto\|path\|name&limit=` | 検索 |
| GET | `/api/raw?root=&path=` | Markdown/Mermaid の生テキスト（フロントで描画） |
| POST | `/api/reload` | 再スキャン |

- パストラバーサル防止: 正規化後に root 配下であることを検証
- 配信範囲はルート配下のみ。ブラウザ直アクセス時のトップ `/` は同じ React UI を配信

---

## 5. 検索機能の設計

### 5.1 クエリ解釈（mode=auto）

1. 先頭の `file:` / `path:` プレフィックスを除去（`file:sample-docs/data/xxx.html` 形式）
2. 前後空白・引用符を除去、`\` → `/`、URL デコード、NFC 正規化
3. `/` を含む、または拡張子を持つ → **パス推測モード**
4. それ以外 → **ファイル名検索モード**
5. パス推測で 0 件なら最終セグメントでファイル名検索へフォールバック

### 5.2 ファイルパス推測（`path_guess.rs`）

入力例: `sample-docs/data/データ移行概要設計.html`

スコアリング（高い順）:

| 条件 | スコア |
|---|---|
| `root名/rel_path` が完全一致 | 100 |
| いずれかの root で `rel_path` が完全一致 | 90 |
| `abs_path` が入力を **suffix** として終端一致（ディレクトリ境界で） | 80 |
| 入力の先頭セグメントが root 名と一致し、残りが suffix 一致 | 75 |
| 拡張子違い（`.md` ↔ `.html` 等）で suffix 一致 | 60 |
| 最終セグメント（ファイル名）が完全一致 | 50 |
| 最終セグメントが fuzzy 一致（→ 5.3 に委譲） | 〜40 |

- セグメント単位の suffix マッチで「リポジトリ名がローカルのディレクトリ名と異なる」ケースに対応
  （例: ローカルでは `~/src/sample-docs` でも `sample-docs-main` でも `data/データ移行概要設計.html` で一致）
- 結果が 1 件でスコア ≥ 80 なら UI はそのまま開く（「I'm feeling lucky」挙動）。複数なら候補リスト表示

### 5.3 ファイル名検索（`name_search.rs`）

入力例: `データ移行概要設計`

1. 正規化: NFKC → lowercase → 全角スペース/アンダースコア/ハイフンを区切りとして扱う
2. 一致判定の優先順位
   1. stem 完全一致
   2. stem 前方一致
   3. stem 部分一致（連続）
   4. AND 検索（空白区切りの全トークンを stem またはパスに含む）
   5. fuzzy（SkimMatcherV2、日本語は文字単位）
3. 同スコアは `modified` 降順 → パス昇順
4. 結果にはマッチ位置を返し、UI でハイライト

### 5.4 将来拡張（スコープ外・設計だけ余地を残す）

- 全文検索（`tantivy` + `lindera` 日本語トークナイザ）
- Markdown の見出し検索

---

## 6. フロントエンド画面

```
src/
  main.tsx, App.tsx
  theme.ts                 MUI テーマ（ダーク/ライト）
  api/                     tauri invoke / fetch のラッパ
  components/
    SearchBar.tsx          Autocomplete + `file:` 貼り付け対応、Ctrl/Cmd+K
    SearchResults.tsx      ハイライト付きリスト、root チップ、種別アイコン
    FileTree.tsx           MUI TreeView（root → ディレクトリ → ファイル）
    viewers/
      HtmlViewer.tsx       iframe (sandbox 適切化、外部ブラウザで開くボタン)
      MarkdownViewer.tsx   react-markdown + mermaid フェンス描画
      MermaidViewer.tsx    mermaid.render → SVG、ズーム、SVG 保存
    SettingsDialog.tsx     roots 追加/削除（ディレクトリ選択ダイアログ）、再スキャン
  hooks/useSearch.ts       debounce 200ms
  routes                   /view/:root/*path（履歴・戻る進むに対応）
```

レイアウト: 左ペイン（検索 + ツリー）／右ペイン（ビューア）。AppBar に検索ボックス、サーバ URL 表示・コピー。

---

## 7. 実装フェーズ（2026-08-21 時点の進捗）

### Phase 0: 雛形 ✅
- [x] Cargo ワークスペース（`crates/docserver-core` + `src-tauri`）、Vite + React + TS、MUI 導入
- [x] `config/setting.yaml` サンプル、README、CI（`.github/workflows/build.yml`: 3 OS でコアテスト → Tauri ビルド）
- [x] アプリアイコン（プレースホルダ。`src-tauri/app-icon.png` を差し替えて `npx tauri icon` で再生成）

### Phase 1: コア ✅
- [x] `config.rs` 読込・相対パス解決（設定ファイル基準）・バリデーション・保存
- [x] `scanner.rs` 走査（`ignore` crate、exclude、拡張子フィルタ、NFC 正規化）
- [x] axum サーバ、`/r/{root}/*` 静的配信（正規化 + canonicalize の二重チェックでトラバーサル拒否）
- [x] `/api/roots` `/api/files` `/api/raw` `/api/config` `/api/reload`、`dist/` 埋め込み配信（SPA フォールバック）

### Phase 2: 検索 ✅
- [x] クエリ解釈（`file:`/`path:` プレフィックス、`\`→`/`、URL デコード、サーバ URL 貼り付け、NFKC、auto 判定）
- [x] パス推測（root+path 完全一致 100 / path 一致 90 / root 名+suffix 85 / suffix 80 / 上位余分 78 / 拡張子違い 60 / ファイル名 50）
- [x] ファイル名検索（完全 100 / 前方 90 / 部分 80- / AND 70- / パス AND 60 / パス部分 50 / fuzzy ≤40）
- [x] 0 件時のファイル名検索フォールバック、`best`（パス推測モードの確信単独ヒット）
- [x] 実リポジトリ (96 ファイル) で受け入れケースを確認

### Phase 3: UI ✅（ブラウザで動作確認。Tauri 内は未確認）
- [x] 検索バー（Ctrl/Cmd+K、モード切替）・結果リスト（ハイライト・root チップ・スコア）
- [x] HtmlViewer(iframe) / MarkdownViewer(GFM + mermaid フェンス + 相対リンクのアプリ内遷移) / MermaidViewer(図/ソース、SVG 保存)
- [x] ファイルツリー、`/view/<root>/<path>` ルーティング、サイドバー幅リサイズ、ダーク/ライト
- [x] 設定ダイアログ（roots 編集 → yaml 保存 → 再スキャン）

### Phase 4: 監視・仕上げ 🔄
- [x] `notify` による増分更新（対象拡張子のファイル／消失パスのみ反応。ディレクトリ自身のイベントは無視して自己ループ防止）
- [ ] **Tauri 実機ビルド（3 OS）** — 開発環境に `pkg-config` / `libwebkit2gtk-4.1-dev` 等が無く未ビルド。README の apt コマンドを実行後 `npm run tauri dev`
- [ ] Windows（`\` パス、長いパス）/ macOS（NFD ファイル名）の実機確認
- [ ] WebView 内での Mermaid / iframe の挙動確認

---

## 8. テスト方針

- Rust: `cargo test`（scanner / search はフィクスチャディレクトリ `tests/fixtures/` を用意。日本語・括弧・空白入りファイル名を含める）
- 検索の受け入れケース
  - `file:sample-docs/data/データ移行概要設計.html` → data 配下の該当 1 件を最上位
  - `data/データ移行概要設計.html` → 同上
  - `データ移行概要設計` → 同ファイル + 類似名（`データ移行_…`）を候補
  - `ﾃﾞｰﾀ移行` / 全角スペース混在 → NFKC 正規化で一致
- フロント: Vitest + React Testing Library（SearchBar のクエリ整形、Viewer の種別分岐）
- E2E: Tauri WebDriver（任意）

---

## 9. リスクと対策

| リスク | 対策 |
|---|---|
| HTML が `../assets/` を相対参照 | ルート配下を丸ごと静的配信し、iframe の URL をファイル実パスに一致させる |
| 外部ブラウザでも閲覧したい | サーバを `127.0.0.1` で常時起動、AppBar に URL 表示。LAN 公開はデフォルト無効 |
| macOS の NFD ファイル名 | インデックス時に NFC へ正規化、配信時は実パスで照合 |
| Windows のパス区切り・`\` 貼り付け | クエリ正規化で `\`→`/` |
| 大量ファイル・巨大リポジトリ | `ignore` crate、include_extensions で早期除外、watch は debounce |
| ポート競合 | `port: 0` 自動割当 + UI 表示 |
| iframe 内 JS の安全性 | ローカル信頼済み文書が前提だが `sandbox="allow-scripts allow-same-origin"` を基本にし設定で緩和可 |

---

## 10. 成果物

- `local_docserver/` 配下: `src/`(React), `src-tauri/`(Rust), `config/setting.yaml`, `doc/`
- README（起動方法、設定、検索構文 `file:` の説明）
- 3 OS 向けインストーラ（GitHub Actions の `tauri-action` で生成）
