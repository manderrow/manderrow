use std::collections::HashMap;
use std::ffi::OsString;
use std::future::join;
use std::ops::ControlFlow;
use std::panic::AssertUnwindSafe;
use std::path::PathBuf;

use anyhow::{anyhow, bail, ensure, Context as _, Result};
use ipc_channel::ipc::IpcSender;
use lexopt::ValueExt;
use parking_lot::Mutex;
use slog::o;
use slog::{debug, info};
use tokio::select;
use tokio::{io::AsyncBufReadExt as _, process::Command};
use triomphe::Arc;
use uuid::Uuid;

use crate::games::PackageLoader;
use crate::ipc::{C2SMessage, Ipc, LogLevel, OutputLine, S2CMessage};

async fn send_ipc(
    log: &slog::Logger,
    ipc: Option<&Ipc>,
    message: impl FnOnce() -> Result<C2SMessage>,
) -> Result<()> {
    if let Some(ipc) = ipc {
        let msg = message()?;
        ipc.send(msg).await?;
    } else {
        info!(log, "{:?}", message()?);
    }
    Ok(())
}

struct DisplayArgList;
impl std::fmt::Display for DisplayArgList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut iter = std::env::args_os();
        if let Some(arg) = iter.next() {
            write!(f, "{:?}", arg)?;
            for arg in iter {
                write!(f, " {:?}", arg)?;
            }
        }
        Ok(())
    }
}

