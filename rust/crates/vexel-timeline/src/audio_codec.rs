//! Bounded decoders for WAV, AIFF/AIFC and CAF linear PCM (16/24/32-bit
//! integer or 32-bit float, mono or stereo). Pure safe Rust; every length is
//! checked against the bytes actually present, so truncated files are rejected.

/// Largest accepted source file, in bytes.
pub const MAX_FILE_BYTES: usize = 256 * 1024 * 1024;
/// Longest accepted clip, in seconds.
pub const MAX_SECONDS: u64 = 600;
pub const MIN_RATE: u32 = 8_000;
pub const MAX_RATE: u32 = 192_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioError {
    /// Larger than [`MAX_FILE_BYTES`].
    TooLarge,
    /// Not a WAV/AIFF/CAF file or a malformed header.
    BadHeader,
    /// Valid container with an unsupported encoding, rate or channel count.
    Unsupported,
    /// A chunk claims more bytes than the file holds.
    Truncated,
    /// Longer than [`MAX_SECONDS`] or empty.
    BadLength,
    Alloc,
}
impl AudioError {
    pub const fn message(self) -> &'static str {
        match self {
            Self::TooLarge => "audio file is larger than 256 MiB",
            Self::BadHeader => "audio file is not a valid WAV, AIFF or CAF",
            Self::Unsupported => "audio encoding is unsupported; use PCM16, PCM24, PCM32 or float32, mono or stereo, 8-192 kHz",
            Self::Truncated => "audio file is truncated",
            Self::BadLength => "audio must be non-empty and at most 10 minutes",
            Self::Alloc => "audio allocation failed",
        }
    }
}

/// Decoded interleaved samples in `-1.0..=1.0`.
#[derive(Debug, Clone, PartialEq)]
pub struct Pcm {
    pub rate: u32,
    pub channels: usize,
    pub samples: Vec<f32>,
}
impl Pcm {
    pub fn frames(&self) -> usize {
        self.samples.len() / self.channels
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Info {
    pub rate: u32,
    pub channels: usize,
    pub frames: usize,
    pub format: &'static str,
}

#[derive(Clone, Copy)]
enum Enc {
    Int,
    Float,
}
struct Layout<'a> {
    rate: u32,
    channels: usize,
    bits: usize,
    enc: Enc,
    big_endian: bool,
    data: &'a [u8],
    format: &'static str,
}

fn be16(b: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_be_bytes(b.get(at..at + 2)?.try_into().ok()?))
}
fn be32(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes(b.get(at..at + 4)?.try_into().ok()?))
}
fn le16(b: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(b.get(at..at + 2)?.try_into().ok()?))
}
fn le32(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

fn wav(b: &[u8]) -> Result<Layout<'_>, AudioError> {
    let mut at = 12;
    let mut fmt: Option<(u16, usize, u32, usize)> = None;
    while at + 8 <= b.len() {
        let id = &b[at..at + 4];
        let size = le32(b, at + 4).ok_or(AudioError::BadHeader)? as usize;
        let body = at + 8;
        if id == b"fmt " {
            if size < 16 || body + size > b.len() {
                return Err(AudioError::BadHeader);
            }
            let mut tag = le16(b, body).ok_or(AudioError::BadHeader)?;
            if tag == 0xFFFE {
                if size < 40 {
                    return Err(AudioError::BadHeader);
                }
                tag = le16(b, body + 24).ok_or(AudioError::BadHeader)?;
            }
            let channels = le16(b, body + 2).ok_or(AudioError::BadHeader)? as usize;
            let rate = le32(b, body + 4).ok_or(AudioError::BadHeader)?;
            let align = le16(b, body + 12).ok_or(AudioError::BadHeader)? as usize;
            let bits = le16(b, body + 14).ok_or(AudioError::BadHeader)? as usize;
            if align != channels * bits / 8 {
                return Err(AudioError::Unsupported);
            }
            fmt = Some((tag, channels, rate, bits));
        } else if id == b"data" {
            let (tag, channels, rate, bits) = fmt.ok_or(AudioError::BadHeader)?;
            let end = body.checked_add(size).ok_or(AudioError::Truncated)?;
            if end > b.len() {
                return Err(AudioError::Truncated);
            }
            let enc = match tag {
                1 => Enc::Int,
                3 => Enc::Float,
                _ => return Err(AudioError::Unsupported),
            };
            return Ok(Layout { rate, channels, bits, enc, big_endian: false, data: &b[body..end], format: "wav" });
        }
        at = body.checked_add(size + (size & 1)).ok_or(AudioError::BadHeader)?;
    }
    Err(AudioError::BadHeader)
}

