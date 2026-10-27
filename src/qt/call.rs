//! `Call`: an asynchronous call into the backend, which the methods that start one return.
//!
//! QML turns a call into a promise with `Async.promise(call)`, or awaits it with `yield` in
//! `Async.co`. What it resolves to depends on the call: a value, or objects, which become QML's
//! once it takes them, so that they live as long as QML refers to them.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qstringlist.h");
        type QStringList = cxx_qt_lib::QStringList;
        include!("cxx-qt-lib/qvariant.h");
        type QVariant = cxx_qt_lib::QVariant;
    }

    /// Whether a call is still running, and how it ended.
    #[qenum(Call)]
    enum CallStatus {
        Running,
        Succeeded,
        Failed,
        /// Cancelled by the user.
        Aborted,
    }

    /// What a call resolves to.
    #[qenum(Call)]
    enum CallResultKind {
        /// Its `value`, which is undefined for a call that returns nothing.
        Value,
        /// An object, or null.
        Object,
        /// A list of objects.
        List,
    }

    #[auto_cxx_name]
    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_uncreatable]
        #[qproperty(CallStatus, status, READ, NOTIFY)]
        #[qproperty(CallResultKind, result_kind, READ)]
        /// The value the call returned, for a call whose `resultKind` is `Value`. Some calls that
        /// return objects return a value besides, as a count of all of what they return pages of.
        #[qproperty(QVariant, value, READ)]
        /// How many objects the call returned, which `takeObject` takes.
        #[qproperty(i32, object_count, READ)]
        /// The messages of the error that failed the call, from the outermost to its cause.
        #[qproperty(QStringList, error_messages, READ)]
        #[qproperty(QString, backtrace, READ)]
        /// The task whose progress is the call's, which `Tasks.get` returns, or -1.
        #[qproperty(i32, task_id, READ, CONSTANT)]
        type Call = super::CallRust;

        /// Takes one of the objects the call returned, which QML then owns. Each can be taken
        /// once, and is null after that.
        #[qinvokable]
        fn take_object(self: Pin<&mut Call>, index: i32) -> *mut QObject;
    }

    impl cxx_qt::Threading for Call {}

    #[namespace = "rust::cxxqtlib1"]
    unsafe extern "C++" {
        include!("cxx-qt-lib/common.h");
        #[rust_name = "new_call"]
        fn make_unique() -> UniquePtr<Call>;
    }
}

use core::pin::Pin;
use std::future::Future;

use cxx::memory::UniquePtrTarget;
use cxx::UniquePtr;
use cxx_qt::casting::Upcast;
use cxx_qt::{CxxQtType, QObject, Threading};
use cxx_qt_lib::{QObjectExt, QString, QStringList, QVariant};

pub use qobject::Call;
use qobject::{CallResultKind, CallStatus};

use crate::{tasks, CommandError};

pub struct CallRust {
    status: CallStatus,
    result_kind: CallResultKind,
    value: QVariant,
    object_count: i32,
    error_messages: QStringList,
    backtrace: QString,
    task_id: i32,
    /// The objects the call returned, which are its children until QML takes them.
    objects: Vec<*mut QObject>,
}

impl Default for CallRust {
    fn default() -> Self {
        Self {
            status: CallStatus::Running,
            result_kind: CallResultKind::Value,
            value: QVariant::default(),
            object_count: 0,
            error_messages: QStringList::default(),
            backtrace: QString::default(),
            task_id: -1,
            objects: Vec::new(),
        }
    }
}

