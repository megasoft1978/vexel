use std::collections::VecDeque;
use std::cell::Cell;
use std::fmt;
use std::io;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::thread::{self, JoinHandle};

use vexel_base::{AllocationGate, BaseStatus, SystemAllocationGate};

const MAX_CAPACITY: usize = 64;
const MAX_WORKERS: usize = 4;

thread_local! {
    static CURRENT_WORKER_QUEUE: Cell<usize> = const { Cell::new(0) };
}

/// Queue limits mirror `platform_task.h` and are validated before allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueueConfig {
    pub capacity: usize,
    pub workers: usize,
}

/// Typed outcomes matching the native queue's status priority and meanings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueError {
    Invalid,
    Full,
    Duplicate,
    Stale,
    NotFound,
    NoMemory,
    Stopped,
}

/// Value reported for a callback, including ordinary callback failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaskOutput {
    pub succeeded: bool,
    pub value: i64,
}

impl TaskOutput {
    pub const fn success(value: i64) -> Self {
        Self { succeeded: true, value }
    }

    pub const fn failure(value: i64) -> Self {
        Self { succeeded: false, value }
    }
}

/// A retired callback result. Polling consumes it and releases its task ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaskResult {
    pub id: u64,
    pub generation: u64,
    pub succeeded: bool,
    pub value: i64,
}

/// Callback-lifetime cooperative cancellation signal.
#[derive(Debug, Clone)]
pub struct CancellationToken<'a>(&'a AtomicBool);

impl CancellationToken<'_> {
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

trait ErasedTask: Send {
    fn invoke(&mut self, token: CancellationToken<'_>) -> TaskOutput;
}

struct OwnedTask<C, F> {
    context: C,
    callback: Option<F>,
}

impl<C, F> ErasedTask for OwnedTask<C, F>
where
    C: Send + 'static,
    F: for<'a> FnOnce(&mut C, CancellationToken<'a>) -> TaskOutput + Send + 'static,
{
    fn invoke(&mut self, token: CancellationToken<'_>) -> TaskOutput {
        (self.callback.take().expect("task callback runs once"))(&mut self.context, token)
    }
}

/// Type-erased owned task context and its one-shot callback.
///
/// The context remains owned by this value until the queue publishes or
/// discards the task, then it is dropped once outside the queue mutex.
pub struct TaskCallback(Box<dyn ErasedTask>);

impl TaskCallback {
    pub fn new<C, F>(context: C, callback: F) -> Self
    where
        C: Send + 'static,
        F: for<'a> FnOnce(&mut C, CancellationToken<'a>) -> TaskOutput + Send + 'static,
    {
        Self(Box::new(OwnedTask { context, callback: Some(callback) }))
    }

    fn invoke(&mut self, token: CancellationToken<'_>) -> TaskOutput {
        self.0.invoke(token)
    }
}

type Callback = TaskCallback;

#[derive(Clone, Copy, PartialEq, Eq)]
enum SlotState {
    Empty,
    Queued,
    CanceledQueued,
    Running,
    Complete,
    Discarded,
}

struct Slot {
    state: SlotState,
    id: u64,
    generation: u64,
    callback: Option<Callback>,
    result: Option<TaskResult>,
}

impl Slot {
    fn empty() -> Self {
        Self {
            state: SlotState::Empty,
            id: 0,
            generation: 0,
            callback: None,
            result: None,
        }
    }

    fn clear(&mut self) {
        self.state = SlotState::Empty;
        self.id = 0;
        self.generation = 0;
        self.result = None;
    }
}

struct State {
    slots: Vec<Slot>,
    pending: VecDeque<usize>,
    generation: u64,
    stopping: bool,
}

struct Shared {
    state: Mutex<State>,
    changed: Condvar,
    capacity: usize,
    cancellations: [AtomicBool; MAX_CAPACITY],
}

/// A bounded worker queue. Clones share the same queue and shutdown lifecycle.
///
/// Callbacks and their captured owned context are dropped outside the queue
/// mutex. A rejected `submit` returns its original callback in the error so its
/// owner keeps every captured value.
pub struct TaskQueue {
    shared: Arc<Shared>,
    workers: Arc<Mutex<Vec<JoinHandle<()>>>>,
}

impl fmt::Debug for TaskQueue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_struct("TaskQueue").field("capacity", &self.shared.capacity)
            .field("generation", &self.generation()).finish()
    }
}

impl Clone for TaskQueue {
    fn clone(&self) -> Self {
        Self { shared: Arc::clone(&self.shared), workers: Arc::clone(&self.workers) }
    }
}

