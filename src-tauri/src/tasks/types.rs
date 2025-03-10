use std::borrow::Cow;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Deserialize, serde::Serialize)]
#[repr(transparent)]
pub struct Id(pub(super) u64);

#[derive(Clone, serde::Serialize)]
pub struct Metadata {
    pub title: Cow<'static, str>,
    pub kind: Kind,
}

#[derive(Debug, Clone, Copy, serde::Serialize)]
pub enum Kind {
    Download,
    Other,
}

#[derive(Clone, serde::Serialize)]
pub struct Progress {
    pub completed_steps: u64,
    pub total_steps: u64,
    pub progress: f64,
}

#[derive(Clone, serde::Serialize)]
pub struct TaskCreated {
    pub id: Id,
    pub metadata: Metadata,
}

#[derive(Clone, serde::Serialize)]
pub struct TaskProgress {
    pub id: Id,
    pub progress: Progress,
}

#[derive(Clone, serde::Serialize)]
pub struct TaskDropped {
    pub id: Id,
    pub status: DropStatus,
}

#[derive(Clone, serde::Serialize)]
#[serde(tag = "status")]
pub enum DropStatus {
    Success,
    Cancelled {
        /// If true, the cancellation was due to the user acting directly on the task. Otherwise, it was likely due to the task's [`Future`](std::future::Future) being dropped.
        direct: bool,
    },
    Failed(Cow<'static, str>),
}

impl TaskCreated {
    pub const EVENT: &str = "task_created";
}

impl TaskProgress {
    pub const EVENT: &str = "task_progress";
}

impl TaskDropped {
    pub const EVENT: &str = "task_dropped";
}
