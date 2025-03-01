pub mod bep_in_ex;
pub mod steam;

use std::path::PathBuf;
use std::sync::LazyLock;

use crate::paths::cache_dir;

pub static LOADERS_DIR: LazyLock<PathBuf> = LazyLock::new(|| cache_dir().join("loaders"));
