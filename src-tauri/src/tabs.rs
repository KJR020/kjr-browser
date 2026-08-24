//! タブの状態管理と操作。
//!
//! 「タブ」の実体は 1 タブ = 1 つの content WebView (ラベル "tab-N")。
//! アクティブなタブの WebView だけを表示し、他は hide() で隠す。
//!
//! 状態 (どのタブがあり、どれがアクティブで、各タブが何を表示しているか) は
//! `TabsState` に集約し、`tauri::Builder::manage` でアプリ全体から参照できるようにする。
//! 状態が変わるたびに `tabs-changed` イベントでツールバー UI に通知する。

use std::collections::HashMap;
use std::sync::Mutex;

use tauri::{
    webview::{PageLoadEvent, WebviewBuilder},
    AppHandle, Emitter, Manager, Url, Webview, WebviewUrl,
};

use crate::window;

/// 新規タブで開くページ
pub const NEW_TAB_URL: &str = "about:blank";

/// 全タブの状態。`Mutex<TabsState>` として manage される。
#[derive(Default)]
pub struct TabsState {
    /// タブラベルの連番 ("tab-1", "tab-2", ...)。閉じても再利用しない
    counter: usize,
    /// アクティブなタブのラベル
    active: String,
    /// タブの並び順 (タブバーの表示順)
    order: Vec<String>,
    /// タブラベル → 表示中 URL
    urls: HashMap<String, String>,
}

/// ツールバー UI に渡すタブ 1 件分の情報 (`tabs-changed` イベントの要素)
#[derive(serde::Serialize, Clone)]
pub struct TabInfo {
    pub label: String,
    pub url: String,
    pub active: bool,
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

/// 現在のタブ一覧を返す (ツールバー起動時の初期描画用)
pub fn list(app: &AppHandle) -> Vec<TabInfo> {
    let state = app.state::<Mutex<TabsState>>();
    let tabs = state.lock().unwrap();
    tab_list(&tabs)
}

/// アクティブなタブの WebView を返す
pub fn active_webview(app: &AppHandle) -> Result<Webview, String> {
    let state = app.state::<Mutex<TabsState>>();
    let label = state.lock().unwrap().active.clone();
    app.webviews()
        .get(&label)
        .cloned()
        .ok_or_else(|| format!("webview {label} not found"))
}

/// 新しいタブ (content WebView) を作ってアクティブにする
pub fn create(app: &AppHandle, url: Url) -> Result<String, String> {
    let window = app.get_window("main").ok_or("main window not found")?;
    let state = app.state::<Mutex<TabsState>>();

    let label = {
        let mut tabs = state.lock().unwrap();
        tabs.counter += 1;
        format!("tab-{}", tabs.counter)
    };

    let (position, size) = window::content_bounds(&window).map_err(|e| e.to_string())?;
    // ページ遷移が完了するたびに URL を控えてツールバーへ通知する
    // (リンククリックなど WebView 内部で起きる遷移もこれで追跡できる)
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

/// 指定タブをアクティブにする (表示切替)
pub fn switch(app: &AppHandle, label: String) -> Result<(), String> {
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
    emit_tabs(app, &tabs);
    Ok(())
}

/// 指定タブを閉じる。アクティブタブを閉じたら隣へ、最後の 1 枚なら空タブを開く
pub fn close(app: &AppHandle, label: String) -> Result<(), String> {
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
            emit_tabs(app, &tabs);
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
        Some(next) => switch(app, next),
        None => create(app, Url::parse(NEW_TAB_URL).unwrap()).map(|_| ()),
    }
}

/// アクティブなタブを指定 URL へ遷移させる
pub fn navigate_active(app: &AppHandle, target: Url) -> Result<(), String> {
    let webview = active_webview(app)?;
    webview.navigate(target.clone()).map_err(|e| e.to_string())?;
    let state = app.state::<Mutex<TabsState>>();
    let mut tabs = state.lock().unwrap();
    let label = tabs.active.clone();
    tabs.urls.insert(label, target.to_string());
    emit_tabs(app, &tabs);
    Ok(())
}

/// アクティブなタブ内で JavaScript を実行する (履歴移動・再読み込みに使用)
pub fn eval_active(app: &AppHandle, script: &str) -> Result<(), String> {
    active_webview(app)?.eval(script).map_err(|e| e.to_string())
}
