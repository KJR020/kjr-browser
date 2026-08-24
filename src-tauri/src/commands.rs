//! IPC 境界 (UI → Rust の入口)。
//!
//! ツールバー UI (ui/toolbar.js) が `invoke("コマンド名")` で呼び出す関数の定義。
//! Web の MVC でいうコントローラ層にあたり、ここには入出力の変換だけを書き、
//! 実際の処理は tabs / url モジュールに委譲する。
//! コマンドを増やしたら main.rs の `generate_handler!` への登録も忘れずに。

use tauri::{AppHandle, Url};

use crate::{tabs, url};

/// 現在のタブ一覧を返す。
/// ツールバー起動時の初期描画用: 最初のタブはツールバーがイベント購読を
/// 登録する前に作られるため、pull で取得しないと取りこぼす。
#[tauri::command]
pub fn list_tabs(app: AppHandle) -> Vec<tabs::TabInfo> {
    tabs::list(&app)
}

/// 新しいタブを開く。URL 省略時は空タブ
#[tauri::command]
pub fn new_tab(app: AppHandle, url: Option<String>) -> Result<(), String> {
    let url = match url {
        Some(u) => url::normalize(&u)?,
        None => Url::parse(tabs::NEW_TAB_URL).unwrap(),
    };
    tabs::create(&app, url).map(|_| ())
}

/// 指定タブに表示を切り替える
#[tauri::command]
pub fn switch_tab(app: AppHandle, label: String) -> Result<(), String> {
    tabs::switch(&app, label)
}

/// 指定タブを閉じる
#[tauri::command]
pub fn close_tab(app: AppHandle, label: String) -> Result<(), String> {
    tabs::close(&app, label)
}

/// アドレスバーの入力先へアクティブタブを遷移させる
#[tauri::command]
pub fn navigate(app: AppHandle, url: String) -> Result<(), String> {
    tabs::navigate_active(&app, url::normalize(&url)?)
}

/// 履歴を 1 つ戻る
#[tauri::command]
pub fn go_back(app: AppHandle) -> Result<(), String> {
    tabs::eval_active(&app, "history.back()")
}

/// 履歴を 1 つ進む
#[tauri::command]
pub fn go_forward(app: AppHandle) -> Result<(), String> {
    tabs::eval_active(&app, "history.forward()")
}

/// アクティブタブを再読み込みする
#[tauri::command]
pub fn reload(app: AppHandle) -> Result<(), String> {
    tabs::eval_active(&app, "location.reload()")
}
