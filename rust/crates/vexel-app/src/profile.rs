//! Opt-in phase timers for the MCP benchmark (`tests/mcp_bench.rs`).
//!
//! Timing is live only when `VEXEL_PROFILE` is set (to anything but `0`) or
//! [`enable`] has been called. Otherwise [`phase`] reads two cached flags and
//! returns a guard that records nothing.

use std::collections::BTreeMap;
use std::sync::Mutex;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

static FORCED: AtomicBool = AtomicBool::new(false);
static FROM_ENV: OnceLock<bool> = OnceLock::new();
static SAMPLES: Mutex<BTreeMap<&'static str, Vec<f64>>> = Mutex::new(BTreeMap::new());

/// Turns phase timing on for this process.
pub fn enable() {
    FORCED.store(true, Ordering::Relaxed);
}

fn enabled() -> bool {
    FORCED.load(Ordering::Relaxed)
        || *FROM_ENV.get_or_init(|| std::env::var_os("VEXEL_PROFILE").is_some_and(|v| v != "0"))
}

/// Records the time from creation to drop under its name when timing is on.
#[must_use = "a phase is timed until its guard is dropped"]
pub struct Phase {
    name: &'static str,
    start: Option<Instant>,
}

/// Starts timing `name` until the returned guard is dropped.
pub fn phase(name: &'static str) -> Phase {
    Phase {
        name,
        start: enabled().then(Instant::now),
    }
}

impl Drop for Phase {
    fn drop(&mut self) {
        if let Some(start) = self.start {
            let ms = start.elapsed().as_secs_f64() * 1000.0;
            if let Ok(mut samples) = SAMPLES.lock() {
                samples.entry(self.name).or_default().push(ms);
            }
        }
    }
}

/// Records an externally measured duration (milliseconds) under `name`.
pub fn record(name: &'static str, ms: f64) {
    if enabled()
        && let Ok(mut samples) = SAMPLES.lock()
    {
        samples.entry(name).or_default().push(ms);
    }
}

/// Removes and returns all recorded samples, keyed by phase name.
pub fn take() -> BTreeMap<&'static str, Vec<f64>> {
    SAMPLES
        .lock()
        .map(|mut samples| std::mem::take(&mut *samples))
        .unwrap_or_default()
}
