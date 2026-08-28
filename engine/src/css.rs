//! CSS パーサ: テキスト → スタイルシート。
//!
//! CSS の構造は HTML よりずっと単純で、「ルール」の並びでできている:
//!
//!   h1, .title { color: red; font-size: 32px; }
//!   ~~~~~~~~~~   ~~~~~~~~~~~~~~~~~~~~~~~~~~~~
//!   セレクタ列    宣言ブロック (プロパティ: 値 の並び)
//!
//! パーサの書き方は html.rs と同じ「1 文字ずつ読み進める」方式。
//!
//! 本物の CSS にはセレクタの結合子 (`div > p`)、擬似クラス (`:hover`)、
//! メディアクエリ、`calc()` などがあるが、ここでは
//! 要素・class・ID の 3 種のセレクタに絞って仕組みを掴む。

use crate::display_list::Color;

/// スタイルシート 1 枚 = ルールの並び
pub struct Stylesheet {
    pub rules: Vec<Rule>,
}

/// ルール 1 個 = 「どれに」(セレクタ) + 「何を」(宣言)
pub struct Rule {
    pub selectors: Vec<Selector>,
    pub declarations: Vec<Declaration>,
}

/// セレクタ (単純セレクタのみ)。`div#main.card` のように組み合わせられる。
/// 3 つのフィールドがすべて None/空なら `*` (全称セレクタ) 扱い
pub struct Selector {
    pub tag_name: Option<String>,
    pub id: Option<String>,
    pub classes: Vec<String>,
}

/// 詳細度 (specificity)。(ID の数, class の数, 要素名の数) の 3 つ組で表す。
/// タプルの比較は左から順に行われるので、これだけで CSS の優先順位が決まる:
/// ID 1 個はどんなに class を並べたルールより強い
pub type Specificity = (usize, usize, usize);

impl Selector {
    pub fn specificity(&self) -> Specificity {
        (
            self.id.iter().count(),
            self.classes.len(),
            self.tag_name.iter().count(),
        )
    }
}

/// 宣言 1 個 = `color: red` のようなプロパティと値の組
pub struct Declaration {
    pub name: String,
    pub value: Value,
}

/// CSS の値。型を分けておくと、描画側で「px なのか色なのか」を
/// 取り違えずに扱える
#[derive(Clone)]
pub enum Value {
    /// `block`, `none`, `bold` など
    Keyword(String),
    /// 長さ。単位は px のみ扱う
    Length(f32),
    /// `#fff` や `red`
    ColorValue(Color),
}

impl Value {
    /// 長さとして取り出す (長さでなければ None)
    pub fn to_px(&self) -> Option<f32> {
        match self {
            Value::Length(px) => Some(*px),
            _ => None,
        }
    }

    /// 色として取り出す
    pub fn to_color(&self) -> Option<Color> {
        match self {
            Value::ColorValue(c) => Some(*c),
            _ => None,
        }
    }

    /// キーワードとして取り出す
    pub fn as_keyword(&self) -> Option<&str> {
        match self {
            Value::Keyword(k) => Some(k),
            _ => None,
        }
    }
}

/// CSS 文字列をパースしてスタイルシートを返す
pub fn parse(source: &str) -> Stylesheet {
    let mut parser = Parser { pos: 0, input: source.to_string() };
    Stylesheet { rules: parser.parse_rules() }
}

struct Parser {
    pos: usize,
    input: String,
}

impl Parser {
    // ---- html.rs と同じ、1 文字単位の読み取り道具 ----

    fn next_char(&self) -> char {
        self.input[self.pos..].chars().next().unwrap()
    }

    fn eof(&self) -> bool {
        self.pos >= self.input.len()
    }

    fn consume_char(&mut self) -> char {
        let c = self.next_char();
        self.pos += c.len_utf8();
        c
    }

    fn consume_while(&mut self, test: impl Fn(char) -> bool) -> String {
        let mut result = String::new();
        while !self.eof() && test(self.next_char()) {
            result.push(self.consume_char());
        }
        result
    }

    fn consume_whitespace(&mut self) {
        self.consume_while(char::is_whitespace);
    }

