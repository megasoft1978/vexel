//! Deterministic helpers for the `effect_apply` presets: integer hash, trig
//! without same-argument sin/cos pairs, number formatting and track builders.

use std::collections::BTreeMap;

/// Most elements one preset may generate.
pub(crate) const MAX_ELEMENTS: usize = 256;
/// Most keys one generated track may carry.
pub(crate) const MAX_TRACK_KEYS: usize = 1024;

/// Apple release builds can merge `sin(x)` and `cos(x)` of one argument into a
/// call whose results differ by an ulp from separate calls; `inline(never)`
/// keeps each a single call so debug and release agree (see vexel-svg-path arc).
#[inline(never)]
pub(crate) fn sin_unpaired(angle: f64) -> f64 {
    angle.sin()
}
#[inline(never)]
pub(crate) fn cos_unpaired(angle: f64) -> f64 {
    angle.cos()
}

pub(crate) fn mix(seed: u32, a: u32, b: u32) -> u32 {
    let mut h = seed ^ a.wrapping_mul(0x9E37_79B1) ^ b.wrapping_mul(0x85EB_CA6B);
    h ^= h >> 16;
    h = h.wrapping_mul(0x7FEB_352D);
    h ^= h >> 15;
    h = h.wrapping_mul(0x846C_A68B);
    h ^ (h >> 16)
}

/// Uniform in [0, 1).
pub(crate) fn unit(seed: u32, a: u32, b: u32) -> f64 {
    f64::from(mix(seed, a, b)) / 4_294_967_296.0
}

pub(crate) fn num(v: f64) -> String {
    let r = (v * 10_000.0).round() / 10_000.0 + 0.0;
    format!("{r}")
}

pub(crate) fn scalar(v: f64) -> Vec<u8> {
    num(v).into_bytes()
}

pub(crate) fn flag(on: bool) -> Vec<u8> {
    if on {
        b"true".to_vec()
    } else {
        b"false".to_vec()
    }
}

/// A `trs` value in the canonical member form.
pub(crate) fn trs(t: [f64; 2], rotate: f64, s: [f64; 2], pivot: [f64; 2], skew: f64) -> Vec<u8> {
    format!(
        "{{\"tx\":{},\"ty\":{},\"rotation_deg\":{},\"sx\":{},\"sy\":{},\"pivot_x\":{},\"pivot_y\":{},\"skew_x_deg\":{}}}",
        num(t[0]),
        num(t[1]),
        num(rotate),
        num(s[0]),
        num(s[1]),
        num(pivot[0]),
        num(pivot[1]),
        num(skew)
    )
    .into_bytes()
}

pub(crate) type Key = (u32, Vec<u8>, Option<&'static str>);

/// A generated property track.
pub(crate) struct FxTrack {
    pub id: String,
    pub target: String,
    pub property: &'static str,
    pub value_type: &'static str,
    pub interpolation: &'static str,
    pub value: Vec<u8>,
    pub keys: Vec<Key>,
}

/// Builds a track; a later key at one tick wins and an un-eased tick-zero key
/// becomes the initial value.
pub(crate) fn track(
    id: String,
    target: &str,
    property: &'static str,
    value_type: &'static str,
    interpolation: &'static str,
    rest: Vec<u8>,
    raw: Vec<Key>,
) -> Result<FxTrack, &'static str> {
    let mut map: BTreeMap<u32, (Vec<u8>, Option<&'static str>)> = BTreeMap::new();
    for (tick, value, ease) in raw {
        map.insert(tick, (value, ease));
    }
    let mut value = rest;
    if let Some((v, None)) = map.get(&0) {
        value = v.clone();
        map.remove(&0);
    }
    if map.len() > MAX_TRACK_KEYS {
        return Err("effect_apply: too many keys for one track; raise period or shorten ticks");
    }
    Ok(FxTrack {
        id,
        target: target.to_owned(),
        property,
        value_type,
        interpolation,
        value,
        keys: map.into_iter().map(|(t, (v, e))| (t, v, e)).collect(),
    })
}

/// Triangle wave between `hi` (at `start`) and `lo`, `period` ticks per cycle.
pub(crate) fn pulse(
    start: u32,
    end: u32,
    lo: f64,
    hi: f64,
    period: u32,
) -> Result<Vec<(u32, f64)>, &'static str> {
    let half = (period / 2).max(1);
    let mut out = Vec::new();
    let (mut t, mut k) = (start, 0u32);
    while t <= end {
        out.push((t, if k % 2 == 0 { hi } else { lo }));
        if out.len() > MAX_TRACK_KEYS {
            return Err("effect_apply: too many keys for one track; raise period or shorten ticks");
        }
        t += half;
        k += 1;
    }
    Ok(out)
}

/// `#rgb` or `#rrggbb`.
pub(crate) fn color_ok(c: &str) -> bool {
    let h = c.strip_prefix('#').unwrap_or("");
    matches!(h.len(), 3 | 6) && h.bytes().all(|b| b.is_ascii_hexdigit())
}
