//! アドレスバー入力の解釈。
//!
//! ユーザーが入力した文字列を「開くべき URL」に変換するルールをここに集める。
//! ネットワークにも UI にも依存しない純粋なロジックなので、単体テストもこのファイルに置く。

use tauri::Url;

/// アドレスバー入力を URL に正規化する。
/// - スキーム付き (`http://` / `https://`) はそのまま
/// - ドメインらしき文字列 (ドットを含みスペースなし) は `https://` を補完
/// - それ以外は DuckDuckGo の検索 URL にする
pub fn normalize(input: &str) -> Result<Url, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("URL が空です".into());
    }
    if trimmed == "about:blank" {
        return Url::parse(trimmed).map_err(|e| e.to_string());
    }
    let candidate = if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        trimmed.to_string()
    } else if trimmed.contains('.') && !trimmed.contains(' ') {
        format!("https://{trimmed}")
    } else {
        return Url::parse_with_params("https://duckduckgo.com/", &[("q", trimmed)])
            .map_err(|e| e.to_string());
    };
    Url::parse(&candidate).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::normalize;

    #[test]
    fn keeps_full_url() {
        assert_eq!(
            normalize("https://example.com/a?b=c").unwrap().as_str(),
            "https://example.com/a?b=c"
        );
    }

    #[test]
    fn adds_https_to_domain() {
        assert_eq!(normalize("example.com").unwrap().as_str(), "https://example.com/");
    }

    #[test]
    fn falls_back_to_search() {
        let url = normalize("rust webview").unwrap();
        assert_eq!(url.host_str(), Some("duckduckgo.com"));
        assert_eq!(url.query(), Some("q=rust+webview"));
    }

    #[test]
    fn rejects_empty() {
        assert!(normalize("   ").is_err());
    }

    #[test]
    fn allows_about_blank() {
        assert_eq!(normalize("about:blank").unwrap().as_str(), "about:blank");
    }
}
