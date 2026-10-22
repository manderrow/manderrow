use std::num::NonZeroUsize;

use manderrow_types::mods::ModId;

use crate::app::{AppHandle, RawJson};
use crate::{tasks, CommandError, Reqwest};

use super::{read_mod_index, SortColumn, SortOption};

pub async fn fetch_mod_index(
    app_handle: &AppHandle,
    reqwest: &Reqwest,
    game: &str,
    refresh: bool,
    task_id: tasks::Id,
) -> Result<(), CommandError> {
    super::fetch_mod_index(Some(app_handle), reqwest, game, refresh, Some(task_id)).await?;

    Ok(())
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

pub async fn query_mod_index(
    game: &str,
    query: &str,
    sort: Vec<SortOption<SortColumn>>,
    skip: Option<usize>,
    limit: Option<NonZeroUsize>,
) -> Result<RawJson, CommandError> {
    let mod_index = read_mod_index(game).await?;

    let buf = super::query_mod_index(&mod_index, query, &sort)?;

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
    Ok(RawJson(unsafe {
        String::from_utf8_unchecked(out_buf)
    }))
}

pub async fn get_from_mod_index(
    game: &str,
    mod_ids: Vec<ModId<'_>>,
) -> Result<RawJson, CommandError> {
    let mod_index = read_mod_index(game).await?;

    let buf = super::get_from_mod_index(&mod_index, &mod_ids).await?;

    let mut out_buf = br#"["#.as_slice().to_owned();
    map_to_json(&mut out_buf, buf.into_iter());
    out_buf.extend(b"]");
    // SAFETY: simd_json only writes valid UTF-8
    Ok(RawJson(unsafe {
        String::from_utf8_unchecked(out_buf)
    }))
}
