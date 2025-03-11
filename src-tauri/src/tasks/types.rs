use std::borrow::Cow;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Deserialize, serde::Serialize)]
#[repr(transparent)]
pub struct Id(pub(super) u64);

#[derive(Clone, serde::Serialize)]
pub struct Metadata {
    pub title: Cow<'static, str>,
    pub kind: Kind,
    pub progress_unit: ProgressUnit,
}

#[derive(Debug, Clone, Copy, serde::Serialize)]
pub enum Kind {
    Download,
    Other,
}

#[derive(Debug, Clone, Copy, serde::Serialize)]
pub enum ProgressUnit {
    Bytes,
    Other,
}

#[derive(Clone, serde::Serialize)]
pub struct Progress {
    pub completed_steps: u64,
    pub total_steps: u64,
    pub completed_progress: u64,
    pub total_progress: u64,
}

#[derive(Clone, serde::Serialize)]
pub struct TaskEvent<T: TaskEventBody> {
    pub id: Id,
    #[serde(flatten)]
    pub body: T,
}

pub trait TaskEventBody: Clone + serde::Serialize {
    const NAME: &str;
}

#[derive(Clone, serde::Serialize)]
pub struct TaskCreated {
    pub metadata: Metadata,
}

impl TaskEventBody for TaskCreated {
    const NAME: &str = "task_created";
}

#[derive(Clone, serde::Serialize)]
pub struct TaskProgress {
    pub progress: Progress,
}

impl TaskEventBody for TaskProgress {
    const NAME: &str = "task_progress";
}

#[derive(Clone, serde::Serialize)]
pub struct TaskDropped {
    pub status: DropStatus,
}

impl TaskEventBody for TaskDropped {
    const NAME: &str = "task_dropped";
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
