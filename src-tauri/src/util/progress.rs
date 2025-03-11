use std::sync::atomic::{AtomicU64, Ordering};

/// Over/underflows in one value will corrupt both values.
#[derive(Default)]
struct AtomicU32x2(AtomicU64);

impl AtomicU32x2 {
    pub fn get(&self) -> (u32, u32) {
        let v = self.0.load(Ordering::Acquire);
        (v as u32, (v >> 32) as u32)
    }

    fn pack(a: u32, b: u32) -> u64 {
        (a as u64) | ((b as u64) << 32)
    }

    pub fn set(&self, a: u32, b: u32) {
        self.0.store(Self::pack(a, b), Ordering::Release);
    }

    pub fn add(&self, a: u32, b: u32) {
        self.0.fetch_add(Self::pack(a, b), Ordering::AcqRel);
    }

    pub fn sub(&self, a: u32, b: u32) {
        self.0.fetch_sub(Self::pack(a, b), Ordering::AcqRel);
    }
}

#[derive(Default)]
pub struct Progress {
    steps: AtomicU32x2,
    completed_progress: AtomicU64,
    total_progress: AtomicU64,
    updates: tokio::sync::Notify,
}

impl Progress {
    pub fn updates(&self) -> &tokio::sync::Notify {
        &self.updates
    }

    pub fn get_steps(&self) -> (u32, u32) {
        self.steps.get()
    }

    pub fn get_progress(&self) -> (u64, u64) {
        (
            self.completed_progress.load(Ordering::Acquire),
            self.total_progress.load(Ordering::Acquire),
        )
    }

    pub fn step(&self) -> Step<'_> {
        self.steps.add(0, 1);
        self.update();
        Step {
            progress: self,
            // complete: 0,
            // total: 0,
        }
    }

    pub fn reset(&self) {
        self.steps.set(0, 0);
        self.completed_progress.store(0, Ordering::Release);
        self.total_progress.store(0, Ordering::Release);
        self.update();
    }

    fn update(&self) {
        self.updates.notify_waiters();
    }
}

pub struct Step<'a> {
    progress: &'a Progress,
    // complete: u64,
    // total: u64,
}

impl<'a> Step<'a> {
    pub fn add(&mut self, complete: u64, total: u64) {
        // self.complete += complete;
        // self.total += total;
        self.progress
            .completed_progress
            .fetch_add(complete, Ordering::AcqRel);
        self.progress
            .total_progress
            .fetch_add(total, Ordering::AcqRel);
        self.progress.update();
    }
}

impl<'a> Drop for Step<'a> {
    fn drop(&mut self) {
        // self.progress
        //     .completed_progress
        //     .fetch_sub(self.complete, Ordering::AcqRel);
        // self.progress
        //     .total_progress
        //     .fetch_sub(self.total, Ordering::AcqRel);
        self.progress.steps.add(1, 0);
        self.progress.update();
    }
}