/// Failed submission retains the callback and therefore all captured context.
pub struct SubmitError {
    pub status: QueueError,
    pub callback: TaskCallback,
}

impl fmt::Debug for SubmitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_struct("SubmitError").field("status", &self.status)
            .field("callback", &"<owned callback>").finish()
    }
}

impl TaskQueue {
    pub fn new(config: QueueConfig) -> Result<Self, QueueError> {
        Self::with_allocation_gate(config, &mut SystemAllocationGate)
    }

    fn with_allocation_gate(
        config: QueueConfig,
        gate: &mut dyn AllocationGate,
    ) -> Result<Self, QueueError> {
        Self::with_allocation_gate_and_spawn(config, gate, spawn_worker)
    }

    fn with_allocation_gate_and_spawn(
        config: QueueConfig,
        gate: &mut dyn AllocationGate,
        mut spawn: impl FnMut(Arc<Shared>) -> io::Result<JoinHandle<()>>,
    ) -> Result<Self, QueueError> {
        if !(1..=MAX_CAPACITY).contains(&config.capacity)
            || !(1..=MAX_WORKERS).contains(&config.workers)
        {
            return Err(QueueError::Invalid);
        }

        let slot_bytes = config.capacity.checked_mul(std::mem::size_of::<Slot>())
            .ok_or(QueueError::NoMemory)?;
        let pending_bytes = config.capacity.checked_mul(std::mem::size_of::<usize>())
            .ok_or(QueueError::NoMemory)?;
        let worker_bytes = config.workers.checked_mul(std::mem::size_of::<JoinHandle<()>>())
            .ok_or(QueueError::NoMemory)?;
        let shared_bytes = std::mem::size_of::<Shared>().checked_add(std::mem::size_of::<State>())
            .ok_or(QueueError::NoMemory)?;
        let worker_owner_bytes = std::mem::size_of::<Mutex<Vec<JoinHandle<()>>>>();
        permit(gate, slot_bytes)?;
        let mut slots = Vec::new();
        slots.try_reserve_exact(config.capacity).map_err(|_| QueueError::NoMemory)?;
        slots.resize_with(config.capacity, Slot::empty);
        permit(gate, pending_bytes)?;
        let mut pending = VecDeque::new();
        pending.try_reserve_exact(config.capacity).map_err(|_| QueueError::NoMemory)?;
        permit(gate, worker_bytes)?;
        let mut handles = Vec::new();
        handles.try_reserve_exact(config.workers).map_err(|_| QueueError::NoMemory)?;
        permit(gate, shared_bytes)?;
        let shared = Arc::new(Shared {
            state: Mutex::new(State { slots, pending, generation: 1, stopping: false }),
            changed: Condvar::new(),
            capacity: config.capacity,
            cancellations: std::array::from_fn(|_| AtomicBool::new(false)),
        });
        permit(gate, worker_owner_bytes)?;
        let worker_set = Arc::new(Mutex::new(handles));
        for _ in 0..config.workers {
            let worker_shared = Arc::clone(&shared);
            match spawn(worker_shared) {
                Ok(handle) => lock(&worker_set).push(handle),
                Err(_) => {
                    {
                        let mut state = lock(&shared.state);
                        state.stopping = true;
                        shared.changed.notify_all();
                    }
                    for handle in lock(&worker_set).drain(..) { let _ = handle.join(); }
                    return Err(QueueError::NoMemory);
                }
            }
        }
        Ok(Self { shared, workers: worker_set })
    }

    /// Submit an owned, `'static` boxed callback. Its captures are task context;
    /// on rejection the same box is returned and is never invoked.
    pub fn submit(&self, id: u64, generation: u64, callback: TaskCallback) -> Result<(), SubmitError> {
        let mut state = lock(&self.shared.state);
        let index = match available_slot(&state, self.shared.capacity, id, generation) {
            Ok(index) => index,
            Err(status) => return Err(SubmitError { status, callback }),
        };
        self.shared.cancellations[index].store(false, Ordering::Release);
        install(&mut state, index, id, generation, callback);
        self.shared.changed.notify_one();
        Ok(())
    }

