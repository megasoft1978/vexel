//! Small rotation affordance helpers, kept independent of native windows.

use super::Bounds;

/// Device-space indicator around a selected shape's bounds. Returns the centre
/// and the four rotation target points offset outside its corners.
pub fn rotation_targets(bounds: Bounds, offset: f64) -> [(f64, f64); 4] {
    [
        (bounds.0 - offset, bounds.1 - offset),
        (bounds.2 + offset, bounds.1 - offset),
        (bounds.2 + offset, bounds.3 + offset),
        (bounds.0 - offset, bounds.3 + offset),
    ]
}

/// Converts a pointer angle delta to a snapped angle when Shift is held.
pub fn snapped_rotation(angle_deg: f64, shift: bool) -> f64 {
    if shift {
        (angle_deg / 15.0).round() * 15.0
    } else {
        angle_deg
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn targets_sit_outside_corners_and_snap_to_fifteen_degrees() {
        assert_eq!(
            rotation_targets((10.0, 20.0, 30.0, 40.0), 8.0),
            [(2.0, 12.0), (38.0, 12.0), (38.0, 48.0), (2.0, 48.0)]
        );
        assert_eq!(snapped_rotation(22.0, true), 15.0);
        assert_eq!(snapped_rotation(22.0, false), 22.0);
    }
}
