//! Fallback and parity policy constants (ADR-0035). Tolerances may be
//! tightened, never loosened.

/// Environment variable; the value `0` disables the GPU display route.
pub const ENV_SWITCH: &str = "VEXEL_GPU";
/// Runtime GPU errors in one session before the GPU route latches off.
pub const LATCH_ERROR_LIMIT: u32 = 3;

/// Parity of GPU against CPU output, per channel, over a whole frame.
/// Fraction of pixels with any channel differing by more than 2/255.
pub const PARITY_OVER_2_MAX_FRACTION: f64 = 0.0002;
/// Fraction of pixels with any channel differing by more than 16/255.
pub const PARITY_OVER_16_MAX_FRACTION: f64 = 0.00002;
/// Mean absolute error per channel, in 0..=1 units.
pub const PARITY_MEAN_ABS_ERROR_MAX: f64 = 0.001;

/// True when `value` of [`ENV_SWITCH`] leaves the GPU route enabled.
pub fn env_enables_gpu(value: Option<&str>) -> bool {
    value.map(str::trim) != Some("0")
}

/// Session latch: counts runtime errors and turns the GPU route off for good
/// at [`LATCH_ERROR_LIMIT`].
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct FallbackLatch {
    errors: u32,
}

impl FallbackLatch {
    pub const fn new() -> Self {
        Self { errors: 0 }
    }
    pub fn record_error(&mut self) {
        self.errors = self.errors.saturating_add(1);
    }
    pub fn is_off(&self) -> bool {
        self.errors >= LATCH_ERROR_LIMIT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latch_trips_at_three() {
        let mut latch = FallbackLatch::new();
        latch.record_error();
        latch.record_error();
        assert!(!latch.is_off());
        latch.record_error();
        assert!(latch.is_off());
    }

    #[test]
    fn env_switch() {
        assert!(!env_enables_gpu(Some("0")));
        assert!(env_enables_gpu(Some("1")));
        assert!(env_enables_gpu(None));
    }
}
