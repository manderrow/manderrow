#[tauri::command]
pub fn get_preferred_locales() -> Vec<String> {
    get_locale::get_preferred_locales()
}