fn extended(b: &[u8], at: usize) -> Option<f64> {
    let raw = b.get(at..at + 10)?;
    let exp = i32::from(u16::from_be_bytes([raw[0] & 0x7F, raw[1]]));
    let mant = u64::from_be_bytes(raw[2..10].try_into().ok()?);
    if exp == 0x7FFF {
        return None;
    }
    let value = mant as f64 * 2f64.powi(exp - 16383 - 63);
    Some(if raw[0] & 0x80 != 0 { -value } else { value })
}

fn aiff(b: &[u8]) -> Result<Layout<'_>, AudioError> {
    let aifc = &b[8..12] == b"AIFC";
    let mut at = 12;
    let mut comm: Option<(usize, usize, f64, Enc, bool)> = None;
    while at + 8 <= b.len() {
        let id = &b[at..at + 4];
        let size = be32(b, at + 4).ok_or(AudioError::BadHeader)? as usize;
        let body = at + 8;
        if id == b"COMM" {
            if size < 18 || body + size > b.len() {
                return Err(AudioError::BadHeader);
            }
            let channels = be16(b, body).ok_or(AudioError::BadHeader)? as usize;
            let bits = be16(b, body + 6).ok_or(AudioError::BadHeader)? as usize;
            let rate = extended(b, body + 8).ok_or(AudioError::BadHeader)?;
            let (enc, little) = if aifc {
                if size < 22 {
                    return Err(AudioError::BadHeader);
                }
                match &b[body + 18..body + 22] {
                    b"NONE" | b"twos" => (Enc::Int, false),
                    b"sowt" => (Enc::Int, true),
                    b"fl32" | b"FL32" => (Enc::Float, false),
                    _ => return Err(AudioError::Unsupported),
                }
            } else {
                (Enc::Int, false)
            };
            comm = Some((channels, bits, rate, enc, little));
        } else if id == b"SSND" {
            let (channels, bits, rate, enc, little) = comm.ok_or(AudioError::BadHeader)?;
            let end = body.checked_add(size).ok_or(AudioError::Truncated)?;
            if size < 8 {
                return Err(AudioError::BadHeader);
            }
            if end > b.len() {
                return Err(AudioError::Truncated);
            }
            let offset = be32(b, body).ok_or(AudioError::BadHeader)? as usize;
            let start = (body + 8).checked_add(offset).filter(|s| *s <= end).ok_or(AudioError::BadHeader)?;
            if !(rate.is_finite() && rate >= f64::from(MIN_RATE) && rate <= f64::from(MAX_RATE)) || rate.fract() != 0.0 {
                return Err(AudioError::Unsupported);
            }
            return Ok(Layout { rate: rate as u32, channels, bits, enc, big_endian: !little, data: &b[start..end], format: "aiff" });
        }
        at = body.checked_add(size + (size & 1)).ok_or(AudioError::BadHeader)?;
    }
    Err(AudioError::BadHeader)
}

fn caf(b: &[u8]) -> Result<Layout<'_>, AudioError> {
    let mut at = 8;
    let mut desc: Option<(f64, Enc, bool, usize, usize)> = None;
    while at + 12 <= b.len() {
        let id = &b[at..at + 4];
        let size = i64::from_be_bytes(b[at + 4..at + 12].try_into().map_err(|_| AudioError::BadHeader)?);
        let body = at + 12;
        if id == b"desc" {
            if size < 32 || body + 32 > b.len() {
                return Err(AudioError::BadHeader);
            }
            let rate = f64::from_be_bytes(b[body..body + 8].try_into().map_err(|_| AudioError::BadHeader)?);
            if &b[body + 8..body + 12] != b"lpcm" {
                return Err(AudioError::Unsupported);
            }
            let flags = be32(b, body + 12).ok_or(AudioError::BadHeader)?;
            let bytes_per_packet = be32(b, body + 16).ok_or(AudioError::BadHeader)? as usize;
            let frames_per_packet = be32(b, body + 20).ok_or(AudioError::BadHeader)?;
            let channels = be32(b, body + 24).ok_or(AudioError::BadHeader)? as usize;
            let bits = be32(b, body + 28).ok_or(AudioError::BadHeader)? as usize;
            if bytes_per_packet != channels * bits / 8 || frames_per_packet != 1 {
                return Err(AudioError::Unsupported);
            }
            let enc = if flags & 1 != 0 { Enc::Float } else { Enc::Int };
            desc = Some((rate, enc, flags & 2 == 0, channels, bits));
        } else if id == b"data" {
            let (rate, enc, big_endian, channels, bits) = desc.ok_or(AudioError::BadHeader)?;
            let end = if size < 0 {
                b.len()
            } else {
                body.checked_add(usize::try_from(size).map_err(|_| AudioError::Truncated)?).ok_or(AudioError::Truncated)?
            };
            if end > b.len() {
                return Err(AudioError::Truncated);
            }
            if end < body + 4 {
                return Err(AudioError::BadHeader);
            }
            if !(rate.is_finite() && rate >= f64::from(MIN_RATE) && rate <= f64::from(MAX_RATE)) || rate.fract() != 0.0 {
                return Err(AudioError::Unsupported);
            }
            return Ok(Layout { rate: rate as u32, channels, bits, enc, big_endian, data: &b[body + 4..end], format: "caf" });
        } else if size < 0 {
            return Err(AudioError::BadHeader);
        }
        at = body.checked_add(usize::try_from(size).map_err(|_| AudioError::BadHeader)?).ok_or(AudioError::BadHeader)?;
    }
    Err(AudioError::BadHeader)
}

