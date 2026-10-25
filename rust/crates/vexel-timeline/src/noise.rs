//! Seeded smooth noise modifier for numeric tracks (shake, flicker, sway, boil).
//!
//! The track member `"noise":{"amplitude":a,"frequency":hz,"seed":n,"octaves":k}` adds
//! `a * noise(tick)` to each numeric component of the sampled value. The function is
//! pure integer hashing plus IEEE `+ * /`, so frames are identical across runs, machines
//! and export paths.
use crate::json::object_fields;
use crate::schema::{DecodeFailure, Error};
use vexel_base::Span;

/// Largest accepted amplitude, in property units (pixels or degrees).
pub const MAX_AMPLITUDE: f64 = 100_000.0;
/// Largest accepted frequency, in cycles per second.
pub const MAX_FREQUENCY: f64 = 240.0;
/// Most octaves summed.
pub const MAX_OCTAVES: u32 = 4;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Noise {
    pub amplitude: f64,
    pub frequency: f64,
    pub seed: u32,
    pub octaves: u32,
}

fn mix(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

/// Lattice value in `[-1, 1]` for integer cell `cell`.
fn lattice(seed: u32, channel: u32, octave: u32, cell: i64) -> f64 {
    let key = (u64::from(seed) << 32 ^ u64::from(channel) << 8 ^ u64::from(octave))
        .wrapping_add((cell as u64).wrapping_mul(0xD6E8_FEB8_6659_FD93));
    let bits = mix(mix(key)) >> 11; // 53 bits
    (bits as f64) / ((1u64 << 52) as f64) - 1.0
}

impl Noise {
    /// Noise in `[-1, 1]` at timeline `tick` for component `channel`.
    pub fn unit(&self, tick: u32, ticks_per_second: u32, channel: u32) -> f64 {
        let base = f64::from(tick) * self.frequency / f64::from(ticks_per_second.max(1));
        let (mut sum, mut norm, mut gain, mut freq) = (0.0, 0.0, 1.0, 1.0);
        for octave in 0..self.octaves.clamp(1, MAX_OCTAVES) {
            let x = base * freq;
            let cell = x.floor();
            let f = x - cell;
            let s = f * f * (3.0 - 2.0 * f);
            let a = lattice(self.seed, channel, octave, cell as i64);
            let b = lattice(self.seed, channel, octave, cell as i64 + 1);
            sum += gain * (a + (b - a) * s);
            norm += gain;
            gain *= 0.5;
            freq *= 2.0;
        }
        sum / norm
    }

    /// `amplitude * unit(..)`.
    pub fn offset(&self, tick: u32, ticks_per_second: u32, channel: u32) -> f64 {
        self.amplitude * self.unit(tick, ticks_per_second, channel)
    }
}

fn bad(offset: usize) -> DecodeFailure {
    DecodeFailure { error: Error::InvalidValue, offset, canceled: false }
}

fn num(source: &[u8], span: Span) -> Option<f64> {
    let text = std::str::from_utf8(source.get(span.start..span.end)?).ok()?;
    text.parse::<f64>().ok().filter(|n| n.is_finite())
}

/// Parses and validates a track `noise` object. Unknown members are rejected.
pub(crate) fn parse(source: &[u8], span: Span) -> Result<Noise, DecodeFailure> {
    let fields = object_fields(source, span, None).map_err(|_| bad(span.start))?;
    if fields.len() > 4 {
        return Err(bad(span.start));
    }
    let mut noise = Noise { amplitude: 0.0, frequency: 1.0, seed: 0, octaves: 1 };
    let mut have_amplitude = false;
    for (i, field) in fields.iter().enumerate() {
        if fields[..i].iter().any(|prior| prior.key == field.key) {
            return Err(bad(field.value.start));
        }
        let value = num(source, field.value).ok_or_else(|| bad(field.value.start))?;
        let whole = value >= 0.0 && value.fract() == 0.0;
        match field.key.as_slice() {
            b"amplitude" if (0.0..=MAX_AMPLITUDE).contains(&value) => {
                noise.amplitude = value;
                have_amplitude = true;
            }
            b"frequency" if value > 0.0 && value <= MAX_FREQUENCY => noise.frequency = value,
            b"seed" if whole && value <= f64::from(u32::MAX) => noise.seed = value as u32,
            b"octaves" if whole && (1.0..=f64::from(MAX_OCTAVES)).contains(&value) => {
                noise.octaves = value as u32;
            }
            _ => return Err(bad(field.value.start)),
        }
    }
    if !have_amplitude {
        return Err(bad(span.start));
    }
    Ok(noise)
}
