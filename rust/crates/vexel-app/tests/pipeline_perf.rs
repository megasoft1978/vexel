//! Pipeline speed gate (WO-RUST-PIPELINE-PERF-01): decode of large flat
//! documents must not grow quadratically. Run in release:
//! `cargo test -p vexel-app --release --test pipeline_perf`.
//! `VEXEL_PERF_PRINT=1` prints the measured values.
mod pipeline_common;
use pipeline_common::{paths, rects, rects_in_group, rects_with_style};
use std::cell::Cell;
use std::time::Instant;
use vexel_app::compose::{ComposeError, compose_document, compose_svg};

fn best_of_three_ms(source: &[u8]) -> f64 {
    (0..3)
        .map(|_| {
            let start = Instant::now();
            let frame = compose_svg(source, 128, 128).expect("synthetic scene composes");
            assert_eq!((frame.width, frame.height), (128, 128));
            start.elapsed().as_secs_f64() * 1000.0
        })
        .fold(f64::INFINITY, f64::min)
}

fn print_enabled() -> bool {
    std::env::var("VEXEL_PERF_PRINT").is_ok_and(|v| v == "1")
}

fn measure(name: &str, source: &[u8]) -> f64 {
    let ms = best_of_three_ms(source);
    if print_enabled() {
        println!("pipeline {name}: best_of_3_ms={ms:.1}");
    }
    ms
}

#[test]
fn r8000_flat_rects_stay_fast() {
    if cfg!(debug_assertions) {
        return; // timing gates are release-only
    }
    let ms = measure("R8000", &rects(8000));
    assert!(ms <= 300.0, "R8000 took {ms:.1} ms, limit 300");
}

#[test]
fn p8000_stroked_paths_stay_fast() {
    if cfg!(debug_assertions) {
        return; // timing gates are release-only
    }
    let ms = measure("P8000", &paths(8000));
    assert!(ms <= 1200.0, "P8000 took {ms:.1} ms, limit 1200");
}

/// Best-of-three at base_sha d0ae379c on the order's host (release, 128x128).
const BASE_S8000_MS: f64 = 1426.5;
const BASE_G8000_MS: f64 = 1353.9;

#[test]
fn s8000_style_block_stays_fast() {
    if cfg!(debug_assertions) {
        return; // timing gates are release-only
    }
    let limit = (BASE_S8000_MS / 5.0).min(1500.0);
    let ms = measure("S8000", &rects_with_style(8000));
    assert!(ms <= limit, "S8000 took {ms:.1} ms, limit {limit:.1}");
}

#[test]
fn g8000_grouped_rects_stay_fast() {
    if cfg!(debug_assertions) {
        return; // timing gates are release-only
    }
    let limit = (BASE_G8000_MS / 5.0).min(1500.0);
    let ms = measure("G8000", &rects_in_group(8000));
    assert!(ms <= limit, "G8000 took {ms:.1} ms, limit {limit:.1}");
}

#[test]
fn rect_decode_scales_without_quadratic_growth() {
    if cfg!(debug_assertions) {
        return; // timing gates are release-only
    }
    let small = measure("R8000(scale)", &rects(8000));
    let large = measure("R16000", &rects(16000));
    assert!(
        large <= 2.6 * small,
        "R16000 {large:.1} ms exceeds 2.6 x R8000 {small:.1} ms"
    );
}

#[test]
fn cancel_stops_r8000_within_one_poll() {
    let source = rects(8000);
    let polls = Cell::new(0usize);
    let flagged_at = Cell::new(None::<usize>);
    let flagged_time = Cell::new(None::<Instant>);
    let cancel = || {
        let n = polls.get() + 1;
        polls.set(n);
        if n >= 200 {
            if flagged_at.get().is_none() {
                flagged_at.set(Some(n));
                flagged_time.set(Some(Instant::now()));
            }
            true
        } else {
            false
        }
    };
    let result = compose_document(&source, 128, 128, &cancel);
    assert!(
        matches!(result, Err(ComposeError::Cancelled)),
        "expected Canceled"
    );
    let flagged = flagged_at.get().expect("flag was raised");
    let after = polls.get() - flagged;
    let latency = flagged_time.get().unwrap().elapsed();
    if print_enabled() {
        println!(
            "pipeline cancel: flag at poll {flagged}, polls after flag {after}, latency {latency:?}"
        );
    }
    assert!(after <= 1, "{after} polls after the flag was raised");
    assert!(latency.as_millis() < 50, "cancel latency {latency:?}");
}
