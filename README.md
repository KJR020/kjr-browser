# kjr-browser

ブラウザの仕組みを理解するために、レンダリングエンジンをフルスクラッチで作る学習プロジェクト。

URL を受け取ってからピクセルを画面に描くまでの全工程を、Rust で自分の手で実装する。

```
URL → [Network] → HTML → [Parser] → DOM → [Style] → スタイルツリー
    → [Layout] → レイアウトツリー → [Paint] → ディスプレイリスト → ピクセル
```

計画の全体像は [docs/learning-plan.md](docs/learning-plan.md) を参照。

## 進捗

| Phase | テーマ | 状態 |
|-------|-------|------|
| 1 | 画面に描画する (ウィンドウ・矩形・テキスト) | ✅ 完了 |
| 2 | HTML → DOM → 画面 | 未着手 |
| 3 | CSS スタイル適用 | 未着手 |
| 4 | レイアウトエンジン | 未着手 |
| 5 | ネットワーク | 未着手 |
| 6 | インタラクション | 未着手 |

## ディレクトリ構成

```
├── engine/              ★ 本線: トイレンダリングエンジン
│   └── src/
│       ├── main.rs          イベントループとウィンドウ (Phase 1)
│       ├── display_list.rs  描画コマンド定義 (パイプラインの中間表現)
│       ├── paint.rs         ラスタライズ (図形 → ピクセル)
│       └── text.rs          フォント処理 (文字 → グリフ → ピクセル)
├── docs/
│   ├── learning-plan.md     フェーズ別の学習計画 (本線のロードマップ)
│   └── architecture.md      シェル編のアーキテクチャ解説
├── src-tauri/           (シェル編・完了) Tauri 製ブラウザシェル
└── ui/                  (シェル編・完了) そのツールバー UI
```

モジュール名はレンダリングパイプラインの工程名に揃えてある。
Phase が進むごとに `html.rs` (パーサ)、`dom.rs`、`css.rs`、`style.rs`、`layout.rs` が
engine/src/ に増えていき、ディレクトリ一覧がそのままパイプライン図になる。

## ビルド・実行

```bash
cd engine
cargo run
```

Linux で必要なパッケージ: `libxkbcommon-x11-0` (winit のキーボード処理が実行時にロードする)。
フォントは実行環境のシステムフォントを使う (探索パスは engine/src/text.rs の `FONT_PATHS`)。

## シェル編について (src-tauri/ + ui/)

一時期 Tauri で「ブラウザのガワ」(タブ・アドレスバー・IPC・プロセス分離) を作った。
動くタブブラウザとして完成しており、Chrome のブラウザプロセス側の構造を学ぶ教材として残している。
解説は [docs/architecture.md](docs/architecture.md)。動かすには:

```bash
cd src-tauri && cargo run
```

## 参考資料

- [Let's build a browser engine!](https://limpet.net/mbrubeck/2014/08/08/toy-layout-engine-1.html) - Matt Brubeck による Rust 製トイブラウザチュートリアル
- [Web Browser Engineering](https://browser.engineering/) - ブラウザの仕組みを体系的に解説する教科書
- [Robinson](https://github.com/mbrubeck/robinson) - 上記チュートリアルの参考実装
- [Servo](https://github.com/servo/servo) - Mozilla の Rust 製ブラウザエンジン
