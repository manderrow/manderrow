use std::num::NonZeroUsize;

use manderrow_types::mods::ModId;
use manderrow_types::util::rkyv::InternedString;
use packed_semver::Version;

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

/// Searches a game's mod index, returning how many mods match and summaries of those that are asked
/// for, which leave out the mods' older versions.
pub async fn query_mod_index(
    game: &str,
    query: &str,
    sort: Vec<SortOption<SortColumn>>,
    skip: Option<usize>,
    limit: Option<NonZeroUsize>,
) -> Result<RawJson, CommandError> {
    let mod_index = read_mod_index(game).await?;

    let results = super::query_mod_index(&mod_index, query, &sort)?;

    let count = results.len();
    let start = skip.unwrap_or(0).min(count);
    let end = limit.map_or(count, |limit| start.saturating_add(limit.get()).min(count));

    let mut out_buf = br#"{"count":"#.as_slice().to_owned();
    simd_json::serde::to_writer(&mut out_buf, &count).unwrap();
    out_buf.extend(br#","mods":["#);
    map_to_json(
        &mut out_buf,
        results[start..end]
            .iter()
            .map(|&(position, _)| mod_index.get(position).summary()),
    );
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

/// A mod's installed version, for `get_mod_updates`.
#[derive(serde::Deserialize)]
pub struct InstalledVersion<'a> {
    pub owner: &'a str,
    pub name: &'a str,
    pub version: Version,
}

/// Looks the installed mods up in a game's mod index, returning summaries of those whose latest
/// version isn't the one installed, in the order given. Mods that aren't in the index are left out.
/// Only what is outdated is sent, which is usually none of a profile's mods.
pub async fn get_mod_updates(
    game: &str,
    installed: Vec<InstalledVersion<'_>>,
) -> Result<RawJson, CommandError> {
    let mod_index = read_mod_index(game).await?;

    let mod_ids = installed
        .iter()
        .map(|m| ModId {
            owner: InternedString(m.owner),
            name: InternedString(m.name),
        })
        .collect::<Vec<_>>();
    let found = super::get_from_mod_index(&mod_index, &mod_ids).await?;

    let mut out_buf = br#"["#.as_slice().to_owned();
    map_to_json(
        &mut out_buf,
        found.into_iter().zip(&installed).filter_map(|(m, installed)| {
            let m = m?;
            let latest = m.versions.first()?;
            (latest.version_number.get() != installed.version).then(|| m.summary())
        }),
    );
    out_buf.extend(b"]");
    // SAFETY: simd_json only writes valid UTF-8
    Ok(RawJson(unsafe {
        String::from_utf8_unchecked(out_buf)
    }))
}
