//! Animated PNG (APNG) encoder: 8-bit RGBA, every frame full-size at (0,0),
//! `dispose_op` NONE, `blend_op` SOURCE. Frame 0 is also the default image.
//! Chunk order: IHDR, acTL, fcTL+IDAT (frame 0), then fcTL+fdAT per later
//! frame (one sequence counter shared by fcTL and fdAT), IEND. Output is
//! deterministic.

use crate::{IDAT_MAX, MAX_DIMENSION, MAX_PIXEL_BYTES, PngError, SIGNATURE, chunk, deflate};

/// At most this many frames per animation.
pub const MAX_FRAMES: usize = 1440;

/// One full-size RGBA8 frame and its delay (`delay_num / delay_den` seconds;
/// a zero denominator means 1/100 per the APNG spec).
#[derive(Debug, Clone, Copy)]
pub struct ApngFrame<'a> {
    pub rgba: &'a [u8],
    pub delay_num: u16,
    pub delay_den: u16,
}

/// Encodes an APNG. `loops` is `num_plays` (0 = forever).
pub fn encode_apng(
    frames: &[ApngFrame<'_>],
    width: u32,
    height: u32,
    loops: u32,
) -> Result<Vec<u8>, PngError> {
    if frames.is_empty() {
        return Err(PngError::ZeroFrames);
    }
    if frames.len() > MAX_FRAMES {
        return Err(PngError::TooManyFrames);
    }
    if width == 0 || height == 0 {
        return Err(PngError::ZeroDimension);
    }
    if width > MAX_DIMENSION || height > MAX_DIMENSION {
        return Err(PngError::DimensionTooLarge);
    }
    let pixel_bytes = u64::from(width) * u64::from(height) * 4;
    if pixel_bytes > MAX_PIXEL_BYTES {
        return Err(PngError::TooManyBytes);
    }
    if frames.iter().any(|f| f.rgba.len() as u64 != pixel_bytes) {
        return Err(PngError::InvalidLength);
    }
    let row = width as usize * 4;
    let mut out = Vec::new();
    out.try_reserve(SIGNATURE.len() + 25 + 20 + 12)
        .map_err(|_| PngError::OutOfMemory)?;
    out.extend_from_slice(&SIGNATURE);
    let mut header = [0u8; 13];
    header[..4].copy_from_slice(&width.to_be_bytes());
    header[4..8].copy_from_slice(&height.to_be_bytes());
    header[8] = 8;
    header[9] = 6;
    chunk(&mut out, b"IHDR", &header);
    let mut actl = [0u8; 8];
    actl[..4].copy_from_slice(&(frames.len() as u32).to_be_bytes());
    actl[4..].copy_from_slice(&loops.to_be_bytes());
    chunk(&mut out, b"acTL", &actl);
    let mut seq = 0u32;
    for (index, frame) in frames.iter().enumerate() {
        let mut raw = Vec::new();
        raw.try_reserve_exact(frame.rgba.len() + height as usize)
            .map_err(|_| PngError::OutOfMemory)?;
        for line in frame.rgba.chunks_exact(row) {
            raw.push(0);
            raw.extend_from_slice(line);
        }
        let zlib = deflate::fixed(&raw)?;
        drop(raw);
        let mut fctl = [0u8; 26];
        fctl[..4].copy_from_slice(&seq.to_be_bytes());
        fctl[4..8].copy_from_slice(&width.to_be_bytes());
        fctl[8..12].copy_from_slice(&height.to_be_bytes());
        // x_offset, y_offset = 0 (bytes 12..20)
        fctl[20..22].copy_from_slice(&frame.delay_num.to_be_bytes());
        fctl[22..24].copy_from_slice(&frame.delay_den.to_be_bytes());
        fctl[24] = 0; // dispose_op NONE
        fctl[25] = 0; // blend_op SOURCE
        seq += 1;
        chunk(&mut out, b"fcTL", &fctl);
        for part in zlib.chunks(IDAT_MAX) {
            if index == 0 {
                chunk(&mut out, b"IDAT", part);
            } else {
                let mut data = Vec::new();
                data.try_reserve_exact(part.len() + 4)
                    .map_err(|_| PngError::OutOfMemory)?;
                data.extend_from_slice(&seq.to_be_bytes());
                data.extend_from_slice(part);
                seq += 1;
                chunk(&mut out, b"fdAT", &data);
            }
        }
    }
    chunk(&mut out, b"IEND", &[]);
    Ok(out)
}
