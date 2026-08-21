# local_docserver

`config/setting.yaml` に列挙した複数のリポジトリ／ディレクトリ配下の **HTML / Markdown / Mermaid** を
ローカル Web サーバとして公開し、横断検索・閲覧できるデスクトップアプリ（Tauri + React + MUI）です。
Windows / macOS / Linux で動作します。

## 構成

```
local_docserver/
├─ config/setting.yaml        # 設定
├─ crates/docserver-core/     # Rust コア（設定・走査・検索・HTTP サーバ）Tauri 非依存
│   └─ src/bin/docserver.rs   # ヘッドレス CLI（サーバだけ起動）
├─ src-tauri/                 # Tauri シェル（デスクトップアプリ）
├─ src/                       # React + MUI フロントエンド
└─ doc/                       # requirements.md（要件）/ design.md（設計）/ tasks.md（タスク）/ plan.md（初期計画）
```

## 設定 `config/setting.yaml`

```yaml
server:
  host: 127.0.0.1
  port: 8765          # 0 で自動割当
  open_browser: false

roots:                # 複数可
  - name: sample-docs                 # 表示名 兼 URL プレフィックス (/r/sample-docs/...)
    path: /home/user/src/sample-docs  # 絶対パス推奨（Windows: C:/Users/user/src/sample-docs）
    exclude: ["node_modules"]
  - name: other-docs
    path: /home/user/src/other-docs

include_extensions: [html, htm, md, markdown, mmd, mermaid]
respect_gitignore: true
watch: true           # 変更監視 → 該当ルートのみ再スキャン
max_depth: 20
```

`roots[].path` は絶対パスで記述してください（相対パスも可ですが、設定ファイルの位置基準で解決されます）。

設定ファイルの探索順: `--config` → `./config/setting.yaml` → `./setting.yaml` → 実行ファイル隣 → OS 設定ディレクトリ
（Linux `~/.config/local_docserver/setting.yaml`、macOS `~/Library/Application Support/local_docserver/`、Windows `%APPDATA%\local_docserver\`）。
GUI の設定ダイアログからも編集・保存できます。

## 検索

検索ボックス（`Ctrl/Cmd+K`）は入力形状から自動でモードを選びます。

| 入力例 | モード | 動作 |
|---|---|---|
| `file:sample-docs/data/データ移行概要設計.html` | パス推測 | `root名/相対パス` 完全一致 → 相対パス一致 → 末尾セグメント一致 → 拡張子違い → ファイル名一致 の順にスコアリング。確信度の高い単一候補は即オープン |
| `data/データ移行概要設計.html`, `C:\repo\data\x.html`, サーバ URL 貼り付け | パス推測 | 同上（`\`・URL エンコード・上位ディレクトリの余分を吸収） |
| `データ移行概要設計` | ファイル名 | 完全 → 前方 → 部分 → AND（空白区切り） → fuzzy。NFKC 正規化で全角/半角・大文字小文字を吸収 |

## 開発

前提: Node.js 20.19+ / 22.12+（Vite 8）, Rust (stable)。Tauri ビルドには各 OS の[前提パッケージ](https://v2.tauri.app/start/prerequisites/)が必要です。

```bash
# Linux (Debian/Ubuntu) の Tauri 前提パッケージ
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev \
  libssl-dev libayatana-appindicator3-dev librsvg2-dev pkg-config libdbus-1-dev

npm install

# コアのテスト / ヘッドレスサーバ（Tauri 不要）
npm run core:test
npm run core:serve -- --port 8765       # http://127.0.0.1:8765/
cargo run -p docserver-core --bin docserver -- --search "file:sample-docs/data/データ移行概要設計.html"

# ブラウザで UI 開発（Vite が /api, /r をヘッドレスサーバへプロキシ）
npm run dev                              # http://localhost:1420/

# デスクトップアプリ
npm run tauri dev
npm run tauri build
```

`npm run build` で生成した `dist/` は `docserver-core` に埋め込まれるため、ヘッドレス CLI 単体でも UI を配信できます
（フロントを更新したら `npm run build` → `cargo build`）。

## HTTP API

| Method | Path | 内容 |
|---|---|---|
| GET | `/r/{root}/{path}` | ルート配下の静的配信（HTML の `../assets/` 等の相対参照のため全ファイル対象。ルート外は拒否） |
| GET | `/api/roots` | ルート一覧と件数 |
| GET | `/api/files?root=` | インデックス済みファイル |
| GET | `/api/search?q=&mode=auto\|path\|name&limit=` | 検索 |
| GET | `/api/raw?root=&path=` | Markdown / Mermaid の生テキスト |
| GET/PUT | `/api/config` | 設定取得・保存（保存後に再スキャン） |
| POST | `/api/reload` | 再スキャン |

## Mermaid

`mermaid` (npm) で WebView 内に描画します。`.mmd` / `.mermaid` 単体ファイルと Markdown 内の ```` ```mermaid ```` フェンスに対応。
既存 HTML 仕様書が `assets/app.js` で自前描画している場合は iframe 内でそのまま動きます。
