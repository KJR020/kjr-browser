//! スタイル解決: DOM + CSS → スタイルツリー。
//!
//! ブラウザは DOM の各ノードについて「結局このプロパティはどの値になるのか」を
//! 決定する必要がある。同じ要素に複数のルールが当たることがあるからだ。
//! その決定は 3 つの仕組みで行われる:
//!
//! 1. **セレクタマッチング** — そのルールはこの要素に当たるか?
//! 2. **カスケード** — 複数当たったとき、どれを優先するか?
//!    (オリジンの順 → 詳細度の順 → 記述順)
//! 3. **継承** — 指定が無いプロパティを親から受け継ぐか?
//!
//! 結果は「DOM と同じ形をしていて、各ノードに確定済みスタイルが付いた木」
//! = スタイルツリー になる。Phase 4 のレイアウトはこの木を入力に取る。

use std::collections::HashMap;

use crate::css::{Rule, Selector, Specificity, Stylesheet, Value};
use crate::dom::{ElementData, Node, NodeType};

/// ブラウザ内蔵のデフォルトスタイル (UA = User Agent スタイルシート)。
///
/// `<h1>` が大きく表示されるのは HTML の機能ではなく、
/// ブラウザがこういう CSS を内部に持っているからである。
/// Phase 2 では Rust のコードで決め打ちしていたものを、
/// Phase 3 では本物の CSS として書けるようになった —
/// 著者スタイルとまったく同じ仕組みで処理される
pub const USER_AGENT_CSS: &str = "
html, body, div, p, h1, h2, h3, ul, ol, li, hr { display: block; }
head, style, script, title { display: none; }
body { color: #33333d; font-size: 18px; background-color: #f7f7f9; }
p { margin-top: 12px; margin-bottom: 12px; }
h1 { font-size: 34px; color: #2b3a67; margin-top: 24px; margin-bottom: 16px; }
h2 { font-size: 26px; color: #2b3a67; margin-top: 20px; margin-bottom: 12px; }
h3 { font-size: 21px; color: #2b3a67; margin-top: 16px; margin-bottom: 10px; }
ul, ol { margin-top: 12px; margin-bottom: 12px; padding-left: 24px; }
li { margin-top: 4px; margin-bottom: 4px; }
hr { height: 2px; background-color: #c9cdd6; margin-top: 16px; margin-bottom: 18px; }
";

/// プロパティ名 → 確定した値
pub type PropertyMap = HashMap<String, Value>;

/// スタイルツリーのノード。DOM ノードへの参照と、確定済みスタイルを持つ
pub struct StyledNode<'a> {
    pub node: &'a Node,
    pub specified_values: PropertyMap,
    pub children: Vec<StyledNode<'a>>,
}

impl StyledNode<'_> {
    /// プロパティの値を引く
    pub fn value(&self, name: &str) -> Option<&Value> {
        self.specified_values.get(name)
    }

    /// 長さプロパティを引く (未指定なら default)
    pub fn px(&self, name: &str, default: f32) -> f32 {
        self.value(name).and_then(|v| v.to_px()).unwrap_or(default)
    }

    /// 色プロパティを引く
    pub fn color(&self, name: &str) -> Option<crate::display_list::Color> {
        self.value(name).and_then(|v| v.to_color())
    }

    /// キーワードプロパティを引く
    pub fn keyword(&self, name: &str) -> Option<&str> {
        self.value(name).and_then(|v| v.as_keyword())
    }

    /// この要素のタグ名
    pub fn tag_name(&self) -> Option<&str> {
        self.node.tag_name()
    }
}

/// 親から子へ受け継がれるプロパティ。
/// `color` を body に指定するだけで文書全体の文字色が変わるのは、
/// この継承の仕組みがあるから。逆に `margin` は継承されない
/// (継承したら入れ子のたびに余白が増えてしまう)
const INHERITED_PROPERTIES: &[&str] = &["color", "font-size"];

/// DOM ツリーとスタイルシートからスタイルツリーを作る。
/// `stylesheets` は優先度の低い順に渡す (UA スタイル → 著者スタイル)。
/// CSS ではオリジンの優先順位が詳細度より強いので、この順番自体が意味を持つ
pub fn style_tree<'a>(root: &'a Node, stylesheets: &[&Stylesheet]) -> StyledNode<'a> {
    style_node(root, stylesheets, &PropertyMap::new())
}

fn style_node<'a>(
    node: &'a Node,
    stylesheets: &[&Stylesheet],
    parent_values: &PropertyMap,
) -> StyledNode<'a> {
    let specified_values = match &node.node_type {
        NodeType::Element(element) => specified_values(element, stylesheets, parent_values),
        // テキストノードにはセレクタが当たらない。親から継承プロパティだけを受け継ぐ
        // (display や margin まで受け継ぐと、テキストが親と同じ箱のように扱われてしまう)
        NodeType::Text(_) => inherited_only(parent_values),
    };
    let children = node
        .children
        .iter()
        .map(|child| style_node(child, stylesheets, &specified_values))
        .collect();
    StyledNode { node, specified_values, children }
}

