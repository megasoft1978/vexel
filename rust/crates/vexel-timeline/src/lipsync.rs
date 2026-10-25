//! Lip-sync: a viseme per tick from a loudness envelope. Pure and
//! deterministic: integer hashing only (no trigonometry, no clocks, no
//! randomness), so the same envelope and seed always give the same mouth.

/// The viseme names a mouth set may use (at most 12), `rest` first.
pub const VISEMES: [&str; 12] = ["rest", "A", "E", "I", "O", "U", "MBP", "FV", "L", "WQ", "CDGK", "TH"];

pub const REST: u8 = 0;
const A: u8 = 1;
const E: u8 = 2;
const I: u8 = 3;
const O: u8 = 4;
const U: u8 = 5;
const MBP: u8 = 6;
const FV: u8 = 7;
const L: u8 = 8;
const WQ: u8 = 9;
const CDGK: u8 = 10;
const TH: u8 = 11;

/// Most words of a `text` hint, and most letters per word.
pub const MAX_WORDS: usize = 256;
pub const MAX_WORD_LEN: usize = 32;
/// Most visemes one word contributes to a spoken run.
pub const MAX_WORD_VISEMES: usize = 8;
/// Ticks a vowel is held before the pseudo-random choice is made again.
const HOLD: u32 = 6;

/// Index of a viseme name in [`VISEMES`].
pub fn index(name: &str) -> Option<u8> {
    VISEMES.iter().position(|v| *v == name).map(|i| i as u8)
}

/// Shapes to try, in order, when a mouth set lacks viseme `v` (then `rest`).
pub fn fallback(v: u8) -> &'static [u8] {
    match v {
        A => &[E, O],
        E => &[A, I],
        I => &[E, A],
        O => &[U, A],
        U => &[O],
        L => &[E, A],
        WQ => &[U, O],
        CDGK => &[E, A],
        TH => &[E, L],
        _ => &[],
    }
}

/// Loudness thresholds on the 0..1 envelope: below `close` the mouth is shut,
/// then small, medium and wide openings start at `close`, `mid` and `wide`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mapping {
    pub close: f32,
    pub mid: f32,
    pub wide: f32,
}

impl Default for Mapping {
    fn default() -> Self {
        Mapping { close: 0.08, mid: 0.35, wide: 0.65 }
    }
}

