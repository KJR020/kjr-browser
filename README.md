# kjr-browser

Tauri で作る軽量ブラウザ。

OS 標準の WebView (Linux: webkit2gtk / macOS: WKWebView / Windows: WebView2) にレンダリングを任せ、
タブ・アドレスバー・履歴などの「ブラウザシェル」を Rust + Tauri で実装する。
Chromium を同梱しないため、バイナリサイズとメモリ使用量を小さく保てる。

## アーキテクチャ

Tauri v2 のマルチ WebView 機能 (`unstable` フィーチャ) を使い、1 つのウィンドウに 2 つの WebView を配置する。

```
┌─────────────────────────────────────────┐
│ main ウィンドウ (tauri)                  │
│ ┌─────────────────────────────────────┐ │
│ │ toolbar WebView (ui/ のローカルHTML) │ │  ← タブ列 + 戻る/進む/更新 + アドレスバー
│ ├─────────────────────────────────────┤ │     IPC (invoke) で Rust を呼ぶ
│ │ tab-N WebView (リモートURL)          │ │  ← タブごとに 1 つの WebView。
│ │  (アクティブなタブだけ表示)           │ │     アクティブ以外は hide()。
│ └─────────────────────────────────────┘ │     IPC 権限なし (サンドボックス)
└─────────────────────────────────────────┘
```

タブの状態 (ラベル・URL・アクティブ) は Rust 側の `TabsState` が一元管理し、
変化のたびに `tabs-changed` イベントでツールバーへ通知する。

- ツールバーは `ui/` の素の HTML/CSS/JS (バンドラ不使用、`withGlobalTauri` で API を利用)
- Rust 側 (`src-tauri/src/main.rs`) が `navigate` / `go_back` / `go_forward` / `reload` コマンドを提供
- コンテンツ側のページ遷移は `url-changed` イベントでツールバーに通知され、アドレスバーが同期される
- リモートページを表示する content WebView には capability を与えず、IPC へアクセスさせない

## ディレクトリ構成

```
├── src-tauri/          Rust / Tauri 本体
│   ├── src/main.rs     ウィンドウ構築・IPC コマンド
│   ├── tauri.conf.json Tauri 設定
│   └── capabilities/   WebView ごとの権限定義
├── ui/                 ツールバー UI (素の HTML/CSS/JS)
└── docs/               計画・メモ
```

## ロードマップ

| Phase | テーマ | 状態 |
|-------|-------|------|
| 1 | MVP: アドレスバー + 単一ページ表示 + 戻る/進む/更新 | ✅ 実装済み |
| 2 | タブ (content WebView の複数管理・切り替え) | ✅ 実装済み |
| 3 | 履歴・ブックマークの永続化 | 未着手 |
| 4 | ショートカットキー・ダウンロード・設定画面 | 未着手 |
| 5 | 配布用ビルド最適化 (AppImage / dmg / msi) | 未着手 |

## ビルド・実行

Linux では webkit2gtk などの開発パッケージが必要:

```bash
sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev librsvg2-dev
```

実行:

```bash
cd src-tauri
cargo run
```

テスト:

```bash
cd src-tauri
cargo test
```

## 参考資料

- [Tauri v2 ドキュメント](https://v2.tauri.app/)
- [Tauri Webview (マルチ WebView API)](https://docs.rs/tauri/latest/tauri/webview/)
- [wry](https://github.com/tauri-apps/wry) - Tauri が内部で使う WebView ライブラリ

---

なお、当初はレンダリングエンジンごとフルスクラッチで実装する計画だった。
その学習計画は [docs/learning-plan.md](docs/learning-plan.md) に残してある
(ブラウザ内部の仕組みを学ぶ資料として有用なため)。
