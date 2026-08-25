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

use crate::dom::Node;

/// 子を持てない要素 (void element)。`<br>` に `</br>` が無いのは文法エラーではなく仕様
const VOID_ELEMENTS: &[&str] = &["br", "hr", "img", "input", "meta", "link"];

/// HTML 文字列をパースして DOM ツリーのルートを返す。
/// `<html>` 要素が無ければ補って必ず 1 本の木にする (ブラウザも同じことをする)
pub fn parse(source: &str) -> Node {
    let mut parser = Parser { pos: 0, input: source.to_string() };
    let mut nodes = parser.parse_nodes();
    if nodes.len() == 1 && nodes[0].tag_name() == Some("html") {
        nodes.remove(0)
    } else {
        Node::elem("html".to_string(), HashMap::new(), nodes)
    }
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
            self.consume_whitespace();
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
        // 改行やインデント由来の連続空白を 1 個にたたむ
        // (HTML では空白の連続は 1 個とみなすルールがある)
        let collapsed: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
        Node::text(collapsed)
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

#[cfg(test)]
mod tests {
    use super::parse;
    use crate::dom::NodeType;

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

    #[test]
    fn parses_nested_elements() {
        let dom = parse("<html><div><p>text</p></div></html>");
        let div = &dom.children[0];
        assert_eq!(div.tag_name(), Some("div"));
        assert_eq!(div.children[0].tag_name(), Some("p"));
    }

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

    #[test]
    fn skips_comments_and_doctype() {
        let dom = parse("<!DOCTYPE html><html><!-- hidden --><p>shown</p></html>");
        assert_eq!(dom.children.len(), 1);
        assert_eq!(dom.children[0].tag_name(), Some("p"));
    }

    #[test]
    fn handles_void_elements() {
        let dom = parse("<html><p>a</p><hr><p>b</p></html>");
        let tags: Vec<_> = dom.children.iter().map(|n| n.tag_name().unwrap()).collect();
        assert_eq!(tags, vec!["p", "hr", "p"]);
    }

    #[test]
    fn wraps_fragments_in_html_root(){
        let dom = parse("<p>a</p><p>b</p>");
        assert_eq!(dom.tag_name(), Some("html"));
        assert_eq!(dom.children.len(), 2);
    }

    #[test]
    fn collapses_whitespace_in_text() {
        let dom = parse("<html><p>a\n   b</p></html>");
        match &dom.children[0].children[0].node_type {
            NodeType::Text(t) => assert_eq!(t, "a b"),
            _ => panic!("expected text node"),
        }
    }
}
