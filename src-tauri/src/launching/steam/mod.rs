pub mod paths;
pub mod proton;

use std::io::Write as _;
use std::ops::BitOrAssign;

use anyhow::{anyhow, bail, Context as _, Result};
use paths::get_steam_exe;
use slog::{debug, info};
use tokio::process::Command;

use crate::ipc::{DoctorFix, OutputLine, Spc};

pub async fn kill_steam(log: &slog::Logger) -> Result<()> {
    #[cfg(windows)]
    {
        use std::mem::MaybeUninit;
        use std::ptr::{addr_of_mut, NonNull};

        use slog::warn;
        use windows::Win32::System::Diagnostics::ToolHelp::PROCESSENTRY32;

        use crate::windows_util::Handle;

        let snapshot = unsafe {
            Handle::new(
                windows::Win32::System::Diagnostics::ToolHelp::CreateToolhelp32Snapshot(
                    windows::Win32::System::Diagnostics::ToolHelp::TH32CS_SNAPPROCESS,
                    0,
                )?,
            )?
        };
        let mut slot = MaybeUninit::<PROCESSENTRY32>::uninit();
        unsafe {
            (&raw mut (*slot.as_mut_ptr()).dwSize)
                .write(size_of::<PROCESSENTRY32>().try_into().unwrap());
        }
        unsafe {
            windows::Win32::System::Diagnostics::ToolHelp::Process32First(
                snapshot.as_raw(),
                slot.as_mut_ptr(),
            )
        }
        .map_err(|e| anyhow!("{e:?}"))?;

        let mut issued_shutdown = false;
        loop {
            let proc = unsafe { slot.assume_init_ref() };
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
                let proc = unsafe {
                    Handle::new(windows::Win32::System::Threading::OpenProcess(
                        windows::Win32::System::Threading::PROCESS_SYNCHRONIZE,
                        false,
                        proc.th32ProcessID,
                    )?)?
                };
                let event = unsafe {
                    windows::Win32::System::Threading::WaitForSingleObject(
                        proc.as_raw(),
                        windows::Win32::System::Threading::INFINITE,
                    )
                };
                match event {
                    windows::Win32::Foundation::WAIT_OBJECT_0 => {}
                    windows::Win32::Foundation::WAIT_FAILED => unsafe {
                        bail!("{:?}", windows::Win32::Foundation::GetLastError())
                    },
                    _ => bail!("Unexpected WAIT_EVENT: {event:?}"),
                }
            }
            if unsafe {
                windows::Win32::System::Diagnostics::ToolHelp::Process32Next(
                    snapshot.as_raw(),
                    slot.as_mut_ptr(),
                )
            }
            .is_err()
            {
                break;
            }
        }
    }
    #[cfg(not(windows))]
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
        #[cfg(target_os = "macos")]
        {
            use std::process::Stdio;
            use std::time::Duration;

            for pid in output.lines() {
                info!(log, "Waiting for Steam process {pid} to shut down");
                // could use https://man.freebsd.org/cgi/man.cgi?query=kvm_getprocs
                while tokio::process::Command::new("ps")
                    .args(["-p", pid])
                    .stdout(Stdio::null())
                    .status()
                    .await?
                    .success()
                {
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }
            }
        }
        #[cfg(target_os = "linux")]
        {
            for pid in output.lines() {
                let pid = pid.parse()?;
                let pidfd = match unsafe { nc::pidfd_open(pid, 0) } {
                    Ok(t) => t,
                    Err(nc::ESRCH) => {
                        info!(log, "Steam process {pid} has already shut down");
                        continue;
                    }
                    Err(errno) => bail!("pidfd_open errno={errno}"),
                };

                info!(log, "Waiting for Steam process {pid} to shut down");

                drop_guard::defer(|| {
                    _ = unsafe { libc::close(pidfd) };
                });
                let mut pollfd = libc::pollfd {
                    fd: pidfd,
                    events: libc::POLLIN,
                    revents: 0,
                };
                loop {
                    let code = unsafe { libc::poll(&mut pollfd, 1, -1) };
                    if code != 0 {
                        bail!("Return code {code} from poll");
                    }
                    if pollfd.revents & libc::POLLIN != 0 {
                        break;
                    }
                }
            }
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            bail!("Not implemented for this platform")
        }
    }
    Ok(())
}

pub fn generate_launch_options() -> Result<String> {
    let bin = std::env::current_exe()
        .unwrap()
        .into_os_string()
        .into_string()
        .map_err(|s| anyhow!("Non-Unicode executable name: {s:?}"))?;
    Ok(format!("{bin:?} wrap %command%"))
}

pub async fn ensure_launch_args_are_applied(
    log: &slog::Logger,
    mut comms: Option<Spc<'_>>,
    game_id: &str,
) -> Result<(), crate::Error> {
    loop {
        let result = apply_launch_args(game_id, true, true).await?;
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
                .acquire_recv()
                .await?
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
                                    serde_json::Value::from(generate_launch_options()?),
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
    overwrite_ok: bool,
    dry_run: bool,
) -> Result<AppliedLaunchArgs> {
    let mut path = paths::resolve_steam_directory().await?;
    path.push("userdata");

    let mut result = AppliedLaunchArgs::Unchanged;

    let launch_options_str = generate_launch_options()?;

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
                let result = apply_launch_args_inner(
                    game_id,
                    overwrite_ok,
                    &launch_options_str,
                    rdr,
                    &mut *wtr,
                )?;
                wtr.flush()?;
                result
            } else {
                apply_launch_args_inner(
                    game_id,
                    overwrite_ok,
                    &launch_options_str,
                    rdr,
                    std::io::empty(),
                )?
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
    launch_options_str: &str,
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
                        value: if value.s != launch_options_str.as_bytes() {
                            if !value.s.is_empty() && !overwrite_ok {
                                bail!("Refusing to overwrite launch options.");
                            }
                            flag = Flag::ModifiedLaunchOptions {
                                overwrote: !value.s.is_empty(),
                            };
                            vdf::Str {
                                s: launch_options_str.as_bytes(),
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
                                            s: launch_options_str.as_bytes(),
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
