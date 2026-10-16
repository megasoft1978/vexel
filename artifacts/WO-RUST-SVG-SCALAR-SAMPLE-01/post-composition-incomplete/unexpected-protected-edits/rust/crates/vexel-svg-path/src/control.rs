use crate::{Diagnostic, DiagnosticKind, failure, oom};
use vexel_base::{ControlledError, Stop, WorkControl, WorkKind};

/// Private adapter used only by the legacy entry points. It never stops and
/// stores no policy or source globally.
pub(crate) struct NoStop;
impl WorkControl for NoStop {
    fn charge(&mut self, _: WorkKind, _: u64) -> Result<(), Stop> {
        Ok(())
    }
    fn checkpoint(&mut self) -> Result<(), Stop> {
        Ok(())
    }
}

pub(crate) fn legacy_error(error: ControlledError<Diagnostic>, offset: usize) -> Diagnostic {
    match error {
        ControlledError::Original(error) => error,
        ControlledError::OutOfMemory => oom(offset),
        ControlledError::Overflow | ControlledError::Limit | ControlledError::Canceled => {
            failure(DiagnosticKind::Limit, offset)
        }
    }
}

/// Account for the real requested capacity (not just a logical item count).
/// Growth is fallible and never requests capacity above the frozen item cap.
/// A caller-owned prefilled vector may already have a larger capacity; that
/// does not change its accepted length cap or invalidate its existing storage.
pub(crate) fn reserve<T>(
    out: &mut Vec<T>,
    additional: usize,
    cap: usize,
    offset: usize,
) -> Result<(), ControlledError<Diagnostic>> {
    let required = out
        .len()
        .checked_add(additional)
        .ok_or(ControlledError::Overflow)?;
    if required > cap {
        return Err(ControlledError::Original(failure(
            DiagnosticKind::Limit,
            offset,
        )));
    }
    if required <= out.capacity() {
        return Ok(());
    }
    let target = out
        .capacity()
        .checked_mul(2)
        .unwrap_or(cap)
        .max(required)
        .min(cap);
    let bytes = target
        .checked_mul(std::mem::size_of::<T>())
        .ok_or(ControlledError::Overflow)?;
    if bytes > isize::MAX as usize {
        return Err(ControlledError::Overflow);
    }
    out.try_reserve_exact(
        target
            .checked_sub(out.len())
            .ok_or(ControlledError::Overflow)?,
    )
    .map_err(|_| ControlledError::Original(oom(offset)))
}
