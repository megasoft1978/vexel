#![forbid(unsafe_code)]
//! Bounded, concurrent storage for asynchronous MCP tasks.
//!
//! Results remain opaque bytes. The store owns one copy until retention or
//! expiry removes the task; [`TaskStore::get`] makes one fallible owned copy.
//! Mutex poisoning is reported as [`TaskStoreError::Poisoned`]; poisoned state
//! is never recovered or accessed. No user callback is invoked while locked.

use std::fmt;
use std::str::FromStr;
use std::sync::Mutex;

use vexel_platform_macos::{fill_random, SystemError};

pub const ACTIVE_LIMIT: usize = 2;
pub const RETAINED_LIMIT: usize = 8;
pub const TASK_LIMIT: usize = ACTIVE_LIMIT + RETAINED_LIMIT;
pub const RESULT_LIMIT: usize = 2 * 1024 * 1024;
pub const TASK_TTL_MS: u64 = 900_000;
pub const MESSAGE_LIMIT: usize = 127;
pub const TASK_ID_BYTES: usize = 32;
const ID_ATTEMPTS: usize = 8;

/// A validated lower-case 128-bit task identifier, rendered as 32 hex bytes.
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct TaskId([u8; TASK_ID_BYTES]);

impl TaskId {
    pub fn parse(value: &str) -> Result<Self, TaskStoreError> {
        if value.len() != TASK_ID_BYTES || !value.bytes().all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase()) {
            return Err(TaskStoreError::InvalidId);
        }
        let mut bytes = [0; TASK_ID_BYTES];
        bytes.copy_from_slice(value.as_bytes());
        Ok(Self(bytes))
    }

    pub fn as_bytes(&self) -> &[u8; TASK_ID_BYTES] { &self.0 }

    pub fn as_str(&self) -> &str {
        // The constructor accepts only ASCII lower-case hexadecimal bytes.
        std::str::from_utf8(&self.0).expect("TaskId invariant")
    }

    fn from_random(random: &[u8; 16]) -> Self {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut encoded = [0; TASK_ID_BYTES];
        for (index, byte) in random.iter().copied().enumerate() {
            encoded[index * 2] = HEX[(byte >> 4) as usize];
            encoded[index * 2 + 1] = HEX[(byte & 0x0f) as usize];
        }
        Self(encoded)
    }
}

impl fmt::Debug for TaskId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.debug_tuple("TaskId").field(&self.as_str()).finish() }
}

impl fmt::Display for TaskId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str(self.as_str()) }
}

impl FromStr for TaskId {
    type Err = TaskStoreError;
    fn from_str(value: &str) -> Result<Self, Self::Err> { Self::parse(value) }
}

/// Task lifecycle values corresponding to the original MCP task store.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TaskStatus { Working, Completed, Cancelled, Failed }

impl TaskStatus {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Working => "working",
            Self::Completed => "completed",
            Self::Cancelled => "cancelled",
            Self::Failed => "failed",
        }
    }

    fn is_terminal(self) -> bool { self != Self::Working }
}

/// Fixed-size UTF-8 task message (at most 127 source bytes).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TaskMessage { bytes: [u8; MESSAGE_LIMIT], length: u8 }

impl TaskMessage {
    fn from_bytes(value: &[u8]) -> Result<Self, TaskStoreError> {
        let value = value.split(|byte| *byte == 0).next().unwrap_or(&[]);
        if value.len() > MESSAGE_LIMIT { return Err(TaskStoreError::MessageTooLong); }
        let mut bytes = [0; MESSAGE_LIMIT];
        bytes[..value.len()].copy_from_slice(value);
        Ok(Self { bytes, length: value.len() as u8 })
    }

    pub fn as_bytes(&self) -> &[u8] { &self.bytes[..usize::from(self.length)] }
    pub fn as_str(&self) -> Option<&str> { std::str::from_utf8(self.as_bytes()).ok() }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TaskSnapshot {
    pub id: TaskId,
    pub message: TaskMessage,
    pub status: TaskStatus,
    pub owner: u64,
    pub created_ms: u64,
    pub updated_ms: u64,
    pub expires_ms: u64,
    pub progress_permille: u16,
    pub result_is_error: bool,
    pub result_length: usize,
}

/// An independently owned opaque result returned together with its snapshot.
#[derive(Debug, Eq, PartialEq)]
pub struct TaskResult { pub bytes: Vec<u8> }

#[derive(Debug, Eq, PartialEq)]
pub enum TaskStoreError {
    InvalidOwner,
    InvalidId,
    InvalidProgress,
    InvalidStatus,
    MessageTooLong,
    ResultTooLarge { length: usize, maximum: usize },
    ResultCapacity { length: usize, capacity: usize },
    NotFound,
    ActiveLimit,
    IdentifierCollision,
    Allocation,
    Entropy(SystemError),
    Poisoned,
}

impl fmt::Display for TaskStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{self:?}") }
}
impl std::error::Error for TaskStoreError {}

