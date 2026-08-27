//! DOM → ディスプレイリスト変換 (Phase 2 の仮実装)。
//!
//! 本物のブラウザではこの間に 2 つの大きな工程が挟まる:
//!
//!   DOM → [Style: CSS を当てる] → スタイルツリー
//!       → [Layout: 位置と大きさを計算] → レイアウトツリー → ディスプレイリスト
//!
//! Phase 2 の時点ではまだ CSS が無いので、このモジュールが
//! 「タグ名ごとの決め打ちスタイル」(ブラウザの UA スタイルシート相当) と
//! 「上から順に縦に積むだけの素朴なレイアウト」を一時的に兼任する。
//! Phase 3 で style.rs に、Phase 4 で layout.rs に、それぞれ役割を明け渡して消える運命。

use crate::display_list::{Color, DisplayCommand};
use crate::dom::{Node, NodeType};

const PAGE_BG: Color = Color { r: 0xf7, g: 0xf7, b: 0xf9 };
const TEXT: Color = Color { r: 0x33, g: 0x33, b: 0x3d };
const HEADING: Color = Color { r: 0x2b, g: 0x3a, b: 0x67 };
const RULE: Color = Color { r: 0xc9, g: 0xcd, b: 0xd6 };

/// タグごとの決め打ちスタイル。ブラウザに内蔵されている
/// 「UA (User Agent) スタイルシート」のごく簡略版。
/// h1 が大きく太く見えるのは HTML の機能ではなく、UA スタイルのおかげ —
/// というのが Phase 3 (CSS) につながる伏線
struct TagStyle {
    font_size: f32,
    color: Color,
    margin_top: f32,
    margin_bottom: f32,
}

fn style_for(tag: &str) -> TagStyle {
    match tag {
        "h1" => TagStyle { font_size: 34.0, color: HEADING, margin_top: 24.0, margin_bottom: 16.0 },
        "h2" => TagStyle { font_size: 26.0, color: HEADING, margin_top: 20.0, margin_bottom: 12.0 },
        "li" => TagStyle { font_size: 18.0, color: TEXT, margin_top: 4.0, margin_bottom: 4.0 },
        _ => TagStyle { font_size: 18.0, color: TEXT, margin_top: 12.0, margin_bottom: 12.0 },
    }
}

/// これより下に「積んで」いく Y 座標カーソル。
/// ブロック要素を 1 つ描くたびに、その高さぶんだけ下に進む。
/// たったこれだけでも「ブロック要素は縦に積み重なる」という
/// CSS 通常フローの基本が再現できる
struct Renderer {
    commands: Vec<DisplayCommand>,
    cursor_y: f32,
    viewport_width: f32,
}

/// DOM ツリーからディスプレイリストを生成する
pub fn render(dom: &Node, viewport_width: f32, viewport_height: f32) -> Vec<DisplayCommand> {
    let mut renderer = Renderer { commands: Vec::new(), cursor_y: 16.0, viewport_width };
    // 背景 (body の背景に相当)
    renderer.commands.push(DisplayCommand::SolidRect {
        x: 0.0,
        y: 0.0,
        width: viewport_width,
        height: viewport_height,
        color: PAGE_BG,
    });
    renderer.render_block_children(dom, 32.0);
    renderer.commands
}

impl Renderer {
    /// ブロック要素の子たちを縦に積みながら描画コマンド化する
    fn render_block_children(&mut self, node: &Node, x: f32) {
        for child in &node.children {
            self.render_block(child, x);
        }
    }

