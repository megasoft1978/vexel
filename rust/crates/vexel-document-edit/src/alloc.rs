//! Significant allocations are fallible. The optional thread-local gate fails
//! a named allocation boundary without replacing the allocator or using unsafe.
use crate::EditError;
use std::cell::Cell;
thread_local! { static REMAINING: Cell<Option<usize>> = const { Cell::new(None) }; }
/// Test/diagnostic seam. `Some(n)` permits n significant allocations, then
/// rejects the next. The gate is local to the calling thread; None disables it.
pub fn allocation_gate(remaining: Option<usize>) {
    REMAINING.with(|v| v.set(remaining));
}
pub(crate) fn checkpoint() -> Result<(), EditError> {
    REMAINING.with(|v| match v.get() {
        None => Ok(()),
        Some(0) => Err(EditError::OutOfMemory),
        Some(n) => {
            v.set(Some(n - 1));
            Ok(())
        }
    })
}
pub(crate) fn vector<T>(capacity: usize) -> Result<Vec<T>, EditError> {
    capacity
        .checked_mul(std::mem::size_of::<T>())
        .ok_or(EditError::Limit)?;
    let mut v = Vec::new();
    if capacity != 0 {
        checkpoint()?;
        v.try_reserve_exact(capacity)
            .map_err(|_| EditError::OutOfMemory)?;
    }
    Ok(v)
}
pub(crate) fn copy(bytes: &[u8]) -> Result<Vec<u8>, EditError> {
    let mut out = vector(bytes.len())?;
    out.extend_from_slice(bytes);
    Ok(out)
}
pub(crate) fn append(out: &mut Vec<u8>, bytes: &[u8]) -> Result<(), EditError> {
    let len = out.len().checked_add(bytes.len()).ok_or(EditError::Limit)?;
    if len > out.capacity() {
        checkpoint()?;
        out.try_reserve_exact(bytes.len())
            .map_err(|_| EditError::OutOfMemory)?;
    }
    out.extend_from_slice(bytes);
    Ok(())
}
