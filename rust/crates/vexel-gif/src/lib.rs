#![forbid(unsafe_code)]
//! First-party GIF89a encoder for the ADR-0029 V1 animation export.
//!
//! Scope: one global palette (exact when the frame set has at most 256
//! colour slots, otherwise a fixed uniform 6x7x6 palette without dithering),
//! variable-width LZW, a NETSCAPE2.0 infinite-loop block and one graphic
//! control block per frame. Every frame is a full-canvas image. Pixels with
//! alpha 0 map to one transparent index; every other alpha is treated as
//! opaque (callers composite partially transparent frames first).
//!
//! Limits mirror the C ImageIO path (`platform_animation_encode_gif`): at
//! most 120 frames, 60 fps, 4096 px per side, 32,000,000 aggregate pixels and
//! 128 MiB of output. Input is validated before any large allocation.
//!
//! Delays are whole centiseconds, as the format requires. For an fps the delay
//! is `round(100 / fps)` centiseconds (ties away from zero) and never below 1,
//! so 30 fps encodes 3 cs (33.3 ms becomes 30 ms) and 60 fps encodes 2 cs.

mod lzw;
mod palette;

pub use palette::MAX_QUANT_ERROR;
use palette::Palette;

pub const MAX_FRAMES: usize = 120;
pub const MAX_FPS: u32 = 60;
pub const MAX_DIMENSION: u32 = 4096;
pub const MAX_TOTAL_PIXELS: u64 = 32_000_000;
pub const MAX_OUTPUT_BYTES: usize = 128 * 1024 * 1024;

/// Typed encoder failure; none of these allocate beyond the failed check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GifError {
    /// No frames were supplied.
    Empty,
    TooManyFrames,
    /// `fps` was 0 or above [`MAX_FPS`].
    InvalidFps,
    /// A frame side was 0 or above [`MAX_DIMENSION`].
    InvalidDimensions,
    /// Frame sizes differ.
    FrameSizeMismatch,
    /// `rgba.len()` is not `width * height * 4`.
    InvalidPixelData,
    /// Total pixels over all frames exceed [`MAX_TOTAL_PIXELS`].
    TooManyPixels,
    /// The encoded stream would exceed [`MAX_OUTPUT_BYTES`].
    OutputTooLarge,
    /// Scratch or output memory could not be reserved.
    OutOfMemory,
}

/// One RGBA8 frame (straight alpha, row-major, top row first).
#[derive(Debug, Clone, Copy)]
pub struct Frame<'a> {
    pub width: u32,
    pub height: u32,
    pub rgba: &'a [u8],
    /// Delay in centiseconds. `None` uses the delay implied by the `fps`
    /// passed to [`encode`].
    pub delay_cs: Option<u16>,
}

/// Centisecond delay for a frame rate: `round(100 / fps)`, minimum 1.
pub fn delay_cs_for_fps(fps: u32) -> Result<u16, GifError> {
    if fps == 0 || fps > MAX_FPS {
        return Err(GifError::InvalidFps);
    }
    Ok(((100 + fps / 2) / fps).max(1) as u16)
}

fn validate(frames: &[Frame<'_>], fps: u32) -> Result<u16, GifError> {
    let default_delay = delay_cs_for_fps(fps)?;
    let Some(first) = frames.first() else {
        return Err(GifError::Empty);
    };
    if frames.len() > MAX_FRAMES {
        return Err(GifError::TooManyFrames);
    }
    let mut total: u64 = 0;
    for frame in frames {
        if frame.width == 0
            || frame.height == 0
            || frame.width > MAX_DIMENSION
            || frame.height > MAX_DIMENSION
        {
            return Err(GifError::InvalidDimensions);
        }
        if frame.width != first.width || frame.height != first.height {
            return Err(GifError::FrameSizeMismatch);
        }
        total += u64::from(frame.width) * u64::from(frame.height);
        if total > MAX_TOTAL_PIXELS {
            return Err(GifError::TooManyPixels);
        }
    }
    let expected = u64::from(first.width) * u64::from(first.height) * 4;
    if frames
        .iter()
        .any(|frame| frame.rgba.len() as u64 != expected)
    {
        return Err(GifError::InvalidPixelData);
    }
    Ok(default_delay)
}

fn reserve(out: &mut Vec<u8>, additional: usize) -> Result<(), GifError> {
    out.try_reserve(additional)
        .map_err(|_| GifError::OutOfMemory)
}

/// Encodes `frames` as a looping GIF89a at `fps`.
pub fn encode(frames: &[Frame<'_>], fps: u32) -> Result<Vec<u8>, GifError> {
    let default_delay = validate(frames, fps)?;
    let (width, height) = (frames[0].width, frames[0].height);
    let palette = Palette::build(frames);
    let bits = palette.table_bits();
    let table_len = 1usize << bits;

    let mut out = Vec::new();
    reserve(&mut out, 13 + table_len * 3 + 19 + 1)?;
    out.extend_from_slice(b"GIF89a");
    out.extend_from_slice(&(width as u16).to_le_bytes());
    out.extend_from_slice(&(height as u16).to_le_bytes());
    out.push(0x80 | 0x70 | (bits as u8 - 1));
    out.extend_from_slice(&[0, 0]);
    for entry in 0..table_len {
        out.extend_from_slice(&palette.colors.get(entry).copied().unwrap_or([0, 0, 0]));
    }
    out.extend_from_slice(&[0x21, 0xFF, 0x0B]);
    out.extend_from_slice(b"NETSCAPE2.0");
    out.extend_from_slice(&[0x03, 0x01, 0x00, 0x00, 0x00]);

    let min_code_size = (bits as u8).max(2);
    let disposal: u8 = if palette.transparent.is_some() { 2 } else { 1 };
    let mut indices: Vec<u8> = Vec::new();
    indices
        .try_reserve_exact(width as usize * height as usize)
        .map_err(|_| GifError::OutOfMemory)?;
    for frame in frames {
        indices.clear();
        indices.extend(frame.rgba.chunks_exact(4).map(|px| palette.index_of(px)));
        let delay = frame.delay_cs.unwrap_or(default_delay);
        reserve(&mut out, 8 + 10 + 1 + indices.len() / 2 + 16)?;
        out.extend_from_slice(&[0x21, 0xF9, 0x04]);
        out.push((disposal << 2) | u8::from(palette.transparent.is_some()));
        out.extend_from_slice(&delay.to_le_bytes());
        out.push(palette.transparent.unwrap_or(0));
        out.push(0);
        out.push(0x2C);
        out.extend_from_slice(&[0, 0, 0, 0]);
        out.extend_from_slice(&(width as u16).to_le_bytes());
        out.extend_from_slice(&(height as u16).to_le_bytes());
        out.push(0);
        lzw::encode(&indices, min_code_size, &mut out);
        if out.len() + 1 > MAX_OUTPUT_BYTES {
            return Err(GifError::OutputTooLarge);
        }
    }
    out.push(0x3B);
    if out.len() > MAX_OUTPUT_BYTES {
        return Err(GifError::OutputTooLarge);
    }
    Ok(out)
}

/// Whether the encoder used an exact palette for `frames` (test/diagnostic
/// aid: when true the decoded RGBA equals the input for alpha in {0, 255}).
pub fn palette_is_exact(frames: &[Frame<'_>]) -> bool {
    Palette::build(frames).is_exact()
}
