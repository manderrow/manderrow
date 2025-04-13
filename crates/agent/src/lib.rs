#![deny(unused_must_use)]
#![feature(once_cell_try_insert)]
#![feature(panic_backtrace_config)]

// mod crash;

use std::num::NonZeroU32;
use std::ptr::NonNull;
use std::sync::{Once, OnceLock};

use manderrow_ipc::client::Ipc;
use manderrow_ipc::ipc_channel::ipc::{IpcOneShotServer, IpcSender};
use manderrow_ipc::{C2SMessage, OutputLine, S2CMessage};

const DEINIT: Once = Once::new();

unsafe extern "C" {
    fn manderrow_agent_crash(msg_ptr: NonNull<u8>, msg_len: usize) -> !;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn manderrow_agent_init(c2s_tx_ptr: Option<NonNull<u8>>, c2s_tx_len: usize) {
    std::panic::set_backtrace_style(std::panic::BacktraceStyle::Full);
    std::panic::set_hook(Box::new(|info| {
        let msg = if let Some(&s) = info.payload().downcast_ref::<&'static str>() {
            s
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.as_str()
        } else {
            "Box<dyn Any>"
        };
        unsafe { manderrow_agent_crash(NonNull::from(msg).cast(), msg.len()) }
        // crash::report_crash(message)
    }));

    let c2s_tx = match c2s_tx_ptr {
        Some(s) => Some(
            std::str::from_utf8(unsafe { NonNull::slice_from_raw_parts(s, c2s_tx_len).as_ref() })
                .expect("Invalid value for option --c2s-tx"),
        ),
        None => None,
    };

    if let Some(c2s_tx) = c2s_tx {
        connect_ipc(c2s_tx).unwrap();
    }
}

static IPC: OnceLock<Ipc> = OnceLock::new();

fn ipc() -> Option<&'static Ipc> {
    IPC.get()
}

#[derive(Debug, thiserror::Error)]
enum ConnectIpcError {
    #[error("Failed to connect to c2s channel: {0}")]
    ConnectC2SError(std::io::Error),
    #[error("Failed to create s2c channel: {0}")]
    CreateS2CError(std::io::Error),
    #[error("Failed to send connect message on c2s channel: {0}")]
    SendConnectError(#[from] manderrow_ipc::ipc_channel::error::SendError),
    #[error("Failed to receive connect message on s2c channel: {0}")]
    RecvConnectError(#[from] manderrow_ipc::ipc_channel::error::RecvError),
    #[error("Invalid connection message received on s2c channel: {0:?}")]
    InvalidRecvConnectMessage(S2CMessage),
    #[error("Invalid pid: {0}")]
    InvalidPid(u32),

    #[error("Global IPC is already set")]
    IpcAlreadySet,
}

fn connect_ipc(c2s_tx: &str) -> Result<(), ConnectIpcError> {
    let c2s_tx = IpcSender::<C2SMessage>::connect(c2s_tx.to_owned())
        .map_err(ConnectIpcError::ConnectC2SError)?;

    let (s2c_rx, s2c_tx) =
        IpcOneShotServer::<S2CMessage>::new().map_err(ConnectIpcError::CreateS2CError)?;
    let pid = std::process::id();
    c2s_tx.send(C2SMessage::Connect {
        s2c_tx,
        pid: NonZeroU32::new(pid).ok_or(ConnectIpcError::InvalidPid(pid))?,
    })?;
    let (s2c_rx, msg) = s2c_rx.accept()?;
    if !matches!(msg, S2CMessage::Connect) {
        return Err(ConnectIpcError::InvalidRecvConnectMessage(msg));
    }

    IPC.set(Ipc::new(c2s_tx, s2c_rx))
        .map_err(|_| ConnectIpcError::IpcAlreadySet)
}

#[unsafe(no_mangle)]
pub extern "C" fn manderrow_agent_deinit(send_exit: bool) {
    DEINIT.call_once(|| {
        if send_exit {
            if let Some(ipc) = ipc() {
                // TODO: send the correct exit code
                _ = ipc.send(&C2SMessage::Exit { code: None });
            }
        }
    });
}

#[repr(u8)]
pub enum StandardOutputChannel {
    Out,
    Err,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn manderrow_agent_send_output_line(
    channel: StandardOutputChannel,
    line_ptr: NonNull<u8>,
    line_len: usize,
) {
    let line = unsafe { NonNull::slice_from_raw_parts(line_ptr, line_len).as_ref() };
    let line = OutputLine::new(line.to_owned());
    if let Some(ipc) = ipc() {
        _ = ipc.send(&C2SMessage::Output {
            channel: match channel {
                StandardOutputChannel::Out => manderrow_ipc::StandardOutputChannel::Out,
                StandardOutputChannel::Err => manderrow_ipc::StandardOutputChannel::Err,
            },
            line,
        });
    }
}

#[repr(u8)]
pub enum LogLevel {
    Critical,
    Error,
    Warning,
    Info,
    Debug,
    Trace,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn manderrow_agent_send_log(
    level: LogLevel,
    scope_ptr: NonNull<u8>,
    scope_len: usize,
    msg_ptr: NonNull<u8>,
    msg_len: usize,
) {
    let scope = unsafe {
        std::str::from_utf8_unchecked(NonNull::slice_from_raw_parts(scope_ptr, scope_len).as_ref())
    };
    let msg = unsafe {
        std::str::from_utf8_unchecked(NonNull::slice_from_raw_parts(msg_ptr, msg_len).as_ref())
    };
    if let Some(ipc) = ipc() {
        _ = ipc.send(&C2SMessage::Log {
            level: match level {
                LogLevel::Critical => manderrow_ipc::LogLevel::Critical,
                LogLevel::Error => manderrow_ipc::LogLevel::Error,
                LogLevel::Warning => manderrow_ipc::LogLevel::Warning,
                LogLevel::Info => manderrow_ipc::LogLevel::Info,
                LogLevel::Debug => manderrow_ipc::LogLevel::Debug,
                LogLevel::Trace => manderrow_ipc::LogLevel::Trace,
            },
            scope: scope.into(),
            message: msg.into(),
        });
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn manderrow_agent_send_crash(msg_ptr: NonNull<u8>, msg_len: usize) {
    let msg = unsafe { NonNull::slice_from_raw_parts(msg_ptr, msg_len).as_ref() };
    let msg = std::str::from_utf8(msg).unwrap_or("<Crash messaged contained invalid UTF-8>");
    if let Some(ipc) = ipc() {
        _ = ipc.send(&C2SMessage::Crash {
            error: msg.to_owned(),
        });
    }
}

// #[unsafe(no_mangle)]
// pub unsafe extern "C" fn manderrow_agent_report_crash(msg_ptr: NonNull<u8>, msg_len: usize) {
//     let msg = unsafe { NonNull::slice_from_raw_parts(msg_ptr, msg_len).as_ref() };
//     match std::str::from_utf8(msg) {
//         Ok(msg) => crash::report_crash(msg),
//         Err(_) => crash::report_crash(format_args!("{:x?}", msg)),
//     }
// }
