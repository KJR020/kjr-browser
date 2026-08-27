---
name: run-app
description: kjr-engine (engine/) をヘッドレス環境の仮想ディスプレイ上で起動し、描画結果をスクリーンショットで確認する手順
---

# kjr-engine をヘッドレス環境で起動・確認する

GUI アプリなので、リモートコンテナでは Xvfb (仮想ディスプレイ) 上で起動し、
ImageMagick の `import` でスクリーンショットを撮って目視確認する。

## 1. 依存パッケージ (初回のみ)

```bash
apt-get install -y libxkbcommon-x11-0 xvfb imagemagick x11-utils
```

`libxkbcommon-x11-0` は winit がキーボード処理のために実行時に動的ロードする。
無いと起動直後に `xkbcommon-dl` で panic する (ビルドは通るので気づきにくい)。

## 2. ビルドと起動

```bash
cd engine && cargo build

# Xvfb は setsid nohup で切り離して起動する。
# そうしないと起動したシェルの終了に巻き込まれて死ぬ
setsid nohup Xvfb :99 -screen 0 1280x900x24 >/dev/null 2>&1 < /dev/null &
sleep 2
DISPLAY=:99 xdotool getdisplaygeometry   # 生きているか確認 (1280 900 が返る)

setsid nohup env DISPLAY=:99 ./target/debug/kjr-engine > /tmp/engine.log 2>&1 < /dev/null &
sleep 5
pgrep -x kjr-engine >/dev/null && echo OK || tail -20 /tmp/engine.log
```

## 3. 確認

```bash
DISPLAY=:99 import -window root /tmp/shot.png
```

撮ったら **必ず Read ツールで画像を目視確認する**。真っ黒・真っ白なら起動失敗。

標準出力にはパース結果の DOM ツリーがダンプされるので、
描画がおかしいときは「パースの問題か描画の問題か」をここで切り分けられる:

```bash
head -30 /tmp/engine.log
```

表示内容を変えたいときは `engine/src/main.rs` の `SAMPLE_HTML` を書き換えて再ビルドする。

## 4. 後片付け

```bash
pkill -x kjr-engine
```

`pkill -f kjr-engine` は使わないこと。実行中のシェル自身のコマンドラインにも
その文字列が含まれるため、自分を巻き添えにして殺してしまう (exit code 143/144)。

## 既知の注意点

- Xvfb にはウィンドウマネージャがないため、リサイズは `xdotool windowsize` に
  ウィンドウ名ではなく X の window id を渡すのが確実 (`xwininfo -root -tree` で確認)。
- フォントは `engine/src/text.rs` の `FONT_PATHS` から探索する。
  「フォントが見つからない」で落ちたら、その環境のフォントパスを追加する。
