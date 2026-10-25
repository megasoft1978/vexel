//! Blink timing: deterministic pseudo-random blink ticks (integer hashing
//! only, so the same seed always gives the same blinks).

use crate::lipsync::mix;

/// Most blinks one call generates or accepts.
pub const MAX_BLINKS: usize = 256;

/// Start ticks of blinks of `duration` ticks inside `[start, end)`: the
/// first blink follows `start` by a gap, each later one follows the previous
/// blink's end by a gap, and every gap lies in `min_gap..=max_gap` (at least
/// 1). A blink that would not end before `end` is not generated. Needs
/// `min_gap <= max_gap`; returns nothing otherwise.
pub fn blink_ticks(seed: u32, start: u32, end: u32, duration: u32, min_gap: u32, max_gap: u32) -> Vec<u32> {
    let mut out = Vec::new();
    if min_gap == 0 || min_gap > max_gap || duration == 0 {
        return out;
    }
    let span = u64::from(max_gap - min_gap) + 1;
    let mut cursor = u64::from(start);
    let mut index = 0u32;
    while out.len() < MAX_BLINKS {
        let gap = u64::from(min_gap) + u64::from(mix(seed, index)) % span;
        let at = cursor + gap;
        if at + u64::from(duration) >= u64::from(end) {
            break;
        }
        out.push(at as u32);
        cursor = at + u64::from(duration);
        index += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_spaced_and_bounded() {
        let a = blink_ticks(5, 0, 1000, 4, 20, 60);
        assert_eq!(a, blink_ticks(5, 0, 1000, 4, 20, 60));
        assert_ne!(a, blink_ticks(6, 0, 1000, 4, 20, 60));
        assert!(a.len() >= 10);
        assert!(a[0] >= 20 && a.windows(2).all(|w| w[1] - w[0] >= 4 + 20 && w[1] - w[0] <= 4 + 60));
        assert!(a.iter().all(|t| t + 4 < 1000));
    }

    #[test]
    fn degenerate_ranges_yield_nothing() {
        assert!(blink_ticks(1, 0, 100, 4, 0, 5).is_empty());
        assert!(blink_ticks(1, 0, 100, 4, 9, 5).is_empty());
        assert!(blink_ticks(1, 0, 10, 4, 20, 30).is_empty());
        assert!(blink_ticks(1, 0, u32::MAX, 4, 1, 1).len() <= MAX_BLINKS);
        assert!(blink_ticks(1, u32::MAX - 3, u32::MAX, 2, 1, 1).is_empty());
    }
}
