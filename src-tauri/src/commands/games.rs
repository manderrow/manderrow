use crate::games::{Game, GAMES};

#[tauri::command]
pub async fn get_games() -> &'static [Game] {
    &*GAMES
}
