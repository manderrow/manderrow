use std::borrow::Cow;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Id(pub(super) u64);

impl Id {
    /// The id as QML knows it. Ids are counted from 0 as tasks are created, so they fit.
    pub fn to_qml(self) -> i32 {
        self.0 as i32
    }

    pub fn from_qml(id: i32) -> Self {
        Self(id as u32 as u64)
    }
}

#[derive(Clone)]
pub struct Metadata {
    pub title: Cow<'static, str>,
    pub kind: Kind,
    pub progress_unit: ProgressUnit,
}

#[derive(Debug, Clone)]
pub enum Kind {
    Aggregate,
    Download { url: String },
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressUnit {
    Bytes,
    Other,
}

#[derive(Clone, Copy)]
pub struct Progress {
    pub completed: u64,
    pub total: u64,
}

/// What happens to a task, which the frontend shows.
#[derive(Clone)]
pub enum Event {
    Created { id: Id, metadata: Metadata },
    Progress { id: Id, progress: Progress },
    /// The task waits on another, whose progress an aggregate task adds up.
    Dependency { id: Id, dependency: Id },
    Dropped { id: Id, status: DropStatus },
}

#[derive(Clone)]
pub enum DropStatus {
    Success {
        success: Option<SuccessInfo>,
    },
    /// By the user, or as the task's [`Future`](std::future::Future) was dropped.
    Cancelled,
    Failed {
        error: Cow<'static, str>,
    },
}

#[derive(Clone, Copy)]
pub enum SuccessInfo {
    Cached,
}