pub async fn run(args: lexopt::Parser) -> Result<()> {
    async fn inner1(mut args: lexopt::Parser) -> Result<()> {
        use lexopt::Arg::*;

        let command = match args.next()?.context("Missing required argument BINARY")? {
            Value(s) => s,
            arg => return Err(arg.unexpected().into()),
        };
        let mut command_args = args.raw_args()?.collect::<Vec<_>>();

        let mut args = Vec::new();
        while let Some(j) = command_args.iter().rposition(|s| s == ";") {
            if let Some(i) = command_args[..j].iter().rposition(|s| s == ";") {
                let range = i..(j + 1);
                // exclude the two ";" delimiters
                let len = range.len() - 2;
                args.extend(command_args.drain(range).skip(1).take(len));
            } else {
                bail!("Found an odd number of argument delimiters");
            }
        }

        // TODO: use lexopt to parse `args`
        let ipc = if let Some(i) = args.iter().position(|s| s == "--c2s-tx") {
            args.remove(i);
            let c2s_tx = IpcSender::<C2SMessage>::connect(
                args.remove(i)
                    .into_string()
                    .map_err(|e| anyhow!("Invalid value for option --c2s-tx: {e:?}"))?,
            )?;

            let (s2c_rx, s2c_tx) = ipc_channel::ipc::IpcOneShotServer::<S2CMessage>::new()?;
            c2s_tx.send(C2SMessage::Connect { s2c_tx })?;
            let (s2c_rx, msg) = s2c_rx.accept()?;
            ensure!(
                matches!(msg, S2CMessage::Connect),
                "Unexpected initial message"
            );

            Some(Ipc {
                c2s_tx,
                s2c_rx: Arc::new(s2c_rx.into()),
            })
        } else {
            None
        };

        let _guard = if let Some(ipc) = &ipc {
            struct Logger {
                c2s_tx: AssertUnwindSafe<Mutex<IpcSender<C2SMessage>>>,
            }

            impl slog::Drain for Logger {
                type Ok = ();

                type Err = slog::Never;

                fn log(
                    &self,
                    record: &slog::Record<'_>,
                    _values: &slog::OwnedKVList,
                ) -> Result<Self::Ok, Self::Err> {
                    _ = tokio::task::block_in_place(|| {
                        self.c2s_tx.lock().send(C2SMessage::Log {
                            level: record.level().into(),
                            scope: "manderrow_wrap".into(),
                            message: record.msg().to_string(),
                        })
                    });
                    Ok(())
                }
            }
            slog_scope::set_global_logger(slog::Logger::root(
                Logger {
                    c2s_tx: AssertUnwindSafe(Mutex::new(ipc.c2s_tx.clone())),
                },
                o!(),
            ))
        } else {
            slog_envlogger::init()?
        };

        let log = slog_scope::logger();

        info!(log, "Wrapper started");
        info!(log, "  args: {}", DisplayArgList);
        info!(log, "  cwd: {:?}", std::env::current_dir()?);

        if let Err(e) = inner(args, command, command_args, &log, ipc.as_ref()).await {
            send_ipc(&log, ipc.as_ref(), || {
                Ok(C2SMessage::Crash {
                    error: format!("{e:?}"),
                })
            })
            .await?;
            Err(e)
        } else {
            Ok(())
        }
    }

    async fn inner(
        args: Vec<OsString>,
        command_name: OsString,
        mut command_args: Vec<OsString>,
        log: &slog::Logger,
        ipc: Option<&Ipc>,
    ) -> Result<()> {
        use lexopt::Arg::*;

        let mut parsed_args = lexopt::Parser::from_args(args.iter().cloned());

        let mut game = None::<String>;
        let mut profile = None::<Uuid>;
        let mut loader = None::<PackageLoader>;
        let mut wrapper_stage2_path = None::<PathBuf>;
        let mut doorstop_path = None::<PathBuf>;

        while let Some(arg) = parsed_args.next()? {
            match arg {
                Long("c2s-tx") => {
                    // handled already
                }
                Long("profile") => {
                    if profile.is_some() {
                        bail!("--profile specified twice");
                    }
                    profile = Some(
                        parsed_args
                            .value()?
                            .into_string()
                            .map_err(|s| anyhow!("Invalid profile id: {s:?}"))?
                            .parse::<Uuid>()?,
                    );
                }
                Long("game") => {
                    if game.is_some() {
                        bail!("--game specified twice");
                    }
                    game = Some(
                        parsed_args
                            .value()?
                            .into_string()
                            .map_err(|s| anyhow!("Invalid game: {s:?}"))?,
                    );
                }
                Long("loader") => {
                    if loader.is_some() {
                        bail!("--loader specified twice");
                    }
                    let name = parsed_args.value()?;
                    loader = Some(
                        name.parse()
                            .with_context(|| format!("Unrecognized mod loader {name:?}"))?,
                    );
                }
                Long("wrapper-stage2") => {
                    if wrapper_stage2_path.is_some() {
                        bail!("--wrapper-stage2 specified twice");
                    }
                    wrapper_stage2_path = Some(parsed_args.value()?.into());
                }
                Long("doorstop-path") => {
                    if doorstop_path.is_some() {
                        bail!("--doorstop-path specified twice");
                    }
                    doorstop_path = Some(parsed_args.value()?.into());
                }
                _ => {
                    return Err(anyhow::Error::from(arg.unexpected())
                        .context(format!("Failed to parse arguments {args:?}")))
                }
            }
        }

        let game = game.context("Missing required option --game")?;

        if let Some(id) = profile {
            let profile = crate::profiles::read_profile(id).await?;
            if profile.game != game {
                bail!(
                    "Specified profile is for the game {:?}, but you are attempting to launch {:?}",
                    profile.game,
                    game
                );
            }
        }

        let mut env = HashMap::default();
        match (profile, loader) {
            (None, Some(_)) => bail!("Cannot launch modded without a profile"),
            (Some(profile), Some(PackageLoader::BepInEx)) => {
                struct CommandBuilder<'a> {
                    env: &'a mut HashMap<String, OsString>,
                    args: &'a mut Vec<OsString>,
                }
                impl<'a> crate::launching::bep_in_ex::CommandBuilder for CommandBuilder<'a> {
                    fn env(&mut self, key: impl AsRef<str>, value: impl AsRef<std::ffi::OsStr>) {
                        self.env
                            .insert(key.as_ref().to_owned(), value.as_ref().to_owned());
                    }

                    fn args(
                        &mut self,
                        args: impl IntoIterator<Item = impl AsRef<std::ffi::OsStr>>,
                    ) {
                        self.args
                            .extend(args.into_iter().map(|s| s.as_ref().to_owned()))
                    }

                    fn arg(&mut self, arg: impl AsRef<std::ffi::OsStr>) {
                        self.args.push(arg.as_ref().to_owned())
                    }
                }
                crate::launching::bep_in_ex::configure_command(
                    &log,
                    &mut CommandBuilder {
                        env: &mut env,
                        args: &mut command_args,
                    },
                    &game,
                    profile,
                    doorstop_path,
                )
                .await?;
            }
            (Some(_), Some(loader)) => {
                bail!("The mod loader {loader:?} is not yet supported by the wrap command")
            }
            (_, None) => {}
        }

        let mut command = Command::new(&command_name);
        command.kill_on_drop(true);

        if let Some(mut i) = command_args.iter().position(|s| {
            s.to_string_lossy()
                .contains("scout-on-soldier-entry-point-v2")
        }) {
            debug!(
                log,
                "Using the stage2 wrapper to inject environment to final process"
            );
            i += 1; // skip the arg
            i += 1; // skip the next arg (--)
            command_args.insert(
                i,
                match wrapper_stage2_path {
                    Some(t) => t.into(),
                    None => {
                        let mut buf = std::env::current_exe()?;
                        buf.set_file_name("manderrow-wrapper-stage2");
                        buf.into_os_string()
                    }
                },
            );

            command.env("MANDERROW_WRAPPER_ENV", serde_json::to_string(&env)?);
        } else {
            debug!(log, "Injecting environment directly");
            command.envs(&env);
        }

        send_ipc(log, ipc, || {
            Ok(C2SMessage::Start {
                command: command_name.clone().into(),
                args: command_args
                    .iter()
                    .cloned()
                    .map(From::from)
                    .collect::<Vec<_>>(),
                env: env
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone().into()))
                    .collect::<HashMap<_, _>>(),
            })
        })
        .await?;

        command.args(command_args);

        if ipc.is_some() {
            command.stdout(std::process::Stdio::piped());
            command.stderr(std::process::Stdio::piped());
        }
        let mut child = match command.spawn() {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(anyhow::Error::new(e)
                    .context(format!("Could not locate command {command_name:?}")))
            }
            Err(e) => return Err(e.into()),
        };

        let tasks = if let Some(ipc) = ipc {
            fn spawn_output_pipe_task<const TRY_PARSE_LOGS: bool>(
                c2s_tx: &IpcSender<C2SMessage>,
                rdr: impl tokio::io::AsyncRead + Unpin + Send + 'static,
                channel: crate::ipc::StandardOutputChannel,
            ) -> tokio::task::JoinHandle<Result<(), anyhow::Error>> {
                let c2s_tx = c2s_tx.clone();
                tokio::task::spawn(async move {
                    let mut rdr = tokio::io::BufReader::new(rdr);
                    let mut buf = Vec::new();
                    loop {
                        rdr.read_until(b'\n', &mut buf).await?;
                        if buf.is_empty() {
                            break Ok(());
                        }
                        if matches!(buf.last(), Some(b'\n')) {
                            buf.pop();
                            if matches!(buf.last(), Some(b'\r')) {
                                buf.pop();
                            }
                        }
                        if TRY_PARSE_LOGS {
                            if let ControlFlow::Break(()) = try_handle_log_record(&c2s_tx, &buf) {
                                buf.clear();
                                continue;
                            }
                        }
                        let line = OutputLine::new(std::mem::take(&mut buf));
                        let c2s_tx = &c2s_tx;
                        _ = tokio::task::block_in_place(move || {
                            c2s_tx.send(C2SMessage::Output { channel, line })
                        });
                    }
                })
            }
            Some((
                spawn_output_pipe_task::<false>(
                    &ipc.c2s_tx,
                    child.stdout.take().unwrap(),
                    crate::ipc::StandardOutputChannel::Out,
                ),
                spawn_output_pipe_task::<true>(
                    &ipc.c2s_tx,
                    child.stderr.take().unwrap(),
                    crate::ipc::StandardOutputChannel::Err,
                ),
            ))
        } else {
            None
        };

        let wait_fut = child.wait();

        let kill_fut = async {
            if let Some(ipc) = ipc {
                while let Ok(msg) = ipc.recv().await {
                    match msg {
                        S2CMessage::Connect => {}
                        S2CMessage::PatientResponse { .. } => {}
                        S2CMessage::Kill => return,
                    }
                }
            }
            std::future::pending().await
        };

        select! {
            _ = kill_fut => {
                child.kill().await?;
                info!(log, "Killed process");
                Ok(())
            }
            r = wait_fut => {
                let status = r?;

                let tasks = if let Some((out, err)) = tasks {
                    let (out, err) = join!(out, err).await;
                    Some((out, err))
                } else {
                    None
                };

                send_ipc(log, ipc, || {
                    Ok(C2SMessage::Exit {
                        code: status.code(),
                    })
                })
                .await?;

                if let Some((out, err)) = tasks {
                    out??;
                    err??;
                }

                Ok(())
            }
        }
    }

    match inner1(args).await {
        Ok(()) => Ok(()),
        Err(e) => {
            tokio::fs::write(
                "manderrow-wrap-crash.txt",
                format!("{e}\nargs: {}", DisplayArgList),
            )
            .await
            .unwrap();
            Err(e)
        }
    }
}

fn try_handle_log_record(c2s_tx: &IpcSender<C2SMessage>, buf: &[u8]) -> ControlFlow<()> {
    if let Some((level, rem)) = buf.split_once(|b| *b == b' ') {
        if let Some((scope, msg)) = rem.split_once(|b| *b == b' ') {
            let level = match level {
                b"fatal" => Some(LogLevel::Critical),
                b"err" => Some(LogLevel::Error),
                b"warn" => Some(LogLevel::Warning),
                b"msg" | b"info" => Some(LogLevel::Info),
                b"debug" => Some(LogLevel::Debug),
                _ => None,
            };
            if let Some(level) = level {
                if let Ok(scope) = std::str::from_utf8(scope) {
                    if scope.chars().all(|c| c.is_ascii_graphic()) {
                        if let Ok(msg) = std::str::from_utf8(msg) {
                            let c2s_tx = c2s_tx;
                            _ = tokio::task::block_in_place(move || {
                                c2s_tx.send(C2SMessage::Log {
                                    level,
                                    scope: scope.into(),
                                    message: msg.to_owned(),
                                })
                            });
                            return ControlFlow::Break(());
                        }
                    }
                }
            }
        }
    }
    ControlFlow::Continue(())
}
