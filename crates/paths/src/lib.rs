use std::path::PathBuf;
use std::sync::OnceLock;

/// Casing depends on platform.
pub const FOLDER_NAME: &'static str = if cfg!(target_os = "macos") || cfg!(target_os = "windows") {
    // use uppercase on macOS and Windows
    "Manderrow"
} else {
    // use lowercase on Linux
    "manderrow"
};

/// Always lowercase.
pub const PRODUCT_NAME: &'static str = "manderrow";

static HOME_DIR: OnceLock<PathBuf> = OnceLock::new();
static CACHE_DIR: OnceLock<PathBuf> = OnceLock::new();
static CONFIG_DIR: OnceLock<PathBuf> = OnceLock::new();
static LOCAL_DATA_DIR: OnceLock<PathBuf> = OnceLock::new();
static RUNTIME_DIR: OnceLock<PathBuf> = OnceLock::new();
static LOGS_DIR: OnceLock<PathBuf> = OnceLock::new();

#[derive(Debug, thiserror::Error)]
pub enum InitError {
    #[error("The {name} directory slot is already filled")]
    AlreadySet { name: &'static str },
    #[error("Unable to determine the {name} directory")]
    UnableToDetermine { name: &'static str },
    #[error("The {name} directory path is not absolute: {path:?}")]
    NotAbsolute { name: &'static str, path: PathBuf },
    #[error("Failed to create the {name} directory: {error}")]
    FailedToCreate {
        name: &'static str,
        error: std::io::Error,
    },
}

fn set(
    name: &'static str,
    slot: &OnceLock<PathBuf>,
    path: Option<PathBuf>,
) -> Result<(), InitError> {
    let path = path.ok_or(InitError::UnableToDetermine { name })?;

    if !path.is_absolute() {
        return Err(InitError::NotAbsolute { name, path });
    }

    std::fs::create_dir_all(&path).map_err(|error| InitError::FailedToCreate { name, error })?;

    slot.set(path).map_err(|_| InitError::AlreadySet { name })?;
    Ok(())
}

pub fn init() -> Result<(), InitError> {
    set("home", &HOME_DIR, dirs::home_dir())?;
    set(
        "cache",
        &CACHE_DIR,
        dirs::cache_dir().map(|mut p| {
            p.push(FOLDER_NAME);
            if cfg!(windows) {
                p.push("cache");
            }
            p
        }),
    )?;
    set(
        "config",
        &CONFIG_DIR,
        dirs::config_dir().map(|mut p| {
            p.push(FOLDER_NAME);
            p
        }),
    )?;
    set(
        "local data",
        &LOCAL_DATA_DIR,
        dirs::data_local_dir().map(|mut p| {
            p.push(FOLDER_NAME);
            p
        }),
    )?;
    set(
        "runtime",
        &RUNTIME_DIR,
        Some(
            dirs::runtime_dir()
                .map(|mut p| {
                    p.push(FOLDER_NAME);
                    p
                })
                .unwrap_or_else(|| {
                    let mut p = std::env::temp_dir();
                    p.push(FOLDER_NAME);
                    p.push("runtime");
                    p
                }),
        ),
    )?;
    set("logs", &LOGS_DIR, Some(local_data_dir().join("logs")))?;

    Ok(())
}

pub fn home_dir() -> &'static PathBuf {
    HOME_DIR.get().unwrap()
}

pub fn cache_dir() -> &'static PathBuf {
    CACHE_DIR.get().unwrap()
}

pub fn config_dir() -> &'static PathBuf {
    CONFIG_DIR.get().unwrap()
}

pub fn local_data_dir() -> &'static PathBuf {
    LOCAL_DATA_DIR.get().unwrap()
}

pub fn runtime_dir() -> &'static PathBuf {
    RUNTIME_DIR.get().unwrap()
}

pub fn logs_dir() -> &'static PathBuf {
    LOGS_DIR.get().unwrap()
}