/// 継承されるプロパティだけを取り出す
fn inherited_only(parent_values: &PropertyMap) -> PropertyMap {
    let mut values = PropertyMap::new();
    for name in INHERITED_PROPERTIES {
        if let Some(value) = parent_values.get(*name) {
            values.insert(name.to_string(), value.clone());
        }
    }
    values
}

/// 1 要素ぶんのスタイルを確定する (カスケードの本体)
fn specified_values(
    element: &ElementData,
    stylesheets: &[&Stylesheet],
    parent_values: &PropertyMap,
) -> PropertyMap {
    // 1. まず親から継承プロパティを引き継ぐ (最も弱い)
    let mut values = inherited_only(parent_values);

    // 2. スタイルシートを優先度の低い順に適用する。
    //    同じオリジン内では詳細度の低い順に適用するので、
    //    強いルールが後から上書きすることになる
    for stylesheet in stylesheets {
        let mut rules = matching_rules(element, stylesheet);
        rules.sort_by(|(a, _), (b, _)| a.cmp(b));
        for (_, rule) in rules {
            for declaration in &rule.declarations {
                values.insert(declaration.name.clone(), declaration.value.clone());
            }
        }
    }
    values
}

/// この要素にマッチするルールを、詳細度付きで集める
fn matching_rules<'a>(
    element: &ElementData,
    stylesheet: &'a Stylesheet,
) -> Vec<(Specificity, &'a Rule)> {
    stylesheet
        .rules
        .iter()
        .filter_map(|rule| match_rule(element, rule))
        .collect()
}

/// ルールのセレクタのうち 1 つでも当たれば、その最大詳細度とともに返す
fn match_rule<'a>(element: &ElementData, rule: &'a Rule) -> Option<(Specificity, &'a Rule)> {
    rule.selectors
        .iter()
        .find(|selector| matches(element, selector))
        .map(|selector| (selector.specificity(), rule))
}

/// セレクタが要素に当たるか判定する。
/// 指定された条件を「すべて」満たす必要がある (AND 条件)
fn matches(element: &ElementData, selector: &Selector) -> bool {
    // タグ名の指定があり、一致しない
    if let Some(tag) = &selector.tag_name {
        if element.tag_name != *tag {
            return false;
        }
    }
    // ID の指定があり、一致しない
    if let Some(id) = &selector.id {
        if element.attributes.get("id") != Some(id) {
            return false;
        }
    }
    // class の指定があり、要素が持っていないものがある
    if !selector.classes.is_empty() {
        let element_classes: Vec<&str> = element
            .attributes
            .get("class")
            .map(|c| c.split_whitespace().collect())
            .unwrap_or_default();
        if !selector.classes.iter().all(|c| element_classes.contains(&c.as_str())) {
            return false;
        }
    }
    true
}

/// DOM から `<style>` 要素の中身をすべて集めて 1 本の CSS 文字列にする。
/// 本物のブラウザは外部 CSS (`<link rel="stylesheet">`) も取得して同様に扱うが、
/// それはネットワークが入る Phase 5 以降の話
pub fn extract_inline_styles(node: &Node) -> String {
    let mut css = String::new();
    collect_styles(node, &mut css);
    css
}

