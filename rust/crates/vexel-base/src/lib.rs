#![forbid(unsafe_code)]
//! Checked byte spans and a bounded, resettable arena for the owned Rust port.
//!
//! Arena storage is owned by aligned blocks. Handles are offsets into chunks,
//! never pointers, so moving an arena does not invalidate them and Rust borrows
//! prevent access while the arena is being changed.

use std::sync::atomic::{AtomicU64, Ordering};

mod work_control;
pub use work_control::{BudgetControl, ControlledError, ControlledStep, Stop, WorkControl, WorkKind, WorkLimits, WorkStats};

#[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
compile_error!("Vexel initially supports Apple Silicon macOS 13+ only");

/// Error outcomes corresponding to the non-success C `base_status` values.
/// Success is represented by `Ok(…)` in Rust.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BaseStatus {
    Invalid,
    Overflow,
    Limit,
    OutOfMemory,
}

/// A checked half-open range into source bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    /// Create a span from an offset and length, returning `None` on overflow.
    pub fn from_offset_length(offset: usize, length: usize) -> Option<Self> {
        Some(Self {
            start: offset,
            end: offset.checked_add(length)?,
        })
    }

    /// Borrow this span from `source` if both endpoints are in bounds.
    pub fn slice(self, source: &[u8]) -> Option<&[u8]> {
        source.get(self.start..self.end)
    }
}

/// Return `a + b`, or `None` if the result cannot fit in `usize`.
pub fn size_add(a: usize, b: usize) -> Option<usize> {
    a.checked_add(b)
}

/// Return `a * b`, or `None` if the result cannot fit in `usize`.
pub fn size_mul(a: usize, b: usize) -> Option<usize> {
    a.checked_mul(b)
}

/// Borrowed immutable byte slice; empty slices may naturally have no address.
pub type Bytes<'a> = &'a [u8];
/// Borrowed mutable byte slice.
pub type MutBytes<'a> = &'a mut [u8];

/// Take a checked subrange. Empty ranges are valid, including at the end.
pub fn bytes_sub(bytes: Bytes<'_>, offset: usize, length: usize) -> Option<Bytes<'_>> {
    let end = offset.checked_add(length)?;
    bytes.get(offset..end)
}

/// Take a checked mutable subrange. Empty ranges are valid, including at the end.
pub fn mut_bytes_sub(
    bytes: MutBytes<'_>,
    offset: usize,
    length: usize,
) -> Option<MutBytes<'_>> {
    let end = offset.checked_add(length)?;
    bytes.get_mut(offset..end)
}

/// Compare byte slices, with length mismatch handled before content comparison.
pub fn bytes_equal(left: Bytes<'_>, right: Bytes<'_>) -> bool {
    left == right
}

/// Ask permission for each backing allocation before attempting it.
///
/// This seam supports deterministic failure injection and bounded allocators.
/// It does not allocate memory itself; the arena separately uses Rust's
/// fallible `try_reserve_exact` API. Return `OutOfMemory` to reject an attempt.
pub trait AllocationGate {
    fn permit(&mut self, requested_bytes: usize) -> Result<(), BaseStatus>;
}

#[derive(Default)]
#[doc(hidden)]
pub struct SystemAllocationGate;

impl AllocationGate for SystemAllocationGate {
    fn permit(&mut self, _requested_bytes: usize) -> Result<(), BaseStatus> {
        Ok(())
    }
}

const MAX_ALIGNMENT: usize = 4096;
static NEXT_ARENA_ID: AtomicU64 = AtomicU64::new(1);

// Each element begins on a 4096-byte boundary, sufficient for every supported
// arena alignment. Vec's element layout carries this alignment contract.
#[repr(align(4096))]
struct AlignedBlock([u8; MAX_ALIGNMENT]);

struct Chunk {
    blocks: Vec<AlignedBlock>,
    capacity: usize,
    used: usize,
}

impl Chunk {
    fn try_new(capacity: usize, gate: &mut dyn AllocationGate) -> Result<Self, BaseStatus> {
        let block_count = capacity
            .checked_add(MAX_ALIGNMENT - 1)
            .ok_or(BaseStatus::Overflow)?
            / MAX_ALIGNMENT;
        let allocation_bytes = block_count
            .checked_mul(std::mem::size_of::<AlignedBlock>())
            .ok_or(BaseStatus::Overflow)?;
        gate.permit(allocation_bytes)?;
        let mut blocks = Vec::new();
        blocks
            .try_reserve_exact(block_count)
            .map_err(|_| BaseStatus::OutOfMemory)?;
        blocks.resize_with(block_count, || AlignedBlock([0; MAX_ALIGNMENT]));
        Ok(Self {
            blocks,
            capacity,
            used: 0,
        })
    }

}

