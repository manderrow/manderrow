use std::collections::HashMap;

use crate::{
    game_reviews::GAME_REVIEWS,
    games::{Game, GAMES},
};

#[tauri::command]
pub async fn get_games() -> &'static [Game] {
    &*GAMES
}

#[tauri::command]
pub async fn get_games_popularity() -> &'static HashMap<String, i64> {
    &*GAME_REVIEWS
}
