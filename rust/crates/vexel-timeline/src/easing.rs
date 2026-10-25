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
    EaseInOutBack,
    EaseInCubic,
    EaseOutCubic,
    EaseInOutCubic,
    EaseInElastic,
    EaseOutElastic,
    EaseInOutElastic,
    EaseInBounce,
    EaseOutBounce,
    EaseInOutBounce,
    /// CSS style cubic bezier with control points `[x1, y1, x2, y2]`.
    Bezier([f64; 4]),
}

const BACK: f64 = 1.70158;
const BACK_IN_OUT: f64 = BACK * 1.525;
const ELASTIC: f64 = std::f64::consts::TAU / 3.0;
const ELASTIC_IN_OUT: f64 = std::f64::consts::TAU / 4.5;
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
            b"ease_out_back" | b"back" => Ease::EaseOutBack,
            b"ease_in_out_back" => Ease::EaseInOutBack,
            b"ease_in_cubic" => Ease::EaseInCubic,
            b"ease_out_cubic" => Ease::EaseOutCubic,
            b"ease_in_out_cubic" => Ease::EaseInOutCubic,
            b"ease_in_elastic" => Ease::EaseInElastic,
            b"ease_out_elastic" | b"elastic" => Ease::EaseOutElastic,
            b"ease_in_out_elastic" => Ease::EaseInOutElastic,
            b"ease_in_bounce" => Ease::EaseInBounce,
            b"ease_out_bounce" | b"bounce" => Ease::EaseOutBounce,
            b"ease_in_out_bounce" => Ease::EaseInOutBounce,
            _ => return None,
        })
    }

    /// Canonical preset name, or `None` for a bezier curve.
    pub fn name(&self) -> Option<&'static str> {
        Some(match self {
            Ease::Step => "step",
            Ease::Linear => "linear",
            Ease::EaseIn => "ease_in",
            Ease::EaseOut => "ease_out",
            Ease::EaseInOut => "ease_in_out",
            Ease::EaseInBack => "ease_in_back",
            Ease::EaseOutBack => "ease_out_back",
            Ease::EaseInOutBack => "ease_in_out_back",
            Ease::EaseInCubic => "ease_in_cubic",
            Ease::EaseOutCubic => "ease_out_cubic",
            Ease::EaseInOutCubic => "ease_in_out_cubic",
            Ease::EaseInElastic => "ease_in_elastic",
            Ease::EaseOutElastic => "ease_out_elastic",
            Ease::EaseInOutElastic => "ease_in_out_elastic",
            Ease::EaseInBounce => "ease_in_bounce",
            Ease::EaseOutBounce => "ease_out_bounce",
            Ease::EaseInOutBounce => "ease_in_out_bounce",
            Ease::Bezier(_) => return None,
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
            Ease::EaseInOutBack => {
                if t < 0.5 {
                    let u = 2.0 * t;
                    0.5 * (u * u * ((BACK_IN_OUT + 1.0) * u - BACK_IN_OUT))
                } else {
                    let u = 2.0 * t - 2.0;
                    0.5 * (u * u * ((BACK_IN_OUT + 1.0) * u + BACK_IN_OUT) + 2.0)
                }
            }
            Ease::EaseInCubic => t * t * t,
            Ease::EaseOutCubic => 1.0 - (1.0 - t).powi(3),
            Ease::EaseInOutCubic => {
                if t < 0.5 {
                    4.0 * t * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
                }
            }
            Ease::EaseInElastic => elastic_in(t),
            Ease::EaseOutElastic => 1.0 - elastic_in(1.0 - t),
            Ease::EaseInOutElastic => {
                let s = (20.0 * t - 11.125) * ELASTIC_IN_OUT;
                if t < 0.5 {
                    -(2f64.powf(20.0 * t - 10.0) * s.sin()) / 2.0
                } else {
                    2f64.powf(-20.0 * t + 10.0) * s.sin() / 2.0 + 1.0
                }
            }
            Ease::EaseOutBounce => bounce_out(t),
            Ease::EaseInBounce => 1.0 - bounce_out(1.0 - t),
            Ease::EaseInOutBounce => {
                if t < 0.5 {
                    (1.0 - bounce_out(1.0 - 2.0 * t)) / 2.0
                } else {
                    (1.0 + bounce_out(2.0 * t - 1.0)) / 2.0
                }
            }
            Ease::Bezier(p) => bezier(p, t),
        }
    }
}

fn elastic_in(t: f64) -> f64 {
    -(2f64.powf(10.0 * t - 10.0)) * ((t * 10.0 - 10.75) * ELASTIC).sin()
}

fn bounce_out(t: f64) -> f64 {
    const N: f64 = 7.5625;
    const D: f64 = 2.75;
    if t < 1.0 / D {
        N * t * t
    } else if t < 2.0 / D {
        let u = t - 1.5 / D;
        N * u * u + 0.75
    } else if t < 2.5 / D {
        let u = t - 2.25 / D;
        N * u * u + 0.9375
    } else {
        let u = t - 2.625 / D;
        N * u * u + 0.984375
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
    let value = if raw.first() == Some(&b'[') {
        span
    } else {
        let fields = object_fields(source, span, None).map_err(|_| bad(span.start))?;
        if fields.len() != 1 || fields[0].key != b"bezier" {
            return Err(bad(span.start));
        }
        fields[0].value
    };
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
