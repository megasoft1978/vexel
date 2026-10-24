//! Counting `#[global_allocator]`. Counting is off except around the
//! diagnostic trial, so timed trials pay only one relaxed load per call.

#![allow(unsafe_code)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::Relaxed};

pub struct Counting;

static ON: AtomicBool = AtomicBool::new(false);
static COUNT: AtomicU64 = AtomicU64::new(0);
static BYTES: AtomicU64 = AtomicU64::new(0);

fn note(size: usize) {
    if ON.load(Relaxed) {
        COUNT.fetch_add(1, Relaxed);
        BYTES.fetch_add(size as u64, Relaxed);
    }
}

// SAFETY: every method forwards to `System` unchanged; the counters are
// atomics and never touch the allocation itself.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        note(layout.size());
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        note(layout.size());
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        note(new_size);
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

/// Zeroes the counters and starts counting.
pub fn start() {
    COUNT.store(0, Relaxed);
    BYTES.store(0, Relaxed);
    ON.store(true, Relaxed);
}

/// Stops counting; returns (allocation calls, bytes requested).
pub fn stop() -> (u64, u64) {
    ON.store(false, Relaxed);
    (COUNT.load(Relaxed), BYTES.load(Relaxed))
}
