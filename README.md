# kjr-browser

Rust で作る学習用トイブラウザ。

ブラウザが URL を受け取ってからピクセルを画面に描くまでの全工程を、自分の手で実装しながら学ぶプロジェクト。

## 目的

- ブラウザエンジンの仕組みを体系的に理解する
- HTML パース、CSS、レイアウト、描画、ネットワーク通信の各レイヤーを実装する
- Rust の実践的なスキルを身につける

## アーキテクチャ

```
URL
 ↓
[Network] HTTP リクエスト → HTML バイト列
 ↓
[HTML Parser] → DOM ツリー
 ↓                          ↗ CSS バイト列
[CSS Parser] → CSS ルール
 ↓
[Style Resolution] → スタイルツリー (DOM + CSS プロパティ)
 ↓
[Layout Engine] → レイアウトツリー (ボックス, 座標, サイズ)
 ↓
[Painting] → ディスプレイリスト (描画コマンド)
 ↓
[Window/Renderer] → ピクセル → 画面
```

## 技術スタック

| カテゴリ | クレート | 用途 |
|---------|---------|------|
| ウィンドウ | winit | クロスプラットフォームウィンドウ管理 |
| ピクセルバッファ | softbuffer | ウィンドウへのピクセル書き込み |
| 2D 描画 | tiny-skia | CPU ベースのラスタライズ |
| HTTP | reqwest | HTTP/HTTPS 通信 |

将来的に html5ever, cssparser 等の本格的なクレートへの差し替えも想定。

## フェーズ

| Phase | テーマ | ゴール |
|-------|-------|--------|
| 1 | 画面に描画する | ウィンドウに矩形とテキストを表示 |
| 2 | HTML → DOM → 画面 | HTML 文字列をパースして描画 |
| 3 | CSS スタイル適用 | CSS でスタイルを変更できる |
| 4 | レイアウトエンジン | ボックスモデルで要素を正しく配置 |
| 5 | ネットワーク | URL から実際の Web ページを取得・表示 |
| 6 | インタラクション | リンククリック、スクロール、ページ遷移 |

詳細は [docs/learning-plan.md](docs/learning-plan.md) を参照。

## ビルド・実行

```bash
cargo run
```

## 参考資料

- [Let's build a browser engine!](https://limpet.net/mbrubeck/2014/08/08/toy-layout-engine-1.html) - Matt Brubeck による Rust 製トイブラウザチュートリアル
- [Web Browser Engineering](https://browser.engineering/) - ブラウザの仕組みを体系的に解説する教科書
- [Robinson](https://github.com/mbrubeck/robinson) - 上記チュートリアルの参考実装
- [Servo](https://github.com/servo/servo) - Mozilla の Rust 製ブラウザエンジン