fn layout(bytes: &[u8]) -> Result<(Layout<'_>, usize), AudioError> {
    if bytes.len() > MAX_FILE_BYTES {
        return Err(AudioError::TooLarge);
    }
    if bytes.len() < 12 {
        return Err(AudioError::BadHeader);
    }
    let l = match &bytes[..4] {
        b"RIFF" if &bytes[8..12] == b"WAVE" => wav(bytes)?,
        b"FORM" if &bytes[8..12] == b"AIFF" || &bytes[8..12] == b"AIFC" => aiff(bytes)?,
        b"caff" => caf(bytes)?,
        _ => return Err(AudioError::BadHeader),
    };
    let ok = (1..=2).contains(&l.channels)
        && (MIN_RATE..=MAX_RATE).contains(&l.rate)
        && match l.enc {
            Enc::Int => matches!(l.bits, 16 | 24 | 32),
            Enc::Float => l.bits == 32,
        };
    if !ok {
        return Err(AudioError::Unsupported);
    }
    let frame_bytes = l.channels * l.bits / 8;
    if l.data.len() % frame_bytes != 0 {
        return Err(AudioError::BadHeader);
    }
    let frames = l.data.len() / frame_bytes;
    if frames == 0 || frames as u64 > u64::from(l.rate) * MAX_SECONDS {
        return Err(AudioError::BadLength);
    }
    Ok((l, frames))
}

/// Validates the header and reports the clip without decoding samples.
pub fn probe(bytes: &[u8]) -> Result<Info, AudioError> {
    let (l, frames) = layout(bytes)?;
    Ok(Info { rate: l.rate, channels: l.channels, frames, format: l.format })
}

/// Decodes the whole clip.
pub fn decode(bytes: &[u8]) -> Result<Pcm, AudioError> {
    let (l, frames) = layout(bytes)?;
    let count = frames * l.channels;
    let width = l.bits / 8;
    let mut samples = Vec::new();
    samples.try_reserve_exact(count).map_err(|_| AudioError::Alloc)?;
    for raw in l.data.chunks_exact(width).take(count) {
        let v = match (l.enc, width) {
            (Enc::Int, 2) => {
                let a = [raw[0], raw[1]];
                f32::from(if l.big_endian { i16::from_be_bytes(a) } else { i16::from_le_bytes(a) }) / 32768.0
            }
            (Enc::Int, 3) => {
                let (hi, mid, lo) = if l.big_endian { (raw[0], raw[1], raw[2]) } else { (raw[2], raw[1], raw[0]) };
                let n = (i32::from(hi as i8) << 16) | (i32::from(mid) << 8) | i32::from(lo);
                n as f32 / 8_388_608.0
            }
            (Enc::Int, _) => {
                let a = [raw[0], raw[1], raw[2], raw[3]];
                (if l.big_endian { i32::from_be_bytes(a) } else { i32::from_le_bytes(a) }) as f32 / 2_147_483_648.0
            }
            (Enc::Float, _) => {
                let a = [raw[0], raw[1], raw[2], raw[3]];
                let f = if l.big_endian { f32::from_be_bytes(a) } else { f32::from_le_bytes(a) };
                if f.is_finite() { f.clamp(-1.0, 1.0) } else { 0.0 }
            }
        };
        samples.push(v);
    }
    Ok(Pcm { rate: l.rate, channels: l.channels, samples })
}

/// A 16-bit PCM WAV file of `samples` (interleaved), for tests and the
/// platform muxer's temporary input.
pub fn encode_wav16(rate: u32, channels: usize, samples: &[i16]) -> Vec<u8> {
    let data = samples.len() * 2;
    let mut out = Vec::with_capacity(44 + data);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data as u32).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&(channels as u16).to_le_bytes());
    out.extend_from_slice(&rate.to_le_bytes());
    out.extend_from_slice(&(rate * channels as u32 * 2).to_le_bytes());
    out.extend_from_slice(&(channels as u16 * 2).to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&(data as u32).to_le_bytes());
    for s in samples {
        out.extend_from_slice(&s.to_le_bytes());
    }
    out
}
