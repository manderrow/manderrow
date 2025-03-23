use crate::{
    games::{Game, GAMES},
    util::search::{self, Score, SortOption},
};

use super::{GAMES_MOD_DOWNLOADS, GAMES_REVIEWS};

#[tauri::command]
pub async fn get_games() -> &'static [Game<'static>] {
    &*GAMES
}

#[derive(Debug, Clone, Copy, serde::Deserialize)]
pub enum SortColumn {
    Relevance,
    Name,
    ModDownloads,
    Popularity,
}

#[tauri::command]
pub async fn search_games(query: String, sort: Vec<SortOption<SortColumn>>) -> Vec<usize> {
    let games_mod_downloads = &**GAMES_MOD_DOWNLOADS;
    let games_reviews = &**GAMES_REVIEWS;
    slog_scope::with_logger(|_logger| {
        let mut buf = GAMES
            .iter()
            .enumerate()
            .filter_map(|(i, g)| {
                if query.is_empty() {
                    Some((i, Score::MAX))
                } else {
                    let score = search::score(&query, &g.name)?;
                    // can be helpful when tweaking the search scoring
                    // slog::trace!(logger, "search_games [{i}] {:?}: {score:?}", g.name);
                    Some((i, score))
                }
            })
            .filter(|&(_, score)| search::should_include(score))
            .collect::<Vec<_>>();
        buf.sort_unstable_by(|(a_i, a_score), (b_i, b_score)| {
            let mut ordering = std::cmp::Ordering::Equal;
            for &SortOption { column, descending } in &sort {
                ordering = match column {
                    SortColumn::Relevance => a_score.cmp(b_score),
                    SortColumn::Name => GAMES[*a_i].name.cmp(&GAMES[*b_i].name),
                    SortColumn::ModDownloads => {
                        games_mod_downloads[*a_i].cmp(&games_mod_downloads[*b_i])
                    }
                    SortColumn::Popularity => games_reviews[*a_i].cmp(&games_reviews[*b_i]),
                };
                if descending {
                    ordering = ordering.reverse();
                }
                if ordering.is_ne() {
                    break;
                }
            }
            ordering
        });
        buf.into_iter().map(|(i, _)| i).collect()
    })
}

#[tauri::command]
pub async fn get_games_popularity() -> &'static str {
    include_str!("gameReviews.json")
}

#[tauri::command]
pub async fn get_game_mods_downloads() -> &'static str {
    include_str!("gameModDownloads.json")
}
