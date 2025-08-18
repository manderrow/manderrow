use std::num::NonZeroUsize;

use manderrow_types::mods::ModId;
use tauri::{AppHandle, State};

use crate::{tasks, CommandError, Reqwest};

use super::{read_mod_index, SortColumn, SortOption};

#[tauri::command]
pub async fn fetch_mod_index(
    app_handle: AppHandle,
    reqwest: State<'_, Reqwest>,
    game: &str,
    refresh: bool,
    task_id: tasks::Id,
) -> Result<(), CommandError> {
    super::fetch_mod_index(Some(&app_handle), &reqwest, game, refresh, Some(task_id)).await?;

    Ok(())
}

#[tauri::command]
pub async fn count_mod_index(game: &str, query: &str) -> Result<usize, CommandError> {
    let mod_index = read_mod_index(game).await?;

    Ok(super::count_mod_index(&mod_index, query)?)
}

fn map_to_json<T: serde::Serialize>(buf: &mut Vec<u8>, it: impl Iterator<Item = T>) {
    let mut it = it.peekable();
    while let Some(m) = it.next() {
        simd_json::serde::to_writer(&mut *buf, &m).unwrap();
        if it.peek().is_some() {
            buf.push(b',');
        }
    }
}

#[tauri::command]
pub async fn query_mod_index(
    game: &str,
    query: &str,
    sort: Vec<SortOption<SortColumn>>,
    skip: Option<usize>,
    limit: Option<NonZeroUsize>,
) -> Result<tauri::ipc::Response, CommandError> {
    let mod_index = read_mod_index(game).await?;

    let out_buf = super::query_mod_index_to_json(&mod_index, query, &sort, skip, limit)?;
    Ok(tauri::ipc::Response::new(out_buf))
}

#[tauri::command]
pub async fn get_from_mod_index(
    game: &str,
    mod_ids: Vec<ModId<'_>>,
) -> Result<tauri::ipc::Response, CommandError> {
    let mod_index = read_mod_index(game).await?;

    let out_buf = super::get_from_mod_index_to_json(&mod_index, &mod_ids)?;
    Ok(tauri::ipc::Response::new(out_buf))
}