#[derive(Debug)]
struct Entry {
    id: TaskId,
    message: TaskMessage,
    status: TaskStatus,
    owner: u64,
    created_ms: u64,
    updated_ms: u64,
    expires_ms: u64,
    progress_permille: u16,
    cancel_requested: bool,
    result_is_error: bool,
    result: Vec<u8>,
}

impl Entry {
    fn snapshot(&self) -> TaskSnapshot {
        TaskSnapshot {
            id: self.id, message: self.message, status: self.status, owner: self.owner,
            created_ms: self.created_ms, updated_ms: self.updated_ms, expires_ms: self.expires_ms,
            progress_permille: self.progress_permille, result_is_error: self.result_is_error,
            result_length: self.result.len(),
        }
    }
}

/// Thread-safe bounded task storage. Ten stable slots match the original
/// first-free-slot reuse order; only task results allocate, with fallible
/// reservation and a strict 2 MiB per-result limit.
pub struct TaskStore { entries: Mutex<Vec<Option<Entry>>> }

impl TaskStore {
    pub fn new() -> Result<Self, TaskStoreError> {
        let mut entries = Vec::new();
        entries.try_reserve_exact(TASK_LIMIT).map_err(|_| TaskStoreError::Allocation)?;
        entries.resize_with(TASK_LIMIT, || None);
        Ok(Self { entries: Mutex::new(entries) })
    }

    /// Start a task using Apple's native cryptographic random source.
    pub fn start(&self, owner: u64, now_ms: u64) -> Result<TaskId, TaskStoreError> {
        if owner == 0 { return Err(TaskStoreError::InvalidOwner); }
        self.start_with_entropy(owner, now_ms, |bytes| fill_random(bytes))
    }

    fn start_with_entropy<F>(&self, owner: u64, now_ms: u64, mut entropy: F) -> Result<TaskId, TaskStoreError>
    where F: FnMut(&mut [u8; 16]) -> Result<(), SystemError> {
        if owner == 0 { return Err(TaskStoreError::InvalidOwner); }
        let default_message = TaskMessage::from_bytes(b"Preparing export")?;
        {
            let mut entries = self.entries.lock().map_err(|_| TaskStoreError::Poisoned)?;
            expire(&mut entries, now_ms);
            let active = entries.iter().flatten().filter(|entry| entry.status == TaskStatus::Working).count();
            if active >= ACTIVE_LIMIT {
                evict_oldest_terminal_if_full(&mut entries);
                return Err(TaskStoreError::ActiveLimit);
            }
        }
        for _ in 0..ID_ATTEMPTS {
            let mut random = [0; 16];
            entropy(&mut random).map_err(TaskStoreError::Entropy)?;
            let id = TaskId::from_random(&random);
            let mut entries = self.entries.lock().map_err(|_| TaskStoreError::Poisoned)?;
            expire(&mut entries, now_ms);
            let active = entries.iter().flatten().filter(|entry| entry.status == TaskStatus::Working).count();
            if entries.iter().flatten().any(|entry| entry.id == id) { continue; }
            evict_oldest_terminal_if_full(&mut entries);
            if active >= ACTIVE_LIMIT { return Err(TaskStoreError::ActiveLimit); }
            let slot = entries.iter().position(Option::is_none).ok_or(TaskStoreError::ActiveLimit)?;
            let expires_ms = now_ms.checked_add(TASK_TTL_MS).unwrap_or(u64::MAX);
            entries[slot] = Some(Entry {
                id, message: default_message, status: TaskStatus::Working, owner,
                created_ms: now_ms, updated_ms: now_ms, expires_ms, progress_permille: 0,
                cancel_requested: false, result_is_error: false, result: Vec::new(),
            });
            return Ok(id);
        }
        Err(TaskStoreError::IdentifierCollision)
    }

    pub fn update(&self, owner: u64, id: &TaskId, now_ms: u64, progress_permille: u16, message: &[u8]) -> Result<(), TaskStoreError> {
        if progress_permille > 1000 { return Err(TaskStoreError::InvalidProgress); }
        let message = TaskMessage::from_bytes(message)?;
        let mut entries = self.entries.lock().map_err(|_| TaskStoreError::Poisoned)?;
        expire(&mut entries, now_ms);
        let entry = find_mut(&mut entries, owner, id).ok_or(TaskStoreError::NotFound)?;
        if entry.status != TaskStatus::Working || progress_permille < entry.progress_permille { return Err(TaskStoreError::InvalidProgress); }
        entry.progress_permille = progress_permille;
        entry.updated_ms = now_ms;
        entry.message = message;
        Ok(())
    }

