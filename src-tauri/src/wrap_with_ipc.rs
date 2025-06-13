use std::io::{BufRead, Write};
use std::ops::ControlFlow;
use std::panic::AssertUnwindSafe;
use std::process::Command;
use std::{ffi::OsString, num::NonZeroU32};

use anyhow::{ensure, Result};
use manderrow_ipc::client::Ipc;
use manderrow_ipc::ipc_channel::ipc::IpcSender;
use manderrow_ipc::{LogLevel, OutputLine, S2CMessage};
use slog::o;
use triomphe::Arc;

use crate::ipc::C2SMessage;

pub fn inner1(
    log_file: std::fs::File,
    command_name: OsString,
    args: Vec<OsString>,
    c2s_tx: Option<String>,
) -> Result<()> {
    let ipc = if let Some(c2s_tx) = c2s_tx {
        let c2s_tx = IpcSender::<C2SMessage>::connect(&c2s_tx)?;

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
            log_file: std::sync::Mutex<std::fs::File>,
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
                tokio::task::block_in_place(|| {
                    if let Ok(mut log_file) = self.log_file.lock() {
                        _ = writeln!(
                            &mut *log_file,
                            "{} manderrow_wrap {}",
                            record.level(),
                            record.msg()
                        );
                    }
                    if let Err(e) = self.ipc.send(&C2SMessage::Log {
                        level: record.level().into(),
                        scope: "manderrow_wrap".into(),
                        message: record.msg().to_string(),
                    }) {
                        if let Ok(mut log_file) = self.log_file.lock() {
                            _ = writeln!(
                                &mut *log_file,
                                "error manderrow_wrap failed to send log message over IPC: {e}"
                            );
                        }
                    }
                });
                Ok(())
            }
        }
        slog_scope::set_global_logger(slog::Logger::root(
            Logger {
                log_file: log_file.into(),
                ipc: AssertUnwindSafe(ipc.clone()),
            },
            o!(),
        ))
    } else {
        slog_envlogger::init()?
    };

    let _log = slog_scope::logger();

    if let Err(e) = inner(args, command_name, ipc.as_ref()) {
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
            return Err(
                anyhow::Error::new(e).context(format!("Could not locate command {command_name:?}"))
            )
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
    ) -> std::io::Result<std::thread::JoinHandle<()>> {
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
                    if let Err(e) = ipc.send(&C2SMessage::Output { channel, line }) {
                        slog_scope::error!("failed to send output line over IPC: {e}");
                    }
                }
            })
    }
    let handles = if let Some(ipc) = ipc {
        Some((
            spawn_output_pipe_task::<false>(
                ipc,
                child.stdout.take().unwrap(),
                crate::ipc::StandardOutputChannel::Out,
            )?,
            spawn_output_pipe_task::<true>(
                &ipc,
                child.stderr.take().unwrap(),
                crate::ipc::StandardOutputChannel::Err,
            )?,
        ))
    } else {
        None
    };

    let status = child.wait()?;

    if let Some((a, b)) = handles {
        if let Err(e) = a.join() {
            slog_scope::error!("stdout forwarder panicked: {e:?}");
        }
        if let Err(e) = b.join() {
            slog_scope::error!("stderr forwarder panicked: {e:?}");
        }
    }

    status.exit_ok()?;

    Ok(())
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
