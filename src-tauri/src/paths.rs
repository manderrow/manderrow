use std::path::PathBuf;
use std::sync::LazyLock;

pub static DATA_LOCAL_DIR: LazyLock<PathBuf> = LazyLock::new(|| dirs::data_local_dir().unwrap());