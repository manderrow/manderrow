pub mod commands;

pub use manderrow_types::games::*;

use std::{borrow::Cow, collections::HashMap, marker::PhantomData, sync::LazyLock};

use anyhow::{Context, Result};

#[derive(Debug, Clone, thiserror::Error)]
#[error("{0}")]
pub struct StringError(String);

static GAMES: LazyLock<Result<Vec<Game>, StringError>> = LazyLock::new(|| {
    serde_json::from_str(include_str!("games.json")).map_err(|e| StringError(e.to_string()))
});

struct IndexedGameData<T>(Vec<T>);

impl<'de, T: Clone + Default + serde::Deserialize<'de>> serde::Deserialize<'de>
    for IndexedGameData<T>
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor<T>(PhantomData<fn() -> T>);
        impl<'de, T: Clone + Default + serde::Deserialize<'de>> serde::de::Visitor<'de> for Visitor<T> {
            type Value = IndexedGameData<T>;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a map of game ids to data values")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                use serde::de::Error;
                let games = GAMES.as_ref().map_err(|e| A::Error::custom(e))?;
                let mut buf = (0..games.len()).map(|_| None::<T>).collect::<Vec<_>>();
                while let Some(id) = map.next_key::<&str>()? {
                    let value = map.next_value()?;
                    let mut iter = games
                        .iter()
                        .enumerate()
                        .filter(|(_, g)| g.thunderstore_id == id)
                        .map(|(i, _)| i);
                    let found = iter.next().ok_or_else(|| {
                        A::Error::invalid_value(serde::de::Unexpected::Str(id), &"a valid game id")
                    })?;
                    buf[found] = Some(value);
                    for i in iter {
                        let value = buf[found].clone();
                        buf[i] = value;
                    }
                }
                Ok(IndexedGameData(
                    buf.into_iter()
                        .enumerate()
                        .map(|(i, o)| o.ok_or_else(|| A::Error::missing_field(games[i].id)))
                        .collect::<Result<Vec<_>, _>>()?,
                ))
            }
        }
        deserializer.deserialize_map(Visitor(PhantomData))
    }
}

static GAMES_MOD_DOWNLOADS: LazyLock<Result<Vec<u64>, StringError>> = LazyLock::new(|| {
    Ok(
        serde_json::from_str::<IndexedGameData<_>>(include_str!("gameModDownloads.json"))
            .map_err(|e| StringError(e.to_string()))?
            .0,
    )
});

static GAMES_REVIEWS: LazyLock<Result<Vec<Option<u64>>, StringError>> = LazyLock::new(|| {
    Ok(
        serde_json::from_str::<IndexedGameData<_>>(include_str!("gameReviews.json"))
            .map_err(|e| StringError(e.to_string()))?
            .0,
    )
});

static GAMES_BY_ID: LazyLock<Result<HashMap<&'static str, &'static Game>, &'static StringError>> =
    LazyLock::new(|| {
        GAMES
            .as_ref()
            .map(|games| games.iter().map(|g| (&*g.id, g)).collect())
    });

pub fn games() -> Result<&'static [Game<'static>]> {
    GAMES
        .as_ref()
        .map(Vec::as_slice)
        .map_err(Clone::clone)
        .context("Failed to load games.json")
}

pub fn games_by_id() -> Result<&'static HashMap<&'static str, &'static Game<'static>>> {
    GAMES_BY_ID
        .as_ref()
        .map_err(Clone::clone)
        .context("Failed to load games.json")
}
