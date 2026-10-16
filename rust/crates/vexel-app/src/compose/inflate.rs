//! Bounded zlib/DEFLATE decoder (RFC 1950, RFC 1951) for the PNG decoder.

use vexel_scene::MediaError;

const LENGTH_BASE: [u16; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258,
];
const LENGTH_EXTRA: [u8; 29] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];
const DIST_BASE: [u16; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
];
const DIST_EXTRA: [u8; 30] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13,
];
const CODE_LENGTH_ORDER: [usize; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];

const CORRUPT: MediaError = MediaError::Corrupt;

struct Bits<'a> {
    data: &'a [u8],
    at: usize,
    bit: u32,
}

impl Bits<'_> {
    fn one(&mut self) -> Result<u32, MediaError> {
        let byte = *self.data.get(self.at).ok_or(CORRUPT)?;
        let value = u32::from(byte >> self.bit) & 1;
        self.bit += 1;
        if self.bit == 8 {
            self.bit = 0;
            self.at += 1;
        }
        Ok(value)
    }
    fn many(&mut self, count: u8) -> Result<u32, MediaError> {
        let mut value = 0;
        for i in 0..count {
            value |= self.one()? << i;
        }
        Ok(value)
    }
}

/// Canonical Huffman table: symbol counts and symbols ordered by code.
struct Huffman {
    counts: [u16; 16],
    symbols: Vec<u16>,
}

impl Huffman {
    fn new(lengths: &[u8]) -> Result<Self, MediaError> {
        let mut counts = [0u16; 16];
        for &length in lengths {
            counts[usize::from(length)] += 1;
        }
        counts[0] = 0;
        let mut left = 1i32;
        for count in &counts[1..] {
            left = left * 2 - i32::from(*count);
            if left < 0 {
                return Err(CORRUPT);
            }
        }
        let mut offsets = [0u16; 16];
        for i in 1..15 {
            offsets[i + 1] = offsets[i] + counts[i];
        }
        let mut symbols = vec![0u16; lengths.len()];
        for (symbol, &length) in lengths.iter().enumerate() {
            if length != 0 {
                symbols[usize::from(offsets[usize::from(length)])] = symbol as u16;
                offsets[usize::from(length)] += 1;
            }
        }
        Ok(Self { counts, symbols })
    }

    fn decode(&self, bits: &mut Bits<'_>) -> Result<u16, MediaError> {
        let (mut code, mut first, mut index) = (0i32, 0i32, 0i32);
        for length in 1..16 {
            code |= bits.one()? as i32;
            let count = i32::from(self.counts[length]);
            if code - count < first {
                return self
                    .symbols
                    .get((index + (code - first)) as usize)
                    .copied()
                    .ok_or(CORRUPT);
            }
            index += count;
            first = (first + count) << 1;
            code <<= 1;
        }
        Err(CORRUPT)
    }
}

/// Inflates a zlib stream that must produce exactly `expected` bytes.
pub(crate) fn inflate(zlib: &[u8], expected: usize) -> Result<Vec<u8>, MediaError> {
    if zlib.len() < 6
        || zlib[0] & 0x0f != 8
        || zlib[0] >> 4 > 7
        || (u16::from(zlib[0]) << 8 | u16::from(zlib[1])) % 31 != 0
        || zlib[1] & 0x20 != 0
    {
        return Err(CORRUPT);
    }
    let mut out = Vec::new();
    out.try_reserve_exact(expected)
        .map_err(|_| MediaError::OutOfMemory)?;
    let mut bits = Bits {
        data: &zlib[2..zlib.len() - 4],
        at: 0,
        bit: 0,
    };
    loop {
        let last = bits.one()?;
        match bits.many(2)? {
            0 => stored(&mut bits, &mut out, expected)?,
            1 => {
                let (lit, dist) = fixed()?;
                block(&mut bits, &mut out, expected, &lit, &dist)?;
            }
            2 => {
                let (lit, dist) = dynamic(&mut bits)?;
                block(&mut bits, &mut out, expected, &lit, &dist)?;
            }
            _ => return Err(CORRUPT),
        }
        if last == 1 {
            break;
        }
    }
    let adler = &zlib[zlib.len() - 4..];
    if out.len() != expected || vexel_png::adler32(&out).to_be_bytes() != *adler {
        return Err(CORRUPT);
    }
    Ok(out)
}

