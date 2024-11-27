use std::{io::Cursor, sync::LazyLock};

use bytes::Bytes;
use flate2::bufread::GzDecoder;
use parking_lot::RwLock;

static MOD_INDEX: LazyLock<RwLock<Vec<Mod>>> = LazyLock::new(|| RwLock::new(Vec::new()));

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
struct Mod {
    name: String,
    full_name: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    icon: Option<String>,
    #[serde(default)]
    version_number: Option<String>,
    #[serde(default)]
    dependencies: Vec<String>,
    #[serde(default)]
    download_url: Option<String>,
    #[serde(default)]
    downloads: Option<String>,
    date_created: String,
    #[serde(default)]
    website_url: Option<String>,
    #[serde(default)]
    is_active: Option<String>,
    uuid4: String,
    #[serde(default)]
    file_size: Option<u64>,
}

async fn fetch_gzipped(url: &str) -> Result<GzDecoder<Cursor<Bytes>>, String> {
    let bytes = tauri_plugin_http::reqwest::get(url)
        .await
        .map_err(|e| e.to_string())?
        .bytes()
        .await
        .map_err(|e| e.to_string())?;
    Ok(GzDecoder::new(Cursor::new(bytes)))
}

#[tauri::command]
async fn fetch_mod_index() -> Result<(), String> {
    let result = (|| async move {
        let chunk_urls = serde_json::from_reader::<_, Vec<String>>(
            fetch_gzipped("https://thunderstore.io/c/lethal-company/api/v1/package-listing-index/")
                .await?,
        )
        .map_err(|e| e.to_string())?;
        *MOD_INDEX.write() =
            futures::future::try_join_all(chunk_urls.into_iter().map(|url| async move {
                tokio::task::spawn(async move {
                    let rdr = fetch_gzipped(&url).await?;
                    tokio::task::block_in_place(|| {
                        serde_json::from_reader::<_, Vec<Mod>>(rdr).map_err(|e| e.to_string())
                    })
                })
                .await
                .map_err(|e| e.to_string())?
            }))
            .await?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        Ok(())
    })()
    .await;
    match result {
        Ok(()) => Ok(()),
        Err(e) => {
            eprintln!("{e}");
            Err(e)
        }
    }
}

#[derive(serde::Serialize)]
struct QueryResult {
    mods: Vec<Mod>,
    count: usize,
}

#[tauri::command]
async fn query_mod_index(query: String) -> QueryResult {
    let mod_index = MOD_INDEX.read();
    let mut iter = mod_index.iter().filter(|m| m.full_name.contains(&query));
    let mods: Vec<_> = iter.by_ref().take(50).take(50).cloned().collect();
    let count = mods.len() + iter.count();
    QueryResult { mods, count }
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
