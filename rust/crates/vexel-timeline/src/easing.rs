//! Per-key easing curves. A key's ease shapes the segment from that key to the next.
use crate::json::{array_spans, object_fields};
use crate::schema::{DecodeFailure, Error};
use vexel_base::Span;

/// Easing curve applied to normalized segment progress.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Ease {
    Step,
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
    EaseInBack,
    EaseOutBack,
    /// CSS style cubic bezier with control points `[x1, y1, x2, y2]`.
    Bezier([f64; 4]),
}

const BACK: f64 = 1.70158;
const BEZIER_Y_LIMIT: f64 = 4.0;

impl Ease {
    pub fn parse_name(name: &[u8]) -> Option<Ease> {
        Some(match name {
            b"step" => Ease::Step,
            b"linear" => Ease::Linear,
            b"ease_in" => Ease::EaseIn,
            b"ease_out" => Ease::EaseOut,
            b"ease_in_out" => Ease::EaseInOut,
            b"ease_in_back" => Ease::EaseInBack,
            b"ease_out_back" => Ease::EaseOutBack,
            _ => return None,
        })
    }

    /// True when the ease holds the segment start value (no interpolation).
    pub fn is_step(&self) -> bool {
        matches!(self, Ease::Step)
    }

    /// Maps progress `t` to eased progress. Inputs outside 0..=1 clamp; `0` maps to
    /// exactly 0 and `1` to exactly 1 for every curve. Never returns NaN.
    pub fn apply(&self, t: f64) -> f64 {
        let t = if t.is_nan() { 0.0 } else { t.clamp(0.0, 1.0) };
        if t == 0.0 {
            return 0.0;
        }
        if t == 1.0 {
            return 1.0;
        }
        match *self {
            Ease::Step => 0.0,
            Ease::Linear => t,
            Ease::EaseIn => t * t,
            Ease::EaseOut => 1.0 - (1.0 - t) * (1.0 - t),
            Ease::EaseInOut => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    1.0 - 2.0 * (1.0 - t) * (1.0 - t)
                }
            }
            Ease::EaseInBack => (BACK + 1.0) * t * t * t - BACK * t * t,
            Ease::EaseOutBack => {
                let u = t - 1.0;
                1.0 + (BACK + 1.0) * u * u * u + BACK * u * u
            }
            Ease::Bezier(p) => bezier(p, t),
        }
    }
}

/// Eases progress `t` with `ease`; see [`Ease::apply`].
pub fn ease_value(ease: Ease, t: f64) -> f64 {
    ease.apply(t)
}

fn bezier(p: [f64; 4], t: f64) -> f64 {
    let [x1, y1, x2, y2] = p;
    let coord = |a: f64, b: f64, s: f64| {
        let m = 1.0 - s;
        3.0 * m * m * s * a + 3.0 * m * s * s * b + s * s * s
    };
    let slope = |a: f64, b: f64, s: f64| {
        let m = 1.0 - s;
        3.0 * m * m * a + 6.0 * m * s * (b - a) + 3.0 * s * s * (1.0 - b)
    };
    // Newton from t, then bisection fallback; bounded iterations, x(s) is monotone.
    let mut s = t;
    let mut solved = false;
    for _ in 0..8 {
        let err = coord(x1, x2, s) - t;
        if err.abs() < 1e-12 {
            solved = true;
            break;
        }
        let d = slope(x1, x2, s);
        if d.abs() < 1e-6 {
            break;
        }
        let next = s - err / d;
        if !(0.0..=1.0).contains(&next) {
            break;
        }
        s = next;
    }
    if !solved {
        let (mut lo, mut hi) = (0.0f64, 1.0f64);
        s = t;
        for _ in 0..64 {
            let x = coord(x1, x2, s);
            if (x - t).abs() < 1e-12 {
                break;
            }
            if x < t {
                lo = s;
            } else {
                hi = s;
            }
            s = 0.5 * (lo + hi);
        }
    }
    let y = coord(y1, y2, s);
    if y.is_finite() {
        y
    } else {
        t
    }
}

fn bad(offset: usize) -> DecodeFailure {
    DecodeFailure {
        error: Error::InvalidValue,
        offset,
        canceled: false,
    }
}

fn number(source: &[u8], span: Span) -> Option<f64> {
    let text = std::str::from_utf8(source.get(span.start..span.end)?).ok()?;
    text.parse::<f64>().ok().filter(|n| n.is_finite())
}

/// Parses and validates a key `ease` value (a name string or `{"bezier":[x1,y1,x2,y2]}`).
pub(crate) fn parse(source: &[u8], span: Span) -> Result<Ease, DecodeFailure> {
    let raw = source
        .get(span.start..span.end)
        .ok_or_else(|| bad(span.start))?;
    if raw.first() == Some(&b'"') {
        let name = crate::schema::string(source, span).map_err(|_| bad(span.start))?;
        return Ease::parse_name(&name).ok_or_else(|| bad(span.start));
    }
    let fields = object_fields(source, span, None).map_err(|_| bad(span.start))?;
    if fields.len() != 1 || fields[0].key != b"bezier" {
        return Err(bad(span.start));
    }
    let value = fields[0].value;
    let spans = array_spans(source, value, 4, None).map_err(|_| bad(value.start))?;
    if spans.len() != 4 {
        return Err(bad(value.start));
    }
    let mut p = [0.0; 4];
    for (slot, s) in p.iter_mut().zip(spans) {
        *slot = number(source, s).ok_or_else(|| bad(s.start))?;
    }
    if !(0.0..=1.0).contains(&p[0])
        || !(0.0..=1.0).contains(&p[2])
        || p[1].abs() > BEZIER_Y_LIMIT
        || p[3].abs() > BEZIER_Y_LIMIT
    {
        return Err(bad(value.start));
    }
    Ok(Ease::Bezier(p))
}
