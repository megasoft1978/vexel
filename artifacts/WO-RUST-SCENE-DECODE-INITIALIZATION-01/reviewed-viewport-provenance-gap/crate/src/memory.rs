use crate::{Diagnostic, ErrorKind, LimitKind, Result};
/// Vec-reported reserved storage, excluding allocator bookkeeping and RSS.
/// Allocation history describes builder reserves and admitted owned payloads;
/// it does not reconstruct allocation calls made by parsers/providers before admission.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MemoryStats {
    pub source_bytes: usize,
    pub live_bytes: usize,
    pub peak_live_bytes: usize,
    pub total_allocated_bytes: usize,
    pub allocation_count: usize,
}
pub(crate) fn bytes<T>(values: &Vec<T>) -> Result<usize> {
    values
        .capacity()
        .checked_mul(std::mem::size_of::<T>())
        .ok_or_else(arithmetic)
}
pub(crate) fn add(a: usize, b: usize) -> Result<usize> {
    a.checked_add(b).ok_or_else(arithmetic)
}
pub(crate) fn arithmetic() -> Diagnostic {
    Diagnostic {
        kind: ErrorKind::Limit(LimitKind::Arithmetic),
        offset: 0,
    }
}
/// Reserve a replacement before transferring old values. Reject excess actual
/// capacity before moving anything. The caller supplies the available overlap budget.
pub(crate) fn reserve<T>(
    values: &mut Vec<T>,
    extra: usize,
    cap: usize,
    overlap: usize,
) -> Result<(usize, usize)> {
    let required = values.len().checked_add(extra).ok_or_else(arithmetic)?;
    if required > cap {
        return Err(Diagnostic {
            kind: ErrorKind::Limit(LimitKind::Nodes),
            offset: 0,
        });
    }
    if required <= values.capacity() {
        return Ok((0, 0));
    }
    let request = required
        .checked_mul(std::mem::size_of::<T>())
        .ok_or_else(arithmetic)?;
    if request > overlap {
        return Err(Diagnostic {
            kind: ErrorKind::Limit(LimitKind::ReservedBytes),
            offset: 0,
        });
    }
    let mut replacement = Vec::new();
    let grown = values
        .capacity()
        .checked_mul(2)
        .unwrap_or(cap)
        .max(8)
        .min(cap)
        .max(required);
    let target = if grown
        .checked_mul(std::mem::size_of::<T>())
        .is_some_and(|n| n <= overlap)
    {
        grown
    } else {
        required
    };
    replacement
        .try_reserve_exact(target)
        .map_err(|_| Diagnostic {
            kind: ErrorKind::OutOfMemory,
            offset: 0,
        })?;
    let actual = bytes(&replacement)?;
    if actual > overlap {
        return Err(Diagnostic {
            kind: ErrorKind::Limit(LimitKind::ReservedBytes),
            offset: 0,
        });
    }
    let old = bytes(values)?;
    replacement.append(values);
    *values = replacement;
    Ok((actual, old))
}
