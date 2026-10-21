#![forbid(unsafe_code)]
//! First-party PNG encoder: 8-bit RGBA (colour type 6), no interlace, filter
//! type 0 on every row, one `IHDR`, `IDAT` chunks and `IEND`.
//!
//! Pixels are written exactly as given (straight alpha is the caller's
//! convention; this crate never converts). The zlib stream uses stored blocks
//! or one fixed-Huffman block with a greedy hash-chain matcher; it never
//! exceeds the stored size. Input is validated before any large allocation
//! and every allocation uses `try_reserve`.

mod apng;
mod checksum;
mod deflate;

pub use apng::{ApngFrame, MAX_FRAMES, encode_apng};
pub use checksum::{adler32, crc32};

/// Each axis is at most this many pixels.
pub const MAX_DIMENSION: u32 = 16384;
/// Pixel bytes (`width * height * 4`) are at most 256 MiB.
pub const MAX_PIXEL_BYTES: u64 = 256 * 1024 * 1024;

const SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
const IDAT_MAX: usize = 1 << 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compression {
    /// Uncompressed deflate blocks.
    Stored,
    /// Fixed-Huffman deflate with LZ77.
    Fixed,
}

/// Typed encoder failure; none allocates beyond the failed check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PngError {
    ZeroDimension,
    /// A side exceeds [`MAX_DIMENSION`].
    DimensionTooLarge,
    /// `width * height * 4` exceeds [`MAX_PIXEL_BYTES`].
    TooManyBytes,
    /// `rgba.len()` is not `width * height * 4`.
    InvalidLength,
    OutOfMemory,
    /// APNG with no frames.
    ZeroFrames,
    /// APNG with more than [`MAX_FRAMES`] frames.
    TooManyFrames,
}

pub(crate) fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    let start = out.len();
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    let crc = checksum::crc32_update(0, &out[start..]);
    out.extend_from_slice(&crc.to_be_bytes());
}

/// Encodes straight RGBA8 (`width * height * 4` bytes, top row first).
pub fn encode_rgba8(
    width: u32,
    height: u32,
    rgba: &[u8],
    compression: Compression,
) -> Result<Vec<u8>, PngError> {
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
    if rgba.len() as u64 != pixel_bytes {
        return Err(PngError::InvalidLength);
    }
    let row = width as usize * 4;
    let mut raw = Vec::new();
    raw.try_reserve_exact(rgba.len() + height as usize)
        .map_err(|_| PngError::OutOfMemory)?;
    for line in rgba.chunks_exact(row) {
        raw.push(0);
        raw.extend_from_slice(line);
    }
    let zlib = match compression {
        Compression::Stored => deflate::stored(&raw)?,
        Compression::Fixed => deflate::fixed(&raw)?,
    };
    drop(raw);
    let idat_chunks = zlib.len().div_ceil(IDAT_MAX);
    let mut out = Vec::new();
    out.try_reserve_exact(SIGNATURE.len() + 25 + zlib.len() + 12 * idat_chunks + 12)
        .map_err(|_| PngError::OutOfMemory)?;
    out.extend_from_slice(&SIGNATURE);
    let mut header = [0u8; 13];
    header[..4].copy_from_slice(&width.to_be_bytes());
    header[4..8].copy_from_slice(&height.to_be_bytes());
    header[8] = 8; // bit depth
    header[9] = 6; // colour type RGBA
    chunk(&mut out, b"IHDR", &header);
    for part in zlib.chunks(IDAT_MAX) {
        chunk(&mut out, b"IDAT", part);
    }
    chunk(&mut out, b"IEND", &[]);
    Ok(out)
}
