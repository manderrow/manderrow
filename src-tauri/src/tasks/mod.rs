//! Task management and monitoring.

pub mod commands;
pub mod types;

use std::{
    borrow::Cow,
    collections::HashMap,
    future::Future,
    mem::ManuallyDrop,
    ops::Deref,
    sync::{atomic::AtomicU64, LazyLock},
};

use anyhow::{anyhow, bail, Result};
use futures::FutureExt;
use tauri::{AppHandle, Emitter};
use tokio::{
    select,
    sync::{oneshot, RwLock},
};

pub use types::*;

const EVENT_TARGET: &str = "main";

pub struct TaskBuilder {
    id: Id,
    metadata: Metadata,
}

struct TaskData {
    cancel: Option<oneshot::Sender<()>>,
}

static TASKS: LazyLock<RwLock<HashMap<Id, TaskData>>> = LazyLock::new(Default::default);

static NEXT_TASK_ID: AtomicU64 = AtomicU64::new(0);

pub enum TaskError<E> {
    Cancelled,
    Failed(E),
    Management(anyhow::Error),
}

impl<E: Into<anyhow::Error>> From<TaskError<E>> for anyhow::Error {
    fn from(value: TaskError<E>) -> Self {
        match value {
            TaskError::Cancelled => anyhow!("Task cancelled"),
            TaskError::Failed(e) => e.into(),
            TaskError::Management(e) => e.context("Task management failed"),
        }
    }
}

#[derive(Clone, Copy)]
pub struct TaskHandle(Option<Id>);

/// You should never drop this struct except by calling [`Self::drop`] with a [status](DropStatus) to ensure that the frontend is informed.
struct OwnedTaskHandleInner<'a> {
    app: &'a AppHandle,
    id: Id,
    cancelled: oneshot::Receiver<()>,
}

impl Id {
    fn emit<T: TaskEventBody>(self, app: &AppHandle, event: T) -> tauri::Result<()> {
        app.emit_to(
            EVENT_TARGET,
            T::NAME,
            TaskEvent {
                id: self,
                body: event,
            },
        )
    }
}

impl TaskHandle {
    pub fn send_progress_manually(
        &self,
        app: &AppHandle,
        completed: u64,
        total: u64,
    ) -> Result<()> {
        if let Some(handle) = self.0 {
            handle.emit(
                app,
                TaskProgress {
                    progress: Progress { completed, total },
                },
            )?;
        }
        Ok(())
    }

    pub fn send_progress(&self, app: &AppHandle, progress: &crate::util::Progress) -> Result<()> {
        if let Some(handle) = self.0 {
            let (completed, total) = progress.get();
            handle.emit(
                app,
                TaskProgress {
                    progress: Progress { completed, total },
                },
            )?;
        }
        Ok(())
    }

    pub fn send_dependency(&self, app: &AppHandle, dependency: Id) -> Result<()> {
        if let Some(handle) = self.0 {
            handle.emit(app, TaskDependency { dependency })?;
        }
        Ok(())
    }

    pub fn allocate_dependency(&self, app: &AppHandle) -> Result<Id> {
        let dependency = allocate_task();
        self.send_dependency(app, dependency)?;
        Ok(dependency)
    }
}

impl Drop for OwnedTaskHandleInner<'_> {
    fn drop(&mut self) {
        tokio::task::block_in_place(|| {
            TASKS.blocking_write().remove(&self.id);
        });
    }
}

impl<'a> Deref for OwnedTaskHandleInner<'a> {
    type Target = Id;

    fn deref(&self) -> &Self::Target {
        &self.id
    }
}

impl OwnedTaskHandleInner<'_> {
    fn drop(self, status: DropStatus) -> Result<()> {
        self.emit(self.app, TaskDropped { status })?;
        Ok(())
    }
}

struct OwnedTaskHandle<'a> {
    inner: ManuallyDrop<OwnedTaskHandleInner<'a>>,
}

impl<'a> OwnedTaskHandle<'a> {
    /// Once a future returned by this method completes, subsequent attempts to poll futures
    /// returned from this method will panic.
    fn cancelled(&mut self) -> CancelledFuture<'_, 'a> {
        CancelledFuture { handle: self }
    }

    fn drop(self, status: DropStatus) -> Result<()> {
        // use ManuallyDrop to avoid calling <TaskHandle as Drop>::drop which exists only to
        // handle cases where the task Future is dropped
        let mut this = ManuallyDrop::new(self);
        // SAFETY: this will not be dropped
        unsafe { ManuallyDrop::take(&mut this.inner) }.drop(status)
    }

