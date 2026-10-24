//! Background renderer for playback frames with latest-wins supersession.
//!
//! Bounded by construction: one render in flight, at most one pending job and
//! one finished frame, each replaced by newer data. The UI thread submits and
//! polls without ever waiting. An epoch bumped by [`PlayWorker::cancel`]
//! (scrub, pause, edit, mode switch) drops every job and frame of older epochs.

use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::thread::{Builder, JoinHandle};

use super::prepared::Prepared;
use super::{NativeFrameResult, NativeFrameTag};

struct Job {
    epoch: u64,
    prepared: Arc<Prepared>,
    tag: NativeFrameTag,
}

struct Done {
    epoch: u64,
    result: NativeFrameResult,
}

#[derive(Default)]
struct State {
    epoch: u64,
    shutdown: bool,
    in_flight: bool,
    pending: Option<Job>,
    done: Option<Done>,
}

#[derive(Default)]
struct Shared {
    state: Mutex<State>,
    wake: Condvar,
}

fn lock(shared: &Shared) -> MutexGuard<'_, State> {
    shared
        .state
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

pub struct PlayWorker {
    shared: Arc<Shared>,
    handle: Option<JoinHandle<()>>,
}

impl PlayWorker {
    pub fn new() -> Option<Self> {
        let shared = Arc::new(Shared::default());
        let thread_shared = shared.clone();
        let handle = Builder::new()
            .name("vexel-play".to_owned())
            .stack_size(16 * 1024 * 1024)
            .spawn(move || run(&thread_shared))
            .ok()?;
        Some(Self {
            shared,
            handle: Some(handle),
        })
    }

    /// Replaces the pending job. An in-flight render still completes.
    pub fn submit(&self, prepared: Arc<Prepared>, tag: NativeFrameTag) {
        let mut state = lock(&self.shared);
        let epoch = state.epoch;
        state.pending = Some(Job {
            epoch,
            prepared,
            tag,
        });
        self.shared.wake.notify_one();
    }

    /// Drops pending and finished work of the current epoch and starts a new one.
    pub fn cancel(&self) {
        let mut state = lock(&self.shared);
        state.epoch += 1;
        state.pending = None;
        state.done = None;
    }

    /// True while a render runs or is waiting to run.
    pub fn busy(&self) -> bool {
        let state = lock(&self.shared);
        state.in_flight || state.pending.is_some()
    }

    /// Newest finished frame of the current epoch.
    pub fn poll(&self) -> Option<NativeFrameResult> {
        let mut state = lock(&self.shared);
        let done = state.done.take()?;
        (done.epoch == state.epoch).then_some(done.result)
    }
}

impl Drop for PlayWorker {
    fn drop(&mut self) {
        lock(&self.shared).shutdown = true;
        self.shared.wake.notify_one();
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn run(shared: &Shared) {
    loop {
        let job = {
            let mut state = lock(shared);
            loop {
                if state.shutdown {
                    return;
                }
                if let Some(job) = state.pending.take() {
                    state.in_flight = true;
                    break job;
                }
                state = shared
                    .wake
                    .wait(state)
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
            }
        };
        let result = job
            .prepared
            .render(job.tag.tick, (job.tag.width, job.tag.height));
        let mut state = lock(shared);
        state.in_flight = false;
        if state.epoch == job.epoch {
            state.done = Some(Done {
                epoch: job.epoch,
                result: NativeFrameResult {
                    tag: job.tag,
                    result,
                },
            });
        }
    }
}
