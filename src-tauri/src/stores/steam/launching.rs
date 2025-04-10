use std::io::Write as _;
use std::ops::BitOrAssign;

use anyhow::{anyhow, bail, Context as _, Result};
use slog::{debug, info};
use tokio::process::Command;

use super::paths::{get_steam_exe, resolve_steam_directory};
use crate::ipc::{DoctorFix, InProcessIpc, OutputLine};

pub async fn kill_steam(log: &slog::Logger) -> Result<()> {
    #[cfg(windows)]
    {
        use std::num::NonZeroU32;
        use std::ptr::NonNull;

        use winsafe::prelude::*;

        let mut issued_shutdown = false;
        for proc in
            winsafe::HPROCESSLIST::CreateToolhelp32Snapshot(winsafe::co::TH32CS::SNAPPROCESS, None)?
                .iter_processes()
        {
            let proc = proc?;
            // winsafe doesn't allow us to access szExeFile without allocating a string. We are **not** doing that for every process on the system.
            let proc = unsafe {
                NonNull::from(proc)
                    .cast::<windows::Win32::System::Diagnostics::ToolHelp::PROCESSENTRY32>()
                    .as_ref()
            };
            let name = unsafe { NonNull::from(&proc.szExeFile).cast::<[u8; 260]>().as_ref() };
            let name = std::ffi::CStr::from_bytes_until_nul(name)?;
            if name.to_bytes() == b"steam.exe" {
                if !issued_shutdown {
                    issued_shutdown = true;
                    info!(log, "Steam is open. Issuing shutdown request.");
                    Command::new(get_steam_exe()?.as_ref())
                        .arg("-shutdown")
                        .status()
                        .await?
                        .exit_ok()?;
                }

                info!(
                    log,
                    "Waiting for Steam process {} to shut down", proc.th32ProcessID
                );
                manderrow_process_util::Pid::from_raw(
                    NonZeroU32::new(proc.th32ProcessID).context("null pid")?,
                )
                .wait_for_exit(log)
                .await?;
            }
        }
    }
    #[cfg(unix)]
    {
        let output = tokio::process::Command::new("pgrep")
            .arg(if cfg!(target_os = "macos") {
                "steam_osx"
            } else {
                "steam"
            })
            .output()
            .await?;
        if output.status.code() == Some(1) {
            if output.stdout.is_empty() && output.stderr.is_empty() {
                debug!(
                    log,
                    "pgrep exited with code 1 and no output. Assuming no processes found."
                );
                return Ok(());
            }
        }
        match output.status.exit_ok() {
            Ok(()) => {}
            Err(e) => {
                return Err(anyhow::Error::from(e).context(format!(
                    "pgrep failed\nstdout: {:?}\nstderr: {:?}",
                    OutputLine::new(output.stdout),
                    OutputLine::new(output.stderr)
                )))
            }
        }

        info!(log, "Steam is open. Issuing shutdown request.");
        Command::new(get_steam_exe()?.as_ref())
            .arg("-shutdown")
            .status()
            .await?
            .exit_ok()?;

        let output = String::from_utf8(output.stdout)?;

        for pid in output.lines() {
            let pid = pid.parse().context("Invalid pid from pgrep")?;
            manderrow_process_util::Pid::from_raw(pid)
                .wait_for_exit(log)
                .await?;
        }
    }
    Ok(())
}

pub fn generate_unix_launch_options() -> Result<String> {
    let bin = std::env::current_exe()
        .context("Failed to get current exe path")?
        .into_os_string()
        .into_string()
        .map_err(|s| anyhow!("Non-Unicode executable name: {s:?}"))?;
    Ok(format!("{bin:?} wrap %command%"))
}

