# kjr-browser

ブラウザの仕組みを理解するために、レンダリングエンジンをフルスクラッチで作る学習プロジェクト。

URL を受け取ってからピクセルを画面に描くまでの全工程を、Rust で自分の手で実装する。
OS の WebView やブラウザエンジンは一切使わない。

```
URL → [Network] → HTML → [Parser] → DOM → [Style] → スタイルツリー
    → [Layout] → レイアウトツリー → [Paint] → ディスプレイリスト → ピクセル
```

計画の全体像は [docs/learning-plan.md](docs/learning-plan.md) を参照。

## 進捗

| Phase | テーマ | 状態 |
|-------|-------|------|
| 1 | 画面に描画する (ウィンドウ・矩形・テキスト) | ✅ 完了 |
| 2 | HTML → DOM → 画面 | ✅ 完了 |
| 3 | CSS スタイル適用 | ✅ 完了 |
| 4 | レイアウトエンジン | 未着手 |
| 5 | ネットワーク | 未着手 |
| 6 | インタラクション | 未着手 |

## ディレクトリ構成

```
├── engine/              トイレンダリングエンジン
│   └── src/
│       ├── main.rs          イベントループとウィンドウ (Phase 1)
│       ├── html.rs          HTML パーサ: テキスト → DOM (Phase 2)
│       ├── dom.rs           DOM ツリーの定義 (Phase 2)
│       ├── css.rs           CSS パーサ: テキスト → スタイルシート (Phase 3)
│       ├── style.rs         スタイル解決: DOM + CSS → スタイルツリー (Phase 3)
│       ├── render.rs        スタイルツリー → ディスプレイリスト
│       ├── display_list.rs  描画コマンド定義 (パイプラインの中間表現)
│       ├── paint.rs         ラスタライズ (図形 → ピクセル)
│       └── text.rs          フォント処理 (文字 → グリフ → ピクセル)
└── docs/
    └── learning-plan.md     フェーズ別の学習計画
```

モジュール名はレンダリングパイプラインの工程名に揃えてある。
Phase が進むごとに `layout.rs` (レイアウトエンジン)、`net.rs` (HTTP) が
engine/src/ に増えていき、ディレクトリ一覧がそのままパイプライン図になる。

## 技術スタック

| クレート | 用途 |
|---------|------|
| winit | クロスプラットフォームのウィンドウ管理とイベントループ |
| softbuffer | ウィンドウへのピクセル書き込み (GPU 不使用) |
| tiny-skia | CPU ラスタライザ (図形 → ピクセル) |
| fontdue | フォントラスタライザ (文字 → グリフ画像) |

## ビルド・実行

```bash
cd engine
cargo run
```

起動すると、パースした DOM ツリーと、`<style>` から取り出した CSS が
標準出力にダンプされる。表示するページは `engine/src/main.rs` の
`SAMPLE_HTML` を書き換えて試せる (HTML も CSS もその場で反映される)。

Linux で必要なパッケージ: `libxkbcommon-x11-0` (winit のキーボード処理が実行時にロードする)。
フォントは実行環境のシステムフォントを使う (探索パスは `engine/src/text.rs` の `FONT_PATHS`)。

## テスト

```bash
cd engine
cargo test
```

## 参考資料

- [Let's build a browser engine!](https://limpet.net/mbrubeck/2014/08/08/toy-layout-engine-1.html) - Matt Brubeck による Rust 製トイブラウザチュートリアル
- [Web Browser Engineering](https://browser.engineering/) - ブラウザの仕組みを体系的に解説する教科書
- [Robinson](https://github.com/mbrubeck/robinson) - 上記チュートリアルの参考実装
- [Servo](https://github.com/servo/servo) - Mozilla の Rust 製ブラウザエンジン
