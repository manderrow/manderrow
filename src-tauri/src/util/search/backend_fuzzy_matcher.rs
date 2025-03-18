use std::ops::Add;

use fuzzy_matcher::FuzzyMatcher;

pub(super) type ScoreValue = i64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Score(pub(super) ScoreValue);

impl Score {
    pub const MIN: Self = Self(i64::MIN);
    pub const MAX: Self = Self(i64::MAX);
}

impl Add for Score {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0.saturating_add(rhs.0))
    }
}

pub fn score(needle: &str, haystack: &str) -> Option<Score> {
    // let mut score = fuzzy_matcher::skim::SkimMatcherV2::default().ignore_case().fuzzy_match(haystack, needle).map(Score);
    let mut score = fuzzy_matcher::clangd::ClangdMatcher::default()
        .ignore_case()
        .fuzzy_match(haystack, needle)
        .map(Score);
    score = add_bonus(score, score_acronym(needle, haystack, 200));
    score
}

pub fn should_include(score: Score) -> bool {
    true
}