pub async fn ensure_unix_launch_args_are_applied(
    log: &slog::Logger,
    mut comms: Option<&mut InProcessIpc>,
    game_id: &str,
) -> Result<(), crate::Error> {
    let args = generate_unix_launch_options()?;
    loop {
        let result = apply_launch_args(game_id, &args, true, true).await?;
        if matches!(
            result,
            AppliedLaunchArgs::Applied | AppliedLaunchArgs::Overwrote
        ) {
            #[derive(serde::Deserialize, serde::Serialize)]
            #[serde(rename_all = "snake_case")]
            enum Fix {
                Apply,
                Retry,
                Abort,
            }
            let Some(ipc) = &mut comms else {
                return Err(anyhow!("Not adding launch options without consent").into());
            };
            let choice = ipc
                .prompt_patient(
                    "launch_options",
                    if matches!(result, AppliedLaunchArgs::Overwrote) {
                        Some("doctor.launch_options.message_overwrite".to_owned())
                    } else {
                        None
                    },
                    None,
                    [
                        DoctorFix {
                            id: Fix::Apply,
                            label: None,
                            confirm_label: None,
                            description: None,
                        },
                        DoctorFix {
                            id: Fix::Retry,
                            label: None,
                            confirm_label: None,
                            description: Some(
                                [(
                                    "launch_options".to_owned(),
                                    serde_json::Value::from(args.clone()),
                                )]
                                .into(),
                            ),
                        },
                        DoctorFix {
                            id: Fix::Abort,
                            label: None,
                            confirm_label: None,
                            description: None,
                        },
                    ],
                )
                .await?;
            match choice {
                Fix::Apply => {
                    kill_steam(log).await?;
                    apply_launch_args(
                        game_id,
                        &args,
                        matches!(result, AppliedLaunchArgs::Overwrote),
                        false,
                    )
                    .await?;
                    break;
                }
                Fix::Retry => {}
                Fix::Abort => return Err(crate::Error::Aborted),
            }
        } else {
            break;
        }
    }
    Ok(())
}

#[derive(Clone, Copy)]
enum AppliedLaunchArgs {
    Unchanged,
    Applied,
    Overwrote,
}

impl BitOrAssign for AppliedLaunchArgs {
    fn bitor_assign(&mut self, rhs: Self) {
        use AppliedLaunchArgs::*;
        *self = match (*self, rhs) {
            (Overwrote, _) | (_, Overwrote) => Overwrote,
            (Applied, _) | (_, Applied) => Applied,
            (Unchanged, Unchanged) => Unchanged,
        };
    }
}

/// Attempts to apply the launch options necessary to use our wrapper to the
/// specified game. If `dry_run` is `true`, this will simply check if the
/// options have already been applied.
///
/// Returns `true` if a change was made, or would be made if this is a dry run.
async fn apply_launch_args(
    game_id: &str,
    args: &str,
    overwrite_ok: bool,
    dry_run: bool,
) -> Result<AppliedLaunchArgs> {
    let mut path = resolve_steam_directory().await?;
    path.push("userdata");

    let mut result = AppliedLaunchArgs::Unchanged;

    let mut iter = tokio::fs::read_dir(&path).await?;
    while let Some(e) = iter.next_entry().await? {
        path.push(e.file_name());
        path.push("config");

        let mut dst = if dry_run {
            None
        } else {
            Some(
                tempfile::NamedTempFile::new_in(&path)
                    .with_context(|| format!("Failed to create temporary file in {path:?}"))?,
            )
        };

        path.push("localconfig.vdf");

        result |= tokio::task::block_in_place(|| {
            let mut wtr = if let Some(ref mut dst) = dst {
                Some(std::io::BufWriter::new(dst.as_file_mut()))
            } else {
                None
            };
            let rdr = vdf::Reader::new(std::io::BufReader::new(std::fs::File::open(&path)?));

            let result = if let Some(ref mut wtr) = wtr {
                let result = apply_launch_args_inner(game_id, overwrite_ok, args, rdr, &mut *wtr)?;
                wtr.flush()?;
                result
            } else {
                apply_launch_args_inner(game_id, overwrite_ok, args, rdr, std::io::empty())?
            };
            drop(wtr);

            if let Some(dst) = dst {
                dst.persist(&path)?;
            }

            Ok::<_, anyhow::Error>(result)
        })
        .with_context(|| format!("Failed to apply launch options to {path:?}"))?;

        path.pop();
        path.pop();
        path.pop();
    }
    Ok(result)
}

