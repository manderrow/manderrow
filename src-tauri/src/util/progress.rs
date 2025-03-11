use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Default)]
pub struct Progress {
    completed: AtomicU64,
    total: AtomicU64,
    updates: tokio::sync::Notify,
}

impl Progress {
    pub fn updates(&self) -> &tokio::sync::Notify {
        &self.updates
    }

    pub fn get(&self) -> (u64, u64) {
        (
            self.completed.load(Ordering::Acquire),
            self.total.load(Ordering::Acquire),
        )
    }

    pub fn add(&self, complete: u64, total: u64) {
        // self.complete += complete;
        // self.total += total;
        self.completed.fetch_add(complete, Ordering::AcqRel);
        self.total.fetch_add(total, Ordering::AcqRel);
        self.update();
    }

    pub fn reset(&self) {
        self.completed.store(0, Ordering::Release);
        self.total.store(0, Ordering::Release);
        self.update();
    }

    fn update(&self) {
        self.updates.notify_waiters();
    }
}