fn collect_styles(node: &Node, out: &mut String) {
    if node.tag_name() == Some("style") {
        for child in &node.children {
            if let NodeType::Text(text) = &child.node_type {
                out.push_str(text);
                out.push('\n');
            }
        }
        return;
    }
    for child in &node.children {
        collect_styles(child, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{css, html};

    /// HTML と CSS からスタイルツリーを作り、最初の子要素のスタイルを取り出すヘルパー
    fn style_first_child(html_src: &str, css_src: &str) -> PropertyMap {
        let dom = html::parse(html_src);
        let sheet = css::parse(css_src);
        let tree = style_tree(&dom, &[&sheet]);
        tree.children[0].specified_values.clone()
    }

    /// 要素セレクタが当たり、宣言が確定値として取り込まれること
    #[test]
    fn matches_element_selector() {
        let values = style_first_child("<html><p>x</p></html>", "p { font-size: 20px; }");
        assert_eq!(values.get("font-size").unwrap().to_px(), Some(20.0));
    }

    /// class セレクタが当たること。class 属性は複数の名前を空白区切りで持てるので、
    /// そのうち 1 つに一致すればマッチする
    #[test]
    fn matches_class_selector() {
        let values =
            style_first_child(r#"<html><p class="a card">x</p></html>"#, ".card { font-size: 9px; }");
        assert_eq!(values.get("font-size").unwrap().to_px(), Some(9.0));
    }

    /// ID セレクタが当たること
    #[test]
    fn matches_id_selector() {
        let values =
            style_first_child(r#"<html><p id="main">x</p></html>"#, "#main { font-size: 7px; }");
        assert_eq!(values.get("font-size").unwrap().to_px(), Some(7.0));
    }

    /// セレクタの条件は AND であること。
    /// `.card` を持たない要素には `p.card` は当たらない
    #[test]
    fn requires_all_conditions_to_match() {
        let values = style_first_child("<html><p>x</p></html>", "p.card { font-size: 99px; }");
        assert!(values.get("font-size").is_none());
    }

    /// 詳細度による競合解決。同じプロパティを 3 つのルールが指定していても、
    /// 最も詳細度の高い ID セレクタの値が採用される (記述順は問わない)
    #[test]
    fn resolves_conflict_by_specificity() {
        let values = style_first_child(
            r#"<html><p id="main" class="card">x</p></html>"#,
            "#main { font-size: 30px; } p { font-size: 10px; } .card { font-size: 20px; }",
        );
        assert_eq!(values.get("font-size").unwrap().to_px(), Some(30.0));
    }

    /// 詳細度が同じ場合は、後に書かれたルールが勝つこと
    #[test]
    fn later_rule_wins_on_equal_specificity() {
        let values = style_first_child(
            "<html><p>x</p></html>",
            "p { font-size: 10px; } p { font-size: 40px; }",
        );
        assert_eq!(values.get("font-size").unwrap().to_px(), Some(40.0));
    }

    /// オリジンの優先順位が詳細度より強いこと。
    /// UA スタイルシートの ID セレクタより、著者スタイルの要素セレクタが勝つ
    #[test]
    fn author_origin_beats_user_agent_origin() {
        let dom = html::parse(r#"<html><p id="main">x</p></html>"#);
        let ua = css::parse("#main { font-size: 50px; }");
        let author = css::parse("p { font-size: 12px; }");
        let tree = style_tree(&dom, &[&ua, &author]);
        let font_size = tree.children[0].specified_values.get("font-size").unwrap().to_px();
        assert_eq!(font_size, Some(12.0));
    }

    /// 継承されるプロパティ (color) は、指定の無い子要素に親から伝わること
    #[test]
    fn inherits_color_to_descendants() {
        let dom = html::parse("<html><body><p>x</p></body></html>");
        let sheet = css::parse("body { color: red; }");
        let tree = style_tree(&dom, &[&sheet]);
        let p = &tree.children[0].children[0];
        let color = p.color("color").unwrap();
        assert_eq!((color.r, color.g, color.b), (0xff, 0x00, 0x00));
    }

    /// 継承されないプロパティ (margin-top) は子に伝わらないこと。
    /// 継承してしまうと入れ子のたびに余白が増えてしまう
    #[test]
    fn does_not_inherit_margin() {
        let dom = html::parse("<html><body><p>x</p></body></html>");
        let sheet = css::parse("body { margin-top: 40px; }");
        let tree = style_tree(&dom, &[&sheet]);
        let p = &tree.children[0].children[0];
        assert!(p.value("margin-top").is_none());
    }

    /// <style> 要素の中身が CSS 文字列として取り出せること
    #[test]
    fn extracts_inline_style_element() {
        let dom = html::parse("<html><head><style>p { color: red; }</style></head></html>");
        let css_text = extract_inline_styles(&dom);
        assert!(css_text.contains("p { color: red; }"), "取り出した CSS: {css_text}");
    }
}
