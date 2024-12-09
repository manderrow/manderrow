use std::{collections::HashMap, sync::LazyLock};

pub static GAME_REVIEWS: LazyLock<HashMap<String, i64>> =
    LazyLock::new(|| serde_json::from_str(include_str!("gameReviews.json")).unwrap());
