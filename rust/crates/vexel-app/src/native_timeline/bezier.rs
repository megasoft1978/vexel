//! Custom cubic-bezier easing for the native timeline key editor.
//!
//! Limits mirror the `vexel-timeline` validator: x1 and x2 in `0..=1`, y1 and
//! y2 within `-4..=4`. The wire form carried by `NativeTimelineAction` and the
//! snapshot is `bezier:x1,y1,x2,y2`.

pub const X_MIN: f64 = 0.0;
pub const X_MAX: f64 = 1.0;
pub const Y_LIMIT: f64 = 4.0;
pub const STEP: f64 = 0.05;
/// CSS `ease-in-out`, the draft used when a key has no custom curve yet.
pub const DEFAULT: [f64; 4] = [0.42, 0.0, 0.58, 1.0];

/// Clamps every control point to the validator range; non-finite values fall
/// back to the matching `DEFAULT` component. Values are rounded to 1/1000.
pub fn clamp(p: [f64; 4]) -> [f64; 4] {
    let mut out = [0.0; 4];
    for (i, slot) in out.iter_mut().enumerate() {
        let v = if p[i].is_finite() { p[i] } else { DEFAULT[i] };
        let v = if i % 2 == 0 {
            v.clamp(X_MIN, X_MAX)
        } else {
            v.clamp(-Y_LIMIT, Y_LIMIT)
        };
        *slot = (v * 1000.0).round() / 1000.0 + 0.0;
    }
    out
}

pub fn encode(p: [f64; 4]) -> String {
    let p = clamp(p);
    format!("bezier:{},{},{},{}", p[0], p[1], p[2], p[3])
}

/// Parses the wire form; the result is clamped.
pub fn parse(ease: &str) -> Option<[f64; 4]> {
    let rest = ease.strip_prefix("bezier:")?;
    let mut p = [0.0; 4];
    let mut parts = rest.split(',');
    for slot in &mut p {
        *slot = parts.next()?.trim().parse::<f64>().ok()?;
    }
    if parts.next().is_some() {
        return None;
    }
    Some(clamp(p))
}

/// The JSON value stored in the key's `ease` field.
pub fn json(p: [f64; 4]) -> String {
    let p = clamp(p);
    format!("{{\"bezier\":[{},{},{},{}]}}", p[0], p[1], p[2], p[3])
}

/// Picker label: the preset name, or `custom` for a bezier curve.
pub fn label(ease: &str) -> &str {
    if parse(ease).is_some() {
        "custom"
    } else {
        ease
    }
}

/// Points of the curve in unit space (x right, y up), `n + 1` samples.
pub fn sample(p: [f64; 4], n: usize) -> Vec<(f64, f64)> {
    let p = clamp(p);
    (0..=n)
        .map(|i| {
            let t = i as f64 / n.max(1) as f64;
            let u = 1.0 - t;
            let at = |a: f64, b: f64| 3.0 * u * u * t * a + 3.0 * u * t * t * b + t * t * t;
            (at(p[0], p[2]), at(p[1], p[3]))
        })
        .collect()
}

/// The editor's working values; a transient UI state, never document state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BezierDraft {
    pub open: bool,
    pub values: [f64; 4],
}

impl Default for BezierDraft {
    fn default() -> Self {
        Self {
            open: false,
            values: DEFAULT,
        }
    }
}

impl BezierDraft {
    /// Moves one control value by one step, clamped to its range.
    pub fn step(&mut self, field: u8, up: bool) {
        let i = usize::from(field).min(3);
        let delta = if up { STEP } else { -STEP };
        self.values[i] += delta;
        self.values = clamp(self.values);
    }

    pub fn set(&mut self, values: [f64; 4]) {
        self.values = clamp(values);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_applies_validator_limits() {
        assert_eq!(clamp([-1.0, 9.0, 2.0, -9.0]), [0.0, 4.0, 1.0, -4.0]);
        assert_eq!(clamp([f64::NAN, f64::INFINITY, 0.5, 0.5])[0], DEFAULT[0]);
    }

    #[test]
    fn wire_form_round_trips_and_labels_custom() {
        let s = encode([0.25, 0.1, 0.25, 1.0]);
        assert_eq!(s, "bezier:0.25,0.1,0.25,1");
        assert_eq!(parse(&s), Some([0.25, 0.1, 0.25, 1.0]));
        assert_eq!(label(&s), "custom");
        assert_eq!(label("ease_in"), "ease_in");
        assert_eq!(parse("bezier:1,2,3"), None);
        assert_eq!(
            json([0.25, 0.1, 0.25, 1.0]),
            "{\"bezier\":[0.25,0.1,0.25,1]}"
        );
    }

    #[test]
    fn draft_steps_stay_in_range() {
        let mut d = BezierDraft::default();
        for _ in 0..100 {
            d.step(0, false);
            d.step(1, true);
        }
        assert_eq!(d.values[0], 0.0);
        assert_eq!(d.values[1], 4.0);
    }

    #[test]
    fn sample_runs_from_origin_to_one() {
        let pts = sample(DEFAULT, 8);
        assert_eq!(pts.len(), 9);
        assert_eq!(pts[0], (0.0, 0.0));
        assert!((pts[8].0 - 1.0).abs() < 1e-9 && (pts[8].1 - 1.0).abs() < 1e-9);
    }
}