    fn fail(self, e: &impl std::fmt::Display) -> Result<()> {
        self.drop(DropStatus::Failed {
            error: e.to_string().into(),
        })
    }
}

impl Drop for OwnedTaskHandle<'_> {
    fn drop(&mut self) {
        // SAFETY: inner has not been dropped yet
        let inner = unsafe { ManuallyDrop::take(&mut self.inner) };
        _ = inner.drop(DropStatus::Cancelled { direct: false });
    }
}

pub fn allocate_task() -> Id {
    Id(NEXT_TASK_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
}

impl TaskBuilder {
    pub fn new(title: impl Into<Cow<'static, str>>) -> Self {
        Self::with_id(allocate_task(), title)
    }

    pub fn with_id(id: Id, title: impl Into<Cow<'static, str>>) -> Self {
        Self {
            id,
            metadata: Metadata {
                title: title.into(),
                kind: Kind::Other,
                progress_unit: ProgressUnit::Other,
            },
        }
    }

    pub fn kind(mut self, kind: Kind) -> Self {
        self.metadata.kind = kind;
        self
    }

    pub fn progress_unit(mut self, progress_unit: ProgressUnit) -> Self {
        self.metadata.progress_unit = progress_unit;
        self
    }

    async fn create<'a>(self, app: &'a AppHandle) -> Result<OwnedTaskHandle<'a>> {
        let (cancel, cancelled) = oneshot::channel();
        match TASKS.write().await.entry(self.id) {
            std::collections::hash_map::Entry::Occupied(_) => {
                // the NEXT_TASK_ID counter not only wrapped around, but also collided with a task that has not been removed yet.
                bail!("User never reboots their computer")
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(TaskData {
                    cancel: Some(cancel),
                });
                self.id.emit(
                    app,
                    TaskCreated {
                        metadata: self.metadata,
                    },
                )?;
                Ok(OwnedTaskHandle {
                    inner: ManuallyDrop::new(OwnedTaskHandleInner {
                        app,
                        id: self.id,
                        cancelled,
                    }),
                })
            }
        }
    }

    pub async fn run<F, T, E>(self, app: Option<&AppHandle>, fut: F) -> Result<T, TaskError<E>>
    where
        F: Future<Output = Result<T, E>>,
        E: std::fmt::Display + Into<anyhow::Error>,
    {
        self.run_with_handle(app, move |_| fut).await
    }

    pub async fn run_with_handle<'a, F, T, E>(
        self,
        app: Option<&'a AppHandle>,
        fut: impl FnOnce(TaskHandle) -> F,
    ) -> Result<T, TaskError<E>>
    where
        F: Future<Output = Result<T, E>>,
        E: std::fmt::Display + Into<anyhow::Error>,
    {
        let mut handle = if let Some(app) = app {
            Some(self.create(app).await.map_err(TaskError::Management)?)
        } else {
            None
        };
        let fut = fut(TaskHandle(handle.as_ref().map(|handle| handle.inner.id)));
        select! {
            () = async { if let Some(handle) = &mut handle {
                handle.cancelled().await
            } else {
                std::future::pending().await
            } } => {
                handle.unwrap().drop(DropStatus::Cancelled { direct: true }).map_err(TaskError::Management)?;
                Err(TaskError::Cancelled)
            }
            r = fut => {
                match r {
                    Ok(t) => {
                        if let Some(handle) = handle {
                            handle.drop(DropStatus::Success).map_err(TaskError::Management)?;
                        }
                        Ok(t)
                    }
                    Err(e) => {
                        if let Some(handle) = handle {
                            handle.fail(&e).map_err(TaskError::Management)?;
                        }
                        Err(TaskError::Failed(e))
                    }
                }
            }
        }
    }
}

pin_project_lite::pin_project! {
    struct CancelledFuture<'a, 'b> {
        handle: &'a mut OwnedTaskHandle<'b>,
    }
}

impl Future for CancelledFuture<'_, '_> {
    type Output = ();

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let this = self.project();
        match this.handle.inner.cancelled.poll_unpin(cx) {
            std::task::Poll::Ready(Ok(())) => std::task::Poll::Ready(()),
            std::task::Poll::Ready(Err(_)) => panic!("TaskData dropped before TaskHandle"),
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
    }
}