fn stored(bits: &mut Bits<'_>, out: &mut Vec<u8>, expected: usize) -> Result<(), MediaError> {
    if bits.bit != 0 {
        bits.bit = 0;
        bits.at += 1;
    }
    let head = bits.data.get(bits.at..bits.at + 4).ok_or(CORRUPT)?;
    let length = usize::from(u16::from_le_bytes([head[0], head[1]]));
    if head[0] != !head[2] || head[1] != !head[3] || out.len() + length > expected {
        return Err(CORRUPT);
    }
    out.extend_from_slice(
        bits.data
            .get(bits.at + 4..bits.at + 4 + length)
            .ok_or(CORRUPT)?,
    );
    bits.at += 4 + length;
    Ok(())
}

fn fixed() -> Result<(Huffman, Huffman), MediaError> {
    let mut lengths = [8u8; 288];
    lengths[144..256].fill(9);
    lengths[256..280].fill(7);
    Ok((Huffman::new(&lengths)?, Huffman::new(&[5u8; 30])?))
}

fn dynamic(bits: &mut Bits<'_>) -> Result<(Huffman, Huffman), MediaError> {
    let literals = bits.many(5)? as usize + 257;
    let distances = bits.many(5)? as usize + 1;
    let code_lengths = bits.many(4)? as usize + 4;
    if literals > 286 || distances > 30 {
        return Err(CORRUPT);
    }
    let mut order = [0u8; 19];
    for &slot in &CODE_LENGTH_ORDER[..code_lengths] {
        order[slot] = bits.many(3)? as u8;
    }
    let meta = Huffman::new(&order)?;
    let mut lengths = vec![0u8; literals + distances];
    let mut i = 0;
    while i < lengths.len() {
        let symbol = meta.decode(bits)?;
        let (value, repeat) = match symbol {
            0..=15 => (symbol as u8, 1),
            16 => (
                *lengths.get(i.wrapping_sub(1)).ok_or(CORRUPT)?,
                3 + bits.many(2)? as usize,
            ),
            17 => (0, 3 + bits.many(3)? as usize),
            _ => (0, 11 + bits.many(7)? as usize),
        };
        if i + repeat > lengths.len() {
            return Err(CORRUPT);
        }
        lengths[i..i + repeat].fill(value);
        i += repeat;
    }
    if lengths[256] == 0 {
        return Err(CORRUPT);
    }
    Ok((
        Huffman::new(&lengths[..literals])?,
        Huffman::new(&lengths[literals..])?,
    ))
}

fn block(
    bits: &mut Bits<'_>,
    out: &mut Vec<u8>,
    expected: usize,
    literal: &Huffman,
    distance: &Huffman,
) -> Result<(), MediaError> {
    loop {
        let symbol = usize::from(literal.decode(bits)?);
        match symbol {
            0..=255 => {
                if out.len() >= expected {
                    return Err(CORRUPT);
                }
                out.push(symbol as u8);
            }
            256 => return Ok(()),
            257..=285 => {
                let i = symbol - 257;
                let length = usize::from(LENGTH_BASE[i]) + bits.many(LENGTH_EXTRA[i])? as usize;
                let d = usize::from(distance.decode(bits)?);
                if d >= 30 {
                    return Err(CORRUPT);
                }
                let back = usize::from(DIST_BASE[d]) + bits.many(DIST_EXTRA[d])? as usize;
                if back > out.len() || out.len() + length > expected {
                    return Err(CORRUPT);
                }
                for _ in 0..length {
                    out.push(out[out.len() - back]);
                }
            }
            _ => return Err(CORRUPT),
        }
    }
}
