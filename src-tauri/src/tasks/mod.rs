//! Task management and monitoring.
//!
//! When running a task that should be exposed to the user, "register" it like so:
//!
//! ```rust
//! // run_task returns a [`TaskError`], but you probably want an [`anyhow::Error`], so convert it, for example via the [`Try`] operator
//! Ok(run_task(Kind::Other, async move {
//!     // do some long-running operation
//! })?)
//! ```

pub mod commands;
pub mod types;

use std::{
    borrow::Cow,
    collections::HashMap,
    future::Future,
    mem::ManuallyDrop,
    sync::{atomic::AtomicU64, LazyLock},
};

use anyhow::{anyhow, bail, Result};
use futures::{
    future::{Fuse, FusedFuture},
    FutureExt,
};
use tauri::{AppHandle, Emitter};
use tokio::{
    select,
    sync::{oneshot, RwLock},
};

use types::*;

const EVENT_TARGET: &str = "main";

pub struct TaskBuilder {
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

// struct TaskHandle<'a> {
//     app: &'a AppHandle,
//     id: Id,
// }

/// You should never drop this struct except by calling [`Self::drop`] with a [status](DropStatus) to ensure that the frontend is informed.
struct TaskHandleInner<'a> {
    app: &'a AppHandle,
    id: Id,
    cancelled: Fuse<oneshot::Receiver<()>>,
}

impl Drop for TaskHandleInner<'_> {
    fn drop(&mut self) {
        tokio::task::block_in_place(|| {
            TASKS.blocking_write().remove(&self.id);
        });
    }
}

impl TaskHandleInner<'_> {
    fn drop(self, status: DropStatus) -> Result<()> {
        self.app.emit_to(
            EVENT_TARGET,
            TaskDropped::EVENT,
            TaskDropped {
                id: self.id,
                status,
            },
        )?;
        Ok(())
    }
}

struct TaskHandle<'a> {
    inner: ManuallyDrop<TaskHandleInner<'a>>,
}

impl<'a> TaskHandle<'a> {
    fn cancelled(&mut self) -> CancelledFuture<'_, 'a> {
        CancelledFuture { handle: self }
    }

    fn drop(self, status: DropStatus) -> Result<()> {
        let mut this = ManuallyDrop::new(self);
        // SAFETY: this will not be dropped
        unsafe { ManuallyDrop::take(&mut this.inner) }.drop(status)
    }

    fn fail(self, e: &impl std::fmt::Display) -> Result<()> {
        self.drop(DropStatus::Failed(e.to_string().into()))
    }
}

impl Drop for TaskHandle<'_> {
    fn drop(&mut self) {
        // SAFETY: inner has not been dropped yet
        let inner = unsafe { ManuallyDrop::take(&mut self.inner) };
        _ = inner.drop(DropStatus::Cancelled { direct: false });
    }
}

impl TaskBuilder {
    pub fn new(title: impl Into<Cow<'static, str>>, kind: Kind) -> Self {
        Self {
            metadata: Metadata {
                title: title.into(),
                kind,
            },
        }
    }

    async fn create<'a>(self, app: &'a AppHandle) -> Result<TaskHandle<'a>> {
        let id = Id(NEXT_TASK_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed));
        let (cancel, cancelled) = oneshot::channel();
        match TASKS.write().await.entry(id) {
            std::collections::hash_map::Entry::Occupied(_) => {
                // the NEXT_TASK_ID counter not only wrapped around, but also collided with a task that has not been removed yet.
                bail!("User never reboots their computer")
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(TaskData {
                    cancel: Some(cancel),
                });
                app.emit_to(
                    EVENT_TARGET,
                    TaskCreated::EVENT,
                    TaskCreated {
                        id,
                        metadata: self.metadata,
                    },
                )?;
                Ok(TaskHandle {
                    inner: ManuallyDrop::new(TaskHandleInner {
                        app,
                        id,
                        cancelled: cancelled.fuse(),
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
        self.run_with_progress(app, move |_| fut).await
    }

    pub async fn run_with_progress<F, T, E>(
        self,
        app: Option<&AppHandle>,
        fut: impl FnOnce(()) -> F,
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
        let fut = fut(());
        select! {
            () = async { if let Some(handle) = &mut handle { handle.cancelled().await } } => {
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
        handle: &'a mut TaskHandle<'b>,
    }
}

impl Future for CancelledFuture<'_, '_> {
    type Output = ();

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let this = self.project();
        if this.handle.inner.cancelled.is_terminated() {
            std::task::Poll::Ready(())
        } else {
            match this.handle.inner.cancelled.poll_unpin(cx) {
                std::task::Poll::Ready(Ok(())) => std::task::Poll::Ready(()),
                std::task::Poll::Ready(Err(_)) => panic!("TaskData dropped before TaskHandle"),
                std::task::Poll::Pending => std::task::Poll::Pending,
            }
        }
    }
}
