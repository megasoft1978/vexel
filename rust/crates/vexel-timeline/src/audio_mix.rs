//! Deterministic offline mixdown of timeline audio tracks to 48 kHz stereo
//! PCM16: linear-interpolation resampling, trim, loop, gain, fades, envelope
//! ducking, then a look-ahead peak limiter. Pure safe Rust; the same inputs
//! always produce identical bytes.
use crate::audio::{AudioKind, AudioTrack};
use crate::audio_codec::{AudioError, Pcm};

/// Output sample rate of the mixdown.
pub const OUT_RATE: u32 = 48_000;
/// Longest mixdown, in output frames (10 minutes).
pub const MAX_OUT_FRAMES: usize = 600 * OUT_RATE as usize;
/// Samples per ducking-envelope block (10 ms).
const BLOCK: usize = 480;
const ACTIVE_LEVEL: f32 = 0.003_16;
const LIMIT_CEILING: f32 = 0.98;
const LOOKAHEAD: usize = 96;
const RELEASE_STEP: f32 = 1.0 / (0.2 * OUT_RATE as f32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MixError {
    Audio(AudioError),
    /// A track's source file is missing, unreadable or outside its directory.
    Source,
    /// More output frames than [`MAX_OUT_FRAMES`], or zero ticks per second.
    Limit,
    Alloc,
}
impl From<AudioError> for MixError {
    fn from(e: AudioError) -> Self {
        Self::Audio(e)
    }
}

/// Output frames covering `ticks` at `ticks_per_second` (floor).
pub fn frames_for_ticks(ticks: u64, ticks_per_second: u32) -> usize {
    (ticks.saturating_mul(u64::from(OUT_RATE)) / u64::from(ticks_per_second.max(1))) as usize
}

fn db_to_gain(db: f64) -> f32 {
    10f64.powf(db / 20.0) as f32
}

struct Plan {
    start: usize,
    end: usize,
    region_start: usize,
    region_len: usize,
    step: f64,
    gain: f32,
    fade_in: usize,
    fade_out: usize,
    looped: bool,
}

fn plan(track: &AudioTrack, pcm: &Pcm, tps: u32, out_frames: usize) -> Option<Plan> {
    let total = pcm.frames();
    let ms = |v: u32| (u64::from(v) * u64::from(pcm.rate) / 1000) as usize;
    let region_start = ms(track.trim_in_ms);
    let region_end = track.trim_out_ms.map_or(total, |o| ms(o).min(total));
    let start = frames_for_ticks(u64::from(track.start_tick), tps);
    if region_end <= region_start || start >= out_frames {
        return None;
    }
    let region_len = region_end - region_start;
    let step = f64::from(pcm.rate) / f64::from(OUT_RATE);
    let end = if track.looped {
        out_frames
    } else {
        (start + (region_len as f64 / step).floor() as usize).min(out_frames)
    };
    (end > start).then(|| Plan {
        start,
        end,
        region_start,
        region_len,
        step,
        gain: db_to_gain(track.gain_db),
        fade_in: frames_for_ticks(u64::from(track.fade_in_ticks), tps),
        fade_out: frames_for_ticks(u64::from(track.fade_out_ticks), tps),
        looped: track.looped,
    })
}

/// Calls `sink(frame, left, right)` for every output frame the track covers.
fn render(plan: &Plan, pcm: &Pcm, mut sink: impl FnMut(usize, f32, f32)) {
    let ch = pcm.channels;
    let at = |index: usize, c: usize| pcm.samples[index * ch + c.min(ch - 1)];
    for i in plan.start..plan.end {
        let rel = i - plan.start;
        let mut pos = rel as f64 * plan.step;
        if plan.looped {
            pos %= plan.region_len as f64;
        }
        let whole = pos.floor();
        let frac = (pos - whole) as f32;
        let local0 = whole as usize;
        let local1 = if plan.looped { (local0 + 1) % plan.region_len } else { (local0 + 1).min(plan.region_len - 1) };
        let (i0, i1) = (plan.region_start + local0, plan.region_start + local1);
        let mut gain = plan.gain;
        if rel < plan.fade_in {
            gain *= rel as f32 / plan.fade_in as f32;
        }
        let remaining = plan.end - i;
        if remaining <= plan.fade_out {
            gain *= remaining as f32 / plan.fade_out as f32;
        }
        let lerp = |c: usize| {
            let (a, b) = (at(i0, c), at(i1, c));
            (a + (b - a) * frac) * gain
        };
        sink(i, lerp(0), lerp(1));
    }
}

fn blocks(out_frames: usize) -> usize {
    out_frames.div_ceil(BLOCK) + 1
}

/// Per-block duck gain (linear) for a track that ducks under `peaks`.
fn duck_envelope(peaks: &[f32], amount_db: f64) -> Vec<f32> {
    let low = db_to_gain(amount_db);
    let mut out = Vec::with_capacity(peaks.len());
    let mut current = 1.0f32;
    for b in 0..peaks.len() {
        let active = peaks[b] > ACTIVE_LEVEL || peaks.get(b + 1).is_some_and(|p| *p > ACTIVE_LEVEL);
        let target = if active { low } else { 1.0 };
        current += (target - current) * if target < current { 0.5 } else { 0.05 };
        out.push(current);
    }
    out
}

fn duck_at(envelope: &[f32], frame: usize) -> f32 {
    let (b, within) = (frame / BLOCK, (frame % BLOCK) as f32 / BLOCK as f32);
    let a = envelope[b.min(envelope.len() - 1)];
    let n = envelope[(b + 1).min(envelope.len() - 1)];
    a + (n - a) * within
}

fn limit(acc: &mut [f32]) -> Vec<f32> {
    let frames = acc.len() / 2;
    let mut gain = Vec::with_capacity(frames);
    for f in 0..frames {
        let peak = acc[f * 2].abs().max(acc[f * 2 + 1].abs());
        gain.push(if peak > LIMIT_CEILING { LIMIT_CEILING / peak } else { 1.0 });
    }
    for i in 1..frames {
        gain[i] = gain[i].min(gain[i - 1] + RELEASE_STEP);
    }
    for i in (0..frames.saturating_sub(1)).rev() {
        gain[i] = gain[i].min(gain[i + 1] + 1.0 / LOOKAHEAD as f32);
    }
    gain
}

/// Mixes `tracks` into `out_frames` of interleaved 48 kHz stereo PCM16.
/// `load` decodes a track's source file; it may be called twice per track
/// (once to measure a ducking envelope, once to mix) and must be repeatable.
pub fn mixdown(
    tracks: &[AudioTrack],
    ticks_per_second: u32,
    out_frames: usize,
    load: &mut dyn FnMut(&AudioTrack) -> Result<Pcm, MixError>,
) -> Result<Vec<i16>, MixError> {
    if ticks_per_second == 0 || out_frames > MAX_OUT_FRAMES {
        return Err(MixError::Limit);
    }
    let mut acc: Vec<f32> = Vec::new();
    acc.try_reserve_exact(out_frames * 2).map_err(|_| MixError::Alloc)?;
    acc.resize(out_frames * 2, 0.0);
    let n_blocks = blocks(out_frames);
    // Pass 1: activity of every track something ducks under.
    let mut peaks: Vec<Option<Vec<f32>>> = vec![None; tracks.len()];
    for (index, track) in tracks.iter().enumerate() {
        let referenced = tracks.iter().any(|t| {
            t.duck.as_ref().is_some_and(|d| d.under == track.id || AudioKind::parse(d.under.as_bytes()) == Some(track.kind))
        });
        if !referenced {
            continue;
        }
        let pcm = load(track)?;
        let mut env = vec![0.0f32; n_blocks];
        if let Some(p) = plan(track, &pcm, ticks_per_second, out_frames) {
            render(&p, &pcm, |i, l, r| {
                let slot = &mut env[i / BLOCK];
                *slot = slot.max(l.abs()).max(r.abs());
            });
        }
        peaks[index] = Some(env);
    }
    // Pass 2: mix.
    for track in tracks {
        let pcm = load(track)?;
        let Some(p) = plan(track, &pcm, ticks_per_second, out_frames) else { continue };
        let envelope = track.duck.as_ref().map(|d| {
            let mut merged = vec![0.0f32; n_blocks];
            for (other, env) in tracks.iter().zip(&peaks) {
                let hit = other.id != track.id
                    && (other.id == d.under || AudioKind::parse(d.under.as_bytes()) == Some(other.kind));
                if let (true, Some(env)) = (hit, env) {
                    for (m, e) in merged.iter_mut().zip(env) {
                        *m = m.max(*e);
                    }
                }
            }
            duck_envelope(&merged, d.amount_db)
        });
        render(&p, &pcm, |i, l, r| {
            let g = envelope.as_ref().map_or(1.0, |e| duck_at(e, i));
            acc[i * 2] += l * g;
            acc[i * 2 + 1] += r * g;
        });
    }
    let gain = limit(&mut acc);
    let mut out = Vec::new();
    out.try_reserve_exact(acc.len()).map_err(|_| MixError::Alloc)?;
    for (i, v) in acc.iter().enumerate() {
        out.push((v * gain[i / 2] * 32767.0).round().clamp(-32767.0, 32767.0) as i16);
    }
    Ok(out)
}
