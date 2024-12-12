#![deny(unused_must_use)]

mod commands;
mod game_reviews;
mod games;
mod ipc;
mod launching;
mod mods;
mod paths;
mod window_state;

use std::collections::HashMap;
use std::ffi::OsString;
use std::sync::OnceLock;

use anyhow::{anyhow, bail, Context};
use ipc::{C2SMessage, OutputLine};
use ipc_channel::ipc::IpcSender;
use log::error;
use parking_lot::Mutex;
use tokio::io::AsyncBufReadExt as _;
use tokio::process::Command;
use uuid::Uuid;

static PRODUCT_NAME: OnceLock<String> = OnceLock::new();
static IDENTIFIER: OnceLock<String> = OnceLock::new();

fn product_name() -> &'static str {
    PRODUCT_NAME.get().unwrap()
}

fn identifier() -> &'static str {
    IDENTIFIER.get().unwrap()
}

#[derive(Debug, Clone, serde::Serialize)]
struct Error {
    message: String,
    backtrace: String,
}

impl<T: std::fmt::Display> From<T> for Error {
    #[track_caller]
    fn from(value: T) -> Self {
        let backtrace = std::backtrace::Backtrace::force_capture();
        error!("{value}\nBacktrace:\n{backtrace}");
        Self {
            message: value.to_string(),
            backtrace: backtrace.to_string(),
        }
    }
}

fn run_app(ctx: tauri::Context<tauri::Wry>) -> anyhow::Result<()> {
    let level_filter = std::env::var("RUST_LOG")
        .map(|s| {
            s.parse::<log::LevelFilter>()
                .expect("Invalid logging configuration")
        })
        .unwrap_or(log::LevelFilter::Info);
    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_http::init())
        .plugin(
            tauri_plugin_log::Builder::new()
                .filter(move |metadata| {
                    metadata.level() <= level_filter
                        && (metadata.level() < log::Level::Trace
                            || (cfg!(debug_assertions) && metadata.target() == "manderrow"))
                })
                .build(),
        )
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(window_state::init())
        .invoke_handler(tauri::generate_handler![
            commands::close_splashscreen::close_splashscreen,
            commands::games::get_games,
            commands::games::get_games_popularity,
            commands::i18n::get_preferred_locales,
            commands::mod_index::fetch_mod_index,
            commands::mod_index::query_mod_index,
            commands::profiles::get_profiles,
            commands::profiles::create_profile,
            commands::profiles::delete_profile,
            commands::profiles::launch_profile,
        ])
        .run(ctx)
        .context("error while running tauri application")
}

async fn run_wrap(args: impl Iterator<Item = OsString>) -> anyhow::Result<()> {
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
            log::set_logger(LOGGER.get().unwrap()).unwrap();
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
                impl<'a> launching::bep_in_ex::CommandBuilder for CommandBuilder<'a> {
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
                launching::bep_in_ex::configure_command(
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn main() -> anyhow::Result<()> {
    let ctx = tauri::generate_context!();
    PRODUCT_NAME
        .set(ctx.config().product_name.clone().unwrap())
        .unwrap();
    IDENTIFIER.set(ctx.config().identifier.clone()).unwrap();

    paths::init().unwrap();

    let mut args = std::env::args_os();
    _ = args.next().unwrap();

    match args.next() {
        Some(cmd) if cmd == "wrap" => tauri::async_runtime::block_on(async move {
            if let Err(e) = run_wrap(args).await {
                tokio::fs::write("/tmp/manderrow-wrap-crash", &format!("{e:?}")).await?;
            }
            Ok(())
        }),
        Some(cmd) => Err(anyhow!("Unrecognized command {cmd:?}")),
        None => run_app(ctx),
    }
}
