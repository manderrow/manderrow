use std::ops::{Add, Div};

use sublime_fuzzy::{FuzzySearch, Scoring};

use super::{add_bonus, score_acronym, score_from_beginning};

pub(super) type ScoreValue = isize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Score(pub(super) ScoreValue);

impl Score {
    pub const MIN: Self = Self(isize::MIN);
    pub const ZERO: Self = Self(0);
    pub const MAX: Self = Self(isize::MAX);
}

impl Add for Score {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl Div<u32> for Score {
    type Output = Self;

    fn div(self, rhs: u32) -> Self::Output {
        Self(self.0 / rhs as ScoreValue)
    }
}

pub fn score(needle: &str, haystack: &str) -> Option<Score> {
    let mut score = FuzzySearch::new(needle, haystack)
        .case_insensitive()
        .score_with(&Scoring {
            bonus_consecutive: 24,
            bonus_word_start: 48,
            bonus_match_case: 0,
            penalty_distance: 4,
        })
        .best_match()
        .map(|m| Score(m.score()));
    if haystack.starts_with(needle) {
        score = score.map(|s| Score(s.0 * 2));
    }
    // let acronym_bonus = score_acronym(needle, haystack, 400);
    // score = add_bonus(score, acronym_bonus);
    // if acronym_bonus.0 < (450 as ScoreValue).isqrt() {
    //     score = add_bonus(score, Score(-(haystack.len().abs_diff(needle.len()).pow(2) as ScoreValue)));
    // }
    // score = add_bonus(score, score_from_beginning(needle, haystack));
    score
}

pub fn should_include(score: Score) -> bool {
    true
}