    pub fn cancel(&self, id: u64) -> Result<(), QueueError> {
        if id == 0 { return Err(QueueError::Invalid); }
        let mut state = lock(&self.shared.state);
        let Some((index, slot)) = state.slots.iter_mut().enumerate().find(|(_, slot)| {
            slot.state != SlotState::Empty && slot.state != SlotState::Discarded && slot.id == id
        }) else { return Err(QueueError::NotFound); };
        self.shared.cancellations[index].store(true, Ordering::Release);
        match slot.state {
            SlotState::Queued => slot.state = SlotState::CanceledQueued,
            SlotState::Complete => slot.state = SlotState::Discarded,
            _ => {}
        }
        self.shared.changed.notify_all();
        Ok(())
    }

    pub fn advance_generation(&self, generation: u64) -> Result<(), QueueError> {
        let mut state = lock(&self.shared.state);
        if generation <= state.generation { return Err(QueueError::Stale); }
        state.generation = generation;
        for (index, slot) in state.slots.iter_mut().enumerate() {
            if slot.state != SlotState::Empty && slot.generation < generation {
                match slot.state {
                    SlotState::Queued => {
                        self.shared.cancellations[index].store(true, Ordering::Release);
                        slot.state = SlotState::CanceledQueued;
                    }
                    SlotState::Complete => slot.state = SlotState::Discarded,
                    SlotState::Running => self.shared.cancellations[index].store(true, Ordering::Release),
                    _ => {}
                }
            }
        }
        self.shared.changed.notify_all();
        Ok(())
    }

    /// Consume one current-generation completion and release its ID.
    pub fn poll(&self) -> Option<TaskResult> {
        let mut state = lock(&self.shared.state);
        let generation = state.generation;
        let slot = state.slots.iter_mut().find(|slot| {
            slot.state == SlotState::Complete && slot.generation == generation
        })?;
        let result = slot.result.take();
        slot.clear();
        result
    }

    /// Request cooperative cancellation. External callers join every worker;
    /// a reentrant call from a worker requests stop and returns to avoid
    /// self-join. An uncooperative callback can delay an external caller.
    pub fn shutdown(&self) {
        self.request_shutdown();
        let queue_id = Arc::as_ptr(&self.shared) as usize;
        if CURRENT_WORKER_QUEUE.with(|current| current.get() == queue_id) { return; }

        let mut workers = lock(&self.workers);
        for handle in workers.drain(..) { let _ = handle.join(); }
    }

    fn request_shutdown(&self) {
        {
            let mut state = lock(&self.shared.state);
            state.stopping = true;
            for (index, slot) in state.slots.iter_mut().enumerate() {
                match slot.state {
                    SlotState::Queued => {
                        self.shared.cancellations[index].store(true, Ordering::Release);
                        slot.state = SlotState::CanceledQueued;
                    }
                    SlotState::Complete => slot.state = SlotState::Discarded,
                    SlotState::Running => self.shared.cancellations[index].store(true, Ordering::Release),
                    _ => {}
                }
            }
            self.shared.changed.notify_all();
        }
    }

    pub fn generation(&self) -> u64 {
        lock(&self.shared.state).generation
    }

    pub fn capacity(&self) -> usize { self.shared.capacity }

}

impl Drop for TaskQueue {
    fn drop(&mut self) {
        if Arc::strong_count(&self.workers) == 1 { self.shutdown(); }
    }
}

fn available_slot(state: &State, capacity: usize, id: u64, generation: u64) -> Result<usize, QueueError> {
    if id == 0 { return Err(QueueError::Invalid); }
    let mut free = None;
    for (index, slot) in state.slots.iter().enumerate() {
        if slot.state != SlotState::Empty && slot.state != SlotState::Discarded && slot.id == id {
            return Err(QueueError::Duplicate);
        }
        if free.is_none() && (slot.state == SlotState::Empty || slot.state == SlotState::Discarded) {
            free = Some(index);
        }
    }
    if generation != state.generation { return Err(QueueError::Stale); }
    if state.stopping { return Err(QueueError::Stopped); }
    let Some(index) = free else { return Err(QueueError::Full); };
    if state.pending.len() == capacity { return Err(QueueError::Full); }
    Ok(index)
}

fn install(state: &mut State, index: usize, id: u64, generation: u64, callback: Callback) {
    let slot = &mut state.slots[index];
    slot.id = id;
    slot.generation = generation;
    slot.result = None;
    slot.callback = Some(callback);
    slot.state = SlotState::Queued;
    state.pending.push_back(index);
}

