//! Cost model and hysteresis for the GPU display route (ADR-0035 addendum).
//!
//! Pure: no GPU, no clocks. the route glue measures, builds a [`Load`] per tick
//! and asks [`Policy::decide`]. Safety fallbacks (latch, env switch, forced
//! modes, unsupported scenes) are decided before this module is consulted.
//!
//! # Model (ms)
//!
//! - CPU full render: `work * CPU_NS_PER_WORK` (work = weighted covered px).
//! - CPU differential tick: `CPU_DIFF_BASE_MS + full * min(1, dirty_px /
//!   canvas_px) + dirty_shapes * CPU_NS_PER_SHAPE`, plus `full /
//!   SEED_AMORT_TICKS` while the cache is cold (the switch costs one full
//!   render). An undiffable scene always pays `full`.
//! - GPU frame: `GPU_BASE_MS + GPU_NS_PER_PX * px + GPU_US_PER_ENTRY * n`.
//! - Both are scaled by a calibration factor: EMA of measured / modelled time
//!   (clamped), so measured tick times replace the priors as they arrive.
//!
//! # Hysteresis
//!
//! The other route must be estimated at least `SWITCH_MARGIN` times cheaper for
//! `SWITCH_STREAK` consecutive ticks, after `MIN_DWELL` ticks on the current
//! route. A `IMMEDIATE_RATIO` gap switches at once (static scene, huge win).

pub(super) const CPU_NS_PER_WORK: f64 = 4.0;
pub(super) const CPU_DIFF_BASE_MS: f64 = 0.15;
pub(super) const CPU_NS_PER_SHAPE: f64 = 2_000.0;
pub(super) const SEED_AMORT_TICKS: f64 = 128.0;
pub(super) const GPU_BASE_MS: f64 = 1.5;
pub(super) const GPU_NS_PER_PX: f64 = 0.4;
pub(super) const GPU_US_PER_ENTRY: f64 = 1.2;
pub(super) const EMA_ALPHA: f64 = 0.3;
/// Modelled times below this are timer noise, not calibration data.
pub(super) const MIN_CAL_MS: f64 = 0.2;
pub(super) const CAL_MIN: f64 = 0.25;
pub(super) const CAL_MAX: f64 = 4.0;
pub(super) const SWITCH_MARGIN: f64 = 1.25;
pub(super) const SWITCH_STREAK: u32 = 3;
pub(super) const MIN_DWELL: u32 = 4;
pub(super) const IMMEDIATE_RATIO: f64 = 3.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Route {
    Cpu,
    Gpu,
}

/// Why a scene can or cannot take the GPU at all.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Verdict<R> {
    /// Lowers and classifies; `work` is the CPU raster work estimate.
    Gpu {
        work: f64,
    },
    Cpu(&'static str),
    Unsupported(R),
}

/// What one tick looks like.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Load {
    pub canvas_px: f64,
    pub work: f64,
    pub entries: usize,
    pub dirty_px: f64,
    pub dirty_shapes: usize,
    /// The diff cache holds a CPU frame to patch.
    pub warm: bool,
    /// The scene can never be diffed.
    pub undiffable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Estimates {
    pub cpu_ms: f64,
    pub gpu_ms: f64,
}

impl Load {
    fn full_ms(&self) -> f64 {
        self.work * CPU_NS_PER_WORK * 1e-6
    }

    /// CPU time of this tick if rendered now (model units).
    fn cpu_actual_raw(&self) -> f64 {
        let full = self.full_ms();
        if self.undiffable || !self.warm {
            return full;
        }
        let frac = if self.canvas_px > 0.0 {
            (self.dirty_px / self.canvas_px).min(1.0)
        } else {
            1.0
        };
        CPU_DIFF_BASE_MS + full * frac + self.dirty_shapes as f64 * CPU_NS_PER_SHAPE * 1e-6
    }

