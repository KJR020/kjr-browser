#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::collections::HashMap;
use std::sync::Mutex;

use tauri::{
    webview::{PageLoadEvent, WebviewBuilder},
    window::WindowBuilder,
    AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, Url, WebviewUrl, Window,
};

/// ツールバー WebView の高さ (論理ピクセル)。タブ列 + ナビ列の 2 段。
const TOOLBAR_HEIGHT: f64 = 80.0;
/// 起動時に表示するページ
const INITIAL_URL: &str = "https://example.com/";
/// 新規タブで開くページ
const NEW_TAB_URL: &str = "about:blank";

/// タブの状態。キーは content WebView のラベル ("tab-N")。
#[derive(Default)]
struct TabsState {
    counter: usize,
    active: String,
    order: Vec<String>,
    urls: HashMap<String, String>,
}

#[derive(serde::Serialize, Clone)]
struct TabInfo {
    label: String,
    url: String,
    active: bool,
}

/// アドレスバー入力を URL に正規化する。
/// - スキーム付きはそのまま
/// - ドメインらしき文字列 (ドットを含みスペースなし) は https:// を付与
/// - それ以外は DuckDuckGo で検索
fn normalize_url(input: &str) -> Result<Url, String> {
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

fn tab_list(tabs: &TabsState) -> Vec<TabInfo> {
    tabs.order
        .iter()
        .map(|label| TabInfo {
            label: label.clone(),
            url: tabs.urls.get(label).cloned().unwrap_or_default(),
            active: *label == tabs.active,
        })
        .collect()
}

/// 現在のタブ一覧をツールバーに通知する
fn emit_tabs(app: &AppHandle, tabs: &TabsState) {
    let _ = app.emit("tabs-changed", tab_list(tabs));
}

/// コンテンツ領域の位置・サイズ (Windows/macOS 用。Linux では GtkBox が配分する)
fn content_bounds(window: &Window) -> tauri::Result<(LogicalPosition<f64>, LogicalSize<f64>)> {
    let scale = window.scale_factor()?;
    let logical: LogicalSize<f64> = window.inner_size()?.to_logical(scale);
    Ok((
        LogicalPosition::new(0.0, TOOLBAR_HEIGHT),
        LogicalSize::new(logical.width, (logical.height - TOOLBAR_HEIGHT).max(0.0)),
    ))
}

/// 新しいタブ (content WebView) を作ってアクティブにする
fn create_tab(app: &AppHandle, url: Url) -> Result<String, String> {
    let window = app.get_window("main").ok_or("main window not found")?;
    let state = app.state::<Mutex<TabsState>>();

    let label = {
        let mut tabs = state.lock().unwrap();
        tabs.counter += 1;
        format!("tab-{}", tabs.counter)
    };

    let (position, size) = content_bounds(&window).map_err(|e| e.to_string())?;
    let builder = WebviewBuilder::new(&label, WebviewUrl::External(url.clone())).on_page_load(
        |webview, payload| {
            if let PageLoadEvent::Finished = payload.event() {
                let app = webview.app_handle();
                let state = app.state::<Mutex<TabsState>>();
                let mut tabs = state.lock().unwrap();
                let label = webview.label().to_string();
                tabs.urls.insert(label, payload.url().to_string());
                emit_tabs(app, &tabs);
            }
        },
    );
    window
        .add_child(builder, position, size)
        .map_err(|e| e.to_string())?;

    let mut tabs = state.lock().unwrap();
    // 新タブをアクティブにし、他を隠す
    for other in &tabs.order {
        if let Some(webview) = app.webviews().get(other.as_str()) {
            let _ = webview.hide();
        }
    }
    tabs.order.push(label.clone());
    tabs.urls.insert(label.clone(), url.to_string());
    tabs.active = label.clone();
    emit_tabs(app, &tabs);
    Ok(label)
}

fn active_webview(app: &AppHandle) -> Result<tauri::Webview, String> {
    let state = app.state::<Mutex<TabsState>>();
    let label = state.lock().unwrap().active.clone();
    app.webviews()
        .get(&label)
        .cloned()
        .ok_or_else(|| format!("webview {label} not found"))
}

/// ツールバー起動時の初期描画用 (イベント購読前のタブ状態を取得する)
#[tauri::command]
fn list_tabs(app: AppHandle) -> Vec<TabInfo> {
    let state = app.state::<Mutex<TabsState>>();
    let tabs = state.lock().unwrap();
    tab_list(&tabs)
}

#[tauri::command]
fn new_tab(app: AppHandle, url: Option<String>) -> Result<(), String> {
    let url = match url {
        Some(u) => normalize_url(&u)?,
        None => Url::parse(NEW_TAB_URL).unwrap(),
    };
    create_tab(&app, url).map(|_| ())
}

#[tauri::command]
fn switch_tab(app: AppHandle, label: String) -> Result<(), String> {
    let state = app.state::<Mutex<TabsState>>();
    let mut tabs = state.lock().unwrap();
    if !tabs.order.contains(&label) {
        return Err(format!("unknown tab: {label}"));
    }
    let webviews = app.webviews();
    for other in &tabs.order {
        if let Some(webview) = webviews.get(other.as_str()) {
            if *other == label {
                let _ = webview.show();
            } else {
                let _ = webview.hide();
            }
        }
    }
    tabs.active = label;
    emit_tabs(&app, &tabs);
    Ok(())
}

#[tauri::command]
fn close_tab(app: AppHandle, label: String) -> Result<(), String> {
    let state = app.state::<Mutex<TabsState>>();
    let (was_active, next_active) = {
        let mut tabs = state.lock().unwrap();
        let Some(index) = tabs.order.iter().position(|l| *l == label) else {
            return Err(format!("unknown tab: {label}"));
        };
        tabs.order.remove(index);
        tabs.urls.remove(&label);
        let was_active = tabs.active == label;
        if !was_active {
            emit_tabs(&app, &tabs);
        }
        (was_active, tabs.order.last().cloned())
    };
    if let Some(webview) = app.webviews().get(label.as_str()) {
        let _ = webview.close();
    }
    if !was_active {
        return Ok(());
    }
    match next_active {
        // 閉じたのがアクティブタブなら隣をアクティブに
        Some(next) => switch_tab(app, next),
        // 最後のタブを閉じたら空タブを開く
        None => new_tab(app, None),
    }
}

#[tauri::command]
fn navigate(app: AppHandle, url: String) -> Result<(), String> {
    let target = normalize_url(&url)?;
    let webview = active_webview(&app)?;
    webview.navigate(target.clone()).map_err(|e| e.to_string())?;
    let state = app.state::<Mutex<TabsState>>();
    let mut tabs = state.lock().unwrap();
    let label = tabs.active.clone();
    tabs.urls.insert(label, target.to_string());
    emit_tabs(&app, &tabs);
    Ok(())
}

#[tauri::command]
fn go_back(app: AppHandle) -> Result<(), String> {
    active_webview(&app)?
        .eval("history.back()")
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn go_forward(app: AppHandle) -> Result<(), String> {
    active_webview(&app)?
        .eval("history.forward()")
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn reload(app: AppHandle) -> Result<(), String> {
    active_webview(&app)?
        .eval("location.reload()")
        .map_err(|e| e.to_string())
}

fn main() {
    tauri::Builder::default()
        .manage(Mutex::new(TabsState::default()))
        .invoke_handler(tauri::generate_handler![
            navigate, go_back, go_forward, reload, new_tab, switch_tab, close_tab, list_tabs
        ])
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
            #[cfg(not(target_os = "linux"))]
            let _ = &toolbar;

            // 最初のタブ
            create_tab(
                app.handle(),
                INITIAL_URL.parse().expect("valid initial url"),
            )
            .map_err(Box::<dyn std::error::Error>::from)?;

            // Windows / macOS では子 WebView の位置・サイズ指定が有効なので、
            // リサイズ時に手動で追従させる (Linux では GtkBox が自動配分)
            #[cfg(not(target_os = "linux"))]
            {
                use tauri::WindowEvent;
                let app_handle = app.handle().clone();
                let win = window.clone();
                window.on_window_event(move |event| {
                    if let WindowEvent::Resized(size) = event {
                        let scale = win.scale_factor().unwrap_or(1.0);
                        let logical: LogicalSize<f64> = size.to_logical(scale);
                        for (label, webview) in app_handle.webviews() {
                            if label == "toolbar" {
                                let _ = webview
                                    .set_size(LogicalSize::new(logical.width, TOOLBAR_HEIGHT));
                            } else {
                                let _ = webview
                                    .set_position(LogicalPosition::new(0.0, TOOLBAR_HEIGHT));
                                let _ = webview.set_size(LogicalSize::new(
                                    logical.width,
                                    (logical.height - TOOLBAR_HEIGHT).max(0.0),
                                ));
                            }
                        }
                    }
                });
            }
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

    #[test]
    fn allows_about_blank() {
        assert_eq!(normalize_url("about:blank").unwrap().as_str(), "about:blank");
    }
}