/// An opaque allocation identity and byte range within an arena.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArenaHandle {
    arena_id: u64,
    chunk: usize,
    offset: usize,
    length: usize,
}

impl ArenaHandle {
    /// Length requested for this allocation.
    pub fn len(self) -> usize {
        self.length
    }

    /// Whether this handle describes an empty allocation.
    pub fn is_empty(self) -> bool {
        self.length == 0
    }
}

/// A borrowed, possibly segmented view of one arena allocation.
///
/// Segments arise because safe standard-library storage uses 4096-aligned
/// blocks; no pointer reinterpretation is needed to expose initialized bytes.
pub struct ArenaSlice<'a> {
    blocks: &'a [AlignedBlock],
    offset: usize,
    length: usize,
}

impl ArenaSlice<'_> {
    pub fn len(&self) -> usize { self.length }
    pub fn is_empty(&self) -> bool { self.length == 0 }
    pub fn get(&self, index: usize) -> Option<u8> {
        if index >= self.length { return None; }
        let absolute = self.offset.checked_add(index)?;
        self.blocks.get(absolute / MAX_ALIGNMENT)?.0.get(absolute % MAX_ALIGNMENT).copied()
    }
    pub fn copy_to(&self, destination: &mut [u8]) -> Option<()> {
        if destination.len() != self.length { return None; }
        for (index, byte) in destination.iter_mut().enumerate() { *byte = self.get(index)?; }
        Some(())
    }
}

/// A mutable borrowed, possibly segmented view of one arena allocation.
pub struct ArenaSliceMut<'a> {
    blocks: &'a mut [AlignedBlock],
    offset: usize,
    length: usize,
}

impl ArenaSliceMut<'_> {
    pub fn len(&self) -> usize { self.length }
    pub fn is_empty(&self) -> bool { self.length == 0 }
    pub fn get(&self, index: usize) -> Option<u8> {
        if index >= self.length { return None; }
        let absolute = self.offset.checked_add(index)?;
        self.blocks.get(absolute / MAX_ALIGNMENT)?.0.get(absolute % MAX_ALIGNMENT).copied()
    }
    pub fn set(&mut self, index: usize, value: u8) -> Option<()> {
        if index >= self.length { return None; }
        let absolute = self.offset.checked_add(index)?;
        *self.blocks.get_mut(absolute / MAX_ALIGNMENT)?.0.get_mut(absolute % MAX_ALIGNMENT)? = value;
        Some(())
    }
    pub fn copy_from(&mut self, source: &[u8]) -> Option<()> {
        if source.len() != self.length { return None; }
        for (index, value) in source.iter().copied().enumerate() { self.set(index, value)?; }
        Some(())
    }
}

/// Observable arena accounting, measured in payload bytes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ArenaStats {
    pub chunk_count: usize,
    pub payload_capacity: usize,
    pub used_bytes: usize,
}

/// Configuration for bounded arena payload storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArenaConfig {
    pub initial_chunk_bytes: usize,
    pub capacity_limit: usize,
}

/// Reusable bounded storage with stable handles, alignment, and reset behavior.
pub struct Arena<G = SystemAllocationGate> {
    id: u64,
    gate: G,
    chunks: Vec<Chunk>,
    initial_chunk_bytes: usize,
    capacity_limit: usize,
    payload_capacity: usize,
}

impl Arena<SystemAllocationGate> {
    /// Create an arena using fallible standard-library allocations.
    pub fn new(config: ArenaConfig) -> Result<Self, BaseStatus> {
        Self::with_allocation_gate(config, SystemAllocationGate)
    }
}

