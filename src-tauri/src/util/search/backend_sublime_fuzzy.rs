use std::ops::{Add, Div, Mul};

use bumpalo::Bump;
use simple_sublime_fuzzy::Query;

pub(super) type ScoreValue = isize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Score(pub(super) ScoreValue);

impl Score {
    #[allow(unused)]
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

impl Mul<u32> for Score {
    type Output = Self;

    fn mul(self, rhs: u32) -> Self::Output {
        Self(self.0 * rhs as ScoreValue)
    }
}

impl Div<u32> for Score {
    type Output = Self;

    fn div(self, rhs: u32) -> Self::Output {
        Self(self.0 / rhs as ScoreValue)
    }
}

impl std::fmt::Display for Score {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone)]
pub struct Needle<'a> {
    pub needle: &'a str,
    pub query: Query<'a>,
}

/// Needle must be normalized to lowercase.
pub fn score(bump: &Bump, needle: &Needle<'_>, haystack: &str) -> Option<Score> {
    let mut score = simple_sublime_fuzzy::best_match(
        &bump,
        &needle.query,
        &simple_sublime_fuzzy::Scoring {
            bonus_consecutive: 24,
            penalty_distance: 4,
        },
        haystack,
    )
    .map(|m| Score(m.score()));
    if starts_with_case_insensitive(haystack, needle.needle) {
        score = score.map(|s| Score(s.0 * 2));
    }
    score
}

pub fn score_non_simple(bump: &Bump, needle: &str, haystack: &str) -> Option<Score> {
    let mut score = sublime_fuzzy::FuzzySearch::new(needle, haystack)
        .case_insensitive()
        .score_with(&sublime_fuzzy::Scoring {
            bonus_consecutive: 24,
            bonus_word_start: 48,
            bonus_match_case: 0,
            penalty_distance: 4,
        })
        .best_match(bump)
        .map(|m| Score(m.score()));
    if haystack.starts_with(needle) {
        score = score.map(|s| Score(s.0 * 2));
    }
    score
}

/// Needle must already be normalized to lowercase.
pub fn starts_with_case_insensitive(haystack: &str, needle: &str) -> bool {
    needle
        .chars()
        .eq(haystack.chars().flat_map(|c| c.to_lowercase()))
}

pub fn should_include(_score: Score) -> bool {
    true
}
