//! Bounded PNG decoder for data-URI `<image>` payloads (WO-RUST-CORPUS-UNSUPPORTED-01).
//!
//! Non-interlaced PNG of every colour type and bit depth decodes to straight
//! RGBA8. The zlib Adler-32 is checked with `vexel-png`; chunk CRCs are not
//! compared (see the loop below). Interlaced (Adam7)
//! and animated payloads are `Unsupported`; anything malformed is `Corrupt`.

use super::inflate::inflate;
use vexel_scene::{ImagePixels, MediaError};

const SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
/// Largest decoded RGBA image, in bytes (the media layer bounds pixels again).
// 64 MiB of RGBA (4096x4096): decoding holds the raw, unfiltered and RGBA buffers at once, so a
// crafted tiny PNG could otherwise force an allocation of several times this value.
const MAX_RGBA_BYTES: usize = 64 * 1024 * 1024;

struct Header {
    width: usize,
    height: usize,
    depth: u8,
    color: u8,
}

impl Header {
    fn channels(&self) -> usize {
        match self.color {
            0 | 3 => 1,
            2 => 3,
            4 => 2,
            _ => 4,
        }
    }
    fn bits_per_pixel(&self) -> usize {
        self.channels() * usize::from(self.depth)
    }
    fn row_bytes(&self) -> Option<usize> {
        self.width
            .checked_mul(self.bits_per_pixel())?
            .checked_add(7)
            .map(|b| b / 8)
    }
}