impl<G: AllocationGate> Arena<G> {
    /// Create an arena with a caller-provided allocation permission seam.
    pub fn with_allocation_gate(config: ArenaConfig, gate: G) -> Result<Self, BaseStatus> {
        if config.initial_chunk_bytes == 0 {
            return Err(BaseStatus::Invalid);
        }
        if config.initial_chunk_bytes > config.capacity_limit {
            return Err(BaseStatus::Limit);
        }
        let id = NEXT_ARENA_ID
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |current| current.checked_add(1))
            .map_err(|_| BaseStatus::Overflow)?;
        Ok(Self {
            id,
            gate,
            chunks: Vec::new(),
            initial_chunk_bytes: config.initial_chunk_bytes,
            capacity_limit: config.capacity_limit,
            payload_capacity: 0,
        })
    }

    /// Reserve `size` bytes with power-of-two alignment up to 4096.
    ///
    /// A zero-sized request succeeds with `None`, matching the C null output.
    /// Any failure leaves arena accounting and existing handles unchanged.
    pub fn push(
        &mut self,
        size: usize,
        alignment: usize,
    ) -> Result<Option<ArenaHandle>, BaseStatus> {
        if alignment == 0 || alignment > MAX_ALIGNMENT || !alignment.is_power_of_two() {
            return Err(BaseStatus::Invalid);
        }
        if size == 0 {
            return Ok(None);
        }

        for (index, chunk) in self.chunks.iter_mut().enumerate() {
            if let Some(offset) = fitting_offset(chunk.used, chunk.capacity, size, alignment) {
                let end = offset.checked_add(size).ok_or(BaseStatus::Overflow)?;
                chunk.used = end;
                return Ok(Some(ArenaHandle {
                    arena_id: self.id,
                    chunk: index,
                    offset,
                    length: size,
                }));
            }
        }
        self.append_chunk(size, alignment).map(Some)
    }

    fn append_chunk(&mut self, size: usize, alignment: usize) -> Result<ArenaHandle, BaseStatus> {
        let remaining = self
            .capacity_limit
            .checked_sub(self.payload_capacity)
            .ok_or(BaseStatus::Overflow)?;
        if size > remaining {
            return Err(BaseStatus::Limit);
        }
        let capacity = self.initial_chunk_bytes.min(remaining).max(size);
        let mut chunk = Chunk::try_new(capacity, &mut self.gate)?;
        let offset = fitting_offset(0, chunk.capacity, size, alignment).ok_or(BaseStatus::Overflow)?;
        let end = offset.checked_add(size).ok_or(BaseStatus::Overflow)?;
        chunk.used = end;
        let new_capacity = self
            .payload_capacity
            .checked_add(capacity)
            .ok_or(BaseStatus::Overflow)?;
        self.chunks
            .try_reserve(1)
            .map_err(|_| BaseStatus::OutOfMemory)?;
        let index = self.chunks.len();
        self.chunks.push(chunk);
        self.payload_capacity = new_capacity;
        Ok(ArenaHandle {
            arena_id: self.id,
            chunk: index,
            offset,
            length: size,
        })
    }

    /// Return a borrowed view of a live handle, or `None` if it is invalid.
    pub fn get(&self, handle: ArenaHandle) -> Option<ArenaSlice<'_>> {
        if handle.arena_id != self.id { return None; }
        let chunk = self.chunks.get(handle.chunk)?;
        let end = handle.offset.checked_add(handle.length)?;
        if end > chunk.capacity { return None; }
        Some(ArenaSlice { blocks: &chunk.blocks, offset: handle.offset, length: handle.length })
    }

    /// Return a mutable borrowed view of a live handle, or `None` if invalid.
    pub fn get_mut(&mut self, handle: ArenaHandle) -> Option<ArenaSliceMut<'_>> {
        if handle.arena_id != self.id { return None; }
        let chunk = self.chunks.get_mut(handle.chunk)?;
        let end = handle.offset.checked_add(handle.length)?;
        if end > chunk.capacity { return None; }
        Some(ArenaSliceMut { blocks: &mut chunk.blocks, offset: handle.offset, length: handle.length })
    }

    /// Check an allocation's alignment. Every chunk base is 4096-byte aligned.
    pub fn handle_is_aligned(&self, handle: ArenaHandle, alignment: usize) -> bool {
        alignment != 0
            && alignment <= MAX_ALIGNMENT
            && alignment.is_power_of_two()
            && self.get(handle).is_some()
            && handle.offset % alignment == 0
    }

    /// Snapshot current payload usage and capacity.
    pub fn stats(&self) -> ArenaStats {
        ArenaStats {
            chunk_count: self.chunks.len(),
            payload_capacity: self.payload_capacity,
            used_bytes: self.chunks.iter().map(|chunk| chunk.used).sum(),
        }
    }

    /// Clear all usage while retaining every allocated chunk for reuse.
    pub fn reset(&mut self) {
        for chunk in &mut self.chunks {
            chunk.used = 0;
        }
    }
}

fn fitting_offset(used: usize, capacity: usize, size: usize, alignment: usize) -> Option<usize> {
    let remainder = used % alignment;
    let padding = if remainder == 0 { 0 } else { alignment - remainder };
    let offset = used.checked_add(padding)?;
    let end = offset.checked_add(size)?;
    (end <= capacity).then_some(offset)
}

