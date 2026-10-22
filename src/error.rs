#[derive(Debug, Clone, serde::Serialize)]
pub enum CommandError {
    Aborted,
    Error {
        messages: Vec<String>,
        backtrace: String,
    },
}

impl From<anyhow::Error> for CommandError {
    #[track_caller]
    fn from(value: anyhow::Error) -> Self {
        let backtrace = if value.backtrace().status() != std::backtrace::BacktraceStatus::Disabled {
            value.backtrace().to_string()
        } else {
            std::backtrace::Backtrace::force_capture().to_string()
        };
        Self::Error {
            messages: value.chain().map(|e| e.to_string()).collect(),
            backtrace,
        }
    }
}

impl From<Error> for CommandError {
    #[track_caller]
    fn from(value: Error) -> Self {
        match value {
            Error::Aborted => Self::Aborted,
            Error::Error(e) => Self::from(e),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Aborted by the user")]
    Aborted,
    #[error(transparent)]
    Error(#[from] anyhow::Error),
}
