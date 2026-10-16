use std::sync::atomic::{AtomicU64, Ordering};

/// Handle identity is scoped to one document ID and one live slot generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DocumentHandle {
    pub document_id: u64,
    pub slot: u32,
    pub generation: u32,
}

/// Byte range in the immutable borrowed source.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DocumentSourceSpan {
    pub offset: usize,
    pub length: usize,
}

/// Deterministic allocator seam used to verify exhaustion without touching the
/// process-wide ID sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg(test)]
pub(crate) struct DocumentIdAllocator {
    next: u64,
}

#[cfg(test)]
impl DocumentIdAllocator {
    pub(crate) const fn starting_at(next: u64) -> Self {
        Self { next }
    }

    pub(crate) fn allocate(&mut self) -> Option<u64> {
        let next = next_document_id(self.next)?;
        let allocated = self.next;
        self.next = next;
        Some(allocated)
    }
}

static NEXT_DOCUMENT_ID: AtomicU64 = AtomicU64::new(1);

pub(crate) fn allocate_document_id() -> Option<u64> {
    NEXT_DOCUMENT_ID
        .try_update(Ordering::Relaxed, Ordering::Relaxed, next_document_id)
        .ok()
}

fn next_document_id(current: u64) -> Option<u64> {
    if current == 0 || current == u64::MAX {
        None
    } else {
        Some(current + 1)
    }
}
