//! Private fallible buffers. Exclusive ledger access holds the available budget
//! during allocation; actual capacity is admitted before any values move or escape.
use vexel_scene::*;
pub(crate) fn allocate<T>(
    ledger: &mut ChargeLedger,
    capacity: usize,
    tag: AllocationTag,
    control: &mut dyn WorkControl,
) -> Result<(Vec<T>, AllocationLease)> {
    control
        .checkpoint()
        .map_err(|s| crate::source::stopped(s, 0))?;
    let requested = capacity
        .checked_mul(std::mem::size_of::<T>())
        .ok_or(Diagnostic {
            kind: ErrorKind::Limit(LimitKind::Arithmetic),
            offset: 0,
        })?;
    if requested > ledger.available() {
        return Err(Diagnostic {
            kind: ErrorKind::Limit(LimitKind::ReservedBytes),
            offset: 0,
        });
    }
    let mut values = Vec::new();
    values.try_reserve_exact(capacity).map_err(|_| Diagnostic {
        kind: ErrorKind::OutOfMemory,
        offset: 0,
    })?;
    let lease = ledger.admit_vec(tag, &values)?;
    if let Err(s) = control.checkpoint() {
        drop(values);
        ledger.release(lease)?;
        return Err(crate::source::stopped(s, 0));
    }
    Ok((values, lease))
}
pub(crate) fn reserve<T>(
    values: &mut Vec<T>,
    lease: &mut Option<AllocationLease>,
    ledger: &mut ChargeLedger,
    additional: usize,
    limit: usize,
    control: &mut dyn WorkControl,
) -> Result<()> {
    let required = values.len().checked_add(additional).ok_or(Diagnostic {
        kind: ErrorKind::Limit(LimitKind::Arithmetic),
        offset: 0,
    })?;
    if required > limit {
        return Err(Diagnostic {
            kind: ErrorKind::Limit(LimitKind::Depth),
            offset: 0,
        });
    }
    if required <= values.capacity() {
        return Ok(());
    }
    let capacity = required
        .max(values.capacity().saturating_mul(2))
        .max(8)
        .min(limit);
    let (mut next, charge) = allocate(ledger, capacity, AllocationTag::None, control)?;
    for value in values.drain(..) {
        if let Err(s) = control.charge(WorkKind::SourceRead, 1) {
            drop(next);
            ledger.release(charge)?;
            return Err(crate::source::stopped(s, 0));
        }
        next.push(value);
    }
    *values = next;
    if let Some(old) = lease.replace(charge) {
        ledger.release(old)?;
    }
    Ok(())
}
pub(crate) fn copy<T: Copy>(
    ledger: &mut ChargeLedger,
    values: &[T],
    tag: AllocationTag,
    control: &mut dyn WorkControl,
) -> Result<(Vec<T>, AllocationLease)> {
    let (mut out, lease) = allocate(ledger, values.len(), tag, control)?;
    for value in values {
        if let Err(s) = control.charge(WorkKind::SourceRead, 1) {
            drop(out);
            ledger.release(lease)?;
            return Err(crate::source::stopped(s, 0));
        }
        out.push(*value);
    }
    Ok((out, lease))
}
