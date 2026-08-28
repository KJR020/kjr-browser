//! HTML パーサ: テキスト → DOM ツリー。
//!
//! パーサの仕事は「1 文字ずつ読み進めながら、構造を組み立てる」こと。
//! この実装は再帰下降パーサと呼ばれる書き方で、
//! 「要素をパースする関数が、子要素のパースのために自分自身を呼ぶ」
//! という再帰がそのまま木のネストに対応する。
//!
//! 本物の HTML パース (WHATWG 仕様) は 80 以上の状態を持つ状態機械で、
//! `<p>` の自動クローズや `<table>` の再配置などのエラー回復を定義しているが、
//! ここでは「行儀の良い HTML」を対象にした最小実装で仕組みを掴む。
//! (仕様準拠にしたくなったら html5ever に差し替えるのが進化パス)

use std::collections::HashMap;

use crate::dom::{Node, NodeType};

/// 子を持てない要素 (void element)。`<br>` に `</br>` が無いのは文法エラーではなく仕様。
/// HTML 仕様が定める 14 種すべてを列挙する。ここに漏れがあると、
/// 閉じタグを待ち続けて後続の兄弟要素を子として飲み込んでしまう
const VOID_ELEMENTS: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param",
    "source", "track", "wbr",
];

/// HTML 文字列をパースして DOM ツリーのルートを返す。
/// `<html>` 要素が無ければ補って必ず 1 本の木にする (ブラウザも同じことをする)
pub fn parse(source: &str) -> Node {
    let mut parser = Parser { pos: 0, input: source.to_string() };
    let mut nodes = parser.parse_nodes();
    // 文書のトップレベルには文章の流れ (インラインの文脈) が無いので、
    // ソースの改行やインデント由来の空白ノードは捨ててよい。
    // 要素の内部では単語の区切りとして意味を持つため残す
    nodes.retain(|node| !is_blank_text(node));
    if nodes.len() == 1 && nodes[0].tag_name() == Some("html") {
        nodes.remove(0)
    } else {
        Node::elem("html".to_string(), HashMap::new(), nodes)
    }
}

/// 中身が空白だけのテキストノードか
fn is_blank_text(node: &Node) -> bool {
    matches!(&node.node_type, NodeType::Text(text) if text.trim().is_empty())
}

/// パーサの状態は「入力文字列」と「今どこまで読んだか (pos)」だけ
struct Parser {
    pos: usize,
    input: String,
}

impl Parser {
    // ---- 低レベルの道具: 1 文字単位の読み取り ----

    /// 今の位置の文字を (読み進めずに) 見る
    fn next_char(&self) -> char {
        self.input[self.pos..].chars().next().unwrap()
    }

    /// 今の位置から文字列 s が始まっているか
    fn starts_with(&self, s: &str) -> bool {
        self.input[self.pos..].starts_with(s)
    }

    /// 入力を最後まで読み切ったか
    fn eof(&self) -> bool {
        self.pos >= self.input.len()
    }

    /// 1 文字読んで位置を進める
    fn consume_char(&mut self) -> char {
        let c = self.next_char();
        self.pos += c.len_utf8();
        c
    }

    /// 条件を満たす間、文字を読み続けて返す (字句解析の基本部品)
    fn consume_while(&mut self, test: impl Fn(char) -> bool) -> String {
        let mut result = String::new();
        while !self.eof() && test(self.next_char()) {
            result.push(self.consume_char());
        }
        result
    }

    /// 空白を読み飛ばす
    fn consume_whitespace(&mut self) {
        self.consume_while(char::is_whitespace);
    }

    /// タグ名・属性名に使える文字を読む
    fn parse_name(&mut self) -> String {
        self.consume_while(|c| c.is_ascii_alphanumeric() || c == '-')
    }

    // ---- ここから構造の組み立て ----

    /// ノードの列 (兄弟) をパースする。閉じタグか入力終端で止まる
    fn parse_nodes(&mut self) -> Vec<Node> {
        let mut nodes = Vec::new();
        loop {
            // ここで空白を読み飛ばしてはいけない。
            // `Hello, <strong>world</strong>!` の "Hello, " の後ろの空白のように、
            // タグの境界にある空白は単語の区切りとして意味を持つ
            if self.eof() || self.starts_with("</") {
                break;
            }
            if let Some(node) = self.parse_node() {
                nodes.push(node);
            }
        }
        nodes
    }

    /// ノード 1 個をパースする。先頭の文字で種類を見分ける (`<` ならタグ系、それ以外はテキスト)
    fn parse_node(&mut self) -> Option<Node> {
        if self.starts_with("<!--") {
            self.consume_comment();
            None
        } else if self.starts_with("<!") {
            // <!DOCTYPE html> などは表示に関係ないので読み飛ばす
            self.consume_while(|c| c != '>');
            if !self.eof() {
                self.consume_char();
            }
            None
        } else if self.next_char() == '<' {
            Some(self.parse_element())
        } else {
            Some(self.parse_text())
        }
    }

