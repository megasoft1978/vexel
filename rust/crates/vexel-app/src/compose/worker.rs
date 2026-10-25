//! Single background composer with latest-wins supersession.
//!
//! Bounded by construction: one pending-job slot and one result slot, each
//! overwritten by newer data. A new request bumps the generation, which makes
//! any in-flight composition cancel at its next checkpoint; results of older
//! generations are never delivered.

use super::{ComposeError, Composed, DiffCache, compose_document_cached};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::thread::{Builder, JoinHandle};

struct Job {
    generation: u64,
    bytes: Arc<Vec<u8>>,
    width: u32,
    height: u32,
}

/// A finished composition for the newest generation.
pub struct WorkerEvent {
    pub generation: u64,
    pub result: Result<Composed, ComposeError>,
}

#[derive(Default)]
struct Shared {
    latest: AtomicU64,
    shutdown: AtomicBool,
    job: Mutex<Option<Job>>,
    wake: Condvar,
    done: Mutex<Option<WorkerEvent>>,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

pub struct ComposeWorker {
    shared: Arc<Shared>,
    handle: Option<JoinHandle<()>>,
}

impl ComposeWorker {
    pub fn new() -> Option<Self> {
        let shared = Arc::new(Shared::default());
        let thread_shared = shared.clone();
        let handle = Builder::new()
            .name("vexel-compose".to_owned())
            .stack_size(16 * 1024 * 1024)
            .spawn(move || run(&thread_shared))
            .ok()?;
        Some(Self {
            shared,
            handle: Some(handle),
        })
    }

    /// Supersedes everything older and returns the new generation.
    pub fn request(&self, bytes: Arc<Vec<u8>>, width: u32, height: u32) -> u64 {
        let mut slot = lock(&self.shared.job);
        let generation = self.shared.latest.fetch_add(1, Ordering::SeqCst) + 1;
        *slot = Some(Job {
            generation,
            bytes,
            width,
            height,
        });
        lock(&self.shared.done).take();
        self.shared.wake.notify_one();
        generation
    }

    /// Supersedes any pending or in-flight work without starting new work.
    pub fn cancel(&self) -> u64 {
        let mut slot = lock(&self.shared.job);
        slot.take();
        lock(&self.shared.done).take();
        self.shared.latest.fetch_add(1, Ordering::SeqCst) + 1
    }

    /// Newest finished result, if any. Stale results are discarded upstream.
    pub fn poll(&self) -> Option<WorkerEvent> {
        let event = lock(&self.shared.done).take()?;
        (event.generation == self.shared.latest.load(Ordering::SeqCst)).then_some(event)
    }
}

impl Drop for ComposeWorker {
    fn drop(&mut self) {
        self.shared.shutdown.store(true, Ordering::SeqCst);
        {
            let _slot = lock(&self.shared.job);
            self.shared.wake.notify_one();
        }
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn run(shared: &Shared) {
    // Frames the worker composed stay for the next edit: only changed shapes repaint.
    let mut cache = DiffCache::display();
    loop {
        let job = {
            let mut slot = lock(&shared.job);
            loop {
                if shared.shutdown.load(Ordering::SeqCst) {
                    return;
                }
                if let Some(job) = slot.take() {
                    break job;
                }
                slot = shared.wake.wait(slot).unwrap_or_else(|p| p.into_inner());
            }
        };
        let superseded = || {
            shared.shutdown.load(Ordering::SeqCst)
                || shared.latest.load(Ordering::SeqCst) != job.generation
        };
        cache.rearm();
        let result = compose_document_cached(
            job.bytes.clone(),
            job.width,
            job.height,
            &superseded,
            &mut cache,
        );
        if !superseded() {
            let mut done = lock(&shared.done);
            if shared.latest.load(Ordering::SeqCst) == job.generation {
                *done = Some(WorkerEvent {
                    generation: job.generation,
                    result,
                });
            }
        }
    }
}