    /// 識別子 (タグ名・class 名・プロパティ名など) に使える文字を読む
    fn parse_identifier(&mut self) -> String {
        self.consume_while(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    }

    // ---- ここから構造の組み立て ----

    fn parse_rules(&mut self) -> Vec<Rule> {
        let mut rules = Vec::new();
        loop {
            self.consume_whitespace();
            if self.eof() {
                break;
            }
            match self.parse_rule() {
                Some(rule) => rules.push(rule),
                // 壊れたルールは読み飛ばす (CSS は「読めない部分は無視」が仕様)
                None => continue,
            }
        }
        rules
    }

    fn parse_rule(&mut self) -> Option<Rule> {
        let selectors = self.parse_selectors();
        let declarations = self.parse_declarations();
        if selectors.is_empty() {
            None
        } else {
            Some(Rule { selectors, declarations })
        }
    }

    /// カンマ区切りのセレクタ列を読む。`{` の手前まで
    fn parse_selectors(&mut self) -> Vec<Selector> {
        let mut selectors = Vec::new();
        loop {
            self.consume_whitespace();
            if self.eof() || self.next_char() == '{' {
                break;
            }
            if self.next_char() == ',' {
                self.consume_char();
                continue;
            }
            let selector = self.parse_selector();
            if selector.tag_name.is_none() && selector.id.is_none() && selector.classes.is_empty() {
                // 何も読めなかった場合は 1 文字進めて無限ループを避ける
                if !self.eof() && self.next_char() != '{' && self.next_char() != ',' {
                    self.consume_char();
                }
            } else {
                selectors.push(selector);
            }
        }
        // 詳細度の高い順に並べておくと、後で最も強いものを取り出しやすい
        selectors.sort_by(|a, b| b.specificity().cmp(&a.specificity()));
        selectors
    }

    /// 単純セレクタ 1 個。`div`, `.card`, `#main`, `div.card#main` など
    fn parse_selector(&mut self) -> Selector {
        let mut selector = Selector { tag_name: None, id: None, classes: Vec::new() };
        while !self.eof() {
            match self.next_char() {
                '#' => {
                    self.consume_char();
                    selector.id = Some(self.parse_identifier());
                }
                '.' => {
                    self.consume_char();
                    selector.classes.push(self.parse_identifier());
                }
                '*' => {
                    // 全称セレクタ: 何も指定しない = すべてにマッチ
                    self.consume_char();
                }
                c if c.is_ascii_alphanumeric() => {
                    selector.tag_name = Some(self.parse_identifier());
                }
                _ => break,
            }
        }
        selector
    }

    /// 宣言ブロック `{ prop: value; ... }` を読む
    fn parse_declarations(&mut self) -> Vec<Declaration> {
        let mut declarations = Vec::new();
        self.consume_whitespace();
        if self.eof() || self.next_char() != '{' {
            return declarations;
        }
        self.consume_char(); // '{'
        loop {
            self.consume_whitespace();
            if self.eof() {
                break;
            }
            if self.next_char() == '}' {
                self.consume_char();
                break;
            }
            match self.parse_declaration() {
                Some(declaration) => declarations.push(declaration),
                None => {
                    // 読めない宣言は次の ; か } まで飛ばす
                    self.consume_while(|c| c != ';' && c != '}');
                    if !self.eof() && self.next_char() == ';' {
                        self.consume_char();
                    }
                }
            }
        }
        declarations
    }

    /// 宣言 1 個 `color: red;`
    fn parse_declaration(&mut self) -> Option<Declaration> {
        let name = self.parse_identifier();
        if name.is_empty() {
            return None;
        }
        self.consume_whitespace();
        if self.eof() || self.next_char() != ':' {
            return None;
        }
        self.consume_char(); // ':'
        self.consume_whitespace();
        let value = self.parse_value();
        self.consume_whitespace();
        if !self.eof() && self.next_char() == ';' {
            self.consume_char();
        }
        Some(Declaration { name, value })
    }

    /// 値。先頭の文字で種類を見分ける (数字なら長さ、# なら色、それ以外はキーワード)
    fn parse_value(&mut self) -> Value {
        if self.eof() {
            return Value::Keyword(String::new());
        }
        match self.next_char() {
            '#' => self.parse_hex_color(),
            c if c.is_ascii_digit() || c == '.' || c == '-' => self.parse_length(),
            _ => {
                let keyword = self.consume_while(|c| c != ';' && c != '}' && c != '\n');
                let keyword = keyword.trim().to_string();
                // 名前付きの色ならその場で色に変換する
                match named_color(&keyword) {
                    Some(color) => Value::ColorValue(color),
                    None => Value::Keyword(keyword),
                }
            }
        }
    }

    /// `16px` / `1.5em` のような長さ。単位は px 以外も読み捨てて数値だけ使う
    fn parse_length(&mut self) -> Value {
        let number = self.consume_while(|c| c.is_ascii_digit() || c == '.' || c == '-');
        self.consume_while(|c| c.is_ascii_alphabetic() || c == '%'); // 単位を読み捨てる
        Value::Length(number.parse().unwrap_or(0.0))
    }

    /// `#fff` (3 桁) と `#ffffff` (6 桁) の両方を受け付ける
    fn parse_hex_color(&mut self) -> Value {
        self.consume_char(); // '#'
        let hex = self.consume_while(|c| c.is_ascii_hexdigit());
        let expand = |s: &str| u8::from_str_radix(s, 16).unwrap_or(0);
        let color = match hex.len() {
            // 3 桁は各桁を 2 回繰り返す (#f00 → #ff0000)
            3 => Color {
                r: expand(&hex[0..1].repeat(2)),
                g: expand(&hex[1..2].repeat(2)),
                b: expand(&hex[2..3].repeat(2)),
            },
            6 => Color {
                r: expand(&hex[0..2]),
                g: expand(&hex[2..4]),
                b: expand(&hex[4..6]),
            },
            _ => Color { r: 0, g: 0, b: 0 },
        };
        Value::ColorValue(color)
    }
}

/// 名前付きの色。本物の CSS には 140 種以上あるが、ここでは代表的なものだけ
fn named_color(name: &str) -> Option<Color> {
    let rgb = match name {
        "black" => (0x00, 0x00, 0x00),
        "white" => (0xff, 0xff, 0xff),
        "red" => (0xff, 0x00, 0x00),
        "green" => (0x00, 0x80, 0x00),
        "blue" => (0x00, 0x00, 0xff),
        "gray" | "grey" => (0x80, 0x80, 0x80),
        "silver" => (0xc0, 0xc0, 0xc0),
        "navy" => (0x00, 0x00, 0x80),
        "teal" => (0x00, 0x80, 0x80),
        "orange" => (0xff, 0xa5, 0x00),
        _ => return None,
    };
    Some(Color { r: rgb.0, g: rgb.1, b: rgb.2 })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 最小のルール `h1 { color: red; }` がセレクタと宣言に分解されること。
    /// 名前付きの色がパース時点で RGB 値に変換されることも確認する
    #[test]
    fn parses_simple_rule() {
        let sheet = parse("h1 { color: red; }");
        assert_eq!(sheet.rules.len(), 1);
        let rule = &sheet.rules[0];
        assert_eq!(rule.selectors[0].tag_name.as_deref(), Some("h1"));
        assert_eq!(rule.declarations[0].name, "color");
        let color = rule.declarations[0].value.to_color().unwrap();
        assert_eq!((color.r, color.g, color.b), (0xff, 0x00, 0x00));
    }

    /// 3 種類のセレクタ (要素・class・ID) がそれぞれ正しい場所に格納されること
    #[test]
    fn parses_selector_kinds() {
        let sheet = parse("div.card#main { color: black; }");
        let selector = &sheet.rules[0].selectors[0];
        assert_eq!(selector.tag_name.as_deref(), Some("div"));
        assert_eq!(selector.id.as_deref(), Some("main"));
        assert_eq!(selector.classes, vec!["card"]);
    }

    /// 詳細度が (ID 数, class 数, 要素数) として数えられること。
    /// タプルの比較順がそのまま CSS の優先順位になる
    #[test]
    fn computes_specificity() {
        let id_sheet = parse("#main { color: red; }");
        let class_sheet = parse(".card { color: red; }");
        let tag_sheet = parse("p { color: red; }");
        let id = id_sheet.rules[0].selectors[0].specificity();
        let class = class_sheet.rules[0].selectors[0].specificity();
        let tag = tag_sheet.rules[0].selectors[0].specificity();
        assert_eq!(id, (1, 0, 0));
        assert_eq!(class, (0, 1, 0));
        assert_eq!(tag, (0, 0, 1));
        assert!(id > class && class > tag, "ID > class > 要素 の順に強い");
    }

    /// カンマ区切りで複数のセレクタを 1 ルールに書けること
    #[test]
    fn parses_selector_list() {
        let sheet = parse("h1, h2, .title { color: navy; }");
        assert_eq!(sheet.rules[0].selectors.len(), 3);
    }

    /// 16 進カラーが 3 桁と 6 桁の両方で読めること。
    /// 3 桁は各桁を 2 回繰り返した色と等しくなる (#f00 == #ff0000)
    #[test]
    fn parses_hex_colors() {
        let sheet = parse("a { color: #f00; } b { color: #00ff80; }");
        let short = sheet.rules[0].declarations[0].value.to_color().unwrap();
        let long = sheet.rules[1].declarations[0].value.to_color().unwrap();
        assert_eq!((short.r, short.g, short.b), (0xff, 0x00, 0x00));
        assert_eq!((long.r, long.g, long.b), (0x00, 0xff, 0x80));
    }

    /// 長さの値から単位が取り除かれ、数値として取り出せること
    #[test]
    fn parses_length_values() {
        let sheet = parse("p { font-size: 16px; margin-top: 1.5em; }");
        assert_eq!(sheet.rules[0].declarations[0].value.to_px(), Some(16.0));
        assert_eq!(sheet.rules[0].declarations[1].value.to_px(), Some(1.5));
    }

    /// 色でも長さでもない値はキーワードとして保持されること (display: none など)
    #[test]
    fn parses_keyword_values() {
        let sheet = parse("p { display: none; }");
        assert_eq!(sheet.rules[0].declarations[0].value.as_keyword(), Some("none"));
    }

    /// 壊れた宣言があっても、そのルール内の後続の宣言は生き残ること
    /// (CSS の「読めない部分だけ無視する」というエラー処理)
    #[test]
    fn skips_malformed_declaration() {
        let sheet = parse("p { color; font-size: 20px; }");
        let names: Vec<&str> =
            sheet.rules[0].declarations.iter().map(|d| d.name.as_str()).collect();
        assert_eq!(names, vec!["font-size"]);
    }
}
