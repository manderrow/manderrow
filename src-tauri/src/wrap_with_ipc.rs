use std::io::{BufRead, Write};
use std::ops::ControlFlow;
use std::panic::AssertUnwindSafe;
use std::process::Command;
use std::{ffi::OsString, num::NonZeroU32};

use anyhow::{ensure, Context as _, Result};
use lexopt::ValueExt;
use manderrow_ipc::client::Ipc;
use manderrow_ipc::ipc_channel::ipc::IpcSender;
use manderrow_ipc::{LogLevel, OutputLine, S2CMessage};
use slog::o;
use triomphe::Arc;

use crate::ipc::C2SMessage;

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

// TODO: I'm not convinced it makes sense for this to all be written in Rust.
//       Sure, it's nice to be able to ship it in a single binary with the app
//       and the other wrapper, but the downside is that we're duplicating a
//       lot of code from the agent and have to settle for some of Rust's poor
//       error handling choices.
pub fn run(args: lexopt::Parser) -> Result<()> {
    std::panic::set_backtrace_style(std::panic::BacktraceStyle::Full);
    std::panic::set_hook(Box::new(|info| {
        _ = std::fs::write(
            "manderrow-wrap-crash.txt",
            format!(
                "{}\nargs: {}",
                if let Some(&s) = info.payload().downcast_ref::<&'static str>() {
                    s
                } else if let Some(s) = info.payload().downcast_ref::<String>() {
                    s.as_str()
                } else {
                    "Box<dyn Any>"
                },
                DisplayArgList
            ),
        );
    }));

    std::fs::write("manderrow-wrap-args.txt", DisplayArgList.to_string()).unwrap();

    fn inner1(mut args: lexopt::Parser) -> Result<()> {
        use lexopt::Arg::*;

        let command = match args.next()?.context("Missing required argument BINARY")? {
            Value(s) => s,
            arg => return Err(arg.unexpected().into()),
        };

        let args = args.raw_args()?.collect::<Vec<_>>();

        // TODO: avoid cloning so much. Not just here. All over dealing with arguments.
        let (manderrow_args, _) = manderrow_args::extract(args.iter().cloned())?;

        let mut log_file = std::fs::File::create("manderrow-wrap.log").unwrap();

        let mut manderrow_args = lexopt::Parser::from_args(manderrow_args);

        let mut c2s_tx = None::<String>;

        while let Some(arg) = manderrow_args.next()? {
            match arg {
                lexopt::Arg::Long("c2s-tx") => {
                    c2s_tx = Some(manderrow_args.value()?.parse()?);
                }
                _ => {}
            }
        }

        writeln!(log_file, "--c2s-tx: {:?}", c2s_tx).unwrap();
        writeln!(
            log_file,
            "Args: {:?}",
            std::env::args_os().collect::<Vec<_>>()
        )
        .unwrap();
        writeln!(
            log_file,
            "Env: {:?}",
            std::env::vars_os().collect::<Vec<_>>()
        )
        .unwrap();

        let ipc = if let Some(c2s_tx) = c2s_tx {
            let c2s_tx = IpcSender::<C2SMessage>::connect(c2s_tx)?;

            let (s2c_rx, s2c_tx) =
                manderrow_ipc::ipc_channel::ipc::IpcOneShotServer::<S2CMessage>::new()?;
            c2s_tx.send(&C2SMessage::Connect { s2c_tx })?;
            let (s2c_rx, msg) = s2c_rx.accept()?;
            ensure!(
                matches!(msg, S2CMessage::Connect),
                "Unexpected initial message"
            );

            Some(Arc::new(Ipc::new(c2s_tx, s2c_rx)))
        } else {
            None
        };

        let _guard = if let Some(ipc) = &ipc {
            struct Logger {
                ipc: AssertUnwindSafe<Arc<Ipc>>,
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
                        self.ipc.send(&C2SMessage::Log {
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
                    ipc: AssertUnwindSafe(ipc.clone()),
                },
                o!(),
            ))
        } else {
            slog_envlogger::init()?
        };

        let log = slog_scope::logger();

        if let Err(e) = inner(args, command, ipc.as_ref()) {
            if let Some(ref ipc) = ipc {
                ipc.send(&C2SMessage::Crash {
                    error: format!("{e:?}"),
                })?;
            }
            Err(e)
        } else {
            Ok(())
        }
    }

    fn inner(args: Vec<OsString>, command_name: OsString, ipc: Option<&Arc<Ipc>>) -> Result<()> {
        let mut command = Command::new(&command_name);
        command.args(args);

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

        if let Some(ipc) = ipc {
            ipc.send(&C2SMessage::Started {
                pid: NonZeroU32::new(child.id()).expect("0 is not a valid pid"),
            })?;
        }

        fn spawn_output_pipe_task<const TRY_PARSE_LOGS: bool>(
            ipc: &Arc<Ipc>,
            rdr: impl std::io::Read + Send + 'static,
            channel: crate::ipc::StandardOutputChannel,
        ) -> std::io::Result<()> {
            let ipc = ipc.clone();
            std::thread::Builder::new()
                .name(format!("std{}-ipc", channel.name()))
                .spawn(move || {
                    let mut rdr = std::io::BufReader::new(rdr);
                    let mut buf = Vec::new();
                    loop {
                        if let Err(_) = rdr.read_until(b'\n', &mut buf) {
                            // TODO: log or something
                            return;
                        }
                        if buf.is_empty() {
                            break;
                        }
                        if matches!(buf.last(), Some(b'\n')) {
                            buf.pop();
                            if matches!(buf.last(), Some(b'\r')) {
                                buf.pop();
                            }
                        }
                        if TRY_PARSE_LOGS {
                            if let ControlFlow::Break(()) = try_handle_log_record(&ipc, &buf) {
                                buf.clear();
                                continue;
                            }
                        }
                        let line = OutputLine::new(std::mem::take(&mut buf));
                        _ = ipc.send(&C2SMessage::Output { channel, line });
                    }
                })?;
            Ok(())
        }
        if let Some(ipc) = ipc {
            spawn_output_pipe_task::<false>(
                ipc,
                child.stdout.take().unwrap(),
                crate::ipc::StandardOutputChannel::Out,
            )?;
            spawn_output_pipe_task::<true>(
                &ipc,
                child.stderr.take().unwrap(),
                crate::ipc::StandardOutputChannel::Err,
            )?;
        }

        let status = child.wait()?;

        status.exit_ok()?;

        Ok(())
    }

    match inner1(args) {
        Ok(()) => Ok(()),
        Err(e) => {
            std::fs::write(
                "manderrow-wrap-crash.txt",
                format!("{e}\nargs: {}", DisplayArgList),
            )
            .unwrap();
            Err(e)
        }
    }
}

fn try_handle_log_record(ipc: &Ipc, buf: &[u8]) -> ControlFlow<()> {
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
                            _ = ipc.send(&C2SMessage::Log {
                                level,
                                scope: scope.into(),
                                message: msg.to_owned(),
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
