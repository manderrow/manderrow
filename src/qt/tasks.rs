//! The `Tasks` QML singleton, which tracks the backend's tasks and their progress, and the `Task`s
//! it gives QML.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!(<QtCore/QObject>);
        include!("cxx-qt-lib/qlist.h");
        type QObjectList = crate::qt::QObjectList;
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    #[qenum(Task)]
    enum TaskKind {
        Other,
        /// A task whose progress is that of the tasks it waits on.
        Aggregate,
        /// A download, from `url`.
        Download,
    }

    #[qenum(Task)]
    enum TaskProgressUnit {
        Other,
        Bytes,
    }

    #[qenum(Task)]
    enum TaskStatus {
        /// Known of, as a call that will create it was started, but not created yet.
        Unstarted,
        Running,
        Succeeded,
        Failed,
        Cancelled,
    }

    #[auto_cxx_name]
    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_uncreatable]
        #[qproperty(i32, id, READ, CONSTANT)]
        #[qproperty(QString, title, READ, NOTIFY = metadata_changed)]
        #[qproperty(TaskKind, kind, READ, NOTIFY = metadata_changed)]
        /// Where a download is from.
        #[qproperty(QString, url, READ, NOTIFY = metadata_changed)]
        #[qproperty(TaskProgressUnit, progress_unit, READ, NOTIFY = metadata_changed)]
        #[qproperty(TaskStatus, status, READ, NOTIFY = status_changed)]
        /// Whether it succeeded, failed, or was cancelled.
        #[qproperty(bool, is_complete, READ, NOTIFY = status_changed)]
        /// Why it failed.
        #[qproperty(QString, error, READ, NOTIFY = status_changed)]
        /// Whether it succeeded with what it would have fetched already at hand.
        #[qproperty(bool, cached, READ, NOTIFY = status_changed)]
        /// How much of it is done, in its `progressUnit`.
        #[qproperty(f64, completed, READ, NOTIFY = progress_changed)]
        /// How much there is to do, or 0 while that's unknown.
        #[qproperty(f64, total, READ, NOTIFY = progress_changed)]
        /// The first of the tasks it waits on that is a download, as an installation does.
        #[qproperty(*mut Task, download, READ, NOTIFY)]
        type Task = super::TaskRust;

        #[qsignal]
        fn metadata_changed(self: Pin<&mut Task>);
        #[qsignal]
        fn status_changed(self: Pin<&mut Task>);
        #[qsignal]
        fn progress_changed(self: Pin<&mut Task>);

        /// Asks the task to stop.
        #[qinvokable]
        fn cancel(self: &Task);

        #[qobject]
        #[qml_element]
        #[qml_singleton]
        /// Every task, in the order they became known.
        #[qproperty(QObjectList, all, READ, NOTIFY)]
        type Tasks = super::TasksRust;

        /// The task with an id, as a call's `taskId`, or null for -1. A task that is yet to be
        /// created is known as unstarted until it is.
        #[qinvokable]
        fn get(self: Pin<&mut Tasks>, id: i32) -> *mut Task;
    }

    impl cxx_qt::Threading for Tasks {}
    impl cxx_qt::Initialize for Tasks {}

    #[namespace = "rust::cxxqtlib1"]
    unsafe extern "C++" {
        include!("cxx-qt-lib/common.h");
        #[rust_name = "new_task"]
        fn make_unique() -> UniquePtr<Task>;
    }
}

use core::pin::Pin;
use std::collections::HashMap;

use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::QString;

use super::child::Child;
use super::outbox::Outbox;
use super::QObjectList;
use crate::tasks::{self, DropStatus, Event, Id, Progress, SuccessInfo};
use qobject::{Task, TaskKind, TaskProgressUnit, TaskStatus};

pub struct TaskRust {
    id: i32,
    title: QString,
    kind: TaskKind,
    url: QString,
    progress_unit: TaskProgressUnit,
    status: TaskStatus,
    is_complete: bool,
    error: QString,
    cached: bool,
    completed: f64,
    total: f64,
    download: *mut Task,
}

