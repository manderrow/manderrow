use std::ffi::{OsStr, OsString};
use std::io::Write;
use std::path::PathBuf;

use anyhow::{anyhow, bail, Context as _, Result};
use uuid::Uuid;

use crate::commands::profiles::read_profile;
use crate::games::{Game, GAMES_BY_ID};
use crate::installing::install_zip;
use crate::Error;

use super::steam::paths::{resolve_steam_app_install_directory, resolve_steam_directory};
use super::steam::{ensure_wine_will_load_dll_override, uses_proton};

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
                        // TODO: don't just discard this
                        value: _value,
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

fn get_url_and_hash(uses_proton: bool) -> Result<(&'static str, &'static str)> {
    Ok(match (std::env::consts::OS, std::env::consts::ARCH, uses_proton) {
        ("macos", "x86_64", false) => ("https://github.com/BepInEx/BepInEx/releases/download/v5.4.23.2/BepInEx_macos_x64_5.4.23.2.zip", "f90cb47010b52e8d2da1fff4b39b4e95f89dc1de9dddca945b685b9bf8e3ef81"),
        ("linux", "x86_64", true) => ("https://github.com/BepInEx/BepInEx/releases/download/v5.4.23.2/BepInEx_win_x64_5.4.23.2.zip", "d11015bf224343bdc429fbf5ac99bd12fffe115bfa5baf0df4ee81759887a116"),
        ("linux", "x86_64", false) => ("https://github.com/BepInEx/BepInEx/releases/download/v5.4.23.2/BepInEx_linux_x64_5.4.23.2.zip", "d655acbbb18dc5202c1ba5f6b87288372307868cc62843e3a78a25abf7a50ad3"),
        ("linux", "x86", true) => ("https://github.com/BepInEx/BepInEx/releases/download/v5.4.23.2/BepInEx_win_x86_5.4.23.2.zip", "db8b95c4dca085d20ce5fc7447f6cf9b18469a5d983e535ac8ea5ae8eea828f3"),
        ("linux", "x86", false) => ("https://github.com/BepInEx/BepInEx/releases/download/v5.4.23.2/BepInEx_linux_x86_5.4.23.2.zip", "99ba36a0d36e6a05db035fd1ac17d9e76740b4e230c598512c07622278222c30"),
        ("windows", "x86_64", false) => ("https://github.com/BepInEx/BepInEx/releases/download/v5.4.23.2/BepInEx_win_x64_5.4.23.2.zip", "d11015bf224343bdc429fbf5ac99bd12fffe115bfa5baf0df4ee81759887a116"),
        ("windows", "x86", false) => ("https://github.com/BepInEx/BepInEx/releases/download/v5.4.23.2/BepInEx_win_x86_5.4.23.2.zip", "db8b95c4dca085d20ce5fc7447f6cf9b18469a5d983e535ac8ea5ae8eea828f3"),
        (os, arch, uses_proton) => bail!("Unsupported platform combo: (os: {os:?}, arch: {arch:?}, uses_proton: {uses_proton})"),
    })
}

fn get_steam_id(game: &Game) -> Option<&str> {
    game.store_platform_metadata
        .iter()
        .find_map(|m| match m {
            crate::games::StorePlatformMetadata::Steam { store_identifier } => {
                Some(store_identifier)
            }
            crate::games::StorePlatformMetadata::SteamDirect { store_identifier } => {
                Some(store_identifier)
            }
            _ => None,
        })
        .map(String::as_str)
}

pub const BEP_IN_EX_FOLDER: &str = "BepInEx";

pub fn get_bep_in_ex_path(profile_id: Uuid) -> PathBuf {
    let mut p = crate::commands::profiles::profile_path(profile_id);
    p.push(BEP_IN_EX_FOLDER);
    p
}

pub async fn configure_command(command: &mut impl CommandBuilder, profile_id: Uuid) -> Result<()> {
    let profile = read_profile(profile_id).await?;
    let game = GAMES_BY_ID.get(&*profile.game).context("No such game")?;
    let steam_id = get_steam_id(game).context("Unsupported store platform")?;

    let uses_proton = uses_proton(steam_id).await?;

    let (url, hash) = get_url_and_hash(uses_proton)?;
    let bep_in_ex = get_bep_in_ex_path(profile_id);
    install_zip(url, Some(hash), &bep_in_ex)
        .await?
        .finish()
        .await?;

    if cfg!(windows) || uses_proton {
        command.args(["--doorstop-enable", "true"]);

        command.args(["--doorstop-target-assembly"]);
        if uses_proton {
            let mut buf = OsString::from("Z:");
            buf.push(
                bep_in_ex
                    .as_os_str()
                    .to_str()
                    .context(anyhow!("Invalid Unicode string: {bep_in_ex:?}"))?,
            );
            buf.push("/BepInEx/core/BepInEx.Preloader.dll");
            command.args([buf]);
        } else {
            let mut p = bep_in_ex.clone();
            p.push("BepInEx");
            p.push("core");
            p.push("BepInEx.Preloader.dll");
            command.args([p]);
        }

        command.args(["--doorstop-mono-debug-enabled", "false"]);
        command.args(["--doorstop-mono-debug-address", "127.0.0.1:10000"]);
        command.args(["--doorstop-mono-debug-suspend", "false"]);
        // specify these only if they have values
        // especially --doorstop-mono-dll-search-path-override, which will cause the doorstop to fail if given an empty string
        // command.args(["--doorstop-mono-dll-search-path-override", ""]);
        // command.args(["--doorstop-clr-corlib-dir", ""]);
        // command.args(["--doorstop-clr-runtime-coreclr-path", ""]);
    } else {
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
        command.env("DOORSTOP_CLR_CORLIB_DIR", "");
        command.env("DOORSTOP_CLR_RUNTIME_CORECLR_PATH", "");
    }

    if cfg!(windows) || uses_proton {
        if uses_proton {
            ensure_wine_will_load_dll_override(steam_id, "winhttp").await?;
        }
        tokio::fs::copy(
            bep_in_ex.join("winhttp.dll"),
            resolve_steam_app_install_directory(steam_id)
                .await?
                .join("winhttp.dll"),
        )
        .await?;
    } else {
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
    }

    Ok(())
}