    /// コメント `<!-- ... -->` を読み飛ばす
    fn consume_comment(&mut self) {
        self.pos += "<!--".len();
        while !self.eof() && !self.starts_with("-->") {
            self.consume_char();
        }
        if !self.eof() {
            self.pos += "-->".len();
        }
    }

    /// テキストノード: 次のタグが始まるまでを 1 つの Text にする
    fn parse_text(&mut self) -> Node {
        let text = self.consume_while(|c| c != '<');
        Node::text(collapse_whitespace(&text))
    }

    /// 要素: `<tag attr="val">子...</tag>` をパースする。
    /// 子のパースで parse_nodes → parse_node → parse_element と再帰するのがミソ
    fn parse_element(&mut self) -> Node {
        // 開きタグ
        assert_eq!(self.consume_char(), '<');
        let tag_name = self.parse_name();
        let attributes = self.parse_attributes();

        // 自己終了タグ `<br/>` と void 要素 `<br>` は子を持たない
        if self.starts_with("/>") {
            self.pos += 2;
            return Node::elem(tag_name, attributes, Vec::new());
        }
        assert_eq!(self.consume_char(), '>');
        if VOID_ELEMENTS.contains(&tag_name.as_str()) {
            return Node::elem(tag_name, attributes, Vec::new());
        }

        // 子ノード (ここが再帰)
        let children = self.parse_nodes();

        // 閉じタグ。タグ名が合わなくても壊れないよう読み捨てる (最低限のエラー耐性)
        if self.starts_with("</") {
            self.pos += 2;
            self.parse_name();
            self.consume_whitespace();
            if !self.eof() && self.next_char() == '>' {
                self.consume_char();
            }
        }

        Node::elem(tag_name, attributes, children)
    }

    /// 属性列: `attr="val" flag attr2='v'` をパースする
    fn parse_attributes(&mut self) -> HashMap<String, String> {
        let mut attributes = HashMap::new();
        loop {
            self.consume_whitespace();
            if self.eof() || self.next_char() == '>' || self.starts_with("/>") {
                break;
            }
            let name = self.parse_name();
            if name.is_empty() {
                // 壊れた入力で無限ループしないための保険
                self.consume_char();
                continue;
            }
            let value = if !self.eof() && self.next_char() == '=' {
                self.consume_char();
                self.parse_attr_value()
            } else {
                // 値なし属性 (<input disabled> など) は空文字にする
                String::new()
            };
            attributes.insert(name, value);
        }
        attributes
    }

    /// 属性値: 引用符付き ("v" / 'v') と引用符なし (v) の両方を受け付ける
    fn parse_attr_value(&mut self) -> String {
        let open = self.next_char();
        if open == '"' || open == '\'' {
            self.consume_char();
            let value = self.consume_while(|c| c != open);
            if !self.eof() {
                self.consume_char();
            }
            value
        } else {
            self.consume_while(|c| !c.is_whitespace() && c != '>')
        }
    }
}

