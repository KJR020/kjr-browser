//! スタイルツリー → ディスプレイリスト変換 (Phase 3 時点の実装)。
//!
//! 本物のブラウザではこの間にレイアウト工程が挟まる:
//!
//!   スタイルツリー → [Layout: 位置と大きさを計算] → レイアウトツリー → ディスプレイリスト
//!
//! Phase 3 の時点ではまだレイアウトエンジンが無いので、このモジュールが
//! 「上から順に縦に積むだけの素朴なレイアウト」を暫定的に兼任している。
//! Phase 4 で layout.rs に切り出し、ここは純粋な「描画コマンドの発行」だけになる予定。
//!
//! Phase 2 との違い: タグ名を見て決め打ちしていたスタイルが消え、
//! すべて StyledNode の確定済みプロパティから読むようになった。
//! 見た目の決定権が Rust のコードから CSS に移っている

use crate::display_list::{Color, DisplayCommand};
use crate::dom::NodeType;
use crate::style::StyledNode;

/// 行の高さ = フォントサイズ x この係数 (CSS の line-height の簡略版)
const LINE_HEIGHT_FACTOR: f32 = 1.4;
/// 本文の左右の余白
const PAGE_MARGIN: f32 = 32.0;

struct Renderer {
    commands: Vec<DisplayCommand>,
    /// 次にブロックを置く Y 座標。1 つ描くたびに下へ進む
    cursor_y: f32,
    viewport_width: f32,
}

/// スタイルツリーからディスプレイリストを生成する
pub fn render(root: &StyledNode, viewport_width: f32, viewport_height: f32) -> Vec<DisplayCommand> {
    let mut renderer = Renderer { commands: Vec::new(), cursor_y: 16.0, viewport_width };

    // ページ全体の背景。body に background-color があればそれを使う
    let page_bg = find_body_background(root).unwrap_or(Color { r: 0xff, g: 0xff, b: 0xff });
    renderer.commands.push(DisplayCommand::SolidRect {
        x: 0.0,
        y: 0.0,
        width: viewport_width,
        height: viewport_height,
        color: page_bg,
    });

    renderer.render_block(root, PAGE_MARGIN);
    renderer.commands
}

/// body 要素の background-color を探す (ページ背景に使う)
fn find_body_background(styled: &StyledNode) -> Option<Color> {
    if styled.tag_name() == Some("body") {
        return styled.color("background-color");
    }
    styled.children.iter().find_map(find_body_background)
}

impl Renderer {
    /// ブロック 1 つを処理して、その高さぶんカーソルを下へ進める
    fn render_block(&mut self, styled: &StyledNode, x: f32) {
        // display: none なら自分も子孫も一切描かない。
        // <head> や <style> が画面に出ないのも、実は UA スタイルシートの
        // `head, style { display: none }` によるもので、特別扱いではない
        if styled.keyword("display") == Some("none") {
            return;
        }
        self.cursor_y += styled.px("margin-top", 0.0);
        let content_x = x + styled.px("padding-left", 0.0);

        // height が指定されたブロック (hr など) は、その高さの矩形として描く。
        // 本物のブラウザでも <hr> は「高さと背景色を持つブロック」でしかない
        if let Some(height) = styled.value("height").and_then(|v| v.to_px()) {
            if let Some(bg) = styled.color("background-color") {
                self.commands.push(DisplayCommand::SolidRect {
                    x: content_x,
                    y: self.cursor_y,
                    width: self.viewport_width - content_x - PAGE_MARGIN,
                    height,
                    color: bg,
                });
            }
            self.cursor_y += height + styled.px("margin-bottom", 0.0);
            return;
        }

        // 行を組み立ててから空白を畳む。ノード単位ではなく行全体で畳むのが要点で、
        // display:none で消えた要素の前後に空白が 2 つ残るのを防げる。
        // 行頭・行末の空白は表示しない (CSS の空白処理と同じ)
        let text = crate::html::collapse_whitespace(&collect_inline_text(styled));
        let text = text.trim();
        if text.is_empty() {
            // テキストを持たないコンテナは、子を順に処理するだけ
            for child in &styled.children {
                self.render_block(child, content_x);
            }
        } else {
            self.draw_text_block(styled, text, content_x);
        }
        self.cursor_y += styled.px("margin-bottom", 0.0);
    }

