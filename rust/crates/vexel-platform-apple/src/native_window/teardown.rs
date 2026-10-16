use super::app::WindowRecord;
use crate::{NativeFault, guard_native_call};
pub(crate) fn next_generation(generation: u32) -> Option<u32> {
    generation.checked_add(1).filter(|v| *v != 0)
}
pub(crate) fn destroy(mut record: WindowRecord) -> Result<(), NativeFault> {
    record.route.invalidate();
    record.view.clear_route();
    record._delegate.clear_route();
    let mut fault = None;
    if let Some(timer) = record.timer.take() {
        if let Err(f) = super::ticks::cancel(&timer.0) {
            fault = Some(f);
        }
    }
    // Cleanup continues after individual faults. All routes were invalidated
    // before any reentrant call, and no app/route RefCell borrow is held here.
    for result in [
        guard_native_call(|| record.native.setDelegate(None)),
        guard_native_call(|| {
            let _ = record.native.makeFirstResponder(None);
        }),
        guard_native_call(|| record.native.setContentView(None)),
        guard_native_call(|| record.native.close()),
    ] {
        if let Err(f) = result {
            fault.get_or_insert(f);
        }
    }
    // Retained/weak release and native dealloc are outside objc2 catch-all and may
    // terminate if foreign cleanup throws. No universal exception fence is claimed.
    drop(record);
    super::ticks::wake();
    fault.map_or(Ok(()), Err)
}
