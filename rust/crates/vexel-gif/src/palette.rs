//! Deterministic global palette selection and pixel indexing.
//!
//! If the whole frame set uses at most 256 slots (opaque colours plus one
//! transparent slot when any pixel has alpha 0) the palette is exact and lists
//! colours in order of first appearance. Otherwise a median-cut palette built
//! from a 5-bit-per-channel population histogram is used, and pixels are
//! Floyd-Steinberg error-diffused against it. Should the adaptive palette not
//! meet [`MAX_QUANT_ERROR`] for every pixel, a fixed 6x7x6 uniform RGB palette
//! (no dithering) is the fallback. Every choice depends only on the input
//! bytes, so encoding is deterministic.

use crate::Frame;
use std::collections::HashMap;

/// Levels per channel of the fixed fallback palette (R, G, B).
const LEVELS: [u32; 3] = [6, 7, 6];
/// Slots in the fixed fallback palette.
pub(crate) const UNIFORM_SIZE: usize = (LEVELS[0] * LEVELS[1] * LEVELS[2]) as usize;
/// Largest per-channel difference between an opaque input pixel and its
/// decoded colour for any quantizing palette (adaptive, dithered or uniform).
/// The uniform fallback's largest step (6 levels: 255/5 = 51) gives a
/// worst-case half step of 25.5, rounded up; the adaptive path enforces the
/// same bound per pixel.
pub const MAX_QUANT_ERROR: u8 = 26;

pub(crate) struct Palette {
    /// RGB triples, at most 256 entries.
    pub colors: Vec<[u8; 3]>,
    /// Index of the transparent slot, if any pixel has alpha 0.
    pub transparent: Option<u8>,
    mode: Mode,
    lookup: HashMap<u32, u8>,
    /// Adaptive mode: nearest palette index for each 5-bit RGB bin.
    bins: Vec<u8>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Exact,
    Adaptive,
    Uniform,
}

const BINS: usize = 1 << 15;

fn bin_of(r: u8, g: u8, b: u8) -> usize {
    (usize::from(r >> 3) << 10) | (usize::from(g >> 3) << 5) | usize::from(b >> 3)
}

#[derive(Clone)]
struct Cell {
    /// Mean colour of the bin's pixels.
    rgb: [u8; 3],
    count: u64,
    sum: [u64; 3],
}

/// Median cut over populated bins; at most `capacity` palette entries.
fn median_cut(cells: &[Cell], capacity: usize) -> Vec<[u8; 3]> {
    let mut boxes: Vec<Vec<Cell>> = vec![cells.to_vec()];
    let range = |b: &[Cell]| -> (usize, u8) {
        let mut best = (0usize, 0u8);
        for c in 0..3 {
            let lo = b.iter().map(|x| x.rgb[c]).min().unwrap_or(0);
            let hi = b.iter().map(|x| x.rgb[c]).max().unwrap_or(0);
            if hi - lo > best.1 {
                best = (c, hi - lo);
            }
        }
        best
    };
    while boxes.len() < capacity {
        let mut pick: Option<(usize, u8, u64)> = None;
        for (i, b) in boxes.iter().enumerate() {
            if b.len() < 2 {
                continue;
            }
            let (_, r) = range(b);
            let n: u64 = b.iter().map(|x| x.count).sum();
            if pick.is_none_or(|(_, pr, pn)| (r, n) > (pr, pn)) {
                pick = Some((i, r, n));
            }
        }
        let Some((i, _, total)) = pick else { break };
        let mut b = boxes.swap_remove(i);
        let (axis, _) = range(&b);
        b.sort_by_key(|x| (x.rgb[axis], x.rgb));
        let mut acc = 0u64;
        let mut cut = 1;
        for (k, x) in b.iter().enumerate() {
            acc += x.count;
            if acc * 2 >= total {
                cut = (k + 1).clamp(1, b.len() - 1);
                break;
            }
        }
        let hi = b.split_off(cut);
        boxes.push(b);
        boxes.push(hi);
    }
    boxes.sort_by_key(|b| b[0].rgb);
    boxes
        .iter()
        .map(|b| {
            let n: u64 = b.iter().map(|x| x.count).sum::<u64>().max(1);
            let mut m = [0u8; 3];
            for (c, slot) in m.iter_mut().enumerate() {
                let s: u64 = b.iter().map(|x| x.sum[c]).sum();
                *slot = ((s + n / 2) / n) as u8;
            }
            m
        })
        .collect()
}

