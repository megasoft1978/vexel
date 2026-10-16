//! Deterministic global palette selection and pixel indexing.
//!
//! If the whole frame set uses at most 256 slots (opaque colours plus one
//! transparent slot when any pixel has alpha 0) the palette is exact and lists
//! colours in order of first appearance. Otherwise a fixed 6x7x6 uniform RGB
//! palette is used (no dithering). That choice depends only on the input
//! bytes, so encoding is deterministic.

use crate::Frame;
use std::collections::HashMap;

/// Levels per channel of the fixed fallback palette (R, G, B).
const LEVELS: [u32; 3] = [6, 7, 6];
/// Slots in the fixed fallback palette.
pub(crate) const UNIFORM_SIZE: usize = (LEVELS[0] * LEVELS[1] * LEVELS[2]) as usize;
/// Largest per-channel difference between an opaque input pixel and its
/// decoded colour when the uniform fallback palette is used. The largest step
/// (6 levels: 255/5 = 51) gives a worst-case half step of 25.5, rounded up.
pub const MAX_QUANT_ERROR: u8 = 26;

pub(crate) struct Palette {
    /// RGB triples, at most 256 entries.
    pub colors: Vec<[u8; 3]>,
    /// Index of the transparent slot, if any pixel has alpha 0.
    pub transparent: Option<u8>,
    /// Exact palette (true) or uniform fallback (false).
    exact: bool,
    lookup: HashMap<u32, u8>,
}

fn pack(px: &[u8]) -> u32 {
    (u32::from(px[0]) << 16) | (u32::from(px[1]) << 8) | u32::from(px[2])
}

fn level(value: u8, levels: u32) -> u32 {
    (u32::from(value) * (levels - 1) + 127) / 255
}

fn level_value(index: u32, levels: u32) -> u8 {
    ((index * 255 + (levels - 1) / 2) / (levels - 1)) as u8
}

impl Palette {
    pub(crate) fn build(frames: &[Frame<'_>]) -> Self {
        let mut lookup: HashMap<u32, u8> = HashMap::new();
        let mut colors: Vec<[u8; 3]> = Vec::new();
        let mut has_transparent = false;
        let mut overflow = false;
        'scan: for frame in frames {
            for px in frame.rgba.chunks_exact(4) {
                if px[3] == 0 {
                    has_transparent = true;
                    continue;
                }
                let key = pack(px);
                if lookup.contains_key(&key) {
                    continue;
                }
                if colors.len() == 256 {
                    overflow = true;
                    break 'scan;
                }
                lookup.insert(key, colors.len() as u8);
                colors.push([px[0], px[1], px[2]]);
            }
        }
        let capacity = if has_transparent { 255 } else { 256 };
        if overflow || colors.len() > capacity {
            return Self::uniform(has_transparent);
        }
        let transparent = has_transparent.then(|| {
            colors.push([0, 0, 0]);
            (colors.len() - 1) as u8
        });
        Self { colors, transparent, exact: true, lookup }
    }

    fn uniform(has_transparent: bool) -> Self {
        let mut colors = Vec::with_capacity(UNIFORM_SIZE + 1);
        for r in 0..LEVELS[0] {
            for g in 0..LEVELS[1] {
                for b in 0..LEVELS[2] {
                    colors.push([
                        level_value(r, LEVELS[0]),
                        level_value(g, LEVELS[1]),
                        level_value(b, LEVELS[2]),
                    ]);
                }
            }
        }
        let transparent = has_transparent.then(|| {
            colors.push([0, 0, 0]);
            (colors.len() - 1) as u8
        });
        Self { colors, transparent, exact: false, lookup: HashMap::new() }
    }

    pub(crate) fn is_exact(&self) -> bool {
        self.exact
    }

    pub(crate) fn index_of(&self, px: &[u8]) -> u8 {
        if px[3] == 0 {
            return self.transparent.unwrap_or(0);
        }
        if self.exact {
            return self.lookup.get(&pack(px)).copied().unwrap_or(0);
        }
        let r = level(px[0], LEVELS[0]);
        let g = level(px[1], LEVELS[1]);
        let b = level(px[2], LEVELS[2]);
        ((r * LEVELS[1] + g) * LEVELS[2] + b) as u8
    }

    /// Bits per palette index, 1..=8 (the table is padded to `1 << bits`).
    pub(crate) fn table_bits(&self) -> u32 {
        let mut bits = 1;
        while (1usize << bits) < self.colors.len() {
            bits += 1;
        }
        bits
    }
}
