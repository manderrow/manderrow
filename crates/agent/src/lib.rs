#![deny(unused_must_use)]
#![feature(core_io_borrowed_buf)]
#![feature(once_cell_try_insert)]
#![feature(panic_backtrace_config)]
#![feature(round_char_boundary)]

use std::mem::MaybeUninit;
use std::num::NonZeroU32;
use std::ptr::NonNull;
use std::sync::OnceLock;

use manderrow_ipc::client::Ipc;
use manderrow_ipc::ipc_channel::ipc::{IpcOneShotServer, IpcSender};
use manderrow_ipc::{C2SMessage, OutputLine, S2CMessage};

unsafe extern "sysv64" {
    fn manderrow_agent_crash(msg_ptr: NonNull<u8>, msg_len: usize) -> !;
}

/// `c2s_tx` must consist entirely of UTF-8 codepoints.
#[unsafe(no_mangle)]
pub unsafe extern "sysv64" fn manderrow_agent_init(
    c2s_tx_ptr: Option<NonNull<u8>>,
    c2s_tx_len: usize,
    error_buf: &mut ErrorBuffer,
) -> InitStatusCode {
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
    }));

    let c2s_tx = match c2s_tx_ptr {
        Some(s) => Some(unsafe {
            std::str::from_utf8_unchecked(NonNull::slice_from_raw_parts(s, c2s_tx_len).as_ref())
        }),
        None => return InitStatusCode::Success,
    };

    if let Some(c2s_tx) = c2s_tx {
        if let Err(e) = connect_ipc(c2s_tx) {
            return match e {
                ConnectIpcError::ConnectC2SError(error) => {
                    error_buf.write(format_args!("Failed to connect to c2s channel: {}", error));
                    InitStatusCode::ConnectC2SError
                }
                ConnectIpcError::CreateS2CError(error) => {
                    error_buf.write(format_args!("Failed to create s2c channel: {}", error));
                    InitStatusCode::CreateS2CError
                }
                ConnectIpcError::SendConnectError(error) => {
                    error_buf.write(format_args!(
                        "Failed to send connect message on c2s channel: {}",
                        error
                    ));
                    InitStatusCode::SendConnectError
                }
                ConnectIpcError::RecvConnectError(error) => {
                    error_buf.write(format_args!(
                        "Failed to receive connect message on s2c channel: {}",
                        error
                    ));
                    InitStatusCode::RecvConnectError
                }
                ConnectIpcError::InvalidRecvConnectMessage(msg) => {
                    error_buf.write(format_args!(
                        "Invalid connection message received on s2c channel: {:?}",
                        msg
                    ));
                    InitStatusCode::InvalidRecvConnectMessage
                }
                ConnectIpcError::InvalidPid => {
                    error_buf.write(format_args!("Invalid pid: 0"));
                    InitStatusCode::InvalidPid
                }
                ConnectIpcError::IpcAlreadySet => {
                    error_buf.write(format_args!("Global IPC is already set"));
                    InitStatusCode::IpcAlreadySet
                }
            };
        }
    }

    InitStatusCode::Success
}

static IPC: OnceLock<Ipc> = OnceLock::new();

fn ipc() -> Option<&'static Ipc> {
    IPC.get()
}

#[repr(C)]
pub struct ErrorBuffer {
    errno: Option<NonZeroU32>,
    message_buf: NonNull<MaybeUninit<u8>>,
    message_len: usize,
}

impl ErrorBuffer {
    pub fn set_errno(&mut self, errno: NonZeroU32) {
        self.errno = Some(errno);
    }

    pub fn write(&mut self, message: impl std::fmt::Display) {
        use std::fmt::Write;

        struct Writer<'a> {
            buf: std::io::BorrowedCursor<'a>,
        }

        impl Write for Writer<'_> {
            fn write_str(&mut self, s: &str) -> std::fmt::Result {
                if s.len() <= self.buf.capacity() {
                    self.buf.append(s.as_bytes());
                } else {
                    let i = s.floor_char_boundary(self.buf.capacity() - 3);
                    assert!(i + 3 <= self.buf.capacity());
                    self.buf.append(&s.as_bytes()[..i]);
                    self.buf.append(b"...");
                }
                Ok(())
            }
        }

        let mut buf = std::io::BorrowedBuf::from(unsafe {
            NonNull::slice_from_raw_parts(self.message_buf, self.message_len).as_mut()
        });
        _ = write!(
            Writer {
                buf: buf.unfilled()
            },
            "{}",
            message
        );
        self.message_len = buf.len();
    }
}

#[repr(u8)]
pub enum InitStatusCode {
    Success,
    ConnectC2SError,
    CreateS2CError,
    SendConnectError,
    RecvConnectError,
    InvalidRecvConnectMessage,
    InvalidPid,
    IpcAlreadySet,
}

enum ConnectIpcError {
    ConnectC2SError(std::io::Error),
    CreateS2CError(std::io::Error),
    SendConnectError(manderrow_ipc::ipc_channel::error::SendError),
    RecvConnectError(manderrow_ipc::ipc_channel::error::RecvError),
    InvalidRecvConnectMessage(S2CMessage),
    InvalidPid,
    IpcAlreadySet,
}

fn connect_ipc(c2s_tx: &str) -> Result<(), ConnectIpcError> {
    let c2s_tx =
        IpcSender::<C2SMessage>::connect(c2s_tx).map_err(ConnectIpcError::ConnectC2SError)?;

    let (s2c_rx, s2c_tx) =
        IpcOneShotServer::<S2CMessage>::new().map_err(ConnectIpcError::CreateS2CError)?;
    // TODO: does this return the real value under Wine?
    let pid = std::process::id();
    c2s_tx
        .send(&C2SMessage::Connect { s2c_tx })
        .map_err(ConnectIpcError::SendConnectError)?;
    c2s_tx
        .send(&C2SMessage::Started {
            pid: NonZeroU32::new(pid).ok_or(ConnectIpcError::InvalidPid)?,
        })
        .map_err(ConnectIpcError::SendConnectError)?;
    let (s2c_rx, msg) = s2c_rx.accept().map_err(ConnectIpcError::RecvConnectError)?;
    if !matches!(msg, S2CMessage::Connect) {
        return Err(ConnectIpcError::InvalidRecvConnectMessage(msg));
    }

    IPC.set(Ipc::new(c2s_tx, s2c_rx))
        .map_err(|_| ConnectIpcError::IpcAlreadySet)
}

#[unsafe(no_mangle)]
pub extern "sysv64" fn manderrow_agent_send_exit(code: i32, with_code: bool) {
    if let Some(ipc) = ipc() {
        _ = ipc.send(&C2SMessage::Exit {
            code: if with_code { Some(code) } else { None },
        });
    }
}

#[repr(u8)]
pub enum StandardOutputChannel {
    Out,
    Err,
}

#[unsafe(no_mangle)]
pub unsafe extern "sysv64" fn manderrow_agent_send_output_line(
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
pub unsafe extern "sysv64" fn manderrow_agent_send_log(
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
pub unsafe extern "sysv64" fn manderrow_agent_send_crash(msg_ptr: NonNull<u8>, msg_len: usize) {
    let msg = unsafe { NonNull::slice_from_raw_parts(msg_ptr, msg_len).as_ref() };
    let msg = std::str::from_utf8(msg).unwrap_or("<Crash messaged contained invalid UTF-8>");
    if let Some(ipc) = ipc() {
        _ = ipc.send(&C2SMessage::Crash {
            error: msg.to_owned(),
        });
    }
}
