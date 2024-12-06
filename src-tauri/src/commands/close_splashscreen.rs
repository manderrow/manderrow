use tauri::{Manager, Window};

#[tauri::command]
pub async fn close_splashscreen(window: Window) {
    let main_window = window.get_webview_window("main").expect("no windows?");
    // main_window.center().unwrap();
    // main_window.maximize().unwrap();
    main_window.show().unwrap();

    let splashscreen_window = window.get_webview_window("splashscreen");

    match splashscreen_window {
        Some(win) => win.close().unwrap(),
        None => { /* Splashscreen window is already closed */ }
    }
}
