//! Client is the game, server is the Manderrow app.
#![deny(unused_must_use)]
#![feature(type_changing_struct_update)]

pub mod client;

pub use bincode;
pub use ipc_channel;

use std::collections::HashMap;
use std::ffi::OsString;
use std::marker::PhantomData;
use std::num::NonZeroU32;
use std::ops::ControlFlow;

use slog::error;
use smol_str::SmolStr;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Deserialize, serde::Serialize)]
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

#[derive(Debug, Clone, Copy, serde::Deserialize, serde::Serialize)]
pub enum StandardOutputChannel {
    Out,
    Err,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
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

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct DoctorFix<T> {
    pub id: T,
    pub label: Option<HashMap<String, serde_json::Value>>,
    pub confirm_label: Option<HashMap<String, serde_json::Value>>,
    pub description: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct DoctorReport {
    pub id: Uuid,
    pub translation_key: String,
    pub message: Option<String>,
    pub message_args: Option<HashMap<String, serde_json::Value>>,
    pub fixes: Vec<DoctorFix<String>>,
}

#[derive(Debug, Copy, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LogLevel {
    Critical,
    Error,
    Warning,
    Info,
    Debug,
    Trace,
}

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

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
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
        scope: SmolStr,
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

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub enum S2CMessage {
    Connect,
    PatientResponse { id: Uuid, choice: String },
    Kill,
}

pub struct PatientChoiceReceiver<T> {
    id: Uuid,
    _marker: PhantomData<T>,
}

impl<T: serde::Serialize> PatientChoiceReceiver<T> {
    pub fn new(
        translation_key: impl Into<String>,
        message: Option<String>,
        message_args: Option<HashMap<String, serde_json::Value>>,
        fixes: impl IntoIterator<Item = DoctorFix<T>>,
    ) -> (Self, C2SMessage) {
        let fixes = fixes
            .into_iter()
            .map(|fix| {
                let serde_json::Value::String(id) =
                    serde_json::to_value(fix.id).expect("Unable to serialize id")
                else {
                    panic!("Id must serialize to a string")
                };
                DoctorFix { id, ..fix }
            })
            .collect::<Vec<_>>();
        let translation_key = translation_key.into();
        let id = Uuid::new_v4();
        (
            Self {
                id,
                _marker: PhantomData,
            },
            C2SMessage::DoctorReport(DoctorReport {
                id,
                translation_key,
                message,
                message_args,
                fixes,
            }),
        )
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PromptError {
    #[error("failed to decode patient choice received from server: {0}")]
    Decode(serde_json::Error),
}

impl<T: serde::de::DeserializeOwned> PatientChoiceReceiver<T> {
    pub fn process(self, response: S2CMessage) -> Result<ControlFlow<T, Self>, PromptError> {
        match response {
            S2CMessage::PatientResponse {
                id: resp_id,
                choice,
            } if resp_id == self.id => Ok(ControlFlow::Break(
                serde_json::from_value(serde_json::Value::String(choice))
                    .map_err(PromptError::Decode)?,
            )),
            _ => Ok(ControlFlow::Continue(self)),
        }
    }
}
