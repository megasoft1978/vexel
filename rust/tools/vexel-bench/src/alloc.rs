//! Counting `#[global_allocator]`. Counting is off except around the
//! diagnostic trial, so timed trials pay only one relaxed load per call.

#![allow(unsafe_code)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering::Relaxed};

pub struct Counting;

static ON: AtomicBool = AtomicBool::new(false);
static COUNT: AtomicU64 = AtomicU64::new(0);
static BYTES: AtomicU64 = AtomicU64::new(0);

/// Live-byte tracking for the `mem` mode: off except in `mem-case` children.
static TRACK: AtomicBool = AtomicBool::new(false);
static LIVE: AtomicI64 = AtomicI64::new(0);
static PEAK: AtomicI64 = AtomicI64::new(0);
static BIG_LIVE: AtomicI64 = AtomicI64::new(0);
static BIG_PEAK: AtomicI64 = AtomicI64::new(0);
/// Allocations at least this large count as pixel-sized buffers.
const BIG: usize = 1 << 20;

fn grow(size: usize) {
    let live = LIVE.fetch_add(size as i64, Relaxed) + size as i64;
    PEAK.fetch_max(live, Relaxed);
    if size >= BIG {
        let big = BIG_LIVE.fetch_add(1, Relaxed) + 1;
        BIG_PEAK.fetch_max(big, Relaxed);
    }
}

fn shrink(size: usize) {
    LIVE.fetch_sub(size as i64, Relaxed);
    if size >= BIG {
        BIG_LIVE.fetch_sub(1, Relaxed);
    }
}

fn note(size: usize) {
    if TRACK.load(Relaxed) {
        grow(size);
    }
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
        if TRACK.load(Relaxed) {
            shrink(layout.size());
        }
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        if TRACK.load(Relaxed) {
            shrink(layout.size());
        }
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

/// Live-heap figures from the tracker (bytes; big = allocations of 1 MiB+).
#[derive(Clone, Copy, Debug)]
pub struct Heap {
    pub live: i64,
    pub peak: i64,
    pub big_live: i64,
    pub big_peak: i64,
}

/// Zeroes the live-byte tracker and starts tracking.
pub fn track_start() {
    for a in [&LIVE, &PEAK, &BIG_LIVE, &BIG_PEAK] {
        a.store(0, Relaxed);
    }
    TRACK.store(true, Relaxed);
}

pub fn heap() -> Heap {
    Heap {
        live: LIVE.load(Relaxed),
        peak: PEAK.load(Relaxed),
        big_live: BIG_LIVE.load(Relaxed),
        big_peak: BIG_PEAK.load(Relaxed),
    }
}

/// Restarts peak measurement from the current live level.
pub fn reset_peak() {
    PEAK.store(LIVE.load(Relaxed), Relaxed);
    BIG_PEAK.store(BIG_LIVE.load(Relaxed), Relaxed);
}
