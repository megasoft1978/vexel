use super::callbacks::TickTarget;
use crate::{NativeFault, guard_native_call};
use objc2::rc::Retained;
use objc2::{runtime::AnyObject, sel};
use objc2_core_foundation::CFRunLoop;
use objc2_foundation::{NSRunLoop, NSRunLoopCommonModes, NSTimer};

pub(crate) fn wake() {
    // CF is outside catch-all. Apple's process-lifetime main run loop is valid;
    // wake_up neither throws an ObjC exception nor synchronously runs callbacks.
    if let Some(main) = CFRunLoop::main() {
        main.wake_up();
    }
}
pub(crate) fn install(target: &TickTarget) -> Result<Retained<NSTimer>, NativeFault> {
    let timer = guard_native_call(|| unsafe {
        // SAFETY: target implements tick:(NSTimer*) with exact void ABI; timer
        // retains it and is scheduled exclusively on the process main run loop.
        let object: &AnyObject = target.as_ref();
        NSTimer::timerWithTimeInterval_target_selector_userInfo_repeats(
            1.0 / 60.0,
            object,
            sel!(tick:),
            None,
            true,
        )
    })?;
    let result = guard_native_call(|| unsafe {
        NSRunLoop::mainRunLoop().addTimer_forMode(&timer, NSRunLoopCommonModes)
    });
    if let Err(fault) = result {
        let _ = guard_native_call(|| timer.invalidate());
        return Err(fault);
    }
    wake();
    Ok(timer)
}
pub(crate) fn cancel(timer: &NSTimer) -> Result<(), NativeFault> {
    guard_native_call(|| timer.invalidate())
}