    pub fn request_cancel(&self, owner: u64, id: &TaskId, now_ms: u64) -> Result<(), TaskStoreError> {
        let mut entries = self.entries.lock().map_err(|_| TaskStoreError::Poisoned)?;
        expire(&mut entries, now_ms);
        let entry = find_mut(&mut entries, owner, id).ok_or(TaskStoreError::NotFound)?;
        if entry.status != TaskStatus::Working { return Err(TaskStoreError::NotFound); }
        entry.cancel_requested = true;
        entry.updated_ms = now_ms;
        entry.message = TaskMessage::from_bytes(b"Cancellation requested")?;
        Ok(())
    }

    /// Unknown, owner-mismatched, terminal, or already cancelled tasks report
    /// cancellation, matching the original cooperative worker polling rule.
    pub fn cancel_requested(&self, owner: u64, id: &TaskId) -> Result<bool, TaskStoreError> {
        let entries = self.entries.lock().map_err(|_| TaskStoreError::Poisoned)?;
        Ok(entries.iter().flatten().find(|entry| entry.owner == owner && entry.id == *id)
            .is_none_or(|entry| entry.cancel_requested || entry.status != TaskStatus::Working))
    }

    pub fn finish(&self, owner: u64, id: &TaskId, now_ms: u64, status: TaskStatus, message: &[u8], result: &[u8], result_is_error: bool) -> Result<(), TaskStoreError> {
        if !status.is_terminal() { return Err(TaskStoreError::InvalidStatus); }
        if result.len() > RESULT_LIMIT { return Err(TaskStoreError::ResultTooLarge { length: result.len(), maximum: RESULT_LIMIT }); }
        let message = TaskMessage::from_bytes(message)?;
        let mut owned_result = Vec::new();
        owned_result.try_reserve_exact(result.len()).map_err(|_| TaskStoreError::Allocation)?;
        owned_result.extend_from_slice(result);
        let mut entries = self.entries.lock().map_err(|_| TaskStoreError::Poisoned)?;
        let entry = find_mut(&mut entries, owner, id).ok_or(TaskStoreError::NotFound)?;
        if entry.status != TaskStatus::Working { return Err(TaskStoreError::NotFound); }
        entry.status = status;
        entry.updated_ms = now_ms;
        entry.expires_ms = now_ms.checked_add(TASK_TTL_MS).unwrap_or(u64::MAX);
        if status == TaskStatus::Completed { entry.progress_permille = 1000; }
        entry.message = message;
        entry.result = owned_result;
        entry.result_is_error = result_is_error;
        Ok(())
    }

    /// Return a consistent snapshot and one owned opaque result copy. Capacity
    /// is checked before allocation or copying; errors expose no partial data.
    pub fn get(&self, owner: u64, id: &TaskId, now_ms: u64, result_capacity: usize) -> Result<(TaskSnapshot, TaskResult), TaskStoreError> {
        let mut entries = self.entries.lock().map_err(|_| TaskStoreError::Poisoned)?;
        expire(&mut entries, now_ms);
        let entry = entries.iter().flatten().find(|entry| entry.owner == owner && entry.id == *id).ok_or(TaskStoreError::NotFound)?;
        if entry.result.len() > result_capacity {
            return Err(TaskStoreError::ResultCapacity { length: entry.result.len(), capacity: result_capacity });
        }
        let snapshot = entry.snapshot();
        let mut bytes = Vec::new();
        bytes.try_reserve_exact(entry.result.len()).map_err(|_| TaskStoreError::Allocation)?;
        bytes.extend_from_slice(&entry.result);
        Ok((snapshot, TaskResult { bytes }))
    }
}

fn find_mut<'a>(entries: &'a mut [Option<Entry>], owner: u64, id: &TaskId) -> Option<&'a mut Entry> {
    entries.iter_mut().filter_map(Option::as_mut).find(|entry| entry.owner == owner && entry.id == *id)
}

fn evict_oldest_terminal_if_full(entries: &mut [Option<Entry>]) {
    let terminal_count = entries.iter().flatten().filter(|entry| entry.status.is_terminal()).count();
    if terminal_count < RETAINED_LIMIT { return; }
    if let Some((index, _)) = entries.iter().enumerate()
        .filter_map(|(index, slot)| slot.as_ref().map(|entry| (index, entry)))
        .filter(|(_, entry)| entry.status.is_terminal())
        .min_by_key(|(index, entry)| (entry.updated_ms, *index))
    {
        entries[index] = None;
    }
}

fn expire(entries: &mut [Option<Entry>], now_ms: u64) {
    for slot in entries {
        let Some(entry) = slot.as_mut() else { continue; };
        if entry.status.is_terminal() && now_ms >= entry.expires_ms {
            *slot = None;
        } else if entry.status == TaskStatus::Working && now_ms >= entry.expires_ms {
            entry.cancel_requested = true;
            entry.message = TaskMessage::from_bytes(b"Task reached its time limit; stopping").expect("bounded static task message");
            entry.updated_ms = now_ms;
        }
    }
}

#[cfg(test)]
mod tests;