fn apply_launch_args_inner<R: std::io::BufRead, W: std::io::Write>(
    game_id: &str,
    overwrite_ok: bool,
    args: &str,
    mut rdr: vdf::Reader<R>,
    mut wtr: W,
) -> Result<AppliedLaunchArgs> {
    use vdf::Event;

    const KEY_PATH: &[&str] = &["UserLocalConfigStore", "Software", "Valve", "Steam", "apps"];
    const LAUNCH_OPTIONS_KEY: &str = "LaunchOptions";
    enum MatcherState {
        MatchingPath(usize),
        SkippingPath {
            depth: usize,
            match_at: usize,
        },
        MatchingGame,
        MatchingLaunchOptions,
        /// Encountered non-game_id Game, skipping it, return to MatchingGame after.
        SkippingGame(usize),
        /// Encountered non-LaunchOptions inside a Game, skipping it, return to MatchingLaunchOptions after.
        SkippingInsideGame(usize),
    }
    enum Flag {
        None,
        MatchedPath(usize),
        MatchedGame,
        MatchedLaunchOptions,
        ModifiedLaunchOptions { overwrote: bool },
    }
    let mut state = MatcherState::MatchingPath(0);
    let mut flag = Flag::None;
    while let Some(event) = rdr.next()? {
        match event {
            Event::GroupStart { key, .. } => {
                vdf::write_io(event, &mut wtr)?;
                match &mut state {
                    MatcherState::MatchingPath(i) if key.s == KEY_PATH[*i].as_bytes() => {
                        match flag {
                            Flag::None => {
                                debug_assert_eq!(*i, 0);
                                flag = Flag::MatchedPath(*i);
                            }
                            Flag::MatchedPath(ref mut j) if *i > *j => {
                                *j = *i;
                            }
                            _ => {}
                        }
                        if *i == KEY_PATH.len() - 1 {
                            state = MatcherState::MatchingGame;
                        } else {
                            *i += 1;
                        }
                    }
                    MatcherState::MatchingPath(i) => {
                        state = MatcherState::SkippingPath {
                            match_at: *i,
                            depth: 0,
                        };
                    }
                    MatcherState::SkippingPath { depth: i, .. }
                    | MatcherState::SkippingGame(i)
                    | MatcherState::SkippingInsideGame(i) => {
                        *i += 1;
                    }
                    MatcherState::MatchingGame if key.s == game_id.as_bytes() => {
                        match flag {
                            Flag::None => unreachable!(),
                            Flag::MatchedPath(_) => {}
                            _ => bail!("Duplicate game entry"),
                        }
                        flag = Flag::MatchedGame;
                        state = MatcherState::MatchingLaunchOptions;
                    }
                    MatcherState::MatchingGame => {
                        state = MatcherState::SkippingGame(0);
                    }
                    MatcherState::MatchingLaunchOptions => {
                        state = MatcherState::SkippingInsideGame(0);
                    }
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
                match flag {
                    Flag::None => unreachable!(),
                    Flag::MatchedPath(_) => unreachable!(),
                    Flag::MatchedGame => {}
                    Flag::MatchedLaunchOptions | Flag::ModifiedLaunchOptions { .. } => {
                        bail!("Duplicate LaunchOptions entry")
                    }
                }
                vdf::write_io(
                    Event::Item {
                        pre_whitespace,
                        key,
                        mid_whitespace,
                        value: if value.s != args.as_bytes() {
                            if !value.s.is_empty() && !overwrite_ok {
                                bail!("Refusing to overwrite launch options.");
                            }
                            flag = Flag::ModifiedLaunchOptions {
                                overwrote: !value.s.is_empty(),
                            };
                            vdf::Str {
                                s: args.as_bytes(),
                                quoted: true,
                            }
                        } else {
                            flag = Flag::MatchedLaunchOptions;
                            value
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
                    MatcherState::MatchingPath(0) => bail!("GroupEnd when MatchingPath(0)"),
                    MatcherState::SkippingPath { depth: 0, match_at } => {
                        state = MatcherState::MatchingPath(*match_at);
                    }
                    MatcherState::SkippingGame(0) => {
                        state = MatcherState::MatchingGame;
                    }
                    MatcherState::SkippingInsideGame(0) => {
                        state = MatcherState::MatchingLaunchOptions;
                    }
                    MatcherState::MatchingPath(i)
                    | MatcherState::SkippingPath { depth: i, .. }
                    | MatcherState::SkippingGame(i)
                    | MatcherState::SkippingInsideGame(i) => {
                        *i -= 1;
                    }
                    MatcherState::MatchingGame => {
                        match flag {
                            Flag::None => unreachable!(),
                            Flag::MatchedPath(_) => {
                                flag = Flag::ModifiedLaunchOptions { overwrote: false };
                                vdf::write_io(
                                    Event::GroupStart {
                                        pre_whitespace: b"\n\t\t\t\t\t",
                                        key: vdf::Str {
                                            s: game_id.as_bytes(),
                                            quoted: true,
                                        },
                                        mid_whitespace: b"\n\t\t\t\t\t",
                                    },
                                    &mut wtr,
                                )?;
                                vdf::write_io(
                                    Event::Item {
                                        pre_whitespace,
                                        key: vdf::Str {
                                            s: LAUNCH_OPTIONS_KEY.as_bytes(),
                                            quoted: true,
                                        },
                                        mid_whitespace: b"\t\t",
                                        value: vdf::Str {
                                            s: args.as_bytes(),
                                            quoted: true,
                                        },
                                    },
                                    &mut wtr,
                                )?;
                                vdf::write_io(
                                    Event::GroupEnd {
                                        pre_whitespace: b"\n\t\t\t\t\t",
                                    },
                                    &mut wtr,
                                )?;
                            }
                            Flag::MatchedGame
                            | Flag::MatchedLaunchOptions
                            | Flag::ModifiedLaunchOptions { .. } => {}
                        }
                        state = MatcherState::MatchingPath(KEY_PATH.len() - 1);
                    }
                    MatcherState::MatchingLaunchOptions => {
                        match flag {
                            Flag::None => unreachable!(),
                            Flag::MatchedPath(_) => unreachable!(),
                            Flag::MatchedGame => {
                                flag = Flag::ModifiedLaunchOptions { overwrote: false };
                                vdf::write_io(
                                    Event::Item {
                                        pre_whitespace,
                                        key: vdf::Str {
                                            s: LAUNCH_OPTIONS_KEY.as_bytes(),
                                            quoted: true,
                                        },
                                        mid_whitespace: b"\t\t",
                                        value: vdf::Str {
                                            s: args.as_bytes(),
                                            quoted: true,
                                        },
                                    },
                                    &mut wtr,
                                )?;
                            }
                            Flag::MatchedLaunchOptions | Flag::ModifiedLaunchOptions { .. } => {}
                        }
                        // go back to MatchingGame, just in case there's something weird going on and there is more than one entry for the game.
                        state = MatcherState::MatchingGame;
                    }
                }
                vdf::write_io(event, &mut wtr)?;
            }
            Event::Comment { .. } => vdf::write_io(event, &mut wtr)?,
            Event::FileEnd { .. } => vdf::write_io(event, &mut wtr)?,
        }
    }

    if !matches!(state, MatcherState::MatchingPath(0)) {
        bail!("Matcher did not complete")
    }

    Ok(match flag {
        Flag::None => bail!("Nothing matched"),
        Flag::MatchedPath(i) => bail!(
            "Game options not found for game_id {game_id:?}, path matched was {:?}",
            &KEY_PATH[..=i]
        ),
        Flag::MatchedGame => {
            unreachable!("MatchedGame, but neither MatchedLaunchOptions nor ModifiedLaunchOptions")
        }
        Flag::MatchedLaunchOptions => AppliedLaunchArgs::Unchanged,
        Flag::ModifiedLaunchOptions { overwrote: false } => AppliedLaunchArgs::Applied,
        Flag::ModifiedLaunchOptions { overwrote: true } => AppliedLaunchArgs::Overwrote,
    })
}
