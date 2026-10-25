//! Per-tick loudness envelope of decoded audio, for driving lip-sync and
//! other audio-reactive animation. Pure and deterministic: channels are
//! averaged to mono and each tick's RMS is accumulated in f64 in sample order.
use crate::audio_codec::Pcm;

/// Most ticks one envelope may cover.
pub const MAX_TICKS: usize = 100_000;

#[derive(Debug, Clone, PartialEq)]
pub struct Envelope {
    /// RMS per tick, scaled so the loudest tick is 1.0 (all zeros for silence).
    pub rms: Vec<f32>,
    /// Loudest tick RMS before scaling, in sample units (0..1).
    pub peak_rms: f32,
}

/// Envelope of `pcm` from `start_ms` into the clip, one value per timeline
/// tick (`ticks_per_second` of them per second), at most `limit` ticks (the
/// whole remaining clip when `None`). `None` for zero rate, or when the
/// request exceeds [`MAX_TICKS`].
pub fn envelope(pcm: &Pcm, ticks_per_second: u32, start_ms: u32, limit: Option<usize>) -> Option<Envelope> {
    if ticks_per_second == 0 || pcm.channels == 0 || pcm.rate == 0 {
        return None;
    }
    let frames = pcm.frames();
    let rate = u64::from(pcm.rate);
    let tps = u64::from(ticks_per_second);
    let first = (u64::from(start_ms) * rate / 1000) as usize;
    let remaining = frames.saturating_sub(first) as u64;
    // Ticks needed to cover the remaining frames (ceiling).
    let covering = ((remaining * tps + rate - 1) / rate) as usize;
    let count = limit.map_or(covering, |l| l.min(MAX_TICKS));
    if count > MAX_TICKS {
        return None;
    }
    let mut rms = Vec::with_capacity(count);
    for tick in 0..count as u64 {
        let from = first + (tick * rate / tps) as usize;
        let to = (first + ((tick + 1) * rate / tps) as usize).min(frames);
        if from >= to {
            rms.push(0.0);
            continue;
        }
        let mut sum = 0.0f64;
        for frame in from..to {
            let base = frame * pcm.channels;
            let mut mono = 0.0f64;
            for c in 0..pcm.channels {
                mono += f64::from(pcm.samples[base + c]);
            }
            mono /= pcm.channels as f64;
            sum += mono * mono;
        }
        rms.push((sum / (to - from) as f64).sqrt() as f32);
    }
    let peak_rms = rms.iter().copied().fold(0.0f32, f32::max);
    if peak_rms > 0.0 {
        for v in &mut rms {
            *v = (*v / peak_rms).clamp(0.0, 1.0);
        }
    }
    Some(Envelope { rms, peak_rms })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pcm(rate: u32, samples: Vec<f32>) -> Pcm {
        Pcm { rate, channels: 1, samples }
    }

    #[test]
    fn loud_then_quiet_then_silent() {
        let mut s = vec![0.8f32; 1000];
        s.extend(vec![0.4f32; 1000]);
        s.extend(vec![0.0f32; 1000]);
        let e = envelope(&pcm(3000, s), 3, 0, None).unwrap();
        assert_eq!(e.rms, vec![1.0, 0.5, 0.0]);
        assert!((e.peak_rms - 0.8).abs() < 1e-6);
    }

    #[test]
    fn deterministic_and_bounded() {
        let s: Vec<f32> = (0..4800).map(|i| ((i as f32) * 0.05).sin() * 0.5).collect();
        let p = pcm(4800, s);
        let a = envelope(&p, 24, 0, None).unwrap();
        assert_eq!(a, envelope(&p, 24, 0, None).unwrap());
        assert_eq!(a.rms.len(), 24);
        assert!(a.rms.iter().all(|v| (0.0..=1.0).contains(v)));
        assert_eq!(envelope(&p, 24, 0, Some(5)).unwrap().rms.len(), 5);
        assert_eq!(envelope(&p, 24, 500, None).unwrap().rms.len(), 12);
        assert!(envelope(&p, 0, 0, None).is_none());
    }
}