    /// ブロック要素 1 つを処理する
    fn render_block(&mut self, node: &Node, x: f32) {
        match &node.node_type {
            NodeType::Element(data) => match data.tag_name.as_str() {
                // 構造タグ: 自分は何も描かず子に任せる
                "html" | "body" => self.render_block_children(node, x),
                // ul は子の li を少し右にずらして描く (インデント)
                "ul" | "ol" => {
                    let style = style_for("ul");
                    self.cursor_y += style.margin_top;
                    self.render_block_children(node, x + 24.0);
                    self.cursor_y += style.margin_bottom;
                }
                // hr は水平線 = 細い矩形
                "hr" => {
                    self.cursor_y += 16.0;
                    self.commands.push(DisplayCommand::SolidRect {
                        x,
                        y: self.cursor_y,
                        width: self.viewport_width - x * 2.0,
                        height: 2.0,
                        color: RULE,
                    });
                    self.cursor_y += 18.0;
                }
                // それ以外はテキストを持つブロックとして描く
                tag => {
                    let style = style_for(tag);
                    let mut text = collect_inline_text(node);
                    if tag == "li" {
                        text = format!("・{text}"); // list-style の超簡略版
                    }
                    if text.is_empty() {
                        // テキストが無ければコンテナ扱いで子だけ処理する
                        self.render_block_children(node, x);
                        return;
                    }
                    self.cursor_y += style.margin_top + style.font_size;
                    self.commands.push(DisplayCommand::Text {
                        text,
                        x,
                        y: self.cursor_y, // Text の y はベースライン位置
                        size: style.font_size,
                        color: style.color,
                    });
                    self.cursor_y += style.margin_bottom;
                }
            },
            // ブロックの直下に裸のテキストがあった場合 (body 直下など)
            NodeType::Text(text) => {
                if !text.is_empty() {
                    let style = style_for("p");
                    self.cursor_y += style.margin_top + style.font_size;
                    self.commands.push(DisplayCommand::Text {
                        text: text.clone(),
                        x,
                        y: self.cursor_y,
                        size: style.font_size,
                        color: style.color,
                    });
                    self.cursor_y += style.margin_bottom;
                }
            }
        }
    }
}

/// 要素の中のテキストを、インライン要素 (<strong> や <a> など) を
/// またいで 1 本の文字列に平坦化する。
/// 本来インライン要素は太字や色などスタイルを変えるが、それは Phase 3 の仕事。
/// ここでは「文の途中にタグがあっても 1 行として扱う」ことだけを実現する
fn collect_inline_text(node: &Node) -> String {
    let mut out = String::new();
    for child in &node.children {
        match &child.node_type {
            NodeType::Text(text) => {
                if !text.is_empty() {
                    if !out.is_empty() {
                        out.push(' ');
                    }
                    out.push_str(text);
                }
            }
            NodeType::Element(_) => {
                let inner = collect_inline_text(child);
                if !inner.is_empty() {
                    if !out.is_empty() {
                        out.push(' ');
                    }
                    out.push_str(&inner);
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::html;

    /// DOM がディスプレイリスト (描画コマンドの列) に変換されること。
    /// 「背景の矩形 1 個 + テキスト 2 個」が期待値で、
    /// h1 と p の中身がテキストコマンドとして順番どおりに出てくることを見る
    #[test]
    fn generates_background_and_text() {
        let dom = html::parse("<html><h1>Title</h1><p>body text</p></html>");
        let commands = render(&dom, 900.0, 600.0);
        // 背景矩形 1 + テキスト 2
        assert_eq!(commands.len(), 3);
        let texts: Vec<&str> = commands
            .iter()
            .filter_map(|c| match c {
                DisplayCommand::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(texts, vec!["Title", "body text"]);
    }

    /// ブロック要素が縦に積み重なること (CSS の通常フローの基本)。
    /// 先に書かれた <p> の方が Y 座標が小さい = 画面の上にあることを確認する
    #[test]
    fn blocks_stack_downward() {
        let dom = html::parse("<html><p>first</p><p>second</p></html>");
        let commands = render(&dom, 900.0, 600.0);
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
    /// 行が分断されず 1 本のテキストにつながること。
    /// DOM 上は「テキスト・要素・テキスト」の 3 ノードに割れているのを平坦化している
    #[test]
    fn flattens_inline_elements() {
        let dom = html::parse("<html><p>a <strong>b</strong> c</p></html>");
        let commands = render(&dom, 900.0, 600.0);
        let text = commands
            .iter()
            .find_map(|c| match c {
                DisplayCommand::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .unwrap();
        assert_eq!(text, "a b c");
    }
}