fn permit(gate: &mut dyn AllocationGate, bytes: usize) -> Result<(), QueueError> {
    gate.permit(bytes).map_err(|status| match status {
        BaseStatus::OutOfMemory | BaseStatus::Overflow | BaseStatus::Limit => QueueError::NoMemory,
        BaseStatus::Invalid => QueueError::Invalid,
    })
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn spawn_worker(shared: Arc<Shared>) -> io::Result<JoinHandle<()>> {
    thread::Builder::new().spawn(move || worker_loop(shared))
}

fn worker_loop(shared: Arc<Shared>) {
    let queue_id = Arc::as_ptr(&shared) as usize;
    CURRENT_WORKER_QUEUE.with(|current| current.set(queue_id));
    let _marker = WorkerMarker(queue_id);
    loop {
        let next = {
            let mut state = lock(&shared.state);
            loop {
                if let Some(index) = state.pending.pop_front() {
                    let stopping = state.stopping;
                    let generation = state.generation;
                    let slot = &mut state.slots[index];
                    if slot.state == SlotState::CanceledQueued || stopping {
                        slot.state = SlotState::Discarded;
                        let callback = slot.callback.take();
                        let token = CancellationToken(&shared.cancellations[index]);
                        break Some((index, slot.id, callback, token, false));
                    }
                    if slot.state == SlotState::Queued && slot.generation == generation {
                        slot.state = SlotState::Running;
                        let callback = slot.callback.take();
                        let token = CancellationToken(&shared.cancellations[index]);
                        break Some((index, slot.id, callback, token, true));
                    }
                    slot.state = SlotState::Discarded;
                    let callback = slot.callback.take();
                    let token = CancellationToken(&shared.cancellations[index]);
                    break Some((index, slot.id, callback, token, false));
                }
                if state.stopping { break None; }
                state = shared.changed.wait(state).unwrap_or_else(std::sync::PoisonError::into_inner);
            }
        };
        let Some((index, id, callback, token, invoke_callback)) = next else { return; };
        let Some(mut callback) = callback else { continue; };
        if !invoke_callback {
            {
                let mut state = lock(&shared.state);
                state.slots[index].clear();
            }
            drop(callback);
            continue;
        }
        #[cfg(not(test))]
        let _ = id;
        #[cfg(test)]
        pause_before_callback(id);
        let cancel_flag = &shared.cancellations[index];
        let output = catch_unwind(AssertUnwindSafe(|| callback.invoke(token)))
            .unwrap_or_else(|_| TaskOutput::failure(0));
        let mut state = lock(&shared.state);
        let generation = state.generation;
        let stopping = state.stopping;
        let slot = &mut state.slots[index];
        if slot.generation == generation && !cancel_flag.load(Ordering::Acquire) && !stopping {
            slot.result = Some(TaskResult {
                id: slot.id,
                generation: slot.generation,
                succeeded: output.succeeded,
                value: output.value,
            });
            slot.state = SlotState::Complete;
        } else {
            slot.clear();
        }
        shared.changed.notify_all();
        drop(state);
        drop(callback);
    }
}

#[cfg(test)]
struct CallbackEntryGate {
    id: u64,
    entered: std::sync::mpsc::Sender<()>,
    resume: Mutex<std::sync::mpsc::Receiver<()>>,
}

#[cfg(test)]
fn callback_entry_gate() -> &'static Mutex<Option<Arc<CallbackEntryGate>>> {
    static GATE: std::sync::OnceLock<Mutex<Option<Arc<CallbackEntryGate>>>> = std::sync::OnceLock::new();
    GATE.get_or_init(|| Mutex::new(None))
}

#[cfg(test)]
fn arm_callback_entry_gate(id: u64) -> (std::sync::mpsc::Receiver<()>, std::sync::mpsc::Sender<()>) {
    let (entered_tx, entered_rx) = std::sync::mpsc::channel();
    let (resume_tx, resume_rx) = std::sync::mpsc::channel();
    *lock(callback_entry_gate()) = Some(Arc::new(CallbackEntryGate {
        id,
        entered: entered_tx,
        resume: Mutex::new(resume_rx),
    }));
    (entered_rx, resume_tx)
}

#[cfg(test)]
fn pause_before_callback(id: u64) {
    let gate = {
        let mut pending = lock(callback_entry_gate());
        if pending.as_ref().is_some_and(|gate| gate.id == id) { pending.take() } else { None }
    };
    if let Some(gate) = gate {
        gate.entered.send(()).expect("callback gate observer dropped");
        lock(&gate.resume).recv().expect("callback gate was not released");
    }
}

struct WorkerMarker(usize);

impl Drop for WorkerMarker {
    fn drop(&mut self) {
        CURRENT_WORKER_QUEUE.with(|current| {
            if current.get() == self.0 { current.set(0); }
        });
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
