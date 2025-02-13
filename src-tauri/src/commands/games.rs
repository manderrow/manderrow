use crate::games::{Game, GAMES};

#[tauri::command]
pub async fn get_games() -> &'static [Game<'static>] {
    &*GAMES
}

#[tauri::command]
pub async fn get_games_popularity() -> &'static str {
    include_str!("../gameReviews.json")
}

#[tauri::command]
pub async fn get_game_mods_downloads() -> &'static str {
    include_str!("../gameModDownloads.json")
}
