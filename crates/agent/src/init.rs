#![deny(unused_must_use)]

use std::ffi::OsString;
use std::num::NonZeroU32;
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use manderrow_ipc::client::Ipc;
use manderrow_ipc::ipc_channel::ipc::{IpcOneShotServer, IpcSender};
use manderrow_ipc::{C2SMessage, S2CMessage};
use slog::{info, o};

static IPC: OnceLock<Ipc> = OnceLock::new();

pub fn ipc() -> Option<&'static Ipc> {
    IPC.get()
}

/// An injection instruction.
pub enum Instruction {
    LoadLibrary { path: PathBuf },
    SetVar { kv: OsString, eq_sign: usize },
    PrependArg { arg: OsString },
    AppendArg { arg: OsString },
}

pub struct Args {
    pub instructions: Vec<Instruction>,

    pub remaining: Vec<OsString>,

    _logger_guard: slog_scope::GlobalLoggerGuard,
}

pub enum MaybeArgs {
    Enabled(Args),
    Disabled(Vec<OsString>),
}

#[derive(Debug, thiserror::Error)]
pub enum InitError {
    #[error(transparent)]
    Args(#[from] manderrow_args::Error),

    #[error("Invalid value for option --{name}: {value:?}")]
    InvalidStringOption { name: &'static str, value: OsString },
    #[error(transparent)]
    Lexopt(#[from] lexopt::Error),

    #[error("Failed to connect to c2s channel: {0}")]
    ConnectC2SError(std::io::Error),
    #[error("Failed to create s2c channel: {0}")]
    CreateS2CError(std::io::Error),
    #[error("Failed to send connect message on c2s channel: {0}")]
    SendConnectError(manderrow_ipc::bincode::Error),
    #[error("Failed to receive connect message on s2c channel: {0}")]
    RecvConnectError(manderrow_ipc::bincode::Error),
    #[error("Invalid connection message received on s2c channel: {0:?}")]
    InvalidRecvConnectMessage(S2CMessage),
    #[error("Failed to set global logger")]
    SetGlobalLogger,
    #[error("Invalid pid: {0}")]
    InvalidPid(u32),

    #[error("Invalid key-value pair to --inject-env: Contains NUL byte")]
    InvalidEnvKVContainsNul,
    #[error("Invalid key-value pair to --inject-env: Missing '='")]
    InvalidEnvKVMissingEq,

    #[error("Global IPC is already set")]
    IpcAlreadySet,
}

/// Parses arguments and sets up IPC connection.
///
/// A `None` return value indicates that the Manderrow agent is disabled.
pub fn init(args: impl IntoIterator<Item = OsString>) -> Result<MaybeArgs, InitError> {
    let (args, remaining_args) = manderrow_args::extract(args)?;

    // get IPC connected ASAP and detect whether we're even enabled.
    let mut enabled = false;
    {
        use lexopt::Arg::*;
        let mut parsed_args = lexopt::Parser::from_args(args.iter().cloned());
        while let Some(arg) = parsed_args.next()? {
            match arg {
                Long("enable") => {
                    enabled = true;
                }
                // handled later
                _ => {}
            }
        }
    }
    if !enabled {
        return Ok(MaybeArgs::Disabled(remaining_args));
    }

    {
        use lexopt::Arg::*;
        let mut parsed_args = lexopt::Parser::from_args(args.iter().cloned());
        while let Some(arg) = parsed_args.next()? {
            match arg {
                Long("c2s-tx") => {
                    let c2s_tx = parse_string_value(&mut parsed_args, "c2s-tx")?;
                    let c2s_tx = IpcSender::<C2SMessage>::connect(c2s_tx)
                        .map_err(InitError::ConnectC2SError)?;

                    let (s2c_rx, s2c_tx) =
                        IpcOneShotServer::<S2CMessage>::new().map_err(InitError::CreateS2CError)?;
                    let pid = std::process::id();
                    c2s_tx
                        .send(C2SMessage::Connect {
                            s2c_tx,
                            pid: NonZeroU32::new(pid).ok_or(InitError::InvalidPid(pid))?,
                        })
                        .map_err(InitError::SendConnectError)?;
                    let (s2c_rx, msg) = s2c_rx.accept().map_err(InitError::RecvConnectError)?;
                    if !matches!(msg, S2CMessage::Connect) {
                        return Err(InitError::InvalidRecvConnectMessage(msg));
                    }

                    IPC.set(Ipc {
                        c2s_tx: c2s_tx.into(),
                        s2c_rx: s2c_rx.into(),
                    })
                    .map_err(|_| InitError::IpcAlreadySet)?;
                }
                // handled later
                _ => {}
            }
        }
    }

    // then hook up logging
    let logger_guard = if let Some(ipc) = ipc() {
        struct Logger<T> {
            c2s_tx: T,
        }

        impl<T: Deref<Target = Mutex<IpcSender<C2SMessage>>>> slog::Drain for Logger<T> {
            type Ok = ();

            type Err = slog::Never;

            fn log(
                &self,
                record: &slog::Record<'_>,
                _values: &slog::OwnedKVList,
            ) -> Result<Self::Ok, Self::Err> {
                if let Ok(lock) = self.c2s_tx.lock() {
                    _ = lock.send(C2SMessage::Log {
                        level: record.level().into(),
                        scope: "manderrow_agent".into(),
                        message: record.msg().to_string(),
                    });
                }
                Ok(())
            }
        }
        slog_scope::set_global_logger(slog::Logger::root(
            Logger {
                c2s_tx: &ipc.c2s_tx,
            },
            o!(),
        ))
    } else {
        slog_envlogger::init().map_err(|_| InitError::SetGlobalLogger)?
    };

    let log = slog_scope::logger();

    info!(log, "Agent started");
    info!(log, "{}", crate::crash::DumpEnvironment);

    use lexopt::Arg::*;

    // finally parse args for real
    let mut parsed_args = lexopt::Parser::from_args(args.iter().cloned());

    let mut instructions = Vec::new();

    while let Some(arg) = parsed_args.next()? {
        match arg {
            Long("enable") => {
                // already handled
            }
            Long("c2s-tx") => {
                // already handled
                _ = parsed_args.value()?;
            }
            Long("insn-load-library") => {
                instructions.push(Instruction::LoadLibrary {
                    path: parsed_args.value()?.into(),
                });
            }
            Long("insn-set-var") => {
                let kv = parsed_args.value()?;
                if kv.as_encoded_bytes().contains(&0) {
                    return Err(InitError::InvalidEnvKVContainsNul);
                }
                let i = kv
                    .as_encoded_bytes()
                    .iter()
                    .position(|b| *b == b'=')
                    .ok_or_else(|| InitError::InvalidEnvKVMissingEq)?;
                instructions.push(Instruction::SetVar { kv: kv, eq_sign: i });
            }
            Long("insn-prepend-arg") => {
                let arg = parsed_args.value()?;
                instructions.push(Instruction::PrependArg { arg });
            }
            Long("insn-append-arg") => {
                let arg = parsed_args.value()?;
                instructions.push(Instruction::AppendArg { arg });
            }
            Long("agent-path") => {
                // arg for wrapper. ignore.
                _ = parsed_args.value()?;
            }
            _ => return Err(arg.unexpected().into()),
        }
    }

    Ok(MaybeArgs::Enabled(Args {
        instructions,
        remaining: remaining_args,
        _logger_guard: logger_guard,
    }))
}

fn parse_string_value(
    parsed_args: &mut lexopt::Parser,
    name: &'static str,
) -> Result<String, InitError> {
    parsed_args
        .value()?
        .into_string()
        .map_err(|s| InitError::InvalidStringOption { name, value: s })
}