fn dist2(a: [u8; 3], b: [i32; 3]) -> i32 {
    (0..3).map(|c| (i32::from(a[c]) - b[c]).pow(2)).sum()
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
            return Self::adaptive(frames, has_transparent, capacity)
                .unwrap_or_else(|| Self::uniform(has_transparent));
        }
        let transparent = has_transparent.then(|| {
            colors.push([0, 0, 0]);
            (colors.len() - 1) as u8
        });
        Self {
            colors,
            transparent,
            mode: Mode::Exact,
            lookup,
            bins: Vec::new(),
        }
    }

    /// Median-cut palette; `None` when some pixel would exceed the error bound.
    fn adaptive(frames: &[Frame<'_>], has_transparent: bool, capacity: usize) -> Option<Self> {
        let mut cells = vec![
            Cell {
                rgb: [0; 3],
                count: 0,
                sum: [0; 3],
            };
            BINS
        ];
        for frame in frames {
            for px in frame.rgba.chunks_exact(4).filter(|p| p[3] != 0) {
                let c = &mut cells[bin_of(px[0], px[1], px[2])];
                c.count += 1;
                for k in 0..3 {
                    c.sum[k] += u64::from(px[k]);
                }
            }
        }
        for c in &mut cells {
            if c.count > 0 {
                for k in 0..3 {
                    c.rgb[k] = ((c.sum[k] + c.count / 2) / c.count) as u8;
                }
            }
        }
        let populated: Vec<Cell> = cells.iter().filter(|c| c.count > 0).cloned().collect();
        let mut colors = median_cut(&populated, capacity);
        let mut bins = vec![0u8; BINS];
        for (i, slot) in bins.iter_mut().enumerate() {
            let c = &cells[i];
            let rep = if c.count > 0 {
                c.rgb
            } else {
                [
                    ((i >> 10) as u8) << 3 | 4,
                    (((i >> 5) & 31) as u8) << 3 | 4,
                    ((i & 31) as u8) << 3 | 4,
                ]
            };
            let rep = [i32::from(rep[0]), i32::from(rep[1]), i32::from(rep[2])];
            *slot = (0..colors.len())
                .min_by_key(|&k| dist2(colors[k], rep))
                .unwrap_or(0) as u8;
        }
        let ok = frames.iter().all(|frame| {
            frame.rgba.chunks_exact(4).filter(|p| p[3] != 0).all(|px| {
                let c = colors[usize::from(bins[bin_of(px[0], px[1], px[2])])];
                (0..3).all(|k| px[k].abs_diff(c[k]) <= MAX_QUANT_ERROR)
            })
        });
        if !ok {
            return None;
        }
        let transparent = has_transparent.then(|| {
            colors.push([0, 0, 0]);
            (colors.len() - 1) as u8
        });
        Some(Self {
            colors,
            transparent,
            mode: Mode::Adaptive,
            lookup: HashMap::new(),
            bins,
        })
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
        Self {
            colors,
            transparent,
            mode: Mode::Uniform,
            lookup: HashMap::new(),
            bins: Vec::new(),
        }
    }

    pub(crate) fn is_exact(&self) -> bool {
        self.mode == Mode::Exact
    }

    pub(crate) fn index_of(&self, px: &[u8]) -> u8 {
        if px[3] == 0 {
            return self.transparent.unwrap_or(0);
        }
        match self.mode {
            Mode::Exact => return self.lookup.get(&pack(px)).copied().unwrap_or(0),
            Mode::Adaptive => return self.bins[bin_of(px[0], px[1], px[2])],
            Mode::Uniform => {}
        }
        let r = level(px[0], LEVELS[0]);
        let g = level(px[1], LEVELS[1]);
        let b = level(px[2], LEVELS[2]);
        ((r * LEVELS[1] + g) * LEVELS[2] + b) as u8
    }

    /// Indexes one frame into `out`. The adaptive palette is Floyd-Steinberg
    /// diffused; a pixel whose diffused choice would exceed
    /// [`MAX_QUANT_ERROR`] takes its plain nearest entry and propagates no error.
    pub(crate) fn index_frame(&self, rgba: &[u8], width: usize, out: &mut Vec<u8>) {
        out.clear();
        if self.mode != Mode::Adaptive {
            out.extend(rgba.chunks_exact(4).map(|px| self.index_of(px)));
            return;
        }
        let mut cur = vec![[0i32; 3]; width + 2];
        let mut next = vec![[0i32; 3]; width + 2];
        for row in rgba.chunks_exact(width * 4) {
            next.iter_mut().for_each(|e| *e = [0; 3]);
            for (x, px) in row.chunks_exact(4).enumerate() {
                if px[3] == 0 {
                    out.push(self.index_of(px));
                    continue;
                }
                let e = cur[x + 1];
                let mut t = [0i32; 3];
                for k in 0..3 {
                    t[k] = (i32::from(px[k]) + e[k] / 16).clamp(0, 255);
                }
                let mut idx = self.bins[bin_of(t[0] as u8, t[1] as u8, t[2] as u8)];
                let mut c = self.colors[usize::from(idx)];
                let mut diffuse = true;
                if (0..3).any(|k| px[k].abs_diff(c[k]) > MAX_QUANT_ERROR) {
                    idx = self.index_of(px);
                    c = self.colors[usize::from(idx)];
                    diffuse = false;
                }
                out.push(idx);
                if diffuse {
                    for k in 0..3 {
                        let d = (t[k] - i32::from(c[k])) * 16;
                        cur[x + 2][k] += d * 7 / 16;
                        next[x][k] += d * 3 / 16;
                        next[x + 1][k] += d * 5 / 16;
                        next[x + 2][k] += d / 16;
                    }
                }
            }
            std::mem::swap(&mut cur, &mut next);
        }
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