impl Mapping {
    /// Finite values with 0 <= close < mid < wide <= 1.
    pub fn valid(&self) -> bool {
        self.close.is_finite()
            && self.mid.is_finite()
            && self.wide.is_finite()
            && self.close >= 0.0
            && self.close < self.mid
            && self.mid < self.wide
            && self.wide <= 1.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Config {
    pub mapping: Mapping,
    /// Moving-average radius in ticks (0 = none).
    pub smoothing: u32,
    pub seed: u32,
}

pub const MAX_SMOOTHING: u32 = 8;

impl Default for Config {
    fn default() -> Self {
        Config { mapping: Mapping::default(), smoothing: 1, seed: 0 }
    }
}

fn mix(seed: u32, x: u32) -> u32 {
    let mut h = seed ^ x.wrapping_mul(0x9E37_79B1);
    h ^= h >> 15;
    h = h.wrapping_mul(0x85EB_CA77);
    h ^= h >> 13;
    h = h.wrapping_mul(0xC2B2_AE3D);
    h ^ (h >> 16)
}

/// Visemes a word is spoken with: vowels, bilabials, f/v, l, w/q, c/d/g/k/t
/// closures and th. Other letters are skipped; repeats collapse.
pub fn word_visemes(word: &str) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    let bytes: Vec<u8> = word.bytes().take(MAX_WORD_LEN).map(|b| b.to_ascii_lowercase()).collect();
    for (i, b) in bytes.iter().enumerate() {
        let v = match b {
            b'a' => A,
            b'e' => E,
            b'i' | b'y' => I,
            b'o' => O,
            b'u' => U,
            b'b' | b'm' | b'p' => MBP,
            b'f' | b'v' => FV,
            b'l' => L,
            b'w' | b'q' => WQ,
            b'c' | b'd' | b'g' | b'k' | b't' | b'n' | b'r' | b's' | b'z' | b'j' | b'x' => {
                // "th" is one sound; any other consonant here is a closure.
                if *b == b't' && bytes.get(i + 1) == Some(&b'h') { TH } else { CDGK }
            }
            _ => continue,
        };
        if out.last() != Some(&v) && out.len() < MAX_WORD_VISEMES {
            out.push(v);
        }
    }
    out
}

fn level_of(value: f32, m: &Mapping) -> u8 {
    if value < m.close { 0 } else if value < m.mid { 1 } else if value < m.wide { 2 } else { 3 }
}

fn vowel(level: u8, seed: u32, at: u32) -> u8 {
    let table: [u8; 3] = match level {
        1 => [O, U, O],
        2 => [E, O, I],
        _ => [A, E, A],
    };
    table[(mix(seed, at) % 3) as usize]
}

/// One viseme index per envelope tick. `words` (see [`word_visemes`]) are
/// spoken in turn, one per voiced run; with no words the vowel follows the
/// opening (wide A/E, medium E/O/I, small O/U) and is held for [`HOLD`] ticks.
pub fn derive(rms: &[f32], cfg: &Config, words: &[Vec<u8>]) -> Vec<u8> {
    let n = rms.len();
    let clean: Vec<f32> = rms.iter().map(|v| if v.is_finite() { v.clamp(0.0, 1.0) } else { 0.0 }).collect();
    let radius = cfg.smoothing.min(MAX_SMOOTHING) as usize;
    let mut level: Vec<u8> = Vec::with_capacity(n);
    for i in 0..n {
        let from = i.saturating_sub(radius);
        let to = (i + radius).min(n - 1);
        let mut sum = 0.0f32;
        for v in &clean[from..=to] {
            sum += *v;
        }
        level.push(level_of(sum / (to - from + 1) as f32, &cfg.mapping));
    }
    // A single tick that differs from equal neighbours is a glitch.
    for i in 1..n.saturating_sub(1) {
        if level[i - 1] == level[i + 1] && level[i] != level[i - 1] {
            level[i] = level[i - 1];
        }
    }
    let mut out = vec![REST; n];
    let mut spoken = 0usize;
    let mut i = 0usize;
    while i < n {
        if level[i] == 0 {
            i += 1;
            continue;
        }
        let start = i;
        while i < n && level[i] != 0 {
            i += 1;
        }
        let len = i - start;
        let word = if len >= 2 && !words.is_empty() {
            spoken += 1;
            Some(&words[(spoken - 1) % words.len()]).filter(|w| !w.is_empty())
        } else {
            None
        };
        let mut seg = start;
        for t in start..i {
            if t > seg && level[t] != level[t - 1] {
                seg = t;
            }
            out[t] = match word {
                Some(w) => w[(t - start) * w.len() / len],
                None => vowel(level[t], cfg.seed, (seg + (t - seg) / HOLD as usize * HOLD as usize) as u32),
            };
        }
        // A voiced run after silence opens on the lips (bilabial) for a tick.
        let after_silence = start == 0 || (start >= 2 && level[start - 1] == 0 && level[start - 2] == 0);
        if word.is_none() && len >= 3 && start > 0 && after_silence {
            out[start] = MBP;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_round_trip() {
        for (i, name) in VISEMES.iter().enumerate() {
            assert_eq!(index(name), Some(i as u8));
        }
        assert_eq!(index("Q"), None);
        assert!(VISEMES.len() <= 12);
    }

    #[test]
    fn silence_is_rest_and_loud_is_open() {
        let mut rms = vec![0.0f32; 10];
        rms.extend(vec![1.0f32; 10]);
        rms.extend(vec![0.0f32; 10]);
        let out = derive(&rms, &Config::default(), &[]);
        assert_eq!(out.len(), 30);
        assert!(out[..8].iter().all(|v| *v == REST));
        assert!(out[12..18].iter().all(|v| *v != REST));
        assert!(out[24..].iter().all(|v| *v == REST));
        let first = out.iter().position(|v| *v != REST).unwrap();
        assert_eq!(out[first], MBP);
    }

    #[test]
    fn deterministic_and_does_not_flap() {
        let rms: Vec<f32> = (0..200).map(|i| if (i / 20) % 2 == 0 { 0.9 } else { 0.0 }).collect();
        let cfg = Config { seed: 7, ..Config::default() };
        let a = derive(&rms, &cfg, &[]);
        assert_eq!(a, derive(&rms, &cfg, &[]));
        // Within one held stretch the viseme changes at most every HOLD ticks.
        let changes = a.windows(2).filter(|w| w[0] != w[1]).count();
        assert!(changes < 200 / 3, "{changes}");
        let other = derive(&rms, &Config { seed: 8, ..cfg }, &[]);
        assert_ne!(a, other, "the seed varies the vowels");
    }

    #[test]
    fn words_drive_the_run() {
        let rms = vec![0.9f32; 12];
        let words = vec![word_visemes("mama"), word_visemes("oh")];
        assert_eq!(word_visemes("mama"), vec![MBP, A, MBP, A]);
        let out = derive(&rms, &Config::default(), &words);
        assert_eq!(out[0], MBP);
        assert_eq!(out[3], A);
        assert!(word_visemes("123").is_empty());
        assert_eq!(word_visemes("the"), vec![TH, E]);
    }

    #[test]
    fn hostile_input_is_bounded() {
        assert!(derive(&[], &Config::default(), &[]).is_empty());
        let out = derive(&[f32::NAN, f32::INFINITY, -1.0, 5.0], &Config { smoothing: 99, ..Config::default() }, &[]);
        assert_eq!(out.len(), 4);
        assert!(!Mapping { close: 0.5, mid: 0.2, wide: 0.9 }.valid());
        assert!(Mapping::default().valid());
        assert_eq!(word_visemes(&"a".repeat(1000)).len(), 1);
    }
}
