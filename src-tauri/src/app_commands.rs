use std::ffi::OsString;

use tauri::{AppHandle, Manager};

use crate::CommandError;

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
