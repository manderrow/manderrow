use std::collections::HashMap;
use std::ffi::OsString;
use std::panic::AssertUnwindSafe;

use anyhow::{anyhow, bail, ensure, Context as _, Result};
use ipc_channel::ipc::IpcSender;
use parking_lot::Mutex;
use slog::o;
use slog::{debug, info};
use tokio::{io::AsyncBufReadExt as _, process::Command};
use uuid::Uuid;

use crate::ipc::{C2SMessage, Ipc, OutputLine, S2CMessage};

async fn send_ipc(
    log: &slog::Logger,
    ipc: &mut Option<&mut Ipc>,
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

pub async fn run(args: impl Iterator<Item = OsString>) -> Result<()> {
    async fn inner1(mut args: impl Iterator<Item = OsString>) -> Result<()> {
        let command = args.next().context("Missing required argument BINARY")?;
        let mut command_args = args.collect::<Vec<_>>();

        let mut args = Vec::new();
        if let Some(j) = command_args.iter().rposition(|s| s == ";") {
            if let Some(i) = command_args[..j].iter().rposition(|s| s == ";") {
                let range = i..(j + 1);
                // exclude the two ";" delimiters
                let len = range.len() - 2;
                args.extend(command_args.drain(range).skip(1).take(len));
            } else {
                bail!("Only found one argument delimiter");
            }
        }

        let mut ipc = if args.first().map(|s| s == "--c2s-tx").unwrap_or(false) {
            args.remove(0);
            let c2s_tx = IpcSender::<C2SMessage>::connect(
                args.remove(0)
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

            Some(Ipc { c2s_tx, s2c_rx })
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

        if let Err(e) = inner(args, command, command_args, &log, ipc.as_mut()).await {
            send_ipc(&log, &mut ipc.as_mut(), || {
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
        mut ipc: Option<&mut Ipc>,
    ) -> Result<()> {
        use lexopt::Arg::*;

        let mut parsed_args = lexopt::Parser::from_args(args.iter().cloned());

        let mut profile = None::<Uuid>;
        let mut loader = None::<OsString>;
        let mut wrapper_stage2_path = None::<OsString>;

        while let Some(arg) = parsed_args.next()? {
            match arg {
                Long("c2s-tx") => {
                    bail!("--c2s-tx must be the first argument to the wrapper");
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
                Long("loader") => {
                    if loader.is_some() {
                        bail!("--loader specified twice");
                    }
                    loader = Some(parsed_args.value()?);
                }
                Long("wrapper-stage2") => {
                    if wrapper_stage2_path.is_some() {
                        bail!("--wrapper-stage2 specified twice");
                    }
                    wrapper_stage2_path = Some(parsed_args.value()?.into());
                }
                _ => {
                    return Err(anyhow::Error::from(arg.unexpected())
                        .context(format!("Failed to parse arguments {args:?}")))
                }
            }
        }

        let profile = profile.context("Missing required option --profile")?;

        let mut env = HashMap::default();
        match loader {
            Some(s) if s == "BepInEx" => {
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
                }
                crate::launching::bep_in_ex::configure_command(
                    &log,
                    &mut CommandBuilder {
                        env: &mut env,
                        args: &mut command_args,
                    },
                    profile,
                )
                .await?;
            }
            Some(name) => bail!("Unsupported loader {name:?} for wrap command"),
            None => {}
        }

        let mut command = Command::new(&command_name);

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
                    Some(t) => t,
                    None => std::env::current_exe()?
                        .with_file_name("manderrow-wrapper-stage2")
                        .into_os_string(),
                },
            );

            command.env("MANDERROW_WRAPPER_ENV", serde_json::to_string(&env)?);
        } else {
            debug!(log, "Injecting environment directly");
            command.envs(&env);
        }

        send_ipc(log, &mut ipc, || {
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

        let tasks = if let Some(ref mut ipc) = ipc {
            fn spawn_task(
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
                spawn_task(
                    &ipc.c2s_tx,
                    child.stdout.take().unwrap(),
                    crate::ipc::StandardOutputChannel::Out,
                ),
                spawn_task(
                    &ipc.c2s_tx,
                    child.stderr.take().unwrap(),
                    crate::ipc::StandardOutputChannel::Err,
                ),
            ))
        } else {
            None
        };

        let status = child.wait().await?;
        if let Some((out, err)) = tasks {
            out.await??;
            err.await??;
        }

        send_ipc(log, &mut ipc, || {
            Ok(C2SMessage::Exit {
                code: status.code(),
            })
        })
        .await?;
        Ok(())
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
