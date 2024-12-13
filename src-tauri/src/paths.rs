use std::path::PathBuf;
use std::sync::OnceLock;

use crate::{identifier, Error};

static HOME_DIR: OnceLock<PathBuf> = OnceLock::new();
static CACHE_DIR: OnceLock<PathBuf> = OnceLock::new();
static LOCAL_DATA_DIR: OnceLock<PathBuf> = OnceLock::new();
static RUNTIME_DIR: OnceLock<PathBuf> = OnceLock::new();

pub fn init() -> Result<(), Error> {
    HOME_DIR
        .set(dirs::home_dir().ok_or("Unable to determine home directory")?)
        .map_err(|_| "Already set")?;
    CACHE_DIR
        .set({
            let mut p = dirs::cache_dir().ok_or("Unable to determine cache directory")?;
            p.push(identifier());
            if cfg!(windows) {
                p.push("cache");
            }
            p
        })
        .map_err(|_| "Already set")?;
    LOCAL_DATA_DIR
        .set({
            let mut p = dirs::data_local_dir().ok_or("Unable to determine local data directory")?;
            p.push(identifier());
            p
        })
        .map_err(|_| "Already set")?;
    RUNTIME_DIR
        .set({
            let mut p = dirs::runtime_dir().unwrap_or_else(|| {
                let mut p = std::env::temp_dir();
                p.push("runtime");
                p
            });
            p.push(identifier());
            p
        })
        .map_err(|_| "Already set")?;

    std::fs::create_dir_all(cache_dir())?;
    std::fs::create_dir_all(local_data_dir())?;
    std::fs::create_dir_all(runtime_dir())?;

    Ok(())
}

pub fn home_dir() -> &'static PathBuf {
    HOME_DIR.get().unwrap()
}

pub fn cache_dir() -> &'static PathBuf {
    CACHE_DIR.get().unwrap()
}

pub fn local_data_dir() -> &'static PathBuf {
    LOCAL_DATA_DIR.get().unwrap()
}

pub fn runtime_dir() -> &'static PathBuf {
    RUNTIME_DIR.get().unwrap()
}
