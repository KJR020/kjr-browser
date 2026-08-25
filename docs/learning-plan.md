# 学習計画: kjr-browser

> **これが本線の計画。** 実装は [engine/](../engine/) ディレクトリで進行中。
> (一時期 Tauri ベースのシェル開発に寄り道した。その成果は src-tauri/ + ui/ に
> 「シェル編」として残っている — ブラウザの「ガワ」側の学習教材としてどうぞ)

## 全体像

ブラウザは URL を受け取り、ピクセルを画面に出力するプログラムである。
このプロジェクトでは、そのパイプラインを6つのフェーズに分けて段階的に実装し、各フェーズで「何が起きているか」を理解する。

```
Phase 1    Phase 5    Phase 2       Phase 3       Phase 4       Phase 1
画面描画 ← ネットワーク ← HTMLパース ← CSSスタイル ← レイアウト ← 描画
(出力側)   (入力側)     (構造化)      (装飾)        (配置)       (表示)
```

> 実装順は出力側（Phase 1: 描画）から始める。
> 画面に何か見えると、後の作業がすべて目で確認できるようになるため。

---

## Phase 1: 画面に描画する

### 学ぶこと

- **ピクセルバッファ**: 画面はピクセル（RGBA値）の2次元配列である
- **イベントループ**: ウィンドウアプリケーションは「イベントを待つ→処理する→描画する」のループで動く
- **ラスタライズ**: 図形の定義（矩形の座標とサイズ）をピクセルに変換する処理

### やること

1. winit でウィンドウを開く
2. softbuffer でピクセルバッファを取得し、背景色を塗る
3. tiny-skia で矩形を描画する
4. テキストを描画する（フォントレンダリングの基本）

### 完了条件

- [x] ウィンドウが開く
- [x] 背景色を指定して塗りつぶせる
- [x] 矩形を任意の位置・サイズ・色で描画できる
- [x] テキストを画面に表示できる (日本語フォントフォールバック付き)

→ ✅ **Phase 1 完了** (engine/src/: main.rs = イベントループ, display_list.rs = 描画コマンド,
paint.rs = ラスタライズ, text.rs = フォント処理)

### 参考資料

- browser.engineering Chapter 2: Drawing to the Screen
- Matt Brubeck Part 7: Painting
- winit Getting Started: https://docs.rs/winit/latest/winit/

---

## Phase 2: HTML → DOM → 画面

### 学ぶこと

- **HTML パーサ**: HTML はテキストであり、パーサがそれを構造化データ（DOM ツリー）に変換する
- **トークナイザ**: 文字列を意味のある単位（タグ、属性、テキスト）に分割する状態機械
- **DOM (Document Object Model)**: HTML 文書の木構造表現。ノードの種類（Element, Text）と親子関係

### やること

1. HTML トークナイザを実装（`<tag attr="val">text</tag>` → トークン列）
2. ツリービルダーでトークンを DOM ツリーに変換
3. DOM ツリーを走査して描画コマンドに変換
4. Phase 1 の描画エンジンで画面に表示

### 完了条件

- [x] `<h1>Hello</h1>` をパースして DOM ノードを生成できる
- [x] ネストしたタグ `<div><p>text</p></div>` を正しくツリー化できる
- [x] 属性 `<div class="main" id="root">` をパースできる
- [x] DOM ツリーから画面に描画できる

→ ✅ **Phase 2 完了** (engine/src/: html.rs = 再帰下降パーサ, dom.rs = ツリー定義,
render.rs = DOM → ディスプレイリスト変換の仮実装。コメント・DOCTYPE・void 要素・
空白の畳み込みにも対応。起動時に DOM ツリーが標準出力へダンプされる)

### 参考資料

- browser.engineering Chapter 4: Constructing an HTML Tree
- Matt Brubeck Part 2: HTML
- WHATWG HTML Standard §13: Parsing HTML documents
- Robinson: dom.rs, html.rs

---

## Phase 3: CSS スタイル適用

### 学ぶこと

- **CSS 構文**: セレクタ + 宣言ブロック（プロパティ: 値）
- **セレクタマッチング**: どの CSS ルールがどの DOM ノードに適用されるか
- **カスケードと詳細度 (Specificity)**: 複数のルールが競合したときの優先順位
- **スタイルツリー**: DOM ツリーの各ノードに計算済みスタイルを付与したツリー

### やること

1. CSS パーサを実装（セレクタ、プロパティ、値）
2. セレクタマッチング（要素、class、ID セレクタ）
3. 詳細度計算とカスケード
4. スタイルツリーの構築（DOM + CSS → StyledNode）
5. スタイルに基づいて描画を変更（背景色、文字色、フォントサイズ）

