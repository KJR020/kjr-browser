//! ディスプレイリスト: 「何を描くか」の中間表現。
//!
//! ブラウザのレンダリングパイプラインは、最終的に
//! 「この矩形をこの色で塗れ」「この文字をここに描け」という
//! 単純な描画コマンドの列 (= ディスプレイリスト) に行き着く。
//!
//!   HTML → DOM → スタイルツリー → レイアウトツリー → **ディスプレイリスト** → ピクセル
//!
//! Phase 1 ではこのリストを手書きするが、Phase 2 以降では
//! HTML/CSS から自動生成されるようになる。つまりこの型が
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

/// Phase 1 用の手書きディスプレイリスト。
/// 「シンプルな Web ページをレイアウトした結果」を人力で再現している。
/// Phase 2〜4 が完成すると、これと同等のリストが HTML/CSS から自動で出てくる
pub fn sample_page() -> Vec<DisplayCommand> {
    const WHITE: Color = Color { r: 0xf7, g: 0xf7, b: 0xf9 };
    const NAVY: Color = Color { r: 0x2b, g: 0x3a, b: 0x67 };
    const BLUE: Color = Color { r: 0x3b, g: 0x5b, b: 0xdb };
    const CARD: Color = Color { r: 0xff, g: 0xff, b: 0xff };
    const TEXT: Color = Color { r: 0x33, g: 0x33, b: 0x3d };

    vec![
        // 背景 (body { background: ... } に相当)
        DisplayCommand::SolidRect { x: 0.0, y: 0.0, width: 900.0, height: 600.0, color: WHITE },
        // ヘッダーバー (header { background: navy } に相当)
        DisplayCommand::SolidRect { x: 0.0, y: 0.0, width: 900.0, height: 64.0, color: NAVY },
        DisplayCommand::Text {
            text: "kjr-engine".into(),
            x: 24.0,
            y: 42.0,
            size: 28.0,
            color: CARD,
        },
        // カード (div { background: white } に相当)
        DisplayCommand::SolidRect { x: 60.0, y: 120.0, width: 780.0, height: 320.0, color: CARD },
        // カードの左端のアクセント線 (border-left に相当)
        DisplayCommand::SolidRect { x: 60.0, y: 120.0, width: 6.0, height: 320.0, color: BLUE },
        DisplayCommand::Text {
            text: "Phase 1: 画面に描画する".into(),
            x: 96.0,
            y: 180.0,
            size: 32.0,
            color: NAVY,
        },
        DisplayCommand::Text {
            text: "このページはHTMLではなく、手書きのディスプレイリストから描画されている。".into(),
            x: 96.0,
            y: 232.0,
            size: 18.0,
            color: TEXT,
        },
        DisplayCommand::Text {
            text: "Phase 2 以降で、この描画コマンド列を HTML/CSS から自動生成する。".into(),
            x: 96.0,
            y: 264.0,
            size: 18.0,
            color: TEXT,
        },
        DisplayCommand::Text {
            text: "URL -> HTML -> DOM -> Style -> Layout -> DisplayList -> Pixels".into(),
            x: 96.0,
            y: 330.0,
            size: 20.0,
            color: BLUE,
        },
    ]
}
