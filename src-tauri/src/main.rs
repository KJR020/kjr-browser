#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{
    webview::{PageLoadEvent, WebviewBuilder},
    window::WindowBuilder,
    AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, Url, WebviewUrl, WindowEvent,
};

/// ツールバー WebView の高さ (論理ピクセル)
const TOOLBAR_HEIGHT: f64 = 48.0;
/// 起動時に表示するページ
const INITIAL_URL: &str = "https://example.com/";

/// アドレスバー入力を URL に正規化する。
/// - スキーム付きはそのまま
/// - ドメインらしき文字列 (ドットを含みスペースなし) は https:// を付与
/// - それ以外は DuckDuckGo で検索
fn normalize_url(input: &str) -> Result<Url, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("URL が空です".into());
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

fn content_webview(app: &AppHandle) -> Result<tauri::Webview, String> {
    app.webviews()
        .get("content")
        .cloned()
        .ok_or_else(|| "content webview not found".into())
}

#[tauri::command]
fn navigate(app: AppHandle, url: String) -> Result<(), String> {
    let content = content_webview(&app)?;
    let target = normalize_url(&url)?;
    content.navigate(target).map_err(|e| e.to_string())
}

#[tauri::command]
fn go_back(app: AppHandle) -> Result<(), String> {
    content_webview(&app)?
        .eval("history.back()")
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn go_forward(app: AppHandle) -> Result<(), String> {
    content_webview(&app)?
        .eval("history.forward()")
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn reload(app: AppHandle) -> Result<(), String> {
    content_webview(&app)?
        .eval("location.reload()")
        .map_err(|e| e.to_string())
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![navigate, go_back, go_forward, reload])
        .setup(|app| {
            let window = WindowBuilder::new(app, "main")
                .title("kjr-browser")
                .inner_size(1200.0, 800.0)
                .build()?;

            let scale = window.scale_factor()?;
            let logical: LogicalSize<f64> = window.inner_size()?.to_logical(scale);

            let toolbar = window.add_child(
                WebviewBuilder::new("toolbar", WebviewUrl::App("index.html".into())),
                LogicalPosition::new(0.0, 0.0),
                LogicalSize::new(logical.width, TOOLBAR_HEIGHT),
            )?;

            let content = window.add_child(
                WebviewBuilder::new(
                    "content",
                    WebviewUrl::External(INITIAL_URL.parse().expect("valid initial url")),
                )
                .on_page_load(|webview, payload| {
                    if let PageLoadEvent::Finished = payload.event() {
                        // ツールバー側がアドレスバー表示を同期するためのイベント
                        let _ = webview.emit("url-changed", payload.url().to_string());
                    }
                }),
                LogicalPosition::new(0.0, TOOLBAR_HEIGHT),
                LogicalSize::new(logical.width, (logical.height - TOOLBAR_HEIGHT).max(0.0)),
            )?;

            // Linux (webkit2gtk) では子 WebView はウィンドウの GtkBox に
            // pack_start(expand=true) で追加され、add_child の位置・サイズ指定や
            // set_bounds は無視される (均等分割になる)。そのため GtkBox の
            // レイアウトに乗り、ツールバー側だけ高さ固定・expand 無効にする。
            // こうするとリサイズ時の再配分も GTK が自動で行う。
            #[cfg(target_os = "linux")]
            toolbar.with_webview(|platform_webview| {
                use gtk::prelude::*;
                let widget = platform_webview.inner();
                widget.set_size_request(-1, TOOLBAR_HEIGHT as i32);
                if let Some(parent) = widget.parent() {
                    if let Ok(gtk_box) = parent.dynamic_cast::<gtk::Box>() {
                        gtk_box.set_child_packing(
                            &widget,
                            false,
                            true,
                            0,
                            gtk::PackType::Start,
                        );
                    }
                }
            })?;

            // Windows / macOS では子 WebView の位置・サイズ指定が有効なので、
            // リサイズ時に手動で追従させる (Linux では no-op)
            let win = window.clone();
            window.on_window_event(move |event| {
                if let WindowEvent::Resized(size) = event {
                    let scale = win.scale_factor().unwrap_or(1.0);
                    let logical: LogicalSize<f64> = size.to_logical(scale);
                    let _ = toolbar.set_size(LogicalSize::new(logical.width, TOOLBAR_HEIGHT));
                    let _ = content.set_position(LogicalPosition::new(0.0, TOOLBAR_HEIGHT));
                    let _ = content.set_size(LogicalSize::new(
                        logical.width,
                        (logical.height - TOOLBAR_HEIGHT).max(0.0),
                    ));
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::normalize_url;

    #[test]
    fn keeps_full_url() {
        assert_eq!(
            normalize_url("https://example.com/a?b=c").unwrap().as_str(),
            "https://example.com/a?b=c"
        );
    }

    #[test]
    fn adds_https_to_domain() {
        assert_eq!(
            normalize_url("example.com").unwrap().as_str(),
            "https://example.com/"
        );
    }

    #[test]
    fn falls_back_to_search() {
        let url = normalize_url("rust webview").unwrap();
        assert_eq!(url.host_str(), Some("duckduckgo.com"));
        assert_eq!(url.query(), Some("q=rust+webview"));
    }

    #[test]
    fn rejects_empty() {
        assert!(normalize_url("   ").is_err());
    }
}
