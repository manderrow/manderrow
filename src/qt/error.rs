//! The backend's errors, as QML shows them.

use cxx_qt_lib::{QString, QStringList};

use crate::CommandError;

/// An error as the properties that QML makes it of with `Async.nativeError`.
#[derive(Default)]
pub struct QmlError {
    /// The messages of the error's cause chain, from the outermost.
    pub messages: QStringList,
    pub backtrace: QString,
}

impl QmlError {
    /// The error, unless it's the user's cancellation, which isn't shown as one.
    pub fn of(error: &CommandError) -> Option<Self> {
        match error {
            CommandError::Aborted => None,
            CommandError::Error {
                messages,
                backtrace,
            } => Some(Self {
                messages: messages.iter().map(QString::from).collect(),
                backtrace: QString::from(backtrace),
            }),
        }
    }
}
