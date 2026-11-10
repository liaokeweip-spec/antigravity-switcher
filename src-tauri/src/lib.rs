mod credential;
mod crypto;
mod models;
mod oauth;
mod process;
mod store;

use models::{AccountItem, AppStateDto};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager,
};

#[tauri::command]
fn get_state() -> AppStateDto {
    store::get_app_state()
}

#[tauri::command]
fn start_google_login(alias: Option<String>) -> Result<AccountItem, String> {
    oauth::login_and_add_account(alias)
}

#[tauri::command]
fn snapshot_current(name: String) -> Result<(), String> {
    store::snapshot_current_account(name)
}

#[tauri::command]
fn switch_account(id: String) -> Result<(), String> {
    store::switch_account(id)
}

#[tauri::command]
fn delete_account(id: String) -> Result<(), String> {
    store::delete_account(id)
}

#[tauri::command]
fn restart_antigravity() -> Result<(), String> {
    process::restart_antigravity()
}

#[tauri::command]
fn kill_antigravity() -> Result<(), String> {
    process::kill_antigravity()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // 系统托盘菜单
            let quit_i = MenuItem::with_id(app, "quit", "退出助手", true, None::<&str>)?;
            let show_i = MenuItem::with_id(app, "show", "显示主窗口", true, None::<&str>)?;
            let restart_i = MenuItem::with_id(app, "restart", "重启反重力桌面端", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_i, &restart_i, &quit_i])?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "quit" => {
                        app.exit(0);
                    }
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "restart" => {
                        let _ = process::restart_antigravity();
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_state,
            start_google_login,
            snapshot_current,
            switch_account,
            delete_account,
            restart_antigravity,
            kill_antigravity
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
