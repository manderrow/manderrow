use std::{collections::HashSet, num::NonZeroUsize};

use tauri::{ipc::Channel, AppHandle};

use crate::{
    mods::{ArchivedModRef, ModId},
    CommandError,
};

use super::{read_mod_index, FetchEvent, SortOption};

#[tauri::command]
pub async fn fetch_mod_index(
    app_handle: AppHandle,
    game: &str,
    refresh: bool,
    on_event: Channel<FetchEvent>,
) -> Result<(), CommandError> {
    super::fetch_mod_index(app_handle, game, refresh, on_event).await?;

    Ok(())
}

fn map_to_json<'a>(buf: &mut Vec<u8>, it: impl Iterator<Item = &'a ArchivedModRef<'a>>) {
    let mut it = it.peekable();
    while let Some(m) = it.next() {
        simd_json::serde::to_writer(&mut *buf, m).unwrap();
        if it.peek().is_some() {
            buf.push(b',');
        }
    }
}

#[tauri::command]
pub async fn count_mod_index(game: &str, query: &str) -> Result<usize, CommandError> {
    let mod_index = read_mod_index(game).await?;

    Ok(super::count_mod_index(&mod_index, query).await?)
}

#[tauri::command]
pub async fn query_mod_index(
    game: &str,
    query: &str,
    sort: Vec<SortOption>,
    skip: Option<usize>,
    limit: Option<NonZeroUsize>,
) -> Result<tauri::ipc::Response, CommandError> {
    let mod_index = read_mod_index(game).await?;

    let buf = super::query_mod_index(&mod_index, query, &sort).await?;

    let count = buf.len();

    let mut out_buf = br#"{"count":"#.as_slice().to_owned();
    simd_json::serde::to_writer(&mut out_buf, &count).unwrap();
    out_buf.extend(br#","mods":["#);
    let mods = buf.into_iter().map(|(m, _)| m);
    match (skip.unwrap_or(0), limit) {
        (0, Some(limit)) => map_to_json(&mut out_buf, mods.take(limit.get())),
        (0, None) => map_to_json(&mut out_buf, mods),
        (skip, Some(limit)) => map_to_json(&mut out_buf, mods.skip(skip).take(limit.get())),
        (skip, None) => map_to_json(&mut out_buf, mods.skip(skip)),
    };
    out_buf.extend(b"]}");
    // SAFETY: simd_json only writes valid UTF-8
    Ok(tauri::ipc::Response::new(unsafe {
        String::from_utf8_unchecked(out_buf)
    }))
}

#[tauri::command]
pub async fn get_from_mod_index(
    game: &str,
    mod_ids: HashSet<ModId<'_>>,
) -> Result<tauri::ipc::Response, CommandError> {
    let mod_index = read_mod_index(game).await?;

    let buf = super::get_from_mod_index(&mod_index, &mod_ids).await?;

    let count = buf.len();

    let mut out_buf = br#"{"count":"#.as_slice().to_owned();
    simd_json::serde::to_writer(&mut out_buf, &count).unwrap();
    out_buf.extend(br#","mods":["#);
    map_to_json(&mut out_buf, buf.into_iter());
    out_buf.extend(b"]}");
    // SAFETY: simd_json only writes valid UTF-8
    Ok(tauri::ipc::Response::new(unsafe {
        String::from_utf8_unchecked(out_buf)
    }))
}