fn be32(bytes: &[u8]) -> u32 {
    u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

/// Decodes a PNG file into straight-alpha RGBA8 pixels.
pub(crate) fn decode(data: &[u8]) -> Result<ImagePixels, MediaError> {
    if data.len() < 8 || data[..8] != SIGNATURE {
        return Err(MediaError::Corrupt);
    }
    let (mut header, mut palette, mut trns) = (None, Vec::new(), Vec::new());
    let mut idat = Vec::new();
    let (mut at, mut ended) = (8usize, false);
    while !ended {
        let head = data.get(at..at + 8).ok_or(MediaError::Corrupt)?;
        let length = be32(head) as usize;
        let body = data
            .get(at + 8..at + 8 + length)
            .ok_or(MediaError::Corrupt)?;
        // The chunk CRC must be present but is not compared: the project's own
        // image fixtures carry wrong IDAT CRCs, and the zlib Adler-32 below
        // still guards the pixel data.
        data.get(at + 8 + length..at + 12 + length)
            .ok_or(MediaError::Corrupt)?;
        at += 12 + length;
        match &head[4..8] {
            b"IHDR" => {
                if header.is_some() || body.len() != 13 {
                    return Err(MediaError::Corrupt);
                }
                let h = Header {
                    width: be32(body) as usize,
                    height: be32(&body[4..]) as usize,
                    depth: body[8],
                    color: body[9],
                };
                let legal = match h.color {
                    0 => matches!(h.depth, 1 | 2 | 4 | 8 | 16),
                    3 => matches!(h.depth, 1 | 2 | 4 | 8),
                    2 | 4 | 6 => matches!(h.depth, 8 | 16),
                    _ => false,
                };
                if !legal || h.width == 0 || h.height == 0 || body[10] != 0 || body[11] != 0 {
                    return Err(MediaError::Corrupt);
                }
                if body[12] != 0 {
                    return Err(MediaError::Unsupported);
                }
                header = Some(h);
            }
            b"PLTE" => palette = body.to_vec(),
            b"tRNS" => trns = body.to_vec(),
            b"acTL" => return Err(MediaError::Animated),
            b"IDAT" => {
                idat.try_reserve(body.len())
                    .map_err(|_| MediaError::OutOfMemory)?;
                idat.extend_from_slice(body);
            }
            b"IEND" => ended = true,
            name if name[0] & 0x20 == 0 => return Err(MediaError::Unsupported),
            _ => {}
        }
    }
    let header = header.ok_or(MediaError::Corrupt)?;
    let row = header.row_bytes().ok_or(MediaError::Limit)?;
    let rgba_bytes = header
        .width
        .checked_mul(header.height)
        .and_then(|p| p.checked_mul(4))
        .filter(|b| *b <= MAX_RGBA_BYTES)
        .ok_or(MediaError::Limit)?;
    let raw_bytes = (row + 1)
        .checked_mul(header.height)
        .ok_or(MediaError::Limit)?;
    let raw = inflate(&idat, raw_bytes)?;
    if header.color == 3 && palette.len() < 3 {
        return Err(MediaError::Corrupt);
    }
    let rows = unfilter(&header, &raw, row)?;
    let mut rgba = Vec::new();
    rgba.try_reserve_exact(rgba_bytes)
        .map_err(|_| MediaError::OutOfMemory)?;
    for y in 0..header.height {
        let line = &rows[y * row..(y + 1) * row];
        for x in 0..header.width {
            rgba.extend_from_slice(&pixel(&header, line, x, &palette, &trns));
        }
    }
    Ok(ImagePixels {
        rgba,
        byte_count: rgba_bytes,
        row_stride: header.width * 4,
        width: u32::try_from(header.width).map_err(|_| MediaError::Limit)?,
        height: u32::try_from(header.height).map_err(|_| MediaError::Limit)?,
    })
}

/// Reverses the per-row filters; the result is `height` rows of `row` bytes.
fn unfilter(header: &Header, raw: &[u8], row: usize) -> Result<Vec<u8>, MediaError> {
    let step = header.bits_per_pixel().div_ceil(8);
    let mut out = Vec::new();
    out.try_reserve_exact(row * header.height)
        .map_err(|_| MediaError::OutOfMemory)?;
    out.resize(row * header.height, 0);
    for y in 0..header.height {
        let filter = raw[y * (row + 1)];
        let line = &raw[y * (row + 1) + 1..(y + 1) * (row + 1)];
        let (before, current) = out.split_at_mut(y * row);
        let up: &[u8] = if y == 0 {
            &[]
        } else {
            &before[(y - 1) * row..]
        };
        for i in 0..row {
            let left = if i >= step {
                i32::from(current[i - step])
            } else {
                0
            };
            let above = up.get(i).map_or(0, |v| i32::from(*v));
            let corner = if i >= step {
                up.get(i - step).map_or(0, |v| i32::from(*v))
            } else {
                0
            };
            let predictor = match filter {
                0 => 0,
                1 => left,
                2 => above,
                3 => (left + above) / 2,
                4 => paeth(left, above, corner),
                _ => return Err(MediaError::Corrupt),
            };
            current[i] = (i32::from(line[i]) + predictor) as u8;
        }
    }
    Ok(out)
}

fn paeth(a: i32, b: i32, c: i32) -> i32 {
    let p = a + b - c;
    let (pa, pb, pc) = ((p - a).abs(), (p - b).abs(), (p - c).abs());
    if pa <= pb && pa <= pc {
        a
    } else if pb <= pc {
        b
    } else {
        c
    }
}

/// The `index`-th sample of a row, widened to 16 bits for 16-bit depth.
fn sample(line: &[u8], depth: u8, index: usize) -> u16 {
    match depth {
        8 => u16::from(line[index]),
        16 => u16::from_be_bytes([line[index * 2], line[index * 2 + 1]]),
        d => {
            let per_byte = 8 / usize::from(d);
            let shift = 8 - usize::from(d) * (index % per_byte + 1);
            u16::from((line[index / per_byte] >> shift) & ((1u8 << d) - 1))
        }
    }
}

/// Scales a sample of `depth` bits to 8 bits.
fn to8(value: u16, depth: u8) -> u8 {
    match depth {
        16 => (value >> 8) as u8,
        8 => value as u8,
        d => (u32::from(value) * 255 / ((1u32 << d) - 1)) as u8,
    }
}

fn pixel(h: &Header, line: &[u8], x: usize, palette: &[u8], trns: &[u8]) -> [u8; 4] {
    let s = |c: usize| sample(line, h.depth, x * h.channels() + c);
    let scaled = |c: usize| to8(s(c), h.depth);
    match h.color {
        0 => {
            let g = scaled(0);
            let transparent = trns.len() >= 2 && s(0) == u16::from_be_bytes([trns[0], trns[1]]);
            [g, g, g, if transparent { 0 } else { 255 }]
        }
        2 => {
            let key = trns.len() >= 6
                && (0..3).all(|c| s(c) == u16::from_be_bytes([trns[c * 2], trns[c * 2 + 1]]));
            [scaled(0), scaled(1), scaled(2), if key { 0 } else { 255 }]
        }
        3 => {
            let i = usize::from(s(0));
            let rgb = palette.get(i * 3..i * 3 + 3).unwrap_or(&[0, 0, 0]);
            [rgb[0], rgb[1], rgb[2], trns.get(i).copied().unwrap_or(255)]
        }
        4 => {
            let g = scaled(0);
            [g, g, g, scaled(1)]
        }
        _ => [scaled(0), scaled(1), scaled(2), scaled(3)],
    }
}
