use std::{collections::HashMap, ffi::OsString, sync::OnceLock};

use anyhow::{anyhow, bail, Context as _};
use ipc_channel::ipc::IpcSender;
use parking_lot::Mutex;
use tokio::{io::AsyncBufReadExt as _, process::Command};
use uuid::Uuid;

use crate::ipc::{C2SMessage, OutputLine};

pub async fn run(args: impl Iterator<Item = OsString>) -> anyhow::Result<()> {
    async fn inner1(mut args: impl Iterator<Item = OsString>) -> anyhow::Result<()> {
        let command = args.next().context("Missing required argument BINARY")?;
        let mut args = args.collect::<Vec<_>>();
        let i = args.iter().rposition(|s| s == ";");

        let mut command_args = Vec::new();
        if let Some(i) = i {
            args.remove(i);
            command_args.extend(args.drain(0..i));
        } else {
            command_args.append(&mut args);
        }

        let c2s_tx = if args.first().map(|s| s == "--c2s-tx").unwrap_or(false) {
            args.remove(0);
            Some(IpcSender::<C2SMessage>::connect(
                args.remove(0)
                    .into_string()
                    .map_err(|e| anyhow!("Invalid value for option --c2s-tx: {e:?}"))?,
            )?)
        } else {
            None
        };

        if let Some(c2s_tx) = &c2s_tx {
            struct Logger {
                c2s_tx: Mutex<IpcSender<C2SMessage>>,
            }

            impl log::Log for Logger {
                fn enabled(&self, metadata: &log::Metadata) -> bool {
                    metadata.level() < log::Level::Trace
                }

                fn log(&self, record: &log::Record) {
                    if self.enabled(record.metadata()) {
                        _ = self.c2s_tx.lock().send(C2SMessage::Log {
                            level: record.level(),
                            message: record.args().to_string(),
                        });
                    }
                }

                fn flush(&self) {}
            }
            static LOGGER: OnceLock<Logger> = OnceLock::new();
            if LOGGER
                .set(Logger {
                    c2s_tx: Mutex::new(c2s_tx.clone()),
                })
                .is_err()
            {
                bail!("LOGGER already set");
            }
            log::set_max_level(log::LevelFilter::Trace);
            log::set_logger(LOGGER.get().unwrap())?;
        }

        if let Err(e) = inner(args, command, command_args, c2s_tx.as_ref()).await {
            if let Some(c2s_tx) = c2s_tx {
                _ = c2s_tx.send(C2SMessage::Crash {
                    error: e.to_string(),
                });
            }
            Err(e)
        } else {
            Ok(())
        }
    }

    async fn inner(
        args: Vec<OsString>,
        command: OsString,
        mut command_args: Vec<OsString>,
        c2s_tx: Option<&IpcSender<C2SMessage>>,
    ) -> anyhow::Result<()> {
        use lexopt::Arg::*;

        let mut args = lexopt::Parser::from_args(args.into_iter());

        let mut profile = None::<Uuid>;
        let mut loader = None::<OsString>;

        while let Some(arg) = args.next()? {
            match arg {
                Long("c2s-tx") => {
                    bail!("--c2s-tx must be the first argument to the wrapper");
                }
                Long("profile") => {
                    if profile.is_some() {
                        bail!("--profile specified twice");
                    }
                    profile = Some(
                        args.value()?
                            .into_string()
                            .map_err(|s| anyhow!("Invalid profile id: {s:?}"))?
                            .parse::<Uuid>()?,
                    );
                }
                Long("loader") => {
                    if loader.is_some() {
                        bail!("--loader specified twice");
                    }
                    loader = Some(args.value()?);
                }
                _ => return Err(arg.unexpected().into()),
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

        if let Some(c2s_tx) = &c2s_tx {
            _ = c2s_tx.send(C2SMessage::Start {
                command: command.clone().into(),
                args: command_args
                    .iter()
                    .cloned()
                    .map(From::from)
                    .collect::<Vec<_>>(),
                env: env
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone().into()))
                    .collect::<HashMap<_, _>>(),
            });
        }

        let mut command = Command::new(command);
        command.envs(env.into_iter()).args(command_args);
        if c2s_tx.is_some() {
            command.stdout(std::process::Stdio::piped());
            command.stderr(std::process::Stdio::piped());
        }
        let mut child = command.spawn()?;

        let tasks = if let Some(c2s_tx) = &c2s_tx {
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
                        let line = String::from_utf8(std::mem::take(&mut buf))
                            .map(|s| OutputLine::Unicode(s))
                            .unwrap_or_else(|e| OutputLine::Bytes(e.into_bytes()));
                        _ = c2s_tx.send(C2SMessage::Output { channel, line });
                    }
                })
            }
            Some((
                spawn_task(
                    c2s_tx,
                    child.stdout.take().unwrap(),
                    crate::ipc::StandardOutputChannel::Out,
                ),
                spawn_task(
                    c2s_tx,
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

        if let Some(c2s_tx) = &c2s_tx {
            _ = c2s_tx.send(C2SMessage::Exit {
                code: status.code(),
            });
        }
        Ok(())
    }

    match inner1(args).await {
        Ok(()) => Ok(()),
        Err(e) => {
            tokio::fs::write("/tmp/manderrow-wrap-crash", e.to_string())
                .await
                .unwrap();
            Err(e)
        }
    }
}
