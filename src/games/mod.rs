pub use manderrow_types::games::*;

use std::{collections::HashMap, marker::PhantomData, sync::LazyLock};

use anyhow::{Context, Result};

use crate::util::search::{self, Score, SortOption};

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

fn games_mod_downloads() -> Result<&'static [u64]> {
    GAMES_MOD_DOWNLOADS
        .as_ref()
        .map(Vec::as_slice)
        .map_err(Clone::clone)
        .context("Failed to load gameModDownloads.json")
}

fn games_reviews() -> Result<&'static [Option<u64>]> {
    GAMES_REVIEWS
        .as_ref()
        .map(Vec::as_slice)
        .map_err(Clone::clone)
        .context("Failed to load gameReviews.json")
}

#[derive(Debug, Clone, Copy)]
pub enum SortColumn {
    Relevance,
    Name,
    ModDownloads,
    Popularity,
}

/// Returns the indices into [`games`] of the games whose names match `query`, sorted by each of
/// `sort` in turn. An empty query matches every game.
pub fn search(query: &str, sort: &[SortOption<SortColumn>]) -> Result<Vec<usize>> {
    let games = games()?;
    let games_mod_downloads = games_mod_downloads()?;
    let games_reviews = games_reviews()?;
    let mut buf = games
        .iter()
        .enumerate()
        .filter_map(|(i, g)| {
            if query.is_empty() {
                Some((i, Score::MAX))
            } else {
                Some((i, search::score(query, &g.name)?))
            }
        })
        .filter(|&(_, score)| search::should_include(score))
        .collect::<Vec<_>>();
    buf.sort_unstable_by(|(a_i, a_score), (b_i, b_score)| {
        let mut ordering = std::cmp::Ordering::Equal;
        for &SortOption { column, descending } in sort {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The game data is compiled in, so a mistake in it would otherwise only show at runtime.
    #[test]
    fn game_data_loads() {
        games().unwrap();
        games_by_id().unwrap();
        games_mod_downloads().unwrap();
        games_reviews().unwrap();
    }

    #[test]
    fn search_matches_names() {
        let sort = [SortOption {
            column: SortColumn::Relevance,
            descending: true,
        }];
        let results = search("lethal company", &sort).unwrap();
        assert_eq!(games().unwrap()[results[0]].id, "lethal-company");
        assert_eq!(search("", &sort).unwrap().len(), games().unwrap().len());
    }
}