### 完了条件

- [ ] `h1 { color: red; }` をパースできる
- [ ] `.class` `#id` `element` セレクタがマッチする
- [ ] 詳細度に基づいて競合を解決できる
- [ ] スタイルが画面描画に反映される

### 参考資料

- browser.engineering Chapter 6: Applying Author Styles
- Matt Brubeck Part 3: CSS, Part 4: Style
- CSS Cascading and Inheritance Level 4
- Robinson: css.rs, style.rs

---

## Phase 4: レイアウトエンジン

### 学ぶこと

- **ボックスモデル**: すべての要素は content + padding + border + margin の矩形
- **ブロックレイアウト**: ブロック要素は縦に積み重なる（通常フロー）
- **インラインレイアウト**: テキストや inline 要素は横に並び、行末で折り返す
- **レイアウトツリー**: スタイルツリーから生成される、座標とサイズを持つツリー

### やること

1. ボックスモデルのデータ構造（Rect, EdgeSizes, Dimensions）
2. ブロックレイアウトアルゴリズム（幅の計算→子の配置→高さの計算）
3. インラインレイアウト（テキストの折り返し）
4. レイアウトツリーから描画コマンドを生成

### 完了条件

- [ ] margin, padding, border が正しく計算される
- [ ] ブロック要素が縦に積み重なる
- [ ] 幅が親要素に制約される
- [ ] テキストが行末で折り返す

### 参考資料

- browser.engineering Chapter 5: Laying Out Pages
- Matt Brubeck Part 5: Boxes, Part 6: Block Layout
- CSS Box Model Level 3
- Robinson: layout.rs

---

## Phase 5: ネットワーク

### 学ぶこと

- **URL の構造**: scheme://host:port/path?query#fragment
- **HTTP プロトコル**: リクエスト（メソッド、ヘッダー、ボディ）とレスポンス（ステータス、ヘッダー、ボディ）
- **DNS 解決**: ホスト名から IP アドレスへの変換（概念のみ）
- **TLS/HTTPS**: 暗号化通信の概要（reqwest が内部で処理）

### やること

1. URL パーサの実装
2. reqwest を使った HTTP GET リクエスト
3. レスポンスの Content-Type に基づいて HTML として処理
4. アドレスバー的な UI（テキスト入力→ページ読み込み）

### 完了条件

- [ ] URL をスキーム、ホスト、ポート、パスに分解できる
- [ ] HTTP で HTML を取得できる
- [ ] HTTPS サイトにアクセスできる
- [ ] 取得した HTML を Phase 2-4 のパイプラインで表示できる

### 参考資料

- browser.engineering Chapter 1: Downloading Web Pages
- RFC 7230: HTTP/1.1 Message Syntax and Routing
- reqwest ドキュメント: https://docs.rs/reqwest/latest/reqwest/

---

## Phase 6: インタラクション

### 学ぶこと

- **ヒットテスト**: クリック座標がどの DOM 要素上にあるかを判定する
- **イベントハンドリング**: マウスクリック、キーボード入力の処理
- **ナビゲーション**: リンククリックによるページ遷移（URL変更→再取得→再描画）
- **スクロール**: ビューポートとドキュメントの関係

### やること

1. マウスクリック座標→レイアウトツリーのヒットテスト
2. `<a href="...">` のクリックでページ遷移
3. キーボード/マウスホイールによるスクロール
4. 戻る/進むの履歴管理

### 完了条件

- [ ] リンクをクリックして別ページに遷移できる
- [ ] ページをスクロールできる
- [ ] 戻る/進むボタンが機能する
- [ ] ホバー時にカーソルが変わる（任意）

### 参考資料

- browser.engineering Chapter 7: Handling Buttons and Links
- browser.engineering Chapter 13: Animations and Compositing
- UI Events Specification (W3C)

---

## 進化パス（トイブラウザの先）

Phase 6 完了後、興味に応じて以下の方向に進化できる:

| 方向 | 内容 | 差し替え候補 |
|------|------|-------------|
| HTML 仕様準拠 | エラー回復、暗黙のタグ挿入 | html5ever |
| CSS 拡張 | Flexbox, Grid, メディアクエリ | cssparser + selectors |
| JavaScript | JS エンジン統合、DOM API | boa (Rust製JSエンジン) |
| GPU 描画 | ハードウェアアクセラレーション | wgpu |
| マルチタブ | タブ管理、プロセス分離 | 独自実装 |
| DevTools | 要素インスペクター | 独自実装 |
