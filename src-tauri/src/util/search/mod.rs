#[cfg(feature = "search-sublime_fuzzy")]
mod backend_sublime_fuzzy;
#[cfg(feature = "search-sublime_fuzzy")]
use backend_sublime_fuzzy as backend;

pub use backend::*;

#[derive(Clone, Copy, serde::Deserialize)]
pub struct SortOption<C> {
    pub column: C,
    pub descending: bool,
}

pub fn add_scores(a: Option<Score>, b: Option<Score>) -> Option<Score> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a + b),
        (None, Some(s)) | (Some(s), None) => Some(s),
        (None, None) => None,
    }
}

#[allow(unused)]
pub fn add_bonus(score: Option<Score>, bonus: Score) -> Option<Score> {
    match (score, bonus) {
        (Some(score), bonus) => Some(score + bonus),
        (None, bonus) if bonus > Score::ZERO => Some(bonus),
        (None, bonus) => None,
    }
}