impl Default for TaskRust {
    fn default() -> Self {
        Self {
            id: -1,
            title: QString::default(),
            kind: TaskKind::Other,
            url: QString::default(),
            progress_unit: TaskProgressUnit::Other,
            status: TaskStatus::Unstarted,
            is_complete: false,
            error: QString::default(),
            cached: false,
            completed: 0.0,
            total: 0.0,
            download: std::ptr::null_mut(),
        }
    }
}

#[derive(Default)]
pub struct TasksRust {
    all: QObjectList,
    tasks: HashMap<Id, Child<Task>>,
    /// The aggregate tasks that wait on each task, with how much of their progress is its.
    aggregates: HashMap<Id, Vec<(Id, Progress)>>,
    /// The tasks that wait on each task and haven't found a download among those they wait on.
    awaiting_download: HashMap<Id, Vec<Id>>,
}

impl cxx_qt::Initialize for qobject::Tasks {
    fn initialize(self: Pin<&mut Self>) {
        let outbox = Outbox::new(self.qt_thread());
        super::app()
            .task_events()
            .subscribe(move |event| outbox.send(move |tasks| tasks.handle(event)));
    }
}

impl qobject::Tasks {
    fn get(self: Pin<&mut Self>, id: i32) -> *mut Task {
        Id::from_qml(id).map_or(std::ptr::null_mut(), |id| self.task(id).as_ptr())
    }

    /// A task, which is made if it's unknown, as a task can be known of before it's created.
    fn task(mut self: Pin<&mut Self>, id: Id) -> &mut Child<Task> {
        if !self.tasks.contains_key(&id) {
            let mut task = qobject::new_task();
            task.pin_mut().rust_mut().id = id.to_qml();
            let task = Child::adopt(self.as_mut(), task);
            let mut rust = self.as_mut().rust_mut();
            rust.all.push(task.as_qobject());
            rust.tasks.insert(id, task);
            self.as_mut().all_changed();
        }
        self.rust_mut()
            .get_mut()
            .tasks
            .get_mut(&id)
            .expect("the task was made")
    }

    fn handle(self: Pin<&mut Self>, event: Event) {
        match event {
            Event::Created { id, metadata } => self.created(id, metadata),
            Event::Progress { id, progress } => self.progressed(id, progress),
            Event::Dependency { id, dependency } => self.depends(id, dependency),
            Event::Dropped { id, status } => self.dropped(id, status),
        }
    }

    fn created(mut self: Pin<&mut Self>, id: Id, metadata: tasks::Metadata) {
        let task = self.as_mut().task(id);
        let download = task.as_ptr();
        let mut task = task.get_mut();
        if task.status != TaskStatus::Unstarted {
            slog_scope::error!("Duplicate id {id:?} in task creation: {:?}", metadata.title);
            return;
        }
        {
            let mut rust = task.as_mut().rust_mut();
            rust.title = QString::from(&*metadata.title);
            rust.kind = TaskKind::from(&metadata.kind);
            rust.url = match &metadata.kind {
                tasks::Kind::Download { url } => QString::from(url),
                _ => QString::default(),
            };
            rust.progress_unit = TaskProgressUnit::from(metadata.progress_unit);
            rust.status = TaskStatus::Running;
        }
        task.as_mut().metadata_changed();
        task.as_mut().status_changed();
        let (kind, unit) = (task.kind, task.progress_unit);

        // the tasks waiting on it that were waiting to know whether it's a download
        let awaiting = self.as_mut().rust_mut().awaiting_download.remove(&id);
        if kind == TaskKind::Download {
            for parent in awaiting.into_iter().flatten() {
                self.as_mut().task(parent).get_mut().set_download(download);
            }
        }
        // an aggregate only adds up the progress of tasks in its unit
        if let Some(mut aggregates) = self.as_mut().rust_mut().aggregates.remove(&id) {
            aggregates.retain(|&(parent, _)| self.as_mut().task(parent).progress_unit == unit);
            if !aggregates.is_empty() {
                self.as_mut().rust_mut().aggregates.insert(id, aggregates);
            }
        }
    }

