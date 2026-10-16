use std::panic::{AssertUnwindSafe, catch_unwind};

/// Stable categories returned by a guarded native boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum NativeFaultKind {
    /// A Rust panic or an Objective-C message exception translated to a Rust
    /// unwind by objc2's audited `catch-all` feature.
    Unwind,
}

/// Small owned error that does not retain arbitrary panic or native payloads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeFault {
    kind: NativeFaultKind,
}

impl NativeFault {
    /// Returns the bounded category for this failure.
    pub const fn kind(self) -> NativeFaultKind {
        self.kind
    }
}

/// Runs a Rust/native operation inside the reusable platform unwind guard.
///
/// Success returns the original value unchanged. Rust panics are caught by
/// `catch_unwind`; Objective-C exceptions from objc2 message sends are first
/// caught by objc2's pinned `catch-all` trampoline and converted to a Rust
/// panic, which this helper catches. The panic payload is discarded before
/// returning the fixed-size fault.
/// If discarding a payload panics, the process is aborted immediately: the
/// secondary panic is never retained and no further unwind crosses the
/// boundary.
///
/// This is not a fence for raw C/CF calls, retain/release or `Drop` cleanup,
/// foreign cleanup exceptions, or operations built with `panic=abort`.
/// Those cases can terminate or escape independently. Callers must not hold
/// application-model borrows across reentrant native calls. An inbound ABI
/// callback must invoke this guard before returning and choose its own
/// ABI-valid fallback on `Err`; this helper does not invent callbacks.
pub fn guard_native_call<T>(operation: impl FnOnce() -> T) -> Result<T, NativeFault> {
    match catch_unwind(AssertUnwindSafe(operation)) {
        Ok(value) => Ok(value),
        Err(payload) => {
            discard_panic_payload(payload);
            Err(NativeFault {
                kind: NativeFaultKind::Unwind,
            })
        }
    }
}

fn discard_panic_payload(payload: Box<dyn std::any::Any + Send>) {
    // A hostile panic payload may itself panic in Drop. Catch that secondary
    // unwind so it cannot cross this boundary or double-panic the process.
    // A secondary panic payload could own unbounded memory. Abort rather than
    // retain it or permit another unwind across the native boundary.
    match catch_unwind(AssertUnwindSafe(|| drop(payload))) {
        Ok(()) => (),
        Err(_secondary_payload) => std::process::abort(),
    }
}
