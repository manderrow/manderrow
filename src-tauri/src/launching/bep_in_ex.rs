use std::ffi::{OsStr, OsString};
use std::io::Write;
use std::path::PathBuf;

use log::debug;
use tauri_plugin_http::reqwest;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;
use zip::result::ZipError;
use zip::ZipArchive;

use crate::paths::{cache_dir, resolve_steam_directory};
use crate::Error;

#[derive(Debug, thiserror::Error)]
enum InstallZipError {
    #[error(transparent)]
    IoError(#[from] std::io::Error),

    #[error(transparent)]
    Blake3HexError(#[from] blake3::HexError),

    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),

    #[error(transparent)]
    ZipError(#[from] zip::result::ZipError),
}

async fn install_zip(
    url: &str,
    hash_str: &str,
    mut target: PathBuf,
) -> Result<PathBuf, InstallZipError> {
    target.push(hash_str);
    match tokio::fs::metadata(&target).await {
        Ok(m) if m.is_dir() => {
            debug!("Zip is already installed to {target:?}");
            return Ok(target);
        }
        Ok(_) => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotADirectory,
                "Target exists, but is not a directory",
            )
            .into())
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e.into()),
    }
    target.pop();

    let path = cache_dir().join(format!("{hash_str}.zip"));
    let hash = blake3::Hash::from_hex(hash_str)?;
    let hash_on_disk = {
        let mut hsr = blake3::Hasher::new();
        match hsr.update_mmap(&path) {
            Ok(_) => Some(hsr.finalize()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(e.into()),
        }
    };

    if hash_on_disk.map(|h| h != hash).unwrap_or(true) {
        let mut resp = reqwest::get(url).await?.error_for_status()?;
        let mut wtr = tokio::fs::File::create(&path).await?;
        while let Some(chunk) = resp.chunk().await? {
            wtr.write_all(&chunk).await?;
        }
        debug!("Cached zip at {path:?}");
    } else {
        debug!("Zip is cached at {path:?}");
    }

    let tmp_dir = tempfile::tempdir_in(&target)?;
    tokio::task::block_in_place(|| {
        let mut archive = ZipArchive::new(std::io::BufReader::new(std::fs::File::open(&path)?))?;
        archive.extract(tmp_dir.path())?;
        Ok::<_, ZipError>(())
    })?;
    target.push(hash_str);
    tokio::fs::rename(tmp_dir.into_path(), &target).await?;
    debug!("Installed zip to {target:?}");
    Ok(target)
}

