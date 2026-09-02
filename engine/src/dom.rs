//! DOM (Document Object Model): HTML 文書の木構造表現。
//!
//! HTML はただのテキストだが、ブラウザは内部でそれを「ノードの木」として持つ。
//! JavaScript の `document.getElementById(...)` などが操作しているのはこの木。
//!
//!   <div class="card"><p>hello</p></div>
//!
//!   ↓ パース (html.rs) すると…
//!
//!   Element("div", {class: "card"})
//!     └─ Element("p")
//!          └─ Text("hello")
//!
//! ノードには大きく 2 種類ある:
//! - **Element**: タグ。名前と属性を持ち、子を持てる
//! - **Text**: タグに挟まれた生のテキスト。木の葉になる
//! (本物の DOM には Comment, Document など他の種類もあるが、まずはこの 2 つで十分)

use std::collections::HashMap;

/// DOM ツリーのノード 1 個。子を Vec で持つことで木になる
pub struct Node {
    pub node_type: NodeType,
    pub children: Vec<Node>,
}

/// ノードの種類
pub enum NodeType {
    Element(ElementData),
    Text(String),
}

/// 要素ノードの中身: タグ名と属性
pub struct ElementData {
    pub tag_name: String,
    pub attributes: HashMap<String, String>,
}

impl Node {
    /// テキストノードを作る
    pub fn text(data: String) -> Node {
        Node { node_type: NodeType::Text(data), children: Vec::new() }
    }

    /// 要素ノードを作る
    pub fn elem(tag_name: String, attributes: HashMap<String, String>, children: Vec<Node>) -> Node {
        Node {
            node_type: NodeType::Element(ElementData { tag_name, attributes }),
            children,
        }
    }

    /// この要素のタグ名 (テキストノードなら None)
    pub fn tag_name(&self) -> Option<&str> {
        match &self.node_type {
            NodeType::Element(data) => Some(&data.tag_name),
            NodeType::Text(_) => None,
        }
    }

    /// デバッグ用: ツリーをインデント付きで文字列化する。
    /// `cargo run` すると起動時に標準出力へダンプされるので、
    /// 「HTML がどう木になったか」を目で確認できる
    pub fn dump(&self, indent: usize) -> String {
        let pad = "  ".repeat(indent);
        let mut out = match &self.node_type {
            NodeType::Element(data) => {
                let attrs: Vec<String> =
                    data.attributes.iter().map(|(k, v)| format!(" {k}=\"{v}\"")).collect();
                format!("{pad}Element <{}{}>\n", data.tag_name, attrs.join(""))
            }
            NodeType::Text(text) => format!("{pad}Text {:?}\n", text),
        };
        for child in &self.children {
            out.push_str(&child.dump(indent + 1));
        }
        out
    }
}
