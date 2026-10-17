//! WO-RUST-UI-MOTION-01 acceptance 7: composing one frame of `ControlVisual`s
//! for 64 animating controls. Zero heap allocation is asserted in every
//! profile with a counting allocator; the 50 microsecond median is asserted
//! only in release (debug skips with a printed reason).
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::time::Instant;
use vexel_ui::motion::{ControlColors, ControlVisual, MAX_CONTROLS, MotionRegistry};

thread_local! { static ALLOCS: Cell<u64> = const { Cell::new(0) }; }
struct Counting;
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        ALLOCS.with(|a| a.set(a.get() + 1));
        unsafe { System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe { System.dealloc(p, l) }
    }
}
#[global_allocator]
static A: Counting = Counting;

const MS: u64 = 1_000_000;

#[test]
fn frame_of_64_animating_controls_is_allocation_free_and_fast() {
    let colors = ControlColors {
        rest_fill: 0x2020_20ff,
        hover_fill: 0x3030_30ff,
        pressed_fill: 0x4040_40ff,
        rest_glyph: 0x8080_80ff,
        hover_glyph: 0xffff_ffff,
        disabled_glyph: 0x5050_50ff,
        ring: 0x4488_ffff,
    };
    let mut reg = MotionRegistry::new(false);
    // Staggered 1 ms apart so every control is mid-transition (in or out) on
    // the hover, press and focus channels at t = 65 ms.
    for id in 1..=64u32 {
        let t = id as u64 * MS;
        reg.pointer_move(Some(id), t);
        reg.pointer_down(Some(id), t);
        reg.focus(Some(id), t);
    }
    let base = 65 * MS;
    let mut out = [(0u32, ControlVisual::EMPTY); MAX_CONTROLS];
    assert!(reg.next_deadline(base).is_some());
    let mut samples = Vec::with_capacity(10_000);
    let before = ALLOCS.with(Cell::get);
    let mut times = [0u64; 10_000];
    for (i, t) in times.iter_mut().enumerate() {
        // The clock moves every iteration so the work cannot be hoisted.
        let now = std::hint::black_box(base + i as u64 % 1000);
        let start = Instant::now();
        let n = reg.visuals_into(now, &colors, &mut out);
        std::hint::black_box(&out);
        *t = start.elapsed().as_nanos() as u64;
        assert_eq!(n, 64);
    }
    let allocs = ALLOCS.with(Cell::get) - before;
    assert_eq!(allocs, 0, "frame composition must not allocate");
    let animating = out
        .iter()
        .filter(|(_, v)| v.hover > 0.0 && v.hover < 1.0)
        .count();
    assert!(animating >= 60, "only {animating} of 64 controls animating");
    samples.extend_from_slice(&times);
    samples.sort_unstable();
    let median_us = samples[samples.len() / 2] as f64 / 1000.0;
    println!("ui_motion frame_budget: 64 controls, median {median_us:.2} us over 10000 iterations");
    if cfg!(debug_assertions) {
        println!("SKIPPED 50 us median assertion: debug profile (run with --release)");
    } else {
        assert!(median_us <= 50.0, "median {median_us} us exceeds 50 us");
    }
}
