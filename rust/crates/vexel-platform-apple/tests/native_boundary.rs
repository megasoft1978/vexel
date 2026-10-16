use std::os::unix::process::ExitStatusExt;
use std::thread;
use std::{env, process::Command};

use objc2_foundation::NSString;
use vexel_platform_apple::{MainThread, NativeFaultKind, guard_native_call};

#[test]
fn guard_preserves_success_value() {
    let value = String::from("unchanged result");
    let result = guard_native_call(|| value).expect("success should pass through");
    assert_eq!(result, "unchanged result");
}

#[test]
fn guard_catches_rust_panic_and_discards_payload() {
    let result = guard_native_call(|| panic!("private panic payload"));
    let fault = result.expect_err("panic should become a fault");
    assert_eq!(fault.kind(), NativeFaultKind::Unwind);
}

#[test]
fn main_thread_capability_is_denied_on_spawned_thread() {
    let denied = thread::spawn(|| MainThread::acquire().is_err())
        .join()
        .expect("thread should complete");
    assert!(denied, "spawned thread must not acquire the capability");
}

#[test]
fn foundation_range_exception_is_contained_by_guard() {
    let string = NSString::from_str("native-boundary");
    let result = guard_native_call(|| {
        // NSString raises a real Foundation NSRangeException for an invalid
        // UTF-16 index. objc2 catch-all converts its Objective-C exception to
        // a Rust unwind; the reusable boundary guard contains that unwind.
        let _ = string.substringFromIndex(usize::MAX);
    });
    let fault = result.expect_err("Foundation exception should become a fault");
    assert_eq!(fault.kind(), NativeFaultKind::Unwind);
}

struct PanicsOnDrop;

impl Drop for PanicsOnDrop {
    fn drop(&mut self) {
        std::panic::panic_any(SecondaryPanicsOnDrop);
    }
}

struct SecondaryPanicsOnDrop;

impl Drop for SecondaryPanicsOnDrop {
    fn drop(&mut self) {
        panic!("secondary panic payload destructor failure");
    }
}

#[test]
fn panic_payload_destructor_aborts_process() {
    const CHILD_MODE: &str = "VEXEL_NATIVE_BOUNDARY_ABORT_CHILD";

    if env::var_os(CHILD_MODE).is_some() {
        let _ = guard_native_call(|| std::panic::panic_any(PanicsOnDrop));
        panic!("guard must abort when dropping the original payload panics");
    }

    let status = Command::new(env::current_exe().expect("test executable path"))
        .arg("--exact")
        .arg("panic_payload_destructor_aborts_process")
        .arg("--nocapture")
        .env(CHILD_MODE, "1")
        .status()
        .expect("spawn death-test child");
    assert_eq!(
        status.signal(),
        Some(6),
        "child must terminate with SIGABRT"
    );
}
