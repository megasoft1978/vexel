//! Decode-ahead ring for playback preview.
//!
//! While playing, the next ticks are predictable, so their SVG snapshots are
//! patched and decoded on short-lived threads while the current tick renders.
//! The ring holds scene graphs keyed by `(generation, tick)`. A new generation
//! (seek, pause, edit, size or scene change) drops every entry and cancels
//! the decodes in flight, so a stale graph is never handed out. Decoding is
//! size independent and deterministic, so frames equal the serial path.
//!
//! Bounds: at most [`WINDOW`] entries (decoding or ready), at most
//! [`THREADS`] decodes at once, and ready graphs are capped at [`BUDGET`]
//! bytes of live scene memory.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::thread::Builder;

use vexel_scene::SceneGraph;

use super::prepared::Prepared;

/// Ticks decoded ahead of the one rendering.
pub const WINDOW: u32 = 3;
/// Concurrent decode threads (the renderer keeps its own band threads).
pub const THREADS: usize = 2;
/// Cap on the live bytes of ready scene graphs.
pub const BUDGET: usize = 192 * 1024 * 1024;

enum Slot {
    Decoding(Arc<AtomicBool>),
    Ready(Box<SceneGraph>, usize),
    Failed,
}

struct State {
    budget: usize,
    generation: u64,
    slots: HashMap<(u64, u32), Slot>,
    inflight: usize,
    ready_bytes: usize,
    last: Option<u32>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            budget: BUDGET,
            generation: 0,
            slots: HashMap::new(),
            inflight: 0,
            ready_bytes: 0,
            last: None,
        }
    }
}

#[derive(Default)]
pub struct Ahead {
    off: AtomicBool,
    #[cfg(test)]
    pub hits: std::sync::atomic::AtomicUsize,
    state: Mutex<State>,
    changed: Condvar,
}

fn lock(ahead: &Ahead) -> MutexGuard<'_, State> {
    ahead
        .state
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

impl State {
    fn drop_slot(&mut self, key: (u64, u32)) {
        match self.slots.remove(&key) {
            Some(Slot::Decoding(alive)) => alive.store(false, Ordering::SeqCst),
            Some(Slot::Ready(_, bytes)) => self.ready_bytes -= bytes,
            _ => {}
        }
    }
}

impl Ahead {
    pub fn set_enabled(&self, on: bool) {
        self.off.store(!on, Ordering::Relaxed);
        if !on {
            self.invalidate();
        }
    }

    /// New generation: every entry is dropped and every decode cancelled.
    pub fn invalidate(&self) {
        let mut state = lock(self);
        state.generation += 1;
        state.last = None;
        let keys: Vec<_> = state.slots.keys().copied().collect();
        for key in keys {
            state.drop_slot(key);
        }
        self.changed.notify_all();
    }

    /// The decoded graph of `tick` if the ring has or is producing it. Waits
    /// for a decode already running (it started earlier than a fresh one).
    pub fn take(&self, tick: u32) -> Option<Box<SceneGraph>> {
        if self.off.load(Ordering::Relaxed) {
            return None;
        }
        let mut state = lock(self);
        let key = (state.generation, tick);
        loop {
            match state.slots.get(&key) {
                Some(Slot::Decoding(_)) => {
                    state = self
                        .changed
                        .wait(state)
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                    if state.generation != key.0 {
                        return None;
                    }
                }
                Some(Slot::Ready(..)) => {
                    let Some(Slot::Ready(graph, bytes)) = state.slots.remove(&key) else {
                        return None;
                    };
                    state.ready_bytes -= bytes;
                    #[cfg(test)]
                    self.hits.fetch_add(1, Ordering::Relaxed);
                    return Some(graph);
                }
                Some(Slot::Failed) => {
                    state.slots.remove(&key);
                    return None;
                }
                None => return None,
            }
        }
    }

    /// After `tick` was requested: forgets entries outside the predicted next
    /// ticks and starts decodes for the missing ones (within the thread cap).
    pub fn plan(&self, prepared: &Arc<Prepared>, tick: u32) {
        if self.off.load(Ordering::Relaxed) {
            return;
        }
        let (duration, repeat) = (prepared.duration_ticks, prepared.repeats());
        let mut state = lock(self);
        let stride = match state.last {
            Some(last) if tick > last && tick - last <= WINDOW => tick - last,
            _ => 1,
        };
        state.last = Some(tick);
        let mut wanted = Vec::new();
        for step in 1..=WINDOW {
            let next = tick + stride * step;
            let next = if next < duration {
                next
            } else if repeat && duration > 0 {
                next % duration
            } else {
                break;
            };
            if next != tick && !wanted.contains(&next) {
                wanted.push(next);
            }
        }
        let generation = state.generation;
        let stale: Vec<_> = state
            .slots
            .keys()
            .copied()
            .filter(|(g, t)| *g != generation || !wanted.contains(t))
            .collect();
        for key in stale {
            state.drop_slot(key);
        }
        for next in wanted {
            let key = (generation, next);
            if state.slots.contains_key(&key)
                || state.inflight >= THREADS
                || state.ready_bytes >= state.budget
            {
                continue;
            }
            let alive = Arc::new(AtomicBool::new(true));
            state.slots.insert(key, Slot::Decoding(alive.clone()));
            state.inflight += 1;
            let worker = prepared.clone();
            let spawned = Builder::new()
                .name("vexel-decode-ahead".to_owned())
                .stack_size(16 * 1024 * 1024)
                .spawn(move || {
                    let cancel = || !alive.load(Ordering::SeqCst);
                    let graph = worker.decode_tick(next, &cancel);
                    worker.ahead().finish(key, graph);
                });
            if spawned.is_err() {
                state.inflight -= 1;
                state.slots.remove(&key);
                break;
            }
        }
    }

    pub(super) fn finish(&self, key: (u64, u32), graph: Option<SceneGraph>) {
        let mut state = lock(self);
        state.inflight -= 1;
        // Only a still-wanted entry of the current generation keeps the result.
        if let Some(Slot::Decoding(alive)) = state.slots.get(&key) {
            if alive.load(Ordering::SeqCst) {
                let slot = match graph {
                    Some(graph) => {
                        let bytes = graph.memory_stats().live_bytes;
                        if state.ready_bytes + bytes > state.budget {
                            Slot::Failed
                        } else {
                            state.ready_bytes += bytes;
                            Slot::Ready(Box::new(graph), bytes)
                        }
                    }
                    None => Slot::Failed,
                };
                state.slots.insert(key, slot);
            } else {
                state.slots.remove(&key);
            }
        }
        self.changed.notify_all();
    }

    /// (entries, ready bytes) held now.
    #[cfg(test)]
    pub fn held(&self) -> (usize, usize) {
        let state = lock(self);
        (state.slots.len(), state.ready_bytes)
    }
}

#[cfg(test)]
impl Ahead {
    pub fn set_budget(&self, bytes: usize) {
        lock(self).budget = bytes;
    }

    pub fn settle(&self) {
        let mut state = lock(self);
        while state.inflight > 0 {
            state = self
                .changed
                .wait(state)
                .unwrap_or_else(|poisoned| poisoned.into_inner());
        }
    }
}