    /// テキストを 1 行として描く (背景色があれば行の裏に矩形を敷く)
    fn draw_text_block(&mut self, styled: &StyledNode, text: &str, x: f32) {
        let font_size = styled.px("font-size", 18.0);
        let color = styled.color("color").unwrap_or(Color { r: 0x33, g: 0x33, b: 0x3d });
        let line_height = font_size * LINE_HEIGHT_FACTOR;

        // list-style の超簡略版
        let text = if styled.tag_name() == Some("li") {
            format!("・{text}")
        } else {
            text.to_string()
        };

        if let Some(bg) = styled.color("background-color") {
            self.commands.push(DisplayCommand::SolidRect {
                x,
                y: self.cursor_y,
                width: self.viewport_width - x - PAGE_MARGIN,
                height: line_height,
                color: bg,
            });
        }

        // ベースライン = 行の上端 + フォントサイズ (下に少し余白が残る位置)
        self.commands.push(DisplayCommand::Text {
            text,
            x,
            y: self.cursor_y + font_size,
            size: font_size,
            color,
        });
        self.cursor_y += line_height;
    }
}

/// 要素の中のテキストを、インライン要素 (<strong> や <a> など) をまたいで
/// 1 本の文字列に平坦化する。display: none の子は飛ばすので、
/// インライン要素にも display: none が効く。
/// (インライン要素ごとに色や太さを変えるのは Phase 4 のインラインレイアウトの仕事)
fn collect_inline_text(styled: &StyledNode) -> String {
    let mut out = String::new();
    for child in &styled.children {
        if child.keyword("display") == Some("none") {
            continue;
        }
        // ブロック要素が入っている場合はコンテナ扱いにするため、テキストを集めない
        if child.keyword("display") == Some("block") {
            return String::new();
        }
        // 区切りの空白を足さずにそのまま連結する。
        // 単語の区切りはパーサ (html.rs の collapse_whitespace) が
        // テキストノードの端に残してくれているので、ここで補うと
        // `world!` が `world !` のように離れてしまう
        match &child.node.node_type {
            NodeType::Text(text) => out.push_str(text),
            NodeType::Element(_) => out.push_str(&collect_inline_text(child)),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{css, html, style};

    /// HTML と CSS からディスプレイリストを生成するヘルパー。
    /// UA スタイルシートも噛ませて、実際の描画と同じ条件にする
    fn render_page(html_src: &str, css_src: &str) -> Vec<DisplayCommand> {
        let dom = html::parse(html_src);
        let ua = css::parse(style::USER_AGENT_CSS);
        let author = css::parse(css_src);
        let styled = style::style_tree(&dom, &[&ua, &author]);
        render(&styled, 900.0, 600.0)
    }

    /// 描画されたテキストだけを順番に取り出す
    fn texts(commands: &[DisplayCommand]) -> Vec<String> {
        commands
            .iter()
            .filter_map(|c| match c {
                DisplayCommand::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect()
    }

    /// テキストを持つブロックが順に描画コマンドになること
    #[test]
    fn generates_text_commands() {
        let commands = render_page("<html><body><h1>Title</h1><p>body</p></body></html>", "");
        assert_eq!(texts(&commands), vec!["Title", "body"]);
    }

    /// CSS で指定した文字色とフォントサイズが、描画コマンドに反映されること。
    /// Phase 3 の肝: 見た目の決定権が Rust のコードから CSS に移っている
    #[test]
    fn applies_css_color_and_font_size() {
        let commands = render_page(
            "<html><body><p>x</p></body></html>",
            "p { color: #ff0000; font-size: 42px; }",
        );
        let DisplayCommand::Text { color, size, .. } =
            commands.iter().find(|c| matches!(c, DisplayCommand::Text { .. })).unwrap()
        else {
            unreachable!()
        };
        assert_eq!((color.r, color.g, color.b), (0xff, 0x00, 0x00));
        assert_eq!(*size, 42.0);
    }

    /// 詳細度の高いルールが実際の描画にまで反映されること
    /// (パーサ → スタイル解決 → 描画 の一連の流れの結合テスト)
    #[test]
    fn specificity_affects_rendering() {
        let commands = render_page(
            r#"<html><body><p id="main">x</p></body></html>"#,
            "p { font-size: 10px; } #main { font-size: 60px; }",
        );
        let DisplayCommand::Text { size, .. } =
            commands.iter().find(|c| matches!(c, DisplayCommand::Text { .. })).unwrap()
        else {
            unreachable!()
        };
        assert_eq!(*size, 60.0);
    }

    /// display: none の要素が、自分も子孫も一切描画されないこと
    #[test]
    fn display_none_hides_element() {
        let commands = render_page(
            r#"<html><body><p>見える</p><p class="x">消える</p></body></html>"#,
            ".x { display: none; }",
        );
        assert_eq!(texts(&commands), vec!["見える"]);
    }

    /// ブロック要素が縦に積み重なること (CSS の通常フローの基本)。
    /// 先に書かれた <p> の方が Y 座標が小さい = 画面の上にある
    #[test]
    fn blocks_stack_downward() {
        let commands = render_page("<html><body><p>first</p><p>second</p></body></html>", "");
        let ys: Vec<f32> = commands
            .iter()
            .filter_map(|c| match c {
                DisplayCommand::Text { y, .. } => Some(*y),
                _ => None,
            })
            .collect();
        assert!(ys[0] < ys[1], "後のブロックほど下に積まれる");
    }

    /// 文の途中にインライン要素 (<strong> など) があっても、
    /// 行が分断されず 1 本のテキストにつながること
    #[test]
    fn flattens_inline_elements() {
        let commands =
            render_page("<html><body><p>a <strong>b</strong> c</p></body></html>", "");
        assert_eq!(texts(&commands), vec!["a b c"]);
    }

    /// background-color が矩形の描画コマンドになること。
    /// 背景はテキストより先に発行される (後から描いたものが上に載るため)
    #[test]
    fn draws_background_behind_text() {
        let commands = render_page(
            "<html><body><p>x</p></body></html>",
            "p { background-color: #00ff00; }",
        );
        let bg_index = commands
            .iter()
            .position(|c| matches!(c, DisplayCommand::SolidRect { color, .. }
                if (color.r, color.g, color.b) == (0x00, 0xff, 0x00)))
            .expect("背景の矩形が見つからない");
        let text_index = commands
            .iter()
            .position(|c| matches!(c, DisplayCommand::Text { .. }))
            .unwrap();
        assert!(bg_index < text_index, "背景はテキストより先に描かれる");
    }

    /// <style> の中身が本文として画面に出てしまわないこと
    #[test]
    fn does_not_render_style_element_content() {
        let commands =
            render_page("<html><head><style>p { color: red; }</style></head><body><p>x</p></body></html>", "");
        assert_eq!(texts(&commands), vec!["x"]);
    }

    /// 句読点の前に空白が入り込まないこと。
    /// パーサが端の空白を保持し、描画側が区切りを補わないことで成立する
    #[test]
    fn keeps_punctuation_attached() {
        let commands =
            render_page("<html><body><p>Hello, <strong>world</strong>!</p></body></html>", "");
        assert_eq!(texts(&commands), vec!["Hello, world!"]);
    }

    /// 空白の無い境界で単語が分断されないこと
    #[test]
    fn does_not_split_adjacent_inline_text() {
        let commands =
            render_page("<html><body><p>foo<strong>bar</strong>baz</p></body></html>", "");
        assert_eq!(texts(&commands), vec!["foobarbaz"]);
    }

    /// インライン要素どうしの間にある空白は、単語の区切りとして残ること
    #[test]
    fn keeps_space_between_inline_elements() {
        let commands = render_page(
            "<html><body><p><strong>a</strong> <strong>b</strong></p></body></html>",
            "",
        );
        assert_eq!(texts(&commands), vec!["a b"]);
    }

    /// ソースの改行やインデントが、表示されるテキストに漏れ出さないこと
    #[test]
    fn ignores_source_indentation() {
        let commands = render_page(
            "<html>\n  <body>\n    <p>text</p>\n  </body>\n</html>",
            "",
        );
        assert_eq!(texts(&commands), vec!["text"]);
    }

    /// display:none で消えた要素の前後の空白が、1 個に畳まれること。
    /// 空白の畳み込みはノード単位ではなく行全体で行われる
    #[test]
    fn collapses_whitespace_around_hidden_element() {
        let commands = render_page(
            r#"<html><body><p>a <span class="x">hidden</span> b</p></body></html>"#,
            ".x { display: none; }",
        );
        assert_eq!(texts(&commands), vec!["a b"]);
    }
}
