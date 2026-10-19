//! Bounded serial worker for automatic recovery capsule publication.

use crate::project_session::RecoverySnapshot;
use crate::project_session::SessionError;
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};

/// Result of publishing one immutable recovery snapshot.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct RecoveryJobResult {
    pub(crate) epoch: u64,
    pub(crate) revision: u64,
    /// True when the atomic replacement was published, even if durability
    /// could not subsequently be confirmed.
    pub(crate) published: bool,
    pub(crate) durability_uncertain: bool,
    pub(crate) error: Option<SessionError>,
}

type Task = Box<dyn FnOnce() -> RecoveryJobResult + Send + 'static>;

#[derive(Default)]
struct State {
    pending: Option<Task>,
    active: bool,
    paused: bool,
    stopping: bool,
    result: Option<RecoveryJobResult>,
}

struct Shared {
    state: Mutex<State>,
    changed: Condvar,
}

/// One serial publisher with a single replaceable pending job and one result slot.
pub(crate) struct RecoveryJob {
    shared: Arc<Shared>,
    thread: Option<JoinHandle<()>>,
}

/// RAII lifecycle barrier. It drops queued work and waits for active publication.
pub(crate) struct RecoveryPause {
    shared: Arc<Shared>,
    resumes_on_drop: bool,
}

impl RecoveryJob {
    pub(crate) fn spawn() -> Result<Self, SessionError> {
        Self::spawn_actor()
    }

    fn spawn_actor() -> Result<Self, SessionError> {
        let shared = Arc::new(Shared {
            state: Mutex::new(State::default()),
            changed: Condvar::new(),
        });
        let worker_shared = Arc::clone(&shared);
        let thread = thread::Builder::new()
            .name("vexel-recovery".to_owned())
            .spawn(move || worker_loop(worker_shared))
            .map_err(|_| SessionError::RecoveryUnavailable)?;
        Ok(Self {
            shared,
            thread: Some(thread),
        })
    }

    /// Replaces any waiting snapshot. Returns false while paused or stopping.
    pub(crate) fn submit(&self, snapshot: RecoverySnapshot) -> bool {
        self.submit_task(Box::new(move || snapshot.publish()))
    }

    fn submit_task(&self, task: Task) -> bool {
        let Ok(mut state) = self.shared.state.lock() else {
            return false;
        };
        if state.paused || state.stopping {
            return false;
        }
        state.pending = Some(task);
        self.shared.changed.notify_all();
        true
    }

    /// Queues a controlled task for in-crate lifecycle barrier tests.
    #[cfg(test)]
    pub(crate) fn submit_test_task(
        &self,
        task: impl FnOnce() -> RecoveryJobResult + Send + 'static,
    ) -> bool {
        self.submit_task(Box::new(task))
    }

    #[cfg(test)]
    fn submit_injected(&self, task: impl FnOnce() -> RecoveryJobResult + Send + 'static) -> bool {
        self.submit_task(Box::new(task))
    }

    /// Pauses selection, drops the pending snapshot, and waits for an active
    /// publication (including its durability result) to finish.
    pub(crate) fn pause(&self) -> RecoveryPause {
        self.pause_inner(|| {})
    }

    /// Test variant that acknowledges the paused-and-cleared state before it
    /// waits for the active publication to finish.
    #[cfg(test)]
    pub(crate) fn pause_test(&self, acknowledge: impl FnOnce()) -> RecoveryPause {
        self.pause_inner(acknowledge)
    }

    fn pause_inner(&self, acknowledge: impl FnOnce()) -> RecoveryPause {
        let mut resumes_on_drop = true;
        if let Ok(mut state) = self.shared.state.lock() {
            state.paused = true;
            state.pending = None;
            self.shared.changed.notify_all();
            acknowledge();
            while state.active {
                match self.shared.changed.wait(state) {
                    Ok(next) => state = next,
                    Err(_) => {
                        resumes_on_drop = false;
                        break;
                    }
                }
            }
        } else {
            resumes_on_drop = false;
        }
        RecoveryPause {
            shared: Arc::clone(&self.shared),
            resumes_on_drop,
        }
    }

    /// Takes the newest terminal result, if any.
    pub(crate) fn poll_result(&self) -> Option<RecoveryJobResult> {
        self.shared.state.lock().ok()?.result.take()
    }

    /// Stops accepting work, drops the pending snapshot, and joins the worker.
    pub(crate) fn shutdown(&mut self) {
        self.shutdown_inner(|| {});
    }

    /// Test variant that acknowledges the stopping-and-cleared state before
    /// joining the worker.
    #[cfg(test)]
    pub(crate) fn shutdown_test(&mut self, acknowledge: impl FnOnce()) {
        self.shutdown_inner(acknowledge);
    }

    fn shutdown_inner(&mut self, acknowledge: impl FnOnce()) {
        if let Ok(mut state) = self.shared.state.lock() {
            state.stopping = true;
            state.pending = None;
            self.shared.changed.notify_all();
            acknowledge();
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl Drop for RecoveryJob {
    fn drop(&mut self) {
        self.shutdown();
    }
}

impl Drop for RecoveryPause {
    fn drop(&mut self) {
        if !self.resumes_on_drop {
            return;
        }
        if let Ok(mut state) = self.shared.state.lock() {
            if !state.stopping {
                state.paused = false;
                self.shared.changed.notify_all();
            }
        }
    }
}

fn worker_loop(shared: Arc<Shared>) {
    loop {
        let task = {
            let Ok(state) = shared.state.lock() else {
                return;
            };
            let mut state = state;
            loop {
                if state.stopping {
                    return;
                }
                if !state.paused {
                    if let Some(task) = state.pending.take() {
                        state.active = true;
                        break task;
                    }
                }
                state = match shared.changed.wait(state) {
                    Ok(next) => next,
                    Err(_) => return,
                };
            }
        };

        let result = task();
        let Ok(mut state) = shared.state.lock() else {
            return;
        };
        state.active = false;
        state.result = Some(result);
        shared.changed.notify_all();
    }
}

#[cfg(test)]
#[path = "recovery_job_tests.rs"]
mod tests;
