//! プラットフォーム層: メインウィンドウの構築とレイアウト。
//!
//! 「1 つのウィンドウに toolbar WebView と content WebView を積む」という
//! 物理的な画面構成と、OS ごとのレイアウト差分をこのモジュールに閉じ込める。
//! タブが何枚あるかといったドメインの関心事はここには置かない (tabs.rs の仕事)。

use tauri::{
    webview::WebviewBuilder, window::WindowBuilder, App, LogicalPosition, LogicalSize, WebviewUrl,
    Window,
};

/// ツールバー WebView の高さ (論理ピクセル)。タブ列 32px + ナビ列 48px の 2 段
pub const TOOLBAR_HEIGHT: f64 = 80.0;

/// コンテンツ領域の位置・サイズ (Windows/macOS 用。Linux では GtkBox が配分する)
pub fn content_bounds(window: &Window) -> tauri::Result<(LogicalPosition<f64>, LogicalSize<f64>)> {
    let scale = window.scale_factor()?;
    let logical: LogicalSize<f64> = window.inner_size()?.to_logical(scale);
    Ok((
        LogicalPosition::new(0.0, TOOLBAR_HEIGHT),
        LogicalSize::new(logical.width, (logical.height - TOOLBAR_HEIGHT).max(0.0)),
    ))
}

/// メインウィンドウとツールバー WebView を構築する。
/// タブ (content WebView) はここでは作らない — 呼び出し側 (main.rs) が
/// このあと tabs::create で最初のタブを開く。
pub fn build_main_window(app: &App) -> Result<(), Box<dyn std::error::Error>> {
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
                gtk_box.set_child_packing(&widget, false, true, 0, gtk::PackType::Start);
            }
        }
    })?;
    #[cfg(not(target_os = "linux"))]
    let _ = &toolbar;

    // Windows / macOS では子 WebView の位置・サイズ指定が有効なので、
    // リサイズ時に手動で追従させる (Linux では GtkBox が自動配分)
    #[cfg(not(target_os = "linux"))]
    {
        use tauri::{Manager, WindowEvent};
        let app_handle = app.handle().clone();
        let win = window.clone();
        window.on_window_event(move |event| {
            if let WindowEvent::Resized(size) = event {
                let scale = win.scale_factor().unwrap_or(1.0);
                let logical: LogicalSize<f64> = size.to_logical(scale);
                for (label, webview) in app_handle.webviews() {
                    if label == "toolbar" {
                        let _ = webview.set_size(LogicalSize::new(logical.width, TOOLBAR_HEIGHT));
                    } else {
                        let _ = webview.set_position(LogicalPosition::new(0.0, TOOLBAR_HEIGHT));
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
}
