//! Delivers what the backend sends to a QObject to it on the Qt thread, in order.

use core::pin::Pin;
use std::sync::Arc;

use cxx_qt::{CxxQtThread, Threading};
use parking_lot::Mutex;

type Delivery<T> = Box<dyn FnOnce(Pin<&mut T>) + Send>;

/// Runs what other threads send to a QObject on the Qt thread, in the order they sent it, with
/// everything sent before the Qt thread gets to it delivered at once, so that a flood of events,
/// such as a game's log, costs it one wake-up rather than one each.
///
/// Since the Qt thread runs what's queued to its objects in the order it was queued, something
/// sent to one object is delivered after everything sent to the others before it.
pub struct Outbox<T: Threading> {
    thread: CxxQtThread<T>,
    queue: Mutex<Vec<Delivery<T>>>,
}

impl<T: Threading + 'static> Outbox<T> {
    pub fn new(thread: CxxQtThread<T>) -> Arc<Self> {
        Arc::new(Self {
            thread,
            queue: Mutex::new(Vec::new()),
        })
    }

    /// Runs `f` with the object on the Qt thread, unless the object has been destroyed by then.
    pub fn send(self: &Arc<Self>, f: impl FnOnce(Pin<&mut T>) + Send + 'static) {
        let mut queue = self.queue.lock();
        let first = queue.is_empty();
        queue.push(Box::new(f));
        drop(queue);
        // a delivery is already on its way otherwise, which will take this one along
        if first {
            let outbox = self.clone();
            // fails only once the object has been destroyed, when there is no one left to deliver to
            _ = self.thread.queue(move |mut object| {
                let deliveries = std::mem::take(&mut *outbox.queue.lock());
                for delivery in deliveries {
                    delivery(object.as_mut());
                }
            });
        }
    }
}
