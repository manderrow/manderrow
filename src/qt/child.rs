//! QObjects that another QObject owns as its children.

use core::pin::Pin;

use cxx::memory::UniquePtrTarget;
use cxx::UniquePtr;
use cxx_qt::casting::Upcast;
use cxx_qt::QObject;
use cxx_qt_lib::{QObjectExt, QObjectMutPtr};

/// A QObject that another QObject owns as its child, which lives as long as its parent does. It
/// belongs in the parent's Rust state, and is only used on the parent's thread.
pub struct Child<T>(*mut T);

impl<T: Upcast<QObject> + UniquePtrTarget> Child<T> {
    /// Makes an object a child of `parent`.
    pub fn adopt<P: Upcast<QObject>>(parent: Pin<&mut P>, mut object: UniquePtr<T>) -> Self {
        object.pin_mut().set_parent(parent);
        Self(object.into_raw())
    }

    /// The child, to give QML.
    pub fn as_ptr(&self) -> *mut T {
        self.0
    }

    /// The child, to give QML in a list.
    pub fn as_qobject(&self) -> QObjectMutPtr {
        // SAFETY: the child lives as long as its parent, which lives as long as what it's given to
        unsafe { QObjectMutPtr::from_raw(Upcast::<QObject>::upcast_ptr(self.0).cast_mut()) }
    }

    pub fn get(&self) -> &T {
        // SAFETY: the child lives as long as its parent, whose state this is in
        unsafe { &*self.0 }
    }

    pub fn get_mut(&mut self) -> Pin<&mut T> {
        // SAFETY: the child lives as long as its parent, whose state this is in, and this borrows
        // it uniquely
        unsafe { Pin::new_unchecked(&mut *self.0) }
    }
}