/// HTML の空白処理: 連続する空白 (改行・タブ含む) を半角スペース 1 個にたたむ。
/// 本来この処理は「行を組み立てるとき」に行全体へ適用されるものなので、
/// render.rs が連結後のテキストにも同じ関数を使う。
/// 端の空白も 1 個として保持するのが要点で、これがあるおかげで
/// `Hello, <strong>world</strong>!` が "Hello, world!" のまま連結できる。
/// 逆に端を落としてしまうと、描画側で区切りを補う羽目になり
/// "Hello, world !" のような余計な空白が入り込む
pub fn collapse_whitespace(text: &str) -> String {
    let mut out = String::new();
    let mut pending_space = false;
    for c in text.chars() {
        if c.is_whitespace() {
            pending_space = true;
        } else {
            if pending_space {
                out.push(' ');
                pending_space = false;
            }
            out.push(c);
        }
    }
    // 末尾の空白も 1 個だけ残す (空白しか無かった場合は " " になる)
    if pending_space {
        out.push(' ');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::parse;
    use crate::dom::NodeType;

    /// 最小のケース: 開きタグ・テキスト・閉じタグを読んで
    /// 「要素ノードの子にテキストノードがぶら下がる」形になること
    #[test]
    fn parses_single_element_with_text() {
        let dom = parse("<html><h1>Hello</h1></html>");
        let h1 = &dom.children[0];
        assert_eq!(h1.tag_name(), Some("h1"));
        match &h1.children[0].node_type {
            NodeType::Text(t) => assert_eq!(t, "Hello"),
            _ => panic!("expected text node"),
        }
    }

    /// 入れ子のタグが親子関係として正しくツリー化されること。
    /// パーサの再帰 (parse_element → parse_nodes → parse_element) が
    /// HTML のネストに対応していることの確認
    #[test]
    fn parses_nested_elements() {
        let dom = parse("<html><div><p>text</p></div></html>");
        let div = &dom.children[0];
        assert_eq!(div.tag_name(), Some("div"));
        assert_eq!(div.children[0].tag_name(), Some("p"));
    }

    /// 属性が名前と値のペアとして取り出せること。
    /// HTML では引用符が二重・一重・無しの 3 通り書けるので、そのすべてを受け付ける
    #[test]
    fn parses_attributes() {
        let dom = parse(r#"<html><div class="main" id=root data-x='1'></div></html>"#);
        let NodeType::Element(data) = &dom.children[0].node_type else {
            panic!("expected element");
        };
        assert_eq!(data.attributes.get("class").unwrap(), "main");
        assert_eq!(data.attributes.get("id").unwrap(), "root");
        assert_eq!(data.attributes.get("data-x").unwrap(), "1");
    }

    /// 画面に出ないもの (DOCTYPE 宣言と HTML コメント) が
    /// DOM ツリーに残らず読み飛ばされること。
    /// 残っていると子の数が 1 にならないので検出できる
    #[test]
    fn skips_comments_and_doctype() {
        let dom = parse("<!DOCTYPE html><html><!-- hidden --><p>shown</p></html>");
        assert_eq!(dom.children.len(), 1);
        assert_eq!(dom.children[0].tag_name(), Some("p"));
    }

    /// void 要素 (<hr> や <br> など閉じタグを持たないタグ) の扱い。
    /// 閉じタグを待ってしまうと後続の <p> が hr の子になってしまうため、
    /// 3 つが兄弟として並ぶことを確認する
    #[test]
    fn handles_void_elements() {
        let dom = parse("<html><p>a</p><hr><p>b</p></html>");
        let tags: Vec<_> = dom.children.iter().map(|n| n.tag_name().unwrap()).collect();
        assert_eq!(tags, vec!["p", "hr", "p"]);
    }

    /// <html> で囲まれていない断片的な HTML を渡された場合に、
    /// ルートを補って必ず 1 本の木にすること (ブラウザも同じ補完をする)
    #[test]
    fn wraps_fragments_in_html_root() {
        let dom = parse("<p>a</p><p>b</p>");
        assert_eq!(dom.tag_name(), Some("html"));
        assert_eq!(dom.children.len(), 2);
    }

    /// テキスト中の改行やインデントの連続空白が 1 個の半角スペースに
    /// まとめられること。HTML では空白の連続を 1 個とみなす仕様があり、
    /// ソースを整形しても表示が変わらないのはこの処理のおかげ
    #[test]
    fn collapses_whitespace_in_text() {
        let dom = parse("<html><p>a\n   b</p></html>");
        match &dom.children[0].children[0].node_type {
            NodeType::Text(t) => assert_eq!(t, "a b"),
            _ => panic!("expected text node"),
        }
    }

    /// void 要素の一覧に漏れが無いこと。閉じタグを待ってしまうと
    /// 後続の兄弟要素を子として飲み込んでしまうので、
    /// 仕様上の全 14 種が兄弟として並ぶことを確認する
    #[test]
    fn handles_all_void_elements() {
        let dom = parse(
            "<html><area><base><br><col><embed><hr><img><input><link><meta><param>\
             <source><track><wbr><p>after</p></html>",
        );
        let tags: Vec<_> = dom.children.iter().filter_map(|n| n.tag_name()).collect();
        assert_eq!(
            tags,
            vec![
                "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta",
                "param", "source", "track", "wbr", "p"
            ],
            "void 要素が後続の兄弟を子にしてしまっている"
        );
    }

    /// タグの境界にある空白がテキストノードの端に 1 個だけ残ること。
    /// ここを落とすと単語がくっつき、逆に描画側で補うと余計な空白が入る
    #[test]
    fn preserves_boundary_whitespace() {
        let dom = parse("<html><p>Hello, <strong>world</strong>!</p></html>");
        let p = &dom.children[0];
        let text_of = |i: usize| match &p.children[i].node_type {
            NodeType::Text(t) => t.clone(),
            _ => panic!("expected text node"),
        };
        assert_eq!(text_of(0), "Hello, ", "閉じ側の空白が残る");
        assert_eq!(text_of(2), "!", "空白が無い側には足さない");
    }

    /// 空白が無い境界には空白が生まれないこと
    #[test]
    fn does_not_invent_whitespace() {
        let dom = parse("<html><p>foo<strong>bar</strong>baz</p></html>");
        match &dom.children[0].children[0].node_type {
            NodeType::Text(t) => assert_eq!(t, "foo"),
            _ => panic!("expected text node"),
        }
    }
}
