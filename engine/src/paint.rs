//! ペイント: ディスプレイリスト → ピクセルバッファ。
//!
//! 「ラスタライズ」と呼ばれる工程。図形の数学的な定義 (座標とサイズ) を、
//! 実際のピクセルの色の並びに変換する。矩形は tiny-skia に任せ、
//! テキストは text モジュールで自前ブレンドする。
//!
//! 出力の Pixmap は幅 x 高さ x 4 バイト (RGBA) のただの配列。
//! 画面とは「この配列を毎フレーム作り直して転送するもの」でしかない、
//! というのが Phase 1 の一番大事な体感ポイント。

use tiny_skia::{Paint, Pixmap, Rect, Transform};

use crate::display_list::DisplayCommand;
use crate::text::FontStack;

/// ディスプレイリストを実行して、window_width x window_height のピクセル画像を作る
pub fn paint(
    commands: &[DisplayCommand],
    fonts: &FontStack,
    width: u32,
    height: u32,
) -> Pixmap {
    let mut pixmap = Pixmap::new(width.max(1), height.max(1)).expect("pixmap allocation");

    // コマンドは「奥から手前」の順に並んでいる前提で、単純に上書きしていく
    // (ペインターズアルゴリズム: 後から描いたものが上に載る)
    for command in commands {
        match command {
            DisplayCommand::SolidRect { x, y, width, height, color } => {
                if let Some(rect) = Rect::from_xywh(*x, *y, *width, *height) {
                    let mut paint = Paint::default();
                    paint.set_color_rgba8(color.r, color.g, color.b, 255);
                    pixmap.fill_rect(rect, &paint, Transform::identity(), None);
                }
            }
            DisplayCommand::Text { text, x, y, size, color } => {
                fonts.draw_text(&mut pixmap, text, *x, *y, *size, *color);
            }
        }
    }
    pixmap
}

/// tiny-skia の RGBA バイト列を、softbuffer が要求する
/// 0x00RRGGBB 形式の u32 に変換して転送する。
/// 「同じピクセルでもライブラリごとにメモリ上の並びが違う」ことがよくあり、
/// グラフィックスプログラミングではこの手の変換が頻出する
pub fn copy_to_buffer(pixmap: &Pixmap, buffer: &mut [u32]) {
    for (i, pixel) in pixmap.pixels().iter().enumerate() {
        let r = pixel.red() as u32;
        let g = pixel.green() as u32;
        let b = pixel.blue() as u32;
        buffer[i] = (r << 16) | (g << 8) | b;
    }
}
