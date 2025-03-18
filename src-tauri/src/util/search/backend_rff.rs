use std::ops::Add;

pub(super) type ScoreValue = f64;

#[derive(Debug, Clone, Copy)]
pub struct Score(pub(super) ScoreValue);

impl Score {
    pub const MIN: Self = Self(f64::NEG_INFINITY);
    pub const MAX: Self = Self(f64::INFINITY);
}

impl Add for Score {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl PartialEq for Score {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other).is_eq()
    }
}

impl Eq for Score {}

impl PartialOrd for Score {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Score {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.total_cmp(&other.0)
    }
}

pub fn score(needle: &str, haystack: &str) -> Option<Score> {
    Some(Score(rff::scorer::score(needle, haystack)))
}

pub fn should_include(score: Score) -> bool {
    score != Score::MIN
}
