//! Client is the game, server is the Manderrow app.
#![deny(unused_must_use)]
#![feature(type_changing_struct_update)]

pub mod client;
#[cfg(feature = "doctor")]
pub mod doctor;

pub use ipc_channel;

use std::collections::HashMap;
use std::ffi::OsString;
use std::num::NonZeroU32;

use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Hash, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum SafeOsString {
    Unicode(String),
    NonUnicodeBytes(Vec<u8>),
    NonUnicodeWide(Vec<u16>),
}

impl From<OsString> for SafeOsString {
    fn from(value: OsString) -> Self {
        match value.into_string() {
            Ok(s) => Self::Unicode(s),
            #[cfg(unix)]
            Err(s) => {
                use std::os::unix::ffi::OsStrExt;
                Self::NonUnicodeBytes(s.as_bytes().to_owned())
            }
            #[cfg(windows)]
            Err(s) => {
                use std::os::windows::ffi::OsStrExt;
                Self::NonUnicodeWide(s.encode_wide().collect::<Vec<_>>())
            }
            #[cfg(not(any(unix, windows)))]
            Err(s) => compile_error!("Unsupported platform"),
        }
    }
}

#[derive(Debug, Clone, Copy, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum StandardOutputChannel {
    Out,
    Err,
}

#[derive(Debug, Clone, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum OutputLine {
    Unicode(String),
    Bytes(Vec<u8>),
}

impl OutputLine {
    pub fn new(bytes: Vec<u8>) -> Self {
        String::from_utf8(bytes)
            .map(|s| OutputLine::Unicode(s))
            .unwrap_or_else(|e| OutputLine::Bytes(e.into_bytes()))
    }
}

#[derive(Debug, Clone, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct DoctorFix<T> {
    pub id: T,
    pub label: Option<HashMap<String, String>>,
    pub confirm_label: Option<HashMap<String, String>>,
    pub description: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct DoctorReport {
    pub id: Uuid,
    pub translation_key: String,
    pub message: Option<String>,
    pub message_args: Option<HashMap<String, String>>,
    pub fixes: Vec<DoctorFix<String>>,
}

#[derive(Debug, Copy, Clone, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "SCREAMING_SNAKE_CASE"))]
pub enum LogLevel {
    Critical,
    Error,
    Warning,
    Info,
    Debug,
    Trace,
}

#[cfg(feature = "slog")]
impl From<slog::Level> for LogLevel {
    fn from(value: slog::Level) -> Self {
        match value {
            slog::Level::Critical => Self::Critical,
            slog::Level::Error => Self::Error,
            slog::Level::Warning => Self::Warning,
            slog::Level::Info => Self::Info,
            slog::Level::Debug => Self::Debug,
            slog::Level::Trace => Self::Trace,
        }
    }
}

#[derive(Debug, Clone, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum C2SMessage {
    Connect {
        s2c_tx: String,
        pid: NonZeroU32,
    },
    Start {
        command: SafeOsString,
        args: Vec<SafeOsString>,
        env: HashMap<String, SafeOsString>,
    },
    Log {
        level: LogLevel,
        scope: String,
        message: String,
    },
    Output {
        channel: StandardOutputChannel,
        line: OutputLine,
    },
    Exit {
        code: Option<i32>,
    },
    Crash {
        error: String,
    },
    DoctorReport(DoctorReport),
}

#[derive(Debug, Clone, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum S2CMessage {
    Connect,
    PatientResponse { id: Uuid, choice: String },
}
