#[cfg(feature = "search-fuzzy-matcher")]
mod backend_fuzzy_matcher;
#[cfg(feature = "search-fuzzy-matcher")]
use backend_fuzzy_matcher as backend;

#[cfg(feature = "search-rff")]
mod backend_rff;
#[cfg(feature = "search-rff")]
use backend_rff as backend;

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

impl Score {
    pub fn non_negative(self) -> Self {
        Self(std::cmp::max(self.0, Self::ZERO.0))
    }
}

pub fn add_scores(a: Option<Score>, b: Option<Score>) -> Option<Score> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a + b),
        (None, Some(s)) | (Some(s), None) => Some(s),
        (None, None) => None,
    }
}

pub fn add_bonus(score: Option<Score>, bonus: Score) -> Option<Score> {
    match (score, bonus) {
        (Some(score), bonus) => Some(score + bonus),
        (None, Score(0)) => None,
        (None, bonus) => Some(bonus),
    }
}

fn score_acronym(needle: &str, haystack: &str, multiplier: ScoreValue) -> Score {
    let needle_len = needle.len();
    let mut needle = needle.chars();
    let words = haystack
        .split_terminator(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty());
    let mut matched: ScoreValue = 0;
    let mut total: ScoreValue = 0;
    for chunk in words {
        if let Some(c) = needle.next() {
            matched += (chunk.chars().next().unwrap().eq_ignore_ascii_case(&c)) as ScoreValue;
        }
        total += 1;
    }
    let total = std::cmp::max(total, needle_len as isize);
    Score(if total != 0 {
        (matched * matched) * multiplier / (total * total)
    } else {
        0
    })
}

fn score_from_beginning(needle: &str, haystack: &str) -> Score {
    const GAP_PENALTY: ScoreValue = 4;
    const MISS_PENALTY: ScoreValue = 10;
    const MATCH_REWARD: ScoreValue = 6;
    const CONSECUTIVE_REWARD: ScoreValue = 1;
    const FIRST_CHAR_REWARD: ScoreValue = 48;
    const FIRST_CHAR_EACH_REWARD: ScoreValue = 4;

    let mut matched: ScoreValue = 0;
    let mut score: ScoreValue = 0;
    let mut miss_penalties: ScoreValue = 0;
    let mut needle = needle.chars();
    // initialize to FIRST_CHAR_REWARD to give a bonus if the very first character of the word matches
    let mut did_match: ScoreValue = FIRST_CHAR_EACH_REWARD;
    let mut i = 0;
    'outer: while i < haystack.len() {
        let h = haystack[i..].chars().next().unwrap();
        let Some(n) = needle.next() else {
            break;
        };
        if h.eq_ignore_ascii_case(&n) {
            if i == 0 {
                score += FIRST_CHAR_REWARD;
            }
            matched += 1;
            score += did_match;
            score += MATCH_REWARD;
            did_match += CONSECUTIVE_REWARD;
        } else {
            let j = i + h.len_utf8();
            let mut n = n;
            let mut did_miss = false;
            loop {
                if let Some((k, _)) = haystack[j..]
                    .char_indices()
                    .find(|(_, h)| h.eq_ignore_ascii_case(&n))
                {
                    i = j + k;
                    score += MATCH_REWARD - GAP_PENALTY;
                    if did_miss {
                        miss_penalties += MISS_PENALTY;
                    }
                    matched += 1;
                    did_match = 1;
                    break;
                } else {
                    match needle.next() {
                        Some(n1) => n = n1,
                        None => break 'outer,
                    }
                    did_miss = true;
                }
            }
        }
        i += h.len_utf8();
    }
    score -= miss_penalties * (haystack.len() as isize - matched);
    Score(score)
}
