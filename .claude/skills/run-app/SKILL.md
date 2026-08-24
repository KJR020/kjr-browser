---
name: run-app
description: kjr-browser の GUI (engine/ のトイエンジン、src-tauri/ の Tauri シェル) をヘッドレス環境の仮想ディスプレイ上で起動・操作・スクリーンショットする手順
---

# kjr-browser をヘッドレス環境で起動・操作する

## 0. どちらを動かすか

- **engine/** (本線: フルスクラッチのトイエンジン) — `cd engine && cargo build` して
  `./target/debug/kjr-engine`。追加の実行時依存: `apt-get install -y libxkbcommon-x11-0`
  (無いと winit が起動時に panic する)。以下の Xvfb / スクリーンショット手順は共通。
- **src-tauri/** (シェル編: Tauri 製タブブラウザ) — 以下の手順の通り。

Tauri 製 GUI アプリなので、リモートコンテナでは Xvfb (仮想ディスプレイ) 上で起動し、
xdotool で操作、ImageMagick の `import` でスクリーンショットを撮って確認する。

## 1. 依存パッケージ (初回のみ)

```bash
apt-get install -y libwebkit2gtk-4.1-dev libgtk-3-dev librsvg2-dev \
  xvfb imagemagick xdotool x11-utils
```

Claude Code リモート環境では HTTPS が MITM プロキシ経由のため、WebView が
HTTPS ページを読めるようプロキシ CA を システムに登録する (初回のみ):

```bash
cp /root/.ccr/ca-bundle.crt /usr/local/share/ca-certificates/agentproxy.crt
update-ca-certificates
```

## 2. ビルドと起動

```bash
cd src-tauri && cargo build

Xvfb :99 -screen 0 1280x900x24 >/dev/null 2>&1 &
DISPLAY=:99 WEBKIT_DISABLE_COMPOSITING_MODE=1 WEBKIT_DISABLE_DMABUF_RENDERER=1 \
  ./target/debug/kjr-browser > /tmp/app.log 2>&1 &
sleep 6   # 起動待ち
```

- `WEBKIT_DISABLE_*` は GPU なし環境での WebKitGTK 描画に必須
- `libEGL warning: DRI3` はただの警告で無害
- Xvfb にはウィンドウマネージャがないため `xdotool windowsize` は
  ウィンドウ名でなく X の window id 指定 (`xwininfo -root -tree` で確認) が確実

## 3. ネットワーク制限とテストページ

コンテナのプロキシは許可ドメイン制で、example.com など一般サイトは 403 になる。
動作確認はローカル HTTP サーバで行う (127.0.0.1 は NO_PROXY 対象):

```bash
mkdir -p /tmp/www && cd /tmp/www
# index.html / page2.html (リンク付き) を作って:
python3 -m http.server 8090 --bind 127.0.0.1 &
```

ブラウザには `http://127.0.0.1:8090/` を入力させる。

## 4. 操作 (xdotool) とスクリーンショット

```bash
export DISPLAY=:99
# アドレスバーをクリックして URL 入力 (座標は ui/ のレイアウト依存)
xdotool mousemove 650 56 click 1        # アドレスバー (2段ツールバーの下段)
xdotool key ctrl+a
xdotool type --delay 30 "http://127.0.0.1:8090/"
xdotool key Return
sleep 4
import -window root /tmp/shot.png       # 撮ったら必ず Read で目視確認する
```

主要 UI の座標 (ウィンドウ 1200x800、ツールバー 2 段 80px):
- 1段目 (y≈16): タブチップ (固定幅 180px、x=8 から順に並ぶ)、その右に + ボタン
- 2段目 (y≈56): 戻る (x≈22) / 進む (x≈62) / 再読み込み (x≈100) / アドレスバー (x≈650)

## 5. 後片付け

```bash
kill %1 %2 2>/dev/null   # app と Xvfb (pid をファイルに控えておくと確実)
```

## 既知の注意点

- Linux では子 WebView は GtkBox に詰められ位置指定が効かない。ツールバーの
  高さ固定は main.rs の GTK コード (`set_child_packing`) が担う。レイアウトが
  半々に割れたらこの処理が壊れている。
- `cargo run` (tauri dev 相当ではない) で十分。frontendDist は `../ui` の静的ファイル。
