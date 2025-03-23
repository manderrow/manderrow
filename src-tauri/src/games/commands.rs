use anyhow::Context;
use tauri::ipc::InvokeResponseBody;

use crate::{
    games::Game,
    util::search::{self, Score, SortOption},
    CommandError,
};

use super::{games, GAMES_MOD_DOWNLOADS, GAMES_REVIEWS};

#[tauri::command]
pub async fn get_games() -> Result<&'static [Game<'static>], CommandError> {
    Ok(games()?)
}

#[derive(Debug, Clone, Copy, serde::Deserialize)]
pub enum SortColumn {
    Relevance,
    Name,
    ModDownloads,
    Popularity,
}

#[tauri::command]
pub async fn search_games(
    query: String,
    sort: Vec<SortOption<SortColumn>>,
) -> Result<Vec<usize>, CommandError> {
    let games_mod_downloads = GAMES_MOD_DOWNLOADS
        .as_ref()
        .map_err(Clone::clone)
        .context("Failed to load gameModDownloads.json")?;
    let games_reviews = GAMES_REVIEWS
        .as_ref()
        .map_err(Clone::clone)
        .context("Failed to load gameReviews.json")?;
    slog_scope::with_logger(|_logger| {
        let games = games()?;
        let mut buf = games
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
                    SortColumn::Name => games[*a_i].name.cmp(&games[*b_i].name),
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
        Ok(buf.into_iter().map(|(i, _)| i).collect())
    })
}

#[tauri::command]
pub async fn get_games_popularity() -> Result<InvokeResponseBody, CommandError> {
    // type check the JSON before sending the raw JSON to the frontend
    GAMES_REVIEWS
        .as_ref()
        .map_err(Clone::clone)
        .context("Failed to load gameReviews.json")?;
    Ok(InvokeResponseBody::Json(
        include_str!("gameReviews.json").to_owned(),
    ))
}

#[tauri::command]
pub async fn get_game_mods_downloads() -> Result<InvokeResponseBody, CommandError> {
    // type check the JSON before sending the raw JSON to the frontend
    GAMES_MOD_DOWNLOADS
        .as_ref()
        .map_err(Clone::clone)
        .context("Failed to load gameModDownloads.json")?;
    Ok(InvokeResponseBody::Json(
        include_str!("gameModDownloads.json").to_owned(),
    ))
}
