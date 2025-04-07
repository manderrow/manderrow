#![deny(unused_must_use)]

use std::ffi::OsString;
use std::ops::Deref;
use std::sync::{Mutex, OnceLock};

use manderrow_ipc::ipc_channel::ipc::{IpcOneShotServer, IpcSender};
use manderrow_ipc::{C2SMessage, Ipc, S2CMessage};
use manderrow_types::agent::Instruction;
use slog::{info, o};
use uuid::Uuid;

static IPC: OnceLock<Ipc> = OnceLock::new();

pub fn ipc() -> Option<&'static Ipc> {
    IPC.get()
}

pub struct Args {
    pub game: String,
    pub profile: Option<Uuid>,
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

    #[error("Missing required option --{name}")]
    MissingRequiredOption { name: &'static str },
    #[error("Duplicate option --{name}")]
    DuplicateOption { name: &'static str },
    #[error("Invalid value for option --{name}: {value:?}")]
    InvalidStringOption { name: &'static str, value: OsString },
    #[error("Invalid value for option --{name}: {value:?}, {error}")]
    InvalidUuidOption {
        name: &'static str,
        value: OsString,
        error: uuid::Error,
    },
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
    #[error("Invalid key-value pair to --inject-env: Contains NUL byte")]
    InvalidEnvKVContainsNul,
    #[error("Invalid key-value pair to --inject-env: Missing '='")]
    InvalidEnvKVMissingEq,

    #[error("Global IPC is already set")]
    IpcAlreadySet,
}

/// A `None` return value indicates that the Manderrow agent or wrapper is disabled.
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
                Long("c2s-tx") => {
                    let c2s_tx = IpcSender::<C2SMessage>::connect(
                        parsed_args.value()?.into_string().map_err(|s| {
                            InitError::InvalidStringOption {
                                name: "c2s-tx",
                                value: s,
                            }
                        })?,
                    )
                    .map_err(InitError::ConnectC2SError)?;

                    let (s2c_rx, s2c_tx) =
                        IpcOneShotServer::<S2CMessage>::new().map_err(InitError::CreateS2CError)?;
                    c2s_tx
                        .send(C2SMessage::Connect { s2c_tx })
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
    if !enabled {
        return Ok(MaybeArgs::Disabled(remaining_args));
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

    info!(log, "Wrapper started");
    info!(log, "{}", crate::crash::DumpEnvironment);

    use lexopt::Arg::*;

    // finally parse args for real
    let mut parsed_args = lexopt::Parser::from_args(args.iter().cloned());

    let mut game = None::<String>;
    let mut profile = None::<Uuid>;
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
            Long("profile") => {
                check_duplicate_option("profile", &profile)?;
                profile = Some(parse_uuid_value(&mut parsed_args, "profile")?);
            }
            Long("game") => {
                check_duplicate_option("game", &game)?;
                game = Some(parse_string_value(&mut parsed_args, "game")?);
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

    let game = game.ok_or(InitError::MissingRequiredOption { name: "game" })?;

    Ok(MaybeArgs::Enabled(Args {
        game,
        profile,
        instructions,
        remaining: remaining_args,
        _logger_guard: logger_guard,
    }))
}

fn check_duplicate_option<T>(name: &'static str, value: &Option<T>) -> Result<(), InitError> {
    if value.is_some() {
        return Err(InitError::DuplicateOption { name });
    } else {
        Ok(())
    }
}

fn check_duplicate_flag(name: &'static str, value: bool) -> Result<(), InitError> {
    if value {
        return Err(InitError::DuplicateOption { name });
    } else {
        Ok(())
    }
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

fn parse_uuid_value(
    parsed_args: &mut lexopt::Parser,
    name: &'static str,
) -> Result<Uuid, InitError> {
    let value = parsed_args.value()?;
    Uuid::try_parse_ascii(value.as_encoded_bytes()).map_err(|e| InitError::InvalidUuidOption {
        name,
        value,
        error: e,
    })
}
