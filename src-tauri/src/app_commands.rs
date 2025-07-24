use std::ffi::OsString;

use anyhow::Context;
use tauri::{AppHandle, Manager, Window};

use crate::CommandError;

#[tauri::command]
pub async fn close(window: Window) -> Result<(), CommandError> {
    window.close().context("Failed to close window")?;
    Ok(())
}

fn is_maximized_impl(window: &Window) -> Result<bool, CommandError> {
    Ok(if cfg!(target_os = "macos") {
        window.is_fullscreen()
    } else {
        window.is_maximized()
    }
    .context("Failed to check if window is maximized")?)
}

#[tauri::command]
pub async fn is_maximized(window: Window) -> Result<bool, CommandError> {
    is_maximized_impl(&window)
}

#[tauri::command]
pub async fn minimize(window: Window) -> Result<(), CommandError> {
    window.minimize().context("Failed to minimize window")?;
    Ok(())
}

#[tauri::command]
pub async fn set_maximized(
    window: Window,
    desired_state: Option<bool>,
) -> Result<(), CommandError> {
    let desired_state = match desired_state {
        Some(b) => b,
        None => !is_maximized_impl(&window)?,
    };
    if cfg!(target_os = "macos") {
        window
            .set_fullscreen(desired_state)
            .context("Failed to set_fullscreen window")?;
    } else {
        if desired_state {
            window.maximize().context("Failed to maximize window")?;
        } else {
            window.unmaximize().context("Failed to unmaximize window")?;
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn start_dragging(window: Window) -> Result<(), CommandError> {
    window
        .start_dragging()
        .context("Failed to start dragging window")?;
    Ok(())
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