/// Starts a call of `future` on the backend's runtime, and returns it to give to QML, which owns
/// it. Once `future` completes, `finish` runs on the Qt thread with its result, to make what the
/// call returns of it, with [`Call::return_value`] and the like, unless QML has dropped the call
/// by then. The call's progress is that of `task`, which `future` creates.
pub fn start<T, F>(
    task: Option<tasks::Id>,
    future: F,
    finish: impl FnOnce(Pin<&mut Call>, T) + Send + 'static,
) -> *mut Call
where
    T: Send + 'static,
    F: Future<Output = Result<T, CommandError>> + Send + 'static,
{
    let mut call = qobject::new_call();
    if let Some(task) = task {
        call.pin_mut().rust_mut().task_id = task.to_qml();
    }
    let thread = call.qt_thread();
    super::runtime().spawn(async move {
        let result = future.await;
        // fails only if QML has dropped the call, when there's no one to tell
        _ = thread.queue(move |mut call| match result {
            Ok(value) => {
                finish(call.as_mut(), value);
                call.set_status(CallStatus::Succeeded);
            }
            Err(e) => call.fail(e),
        });
    });
    // QML owns an object without a parent that a method returns
    call.into_raw()
}

impl Call {
    /// Returns a value from the call.
    pub fn return_value(mut self: Pin<&mut Self>, value: QVariant) {
        let mut rust = self.as_mut().rust_mut();
        rust.result_kind = CallResultKind::Value;
        rust.value = value;
    }

    /// Returns an object from the call, or null.
    pub fn return_object<T: Upcast<QObject> + UniquePtrTarget>(
        mut self: Pin<&mut Self>,
        object: Option<UniquePtr<T>>,
    ) {
        self.as_mut().rust_mut().result_kind = CallResultKind::Object;
        if let Some(object) = object {
            self.adopt(object);
        }
    }

    /// Returns a list of objects from the call.
    pub fn return_objects<T: Upcast<QObject> + UniquePtrTarget>(
        mut self: Pin<&mut Self>,
        objects: impl IntoIterator<Item = UniquePtr<T>>,
    ) {
        self.as_mut().rust_mut().result_kind = CallResultKind::List;
        for object in objects {
            self.as_mut().adopt(object);
        }
    }

    /// Returns a value besides the objects the call returns, as the count of all of what they're
    /// a page of.
    pub fn set_value(mut self: Pin<&mut Self>, value: QVariant) {
        self.as_mut().rust_mut().value = value;
    }

    /// Keeps an object as the call's child until QML takes it.
    fn adopt<T: Upcast<QObject> + UniquePtrTarget>(
        mut self: Pin<&mut Self>,
        mut object: UniquePtr<T>,
    ) {
        object.pin_mut().set_parent(self.as_mut());
        let pointer = std::ptr::from_ref::<QObject>(Upcast::upcast(&*object)).cast_mut();
        // owned by the call now, as its child
        _ = object.into_raw();
        let mut rust = self.rust_mut();
        rust.objects.push(pointer);
        rust.object_count += 1;
    }

    fn set_status(mut self: Pin<&mut Self>, status: CallStatus) {
        self.as_mut().rust_mut().status = status;
        self.status_changed();
    }

    fn fail(mut self: Pin<&mut Self>, error: CommandError) {
        match error {
            CommandError::Aborted => self.set_status(CallStatus::Aborted),
            CommandError::Error {
                messages,
                backtrace,
            } => {
                let mut rust = self.as_mut().rust_mut();
                rust.error_messages = messages.iter().map(QString::from).collect();
                rust.backtrace = QString::from(&backtrace);
                self.set_status(CallStatus::Failed);
            }
        }
    }

    fn take_object(mut self: Pin<&mut Self>, index: i32) -> *mut QObject {
        let Some(slot) = usize::try_from(index)
            .ok()
            .and_then(|index| self.as_mut().rust_mut().get_mut().objects.get_mut(index))
        else {
            return std::ptr::null_mut();
        };
        let object = std::mem::replace(slot, std::ptr::null_mut());
        if !object.is_null() {
            // SAFETY: the objects the call hasn't given QML yet are its children, which live as
            // long as it does
            unsafe { Pin::new_unchecked(&mut *object) }.unset_parent();
        }
        // QML owns an object without a parent that a method returns
        object
    }
}
