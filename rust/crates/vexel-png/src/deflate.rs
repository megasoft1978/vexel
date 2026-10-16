//! zlib streams made of stored blocks or one fixed-Huffman block with a
//! greedy hash-chain LZ77 matcher (32 KiB window, bounded chain length).

use crate::{PngError, checksum::adler32};

const WINDOW: usize = 32 * 1024;
const HASH_BITS: u32 = 15;
const MAX_CHAIN: usize = 32;
const MIN_MATCH: usize = 3;
const MAX_MATCH: usize = 258;
const NONE: u32 = u32::MAX;
const STORED_BLOCK: usize = 65535;

const LEN_BASE: [u16; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258,
];
const LEN_EXTRA: [u8; 29] = [
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

fn reserve<T>(v: &mut Vec<T>, n: usize) -> Result<(), PngError> {
    v.try_reserve_exact(n).map_err(|_| PngError::OutOfMemory)
}

/// Exact length of [`stored`] output for `raw_len` input bytes.
pub(crate) fn stored_len(raw_len: usize) -> usize {
    2 + raw_len + 5 * raw_len.div_ceil(STORED_BLOCK).max(1) + 4
}

/// zlib stream with stored blocks only.
pub(crate) fn stored(raw: &[u8]) -> Result<Vec<u8>, PngError> {
    let mut out = Vec::new();
    reserve(&mut out, stored_len(raw.len()))?;
    out.extend_from_slice(&[0x78, 0x01]);
    let mut blocks = raw.chunks(STORED_BLOCK).peekable();
    if blocks.peek().is_none() {
        out.extend_from_slice(&[1, 0, 0, 0xff, 0xff]);
    }
    while let Some(block) = blocks.next() {
        let len = block.len() as u16;
        out.push(u8::from(blocks.peek().is_none()));
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&(!len).to_le_bytes());
        out.extend_from_slice(block);
    }
    out.extend_from_slice(&adler32(raw).to_be_bytes());
    Ok(out)
}

struct Bits {
    out: Vec<u8>,
    acc: u64,
    count: u32,
}

impl Bits {
    /// Appends `n` bits of `value`, least significant first.
    fn put(&mut self, value: u32, n: u32) {
        self.acc |= u64::from(value) << self.count;
        self.count += n;
        while self.count >= 8 {
            self.out.push(self.acc as u8);
            self.acc >>= 8;
            self.count -= 8;
        }
    }

    /// Appends a Huffman code, most significant bit first.
    fn code(&mut self, code: u32, n: u32) {
        self.put(code.reverse_bits() >> (32 - n), n);
    }

    fn literal_or_length(&mut self, symbol: u32) {
        match symbol {
            0..=143 => self.code(0x30 + symbol, 8),
            144..=255 => self.code(0x190 + symbol - 144, 9),
            256..=279 => self.code(symbol - 256, 7),
            _ => self.code(0xC0 + symbol - 280, 8),
        }
    }

    fn finish(mut self) -> Vec<u8> {
        if self.count > 0 {
            self.out.push(self.acc as u8);
        }
        self.out
    }
}

struct Matcher {
    head: Vec<u32>,
    prev: Vec<u32>,
}

impl Matcher {
    fn new() -> Result<Self, PngError> {
        let (mut head, mut prev) = (Vec::new(), Vec::new());
        reserve(&mut head, 1 << HASH_BITS)?;
        reserve(&mut prev, WINDOW)?;
        head.resize(1 << HASH_BITS, NONE);
        prev.resize(WINDOW, NONE);
        Ok(Self { head, prev })
    }

    fn hash(data: &[u8], at: usize) -> usize {
        let v = u32::from(data[at]) | u32::from(data[at + 1]) << 8 | u32::from(data[at + 2]) << 16;
        (v.wrapping_mul(0x9E37_79B1) >> (32 - HASH_BITS)) as usize
    }

    fn insert(&mut self, data: &[u8], at: usize) {
        if at + MIN_MATCH <= data.len() {
            let h = Self::hash(data, at);
            self.prev[at & (WINDOW - 1)] = self.head[h];
            self.head[h] = at as u32;
        }
    }

    /// Longest earlier match at `at` as `(length, distance)`, if any.
    fn find(&self, data: &[u8], at: usize) -> Option<(usize, usize)> {
        if at + MIN_MATCH > data.len() {
            return None;
        }
        let limit = MAX_MATCH.min(data.len() - at);
        let mut best = (0, 0);
        let mut candidate = self.head[Self::hash(data, at)];
        let mut chain = 0;
        while candidate != NONE && chain < MAX_CHAIN {
            let c = candidate as usize;
            let distance = at - c;
            if distance > WINDOW {
                break;
            }
            let mut len = 0;
            while len < limit && data[c + len] == data[at + len] {
                len += 1;
            }
            if len > best.0 {
                best = (len, distance);
                if len == limit {
                    break;
                }
            }
            candidate = self.prev[c & (WINDOW - 1)];
            chain += 1;
        }
        (best.0 >= MIN_MATCH).then_some(best)
    }
}

fn table_index(table: &[u16], value: usize) -> usize {
    table
        .iter()
        .rposition(|&base| usize::from(base) <= value)
        .unwrap_or(0)
}

/// zlib stream with one fixed-Huffman block, or the stored form when that is
/// not smaller (so the result never exceeds [`stored_len`]).
pub(crate) fn fixed(raw: &[u8]) -> Result<Vec<u8>, PngError> {
    let mut matcher = Matcher::new()?;
    let mut bits = Bits {
        out: Vec::new(),
        acc: 0,
        count: 0,
    };
    reserve(&mut bits.out, stored_len(raw.len()) + 16)?;
    bits.out.extend_from_slice(&[0x78, 0x01]);
    bits.put(1, 1); // BFINAL
    bits.put(1, 2); // BTYPE 01
    let mut at = 0;
    while at < raw.len() {
        let step = match matcher.find(raw, at) {
            Some((len, distance)) => {
                let li = table_index(&LEN_BASE, len);
                bits.literal_or_length(257 + li as u32);
                bits.put(
                    (len - usize::from(LEN_BASE[li])) as u32,
                    u32::from(LEN_EXTRA[li]),
                );
                let di = table_index(&DIST_BASE, distance);
                bits.code(di as u32, 5);
                bits.put(
                    (distance - usize::from(DIST_BASE[di])) as u32,
                    u32::from(DIST_EXTRA[di]),
                );
                len
            }
            None => {
                bits.literal_or_length(u32::from(raw[at]));
                1
            }
        };
        for p in at..at + step {
            matcher.insert(raw, p);
        }
        at += step;
        // Incompressible input: stop early, the stored form is smaller.
        if bits.out.len() >= stored_len(raw.len()) {
            return stored(raw);
        }
    }
    bits.literal_or_length(256);
    let mut out = bits.finish();
    out.extend_from_slice(&adler32(raw).to_be_bytes());
    if out.len() >= stored_len(raw.len()) {
        return stored(raw);
    }
    Ok(out)
}
