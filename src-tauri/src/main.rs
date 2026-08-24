//! kjr-browser のエントリーポイント。
//!
//! ここには「アプリの配線」だけを書く: 状態の登録、IPC コマンドの登録、
//! 起動時のセットアップ手順。各レイヤーの実体は以下のモジュールにある
//! (依存は上から下への一方向。詳細は docs/architecture.md):
//!
//! - commands.rs … IPC 境界。ツールバー UI からの invoke を受ける
//! - tabs.rs     … ドメイン層。タブの状態管理と操作
//! - url.rs      … ユーティリティ。アドレスバー入力の解釈 (純粋ロジック)
//! - window.rs   … プラットフォーム層。ウィンドウ構築と OS 差分の吸収

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod tabs;
mod url;
mod window;

use std::sync::Mutex;

/// 起動時に最初のタブで開くページ
const INITIAL_URL: &str = "https://example.com/";

fn main() {
    tauri::Builder::default()
        .manage(Mutex::new(tabs::TabsState::default()))
        .invoke_handler(tauri::generate_handler![
            commands::list_tabs,
            commands::new_tab,
            commands::switch_tab,
            commands::close_tab,
            commands::navigate,
            commands::go_back,
            commands::go_forward,
            commands::reload,
        ])
        .setup(|app| {
            window::build_main_window(app)?;
            tabs::create(app.handle(), INITIAL_URL.parse().expect("valid initial url"))
                .map_err(Box::<dyn std::error::Error>::from)?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
