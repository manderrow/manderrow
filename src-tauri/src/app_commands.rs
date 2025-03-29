use std::ffi::OsString;

use tauri::{AppHandle, Manager, Window};

use crate::window_state::WindowExt;
use crate::CommandError;

#[tauri::command]
pub async fn close_splashscreen(main_window: Window) {
    let _ = main_window.restore_state();

    main_window.show().unwrap();

    match main_window.get_webview_window("splashscreen") {
        Some(win) => win.close().unwrap(),
        None => { /* Splashscreen window is already closed */ }
    }
}

#[tauri::command]
pub async fn relaunch(app: AppHandle) -> Result<(), CommandError> {
    app.cleanup_before_exit();
    let mut env = app.env();
    env.args_os = vec![
        // this will be ignored by tauri, so just give an empty string
        OsString::new(),
        "--relaunch".into(),
        std::process::id().to_string().into(),
    ];
    tauri::process::restart(&env)
}