async fn apply_launch_args(game_id: &str) -> Result<(), Error> {
    let mut path = resolve_steam_directory().await?;
    path.push("userdata");

    let mut iter = tokio::fs::read_dir(&path).await?;
    while let Some(e) = iter.next_entry().await? {
        tokio::task::block_in_place(|| {
            use vdf::Event;

            path.push(e.file_name());
            path.push("config");
            let mut dst = tempfile::NamedTempFile::new_in(&path)?;
            let mut wtr = std::io::BufWriter::new(dst.as_file_mut());
            path.push("localconfig.vdf");
            let mut rdr = vdf::Reader::new(std::fs::File::open(&path)?);

            const KEY_PATH: &[&str] =
                &["UserLocalConfigStore", "Software", "Valve", "Steam", "apps"];
            const LAUNCH_OPTIONS_KEY: &str = "LaunchOptions";
            enum MatcherState {
                MatchingPath(usize),
                SkippingPath(usize),
                MatchingGame,
                MatchingLaunchOptions,
                SkippingGame(usize),
                SkippingInsideGame(usize),
                Done,
            }
            let mut state = MatcherState::MatchingPath(0);
            let mut launch_options_str = std::env::current_exe()
                .unwrap()
                .into_os_string()
                .into_string()
                .map_err(|_| "Non-Unicode executable name")?;
            launch_options_str.push_str(" wrap %command%");
            let mut matched_launch_options = false;
            while let Some(event) = rdr.next()? {
                match event {
                    Event::GroupStart { key, .. } => {
                        vdf::write_io(event, &mut wtr)?;
                        if let MatcherState::MatchingPath(i) = state {
                            if i < KEY_PATH.len() {
                                if key.s != KEY_PATH[i].as_bytes() {
                                    state = MatcherState::SkippingPath(i);
                                }
                            }
                        }
                        match &mut state {
                            MatcherState::MatchingPath(i)
                            | MatcherState::SkippingPath(i)
                            | MatcherState::SkippingGame(i)
                            | MatcherState::SkippingInsideGame(i) => {
                                *i += 1;
                            }
                            MatcherState::MatchingGame if key.s == game_id.as_bytes() => {
                                state = MatcherState::MatchingLaunchOptions;
                            }
                            MatcherState::MatchingGame => {
                                state = MatcherState::SkippingGame(0);
                            }
                            MatcherState::MatchingLaunchOptions => {
                                state = MatcherState::SkippingInsideGame(0);
                            }
                            MatcherState::Done => {}
                        }
                    }
                    Event::Item {
                        pre_whitespace,
                        key,
                        mid_whitespace,
                        value,
                    } if matches!(state, MatcherState::MatchingLaunchOptions)
                        && key.s == LAUNCH_OPTIONS_KEY.as_bytes() =>
                    {
                        matched_launch_options = true;
                        vdf::write_io(
                            Event::Item {
                                pre_whitespace,
                                key,
                                mid_whitespace,
                                value: vdf::Str {
                                    s: launch_options_str.as_bytes(),
                                    quoted: true,
                                },
                            },
                            &mut wtr,
                        )?;
                    }
                    Event::Item { .. } => {
                        vdf::write_io(event, &mut wtr)?;
                    }
                    Event::GroupEnd { pre_whitespace } => {
                        match &mut state {
                            MatcherState::MatchingPath(i) | MatcherState::SkippingPath(i) => {
                                *i -= 1;
                            }
                            MatcherState::SkippingGame(0) => {
                                state = MatcherState::MatchingGame;
                            }
                            MatcherState::SkippingGame(i) => {
                                *i -= 1;
                            }
                            MatcherState::SkippingInsideGame(0) => {
                                state = MatcherState::MatchingLaunchOptions;
                            }
                            MatcherState::SkippingInsideGame(i) => {
                                *i -= 1;
                            }
                            MatcherState::MatchingGame => {
                                return Err("Game not installed".into());
                            }
                            MatcherState::MatchingLaunchOptions => {
                                if !matched_launch_options {
                                    vdf::write_io(
                                        Event::Item {
                                            pre_whitespace,
                                            key: vdf::Str {
                                                s: LAUNCH_OPTIONS_KEY.as_bytes(),
                                                quoted: true,
                                            },
                                            mid_whitespace: b"\t\t",
                                            value: vdf::Str {
                                                s: launch_options_str.as_bytes(),
                                                quoted: true,
                                            },
                                        },
                                        &mut wtr,
                                    )?;
                                }
                                state = MatcherState::Done;
                            }
                            MatcherState::Done => {}
                        }
                        vdf::write_io(event, &mut wtr)?;
                    }
                    Event::Comment { .. } => vdf::write_io(event, &mut wtr)?,
                    Event::FileEnd { .. } => vdf::write_io(event, &mut wtr)?,
                }
            }

            wtr.flush()?;
            drop(wtr);
            dst.persist(&path)?;

            path.pop();
            path.pop();
            path.pop();

            Ok::<_, Error>(())
        })?;
    }

    Ok(())
}

pub trait CommandBuilder {
    fn env(&mut self, key: impl AsRef<str>, value: impl AsRef<OsStr>);

    fn args(&mut self, args: impl IntoIterator<Item = impl AsRef<OsStr>>);
}

pub async fn configure_command(
    command: &mut impl CommandBuilder,
    profile: Uuid,
) -> anyhow::Result<()> {
    let bep_in_ex = install_zip(
        "https://github.com/BepInEx/BepInEx/releases/download/v5.4.23.2/BepInEx_macos_x64_5.4.23.2.zip",
        "f90cb47010b52e8d2da1fff4b39b4e95f89dc1de9dddca945b685b9bf8e3ef81",
        crate::commands::profiles::profile_path(profile)
    ).await?;

    command.env("DOORSTOP_ENABLED", "1");
    command.env(
        "DOORSTOP_TARGET_ASSEMBLY",
        bep_in_ex.join("BepInEx/core/BepInEx.Preloader.dll"),
    );
    command.env("DOORSTOP_IGNORE_DISABLED_ENV", "0");
    command.env("DOORSTOP_MONO_DLL_SEARCH_PATH_OVERRIDE", "");
    command.env("DOORSTOP_MONO_DEBUG_ENABLED", "0");
    command.env("DOORSTOP_MONO_DEBUG_ADDRESS", "127.0.0.1:10000");
    command.env("DOORSTOP_MONO_DEBUG_SUSPEND", "0");
    command.env("DOORSTOP_CLR_RUNTIME_CORECLR_PATH", "");
    command.env("DOORSTOP_CLR_CORLIB_DIR", "");

    for var in ["LD_LIBRARY_PATH", "DYLD_LIBRARY_PATH"] {
        let base = std::env::var_os(var).unwrap_or_else(OsString::new);
        let mut buf = bep_in_ex.as_os_str().to_owned();
        if !base.is_empty() {
            buf.push(":");
            buf.push(base);
        }

        command.env(var, buf);
    }

    for var in ["LD_PRELOAD", "DYLD_INSERT_LIBRARIES"] {
        let base = std::env::var_os(var).unwrap_or_else(OsString::new);
        let mut buf = OsString::from(if cfg!(target_os = "macos") {
            "libdoorstop.dylib"
        } else {
            "libdoorstop.so"
        });
        if !base.is_empty() {
            buf.push(":");
            buf.push(base);
        }

        command.env(var, buf);
    }

    Ok(())
}
