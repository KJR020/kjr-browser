//! ディスプレイリスト: 「何を描くか」の中間表現。
//!
//! ブラウザのレンダリングパイプラインは、最終的に
//! 「この矩形をこの色で塗れ」「この文字をここに描け」という
//! 単純な描画コマンドの列 (= ディスプレイリスト) に行き着く。
//!
//!   HTML → DOM → スタイルツリー → レイアウトツリー → **ディスプレイリスト** → ピクセル
//!
//! Phase 1 ではこのリストを手書きしていたが、Phase 2 からは
//! render.rs が DOM から自動生成する。この型が
//! 「パース・レイアウトの世界」と「描画の世界」の接続点になる。

/// 不透明な RGB 色。CSS の `#rrggbb` に相当
#[derive(Clone, Copy)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

/// 描画コマンド 1 件。ブラウザはどんな複雑なページも
/// 最終的にこの程度の単純な命令の列に分解する
pub enum DisplayCommand {
    /// 単色の矩形 (CSS の background-color や border の正体)
    SolidRect {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: Color,
    },
    /// テキスト 1 行。`y` はベースライン (文字が「座る」線) の位置
    Text {
        text: String,
        x: f32,
        y: f32,
        size: f32,
        color: Color,
    },
}

