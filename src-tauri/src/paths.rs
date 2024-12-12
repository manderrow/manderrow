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

pub async fn resolve_steam_directory() -> Result<PathBuf, Error> {
    const ERROR_MSG: &str = "Could not locate Steam";
    if cfg!(target_os = "macos") {
        let path = home_dir().join("Library/Application Support/Steam");
        if tokio::fs::try_exists(&path).await? {
            Ok(path)
        } else {
            Err(ERROR_MSG.into())
        }
    } else if cfg!(target_os = "linux") {
        const PREFIXES: &[&[&str]] = &[&[], &[".var", "app", "com.valvesoftware.Steam"]];
        const PATHS: &[&[&str]] = &[
            &[".local", "share", "Steam"],
            &[".steam", "steam"],
            &[".steam", "root"],
            &[".steam"],
        ];
        let mut buf = home_dir().to_owned();
        for &prefix in PREFIXES {
            for &segment in prefix {
                buf.push(segment);
            }
            for &path in PATHS {
                for &segment in path {
                    buf.push(segment);
                }
                if tokio::fs::try_exists(&buf).await? {
                    return Ok(buf);
                }
                for _ in path {
                    buf.pop();
                }
            }
            for _ in prefix {
                buf.pop();
            }
        }
        Err(ERROR_MSG.into())
    } else if cfg!(windows) {
        #[cfg(windows)]
        {
            use registry::{Data, Hive, Security};
            let regkey =
                Hive::LocalMachine.open(r"SOFTWARE\\WOW6432Node\\Valve\\Steam", Security::Read)?;
            match regkey.value("InstallPath")? {
                Data::String(s) | Data::ExpandString(s) => Ok(s.to_string()?),
                _ => Err("Unexpected data type in registry".into()),
            }
        }
        #[cfg(not(windows))]
        unreachable!()
    } else {
        Err("Unsupported operating system".into())
    }
}