    /// CPU time to compare against the GPU: steady-state diff cost with the
    /// cold-cache seed amortised.
    fn cpu_raw(&self) -> f64 {
        if self.undiffable {
            return self.full_ms();
        }
        let steady = Self {
            warm: true,
            ..*self
        }
        .cpu_actual_raw();
        if self.warm {
            steady
        } else {
            steady + self.full_ms() / SEED_AMORT_TICKS
        }
    }

    fn gpu_raw(&self) -> f64 {
        GPU_BASE_MS
            + GPU_NS_PER_PX * 1e-6 * self.canvas_px
            + GPU_US_PER_ENTRY * 1e-3 * self.entries as f64
    }
}

fn ema(cal: f64, measured: f64, modelled: f64) -> f64 {
    if !(measured.is_finite() && modelled.is_finite()) || modelled < MIN_CAL_MS || measured < 0.0 {
        return cal;
    }
    let ratio = (measured / modelled).clamp(CAL_MIN, CAL_MAX);
    (cal + EMA_ALPHA * (ratio - cal)).clamp(CAL_MIN, CAL_MAX)
}

/// Calibrated estimator plus the flap guard.
#[derive(Debug, Clone, Copy)]
pub(super) struct Policy {
    cpu_cal: f64,
    gpu_cal: f64,
    /// The first GPU frame of a thread pays device and pipeline creation.
    gpu_warm: bool,
    route: Option<Route>,
    dwell: u32,
    streak: u32,
}

impl Policy {
    pub(super) const fn new() -> Self {
        Self {
            cpu_cal: 1.0,
            gpu_cal: 1.0,
            gpu_warm: false,
            route: None,
            dwell: 0,
            streak: 0,
        }
    }

    pub(super) fn estimate(&self, load: &Load) -> Estimates {
        Estimates {
            cpu_ms: load.cpu_raw() * self.cpu_cal,
            gpu_ms: load.gpu_raw() * self.gpu_cal,
        }
    }

    /// Feeds a measured CPU tick (ms) rendered under `load`.
    pub(super) fn observe_cpu(&mut self, ms: f64, load: &Load) {
        self.cpu_cal = ema(self.cpu_cal, ms, load.cpu_actual_raw());
    }

    /// Feeds a measured GPU frame (ms) rendered under `load`.
    pub(super) fn observe_gpu(&mut self, ms: f64, load: &Load) {
        if self.gpu_warm {
            self.gpu_cal = ema(self.gpu_cal, ms, load.gpu_raw());
        }
        self.gpu_warm = true;
    }

    /// The route the last decision (or fallback) put the next tick on.
    #[cfg(test)]
    pub(super) fn route(&self) -> Option<Route> {
        self.route
    }

    /// A fallback (unsupported scene, GPU error) rendered on the CPU.
    pub(super) fn force_cpu(&mut self) {
        if self.route != Some(Route::Cpu) {
            self.route = Some(Route::Cpu);
            self.dwell = 0;
            self.streak = 0;
        }
    }

    /// Route for this tick given the estimates.
    pub(super) fn decide(&mut self, e: Estimates) -> Route {
        let cheaper = if e.gpu_ms < e.cpu_ms {
            Route::Gpu
        } else {
            Route::Cpu
        };
        let Some(current) = self.route else {
            self.route = Some(cheaper);
            return cheaper;
        };
        let (cur, other, to) = match current {
            Route::Cpu => (e.cpu_ms, e.gpu_ms, Route::Gpu),
            Route::Gpu => (e.gpu_ms, e.cpu_ms, Route::Cpu),
        };
        self.dwell = self.dwell.saturating_add(1);
        if cur >= other * IMMEDIATE_RATIO {
            return self.switch(to);
        }
        if cur >= other * SWITCH_MARGIN {
            self.streak += 1;
        } else {
            self.streak = 0;
        }
        if self.streak >= SWITCH_STREAK && self.dwell >= MIN_DWELL {
            return self.switch(to);
        }
        current
    }

