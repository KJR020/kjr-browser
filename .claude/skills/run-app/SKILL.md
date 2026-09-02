---
name: run-app
description: kjr-engine (engine/) をヘッドレス環境の仮想ディスプレイ上で起動し、描画結果をスクリーンショットで確認する手順
---

# kjr-engine をヘッドレス環境で起動・確認する

GUI アプリなので、リモートコンテナでは Xvfb (仮想ディスプレイ) 上で起動し、
ImageMagick の `import` でスクリーンショットを撮って目視確認する。

## 1. 依存パッケージ (初回のみ)

```bash
apt-get install -y libxkbcommon-x11-0 fonts-dejavu-core fonts-ipafont-gothic \
  xvfb imagemagick x11-utils
```

- `libxkbcommon-x11-0` は winit がキーボード処理のために実行時に動的ロードする。
  無いと起動直後に `xkbcommon-dl` で panic する (ビルドは通るので気づきにくい)。
- フォントパッケージが 1 つも無いと `FontStack::load_system_fonts()` が失敗し、
  ウィンドウを作る前に `.expect("load fonts")` で落ちる。
  日本語表示には `fonts-ipafont-gothic` のような CJK フォントが要る。

## 2. ビルドと起動

起動したプロセスは PID をファイルに控える。後片付けで確実に止めるため。

```bash
cd engine && cargo build

# Xvfb は setsid nohup で切り離して起動する。
# そうしないと起動したシェルの終了に巻き込まれて死ぬ。
# PID は `sh -c 'echo $$ …; exec …'` で取る (理由は下の注意書き)
setsid nohup sh -c 'echo $$ > /tmp/kjr-xvfb.pid; exec Xvfb :99 -screen 0 1280x900x24' \
  >/dev/null 2>&1 < /dev/null &
sleep 2
DISPLAY=:99 xdotool getdisplaygeometry   # 生きているか確認 (1280 900 が返る)

setsid nohup sh -c 'echo $$ > /tmp/kjr-engine.pid;
  exec env DISPLAY=:99 ./target/debug/kjr-engine "$@"' _ "$@" \
  > /tmp/engine.log 2>&1 < /dev/null &
sleep 5
pgrep -x kjr-engine >/dev/null && echo OK || tail -20 /tmp/engine.log
```

**`$!` は使えない。** `setsid` は自分がプロセスグループリーダーだと fork するため、
`$!` が返すのは終了済みのラッパーの PID で、本体の PID とは別物になる
(実測: `$!` は 2663、実際の Xvfb は 2665)。
`sh -c` の中で `$$` を書き出してから `exec` すると、プロセスが置き換わるだけなので
PID が一致する。

任意の HTML を開くときは最後の行に引数を渡す:

```bash
setsid nohup sh -c 'echo $$ > /tmp/kjr-engine.pid;
  exec env DISPLAY=:99 ./target/debug/kjr-engine target/doc/kjr_engine/index.html' \
  > /tmp/engine.log 2>&1 < /dev/null &
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

**エンジンと Xvfb の両方**を、控えておいた PID で止める。
エンジンだけ止めると Xvfb が :99 を掴んだまま残り、次回の起動が古いディスプレイを
再利用したり、新しいサーバの起動に失敗したりする。

```bash
for f in /tmp/kjr-engine.pid /tmp/kjr-xvfb.pid; do
  [ -f "$f" ] && kill "$(cat "$f")" 2>/dev/null
  rm -f "$f"
done
sleep 3   # SIGTERM を受けてから終了しきるまで数秒かかる
pgrep -x Xvfb >/dev/null && echo "まだ残っている: $(pgrep -a Xvfb)" || echo "停止済み"
```

PID ファイルが無い (前回異常終了した等) ときのフォールバックは `pkill -x <名前>`。
**`pkill -f` は使わないこと。** 実行中のシェル自身のコマンドラインにもその文字列が
含まれるため、自分を巻き添えにして殺してしまう (exit code 143/144)。

なお、続けて何度も起動するなら Xvfb は落とさず使い回してよい。
その場合も「エンジンだけ止めて Xvfb は残す」ことを意識して行うこと
(残ったことに気づかないまま放置するのが一番まずい)。

## 既知の注意点

- Xvfb にはウィンドウマネージャがないため、リサイズは `xdotool windowsize` に
  ウィンドウ名ではなく X の window id を渡すのが確実 (`xwininfo -root -tree` で確認)。
- フォントは `engine/src/text.rs` の `FONT_PATHS` から探索する。
  「フォントが見つからない」で落ちたら、その環境のフォントパスを追加する。
