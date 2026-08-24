//! テキスト描画: 文字 → グリフ画像 → ピクセル。
//!
//! 「文字を描く」は実は 2 段階の処理になっている:
//! 1. **ラスタライズ**: フォントファイル内の輪郭データ (ベジェ曲線) から、
//!    文字 1 つぶんの小さなグレースケール画像 (グリフ) を作る … fontdue の仕事
//! 2. **合成 (ブレンド)**: グリフ画像の各ピクセルの濃度 (カバレッジ) を
//!    アルファ値として、背景色と文字色を混ぜて描き込む … このモジュールの仕事
//!
//! 本物のブラウザではさらにシェーピング (合字・カーニング・アラビア文字の
//! 連結など、HarfBuzz が担当) が入るが、ここでは「1 文字ずつ横に並べる」
//! 最小実装で本質だけを見る。

use fontdue::{Font, FontSettings};
use tiny_skia::Pixmap;

use crate::display_list::Color;

/// フォントファイルの探索候補 (上から順に試す)。
/// OS ごとにフォントの置き場所が違うので、複数の候補を持っておく。
/// 2 つ以上ロードできた場合は「最初のフォントに無い文字は次で探す」
/// フォールバックを行う (日本語表示のため)
const FONT_PATHS: &[&str] = &[
    // Linux
    "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
    "/usr/share/fonts/truetype/fonts-japanese-gothic.ttf",
    "/usr/share/fonts/opentype/ipafont-gothic/ipag.ttf",
    // macOS
    "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
    "/System/Library/Fonts/Supplemental/Arial.ttf",
    // Windows
    "C:\\Windows\\Fonts\\meiryo.ttc",
    "C:\\Windows\\Fonts\\arial.ttf",
];

/// ロード済みフォントの集まり。グリフが見つかるまで順に探す
pub struct FontStack {
    fonts: Vec<Font>,
}

impl FontStack {
    /// システムフォントを探してロードする。1 つも見つからなければ Err
    pub fn load_system_fonts() -> Result<Self, String> {
        let mut fonts = Vec::new();
        for path in FONT_PATHS {
            if let Ok(bytes) = std::fs::read(path) {
                if let Ok(font) = Font::from_bytes(bytes, FontSettings::default()) {
                    fonts.push(font);
                }
            }
        }
        if fonts.is_empty() {
            Err(format!(
                "フォントが見つからない。text.rs の FONT_PATHS に環境のフォントを追加して: {FONT_PATHS:?}"
            ))
        } else {
            Ok(Self { fonts })
        }
    }

    /// この文字のグリフを持っているフォントを返す (フォントフォールバック)
    fn font_for(&self, ch: char) -> &Font {
        self.fonts
            .iter()
            .find(|f| f.lookup_glyph_index(ch) != 0)
            .unwrap_or(&self.fonts[0])
    }

    /// 1 行のテキストを pixmap に描く。
    /// `baseline_y` は文字が「座る」線。英字の p や y はこの線より下にはみ出す
    pub fn draw_text(
        &self,
        pixmap: &mut Pixmap,
        text: &str,
        x: f32,
        baseline_y: f32,
        size: f32,
        color: Color,
    ) {
        let width = pixmap.width() as i32;
        let height = pixmap.height() as i32;
        // ペン位置。1 文字描くごとに advance (送り幅) だけ右へ進む
        let mut pen_x = x;

        for ch in text.chars() {
            let font = self.font_for(ch);
            // ラスタライズ: 文字と大きさから「濃度の 2 次元配列」を得る
            // metrics には配置情報 (大きさ・ベースラインからのオフセット・送り幅) が入る
            let (metrics, coverage) = font.rasterize(ch, size);

            // グリフ画像の左上隅を計算する。
            // ymin は「ベースラインからグリフ下端までの距離」(下にはみ出すなら負)
            let glyph_left = pen_x as i32 + metrics.xmin;
            let glyph_top = baseline_y as i32 - metrics.height as i32 - metrics.ymin;

            // カバレッジをアルファ値としてブレンドしながら描き込む
            let data = pixmap.data_mut(); // RGBA (premultiplied) のバイト列
            for gy in 0..metrics.height {
                for gx in 0..metrics.width {
                    let px = glyph_left + gx as i32;
                    let py = glyph_top + gy as i32;
                    if px < 0 || py < 0 || px >= width || py >= height {
                        continue; // 画面外にはみ出た部分は描かない (クリッピング)
                    }
                    let alpha = coverage[gy * metrics.width + gx] as u32; // 0..=255
                    if alpha == 0 {
                        continue;
                    }
                    // out = 文字色 * α + 背景色 * (1 - α)  (アルファブレンディング)
                    let i = (py as usize * width as usize + px as usize) * 4;
                    data[i] = ((color.r as u32 * alpha + data[i] as u32 * (255 - alpha)) / 255) as u8;
                    data[i + 1] =
                        ((color.g as u32 * alpha + data[i + 1] as u32 * (255 - alpha)) / 255) as u8;
                    data[i + 2] =
                        ((color.b as u32 * alpha + data[i + 2] as u32 * (255 - alpha)) / 255) as u8;
                    // data[i + 3] (アルファ) は背景が不透明なので 255 のまま
                }
            }
            pen_x += metrics.advance_width;
        }
    }
}