    fn progressed(mut self: Pin<&mut Self>, id: Id, progress: Progress) {
        self.as_mut()
            .task(id)
            .get_mut()
            .set_progress(progress.completed as f64, progress.total as f64);
        // what it has added to each aggregate's progress, which this replaces, so that each update
        // is constant time however many tasks there are
        let Some(mut aggregates) = self.as_mut().rust_mut().aggregates.remove(&id) else {
            return;
        };
        for (parent, counted) in &mut aggregates {
            let mut parent = self.as_mut().task(*parent).get_mut();
            let completed = parent.completed + progress.completed as f64 - counted.completed as f64;
            let total = parent.total + progress.total as f64 - counted.total as f64;
            parent.as_mut().set_progress(completed, total);
            *counted = progress;
        }
        self.as_mut().rust_mut().aggregates.insert(id, aggregates);
    }

    fn depends(mut self: Pin<&mut Self>, id: Id, dependency: Id) {
        let task = self.as_mut().task(id);
        let (kind, unit, has_download) = (task.kind, task.progress_unit, !task.download.is_null());
        let dependency_task = self.as_mut().task(dependency);
        let download = dependency_task.as_ptr();
        let (dependency_status, dependency_kind, dependency_unit) = (
            dependency_task.status,
            dependency_task.kind,
            dependency_task.progress_unit,
        );

        if !has_download {
            if dependency_kind == TaskKind::Download {
                self.as_mut().task(id).get_mut().set_download(download);
            } else if dependency_status == TaskStatus::Unstarted {
                // it may turn out to be one once it's created
                self.as_mut()
                    .rust_mut()
                    .awaiting_download
                    .entry(dependency)
                    .or_default()
                    .push(id);
            }
        }
        if kind == TaskKind::Aggregate
            && (dependency_status == TaskStatus::Unstarted || dependency_unit == unit)
        {
            let counted = Progress {
                completed: 0,
                total: 0,
            };
            self.as_mut()
                .rust_mut()
                .aggregates
                .entry(dependency)
                .or_default()
                .push((id, counted));
        }
    }

    fn dropped(mut self: Pin<&mut Self>, id: Id, status: DropStatus) {
        let mut task = self.as_mut().task(id).get_mut();
        {
            let mut rust = task.as_mut().rust_mut();
            rust.status = TaskStatus::from(&status);
            match status {
                DropStatus::Success { success } => {
                    rust.cached = matches!(success, Some(SuccessInfo::Cached));
                }
                DropStatus::Cancelled => {}
                DropStatus::Failed { error } => rust.error = QString::from(&*error),
            }
            rust.is_complete = true;
        }
        task.status_changed();
        let mut rust = self.as_mut().rust_mut();
        rust.aggregates.remove(&id);
        rust.awaiting_download.remove(&id);
    }
}

impl Task {
    fn set_progress(mut self: Pin<&mut Self>, completed: f64, total: f64) {
        let mut rust = self.as_mut().rust_mut();
        rust.completed = completed;
        rust.total = total;
        self.progress_changed();
    }

    fn set_download(mut self: Pin<&mut Self>, download: *mut Task) {
        if self.download.is_null() {
            self.as_mut().rust_mut().download = download;
            self.download_changed();
        }
    }

    fn cancel(&self) {
        let Some(id) = Id::from_qml(self.id) else {
            return;
        };
        super::runtime().spawn(async move {
            if let Err(e) = tasks::cancel(id).await {
                slog_scope::error!("Unable to cancel task {id:?}: {e:#}");
            }
        });
    }
}

impl From<&tasks::Kind> for TaskKind {
    fn from(kind: &tasks::Kind) -> Self {
        match kind {
            tasks::Kind::Aggregate => Self::Aggregate,
            tasks::Kind::Download { .. } => Self::Download,
            tasks::Kind::Other => Self::Other,
        }
    }
}

impl From<tasks::ProgressUnit> for TaskProgressUnit {
    fn from(unit: tasks::ProgressUnit) -> Self {
        match unit {
            tasks::ProgressUnit::Bytes => Self::Bytes,
            tasks::ProgressUnit::Other => Self::Other,
        }
    }
}

/// The status of a task that has completed.
impl From<&DropStatus> for TaskStatus {
    fn from(status: &DropStatus) -> Self {
        match status {
            DropStatus::Success { .. } => Self::Succeeded,
            DropStatus::Cancelled => Self::Cancelled,
            DropStatus::Failed { .. } => Self::Failed,
        }
    }
}