    fn switch(&mut self, to: Route) -> Route {
        self.route = Some(to);
        self.dwell = 0;
        self.streak = 0;
        to
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load(work: f64, entries: usize, dirty_px: f64, dirty_shapes: usize) -> Load {
        Load {
            canvas_px: 810_000.0,
            work,
            entries,
            dirty_px,
            dirty_shapes,
            warm: true,
            undiffable: false,
        }
    }

    fn est(cpu_ms: f64, gpu_ms: f64) -> Estimates {
        Estimates { cpu_ms, gpu_ms }
    }

    #[test]
    fn many_dirty_shapes_choose_gpu() {
        let mut p = Policy::new();
        let l = load(2.0e7, 900, 700_000.0, 800);
        let e = p.estimate(&l);
        assert!(e.cpu_ms > 3.0 * e.gpu_ms, "{e:?}");
        assert_eq!(p.decide(e), Route::Gpu);
    }

    #[test]
    fn tiny_dirty_area_stays_cpu() {
        let mut p = Policy::new();
        let l = load(2.0e7, 900, 2_000.0, 3);
        let e = p.estimate(&l);
        assert!(e.cpu_ms < e.gpu_ms, "{e:?}");
        assert_eq!(p.decide(e), Route::Cpu);
    }

    #[test]
    fn noisy_alternating_estimates_do_not_flap() {
        let mut p = Policy::new();
        assert_eq!(p.decide(est(4.0, 5.0)), Route::Cpu);
        let mut switches = 0;
        let mut last = Route::Cpu;
        for i in 0..200 {
            let cpu = if i % 2 == 0 { 8.0 } else { 4.0 };
            let r = p.decide(est(cpu, 5.0));
            switches += usize::from(r != last);
            last = r;
        }
        assert_eq!(switches, 0);
        // Within the margin: never switches either.
        for i in 0..200 {
            let (c, g) = if i % 2 == 0 { (9.0, 10.0) } else { (10.0, 9.0) };
            assert_eq!(p.decide(est(c, g)), Route::Cpu);
        }
    }

    #[test]
    fn sustained_advantage_switches_back_after_dwell() {
        let mut p = Policy::new();
        assert_eq!(p.decide(est(20.0, 5.0)), Route::Gpu);
        // CPU 2.0x cheaper (below the immediate ratio): needs the streak and dwell.
        let mut ticks = 0;
        while p.decide(est(2.5, 5.0)) == Route::Gpu {
            ticks += 1;
            assert!(ticks < 10);
        }
        assert_eq!(ticks + 1, MIN_DWELL);
        // And a huge gap goes at once.
        let mut q = Policy::new();
        q.decide(est(20.0, 5.0));
        assert_eq!(q.decide(est(0.5, 5.0)), Route::Cpu);
    }

    #[test]
    fn unsupported_scene_pins_cpu() {
        let mut p = Policy::new();
        assert_eq!(p.decide(est(50.0, 1.0)), Route::Gpu);
        p.force_cpu();
        assert_eq!(p.route(), Some(Route::Cpu));
        // Dwell restarts: no instant bounce back to the GPU.
        assert_eq!(p.decide(est(50.0, 20.0)), Route::Cpu);
    }

    #[test]
    fn measured_times_recalibrate_estimates() {
        let mut p = Policy::new();
        let l = load(1.0e7, 500, 810_000.0, 500);
        let before = p.estimate(&l);
        p.observe_gpu(1000.0, &l); // warm-up frame is ignored
        for _ in 0..30 {
            p.observe_gpu(before.gpu_ms * 4.0, &l);
            p.observe_cpu(before.cpu_ms * 0.5, &l);
        }
        let after = p.estimate(&l);
        assert!(after.gpu_ms > 3.5 * before.gpu_ms);
        assert!(after.cpu_ms < 0.6 * before.cpu_ms);
    }

    #[test]
    fn cold_cache_amortises_the_seed_and_undiffable_pays_full() {
        let mut l = load(2.0e7, 900, 0.0, 0);
        let warm = l.cpu_raw();
        l.warm = false;
        assert!(l.cpu_raw() > warm);
        assert!(l.cpu_raw() < l.full_ms());
        l.undiffable = true;
        assert_eq!(l.cpu_raw(), l.full_ms());
    }
}
