use std::{
    collections::HashMap, io::{Cursor, Read}, sync::LazyLock
};

use parking_lot::RwLock;

static MOD_INDEX: LazyLock<RwLock<Vec<Mod>>> =
    LazyLock::new(|| RwLock::new(Vec::new()));

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
struct Mod {
    namespace: String,
    name: String,
    version_number: String,
    file_format: Option<String>,
    file_size: u64,
    dependencies: Vec<String>,
    #[serde(flatten)]
    extra: HashMap<String, serde_json::Value>,
}

#[tauri::command]
async fn fetch_mod_index() -> Result<(), String> {
    let result = (|| async move {
        let bytes =
        tauri_plugin_http::reqwest::get("https://thunderstore.io/api/experimental/package-index")
            .await
            .map_err(|e| e.to_string())?
            .bytes()
            .await
            .map_err(|e| e.to_string())?;
        let mut buf = Vec::new();
        flate2::read::GzDecoder::new(Cursor::new(bytes))
            .read_to_end(&mut buf)
            .map_err(|e| e.to_string())?;
        let mut dec_buf = Vec::new();
        for (i, line) in buf.split(|&b| b == b'\n').enumerate().filter(|(_, line)| !line.is_empty()) {
            dec_buf.push(serde_json::from_slice(line).map_err(|e| format!("At line {} while parsing {line:?}: {}", i + 1, e.to_string()))?)
        }
        *MOD_INDEX.write() = dec_buf;
        Ok(())
    })().await;
    match result {
        Ok(()) => Ok(()),
        Err(e) => {
            eprintln!("{e}");
            Err(e)
        }
    }
}

#[tauri::command]
async fn query_mod_index(query: String) -> Vec<Mod> {
    MOD_INDEX.read().iter().filter(|m| m.namespace.contains(&query) || m.name.contains(&query)).take(50).cloned().collect()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![fetch_mod_index, query_mod_index])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
