use std::sync::atomic::{AtomicU64, Ordering};

use event_listener::Event;

#[derive(Default)]
pub struct Progress {
    progress: AtomicU64,
    updates: Event,
}

impl Progress {
    pub fn updates(&self) -> &Event {
        &self.updates
    }

    pub fn get(&self) -> (u32, u32) {
        let v = self.progress.load(Ordering::Acquire);
        (v as u32, (v >> 32) as u32)
    }

    fn pack_progress(complete: u32, total: u32) -> u64 {
        (complete as u64) | ((total as u64) << 32)
    }

    pub fn set(&self, complete: u32, total: u32) {
        self.progress
            .store(Self::pack_progress(complete, total), Ordering::Release);
        _ = self.updates.notify(usize::MAX);
    }

    pub fn inc(&self, complete: u32, total: u32) {
        self.progress
            .fetch_add(Self::pack_progress(complete, total), Ordering::AcqRel);
        _ = self.updates.notify(usize::MAX);
    }
}
