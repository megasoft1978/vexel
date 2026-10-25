//! Hole filling for underlays: masked pixels are rebuilt from the surrounding
//! known pixels in order of distance from the hole boundary (a fast-marching
//! front, after Telea 2004), each taking an inverse-square-distance average of
//! the known pixels around it.

use super::{MAX_INPAINT_PIXELS, Rgba, TraceError};
use std::cmp::Reverse;
use std::collections::BinaryHeap;

const RADII: [i64; 3] = [4, 8, 24];

/// Returns RGBA8 where every `mask` pixel is replaced (and made opaque).
pub fn inpaint(img: &Rgba<'_>, mask: &[bool]) -> Result<Vec<u8>, TraceError> {
    let (w, h) = (img.width, img.height);
    if mask.len() != w * h || mask.iter().all(|&m| m) {
        return Err(TraceError::BadRegion);
    }
    let masked = mask.iter().filter(|&&m| m).count();
    if masked > MAX_INPAINT_PIXELS || (w * h - masked) * 100 < w * h {
        return Err(TraceError::MaskTooLarge);
    }
    let mut out = img.data.to_vec();
    let mut known: Vec<bool> = mask.iter().map(|&m| !m).collect();
    let mut dist = vec![u32::MAX; w * h];
    let mut heap: BinaryHeap<Reverse<(u32, u32)>> = BinaryHeap::new();
    let neighbours = |i: usize| -> [Option<(usize, u32)>; 8] {
        let (x, y) = ((i % w) as i64, (i / w) as i64);
        let mut v = [None; 8];
        for (slot, (dx, dy)) in v.iter_mut().zip([
            (-1, 0),
            (1, 0),
            (0, -1),
            (0, 1),
            (-1, -1),
            (1, -1),
            (-1, 1),
            (1, 1),
        ]) {
            let (nx, ny) = (x + dx, y + dy);
            if nx >= 0 && ny >= 0 && (nx as usize) < w && (ny as usize) < h {
                *slot = Some((
                    ny as usize * w + nx as usize,
                    if dx != 0 && dy != 0 { 14 } else { 10 },
                ));
            }
        }
        v
    };
    for i in 0..w * h {
        if mask[i] {
            let d = neighbours(i)
                .into_iter()
                .flatten()
                .filter(|&(j, _)| known[j])
                .map(|(_, c)| c)
                .min();
            if let Some(d) = d {
                dist[i] = d;
                heap.push(Reverse((d, i as u32)));
            }
        }
    }
    while let Some(Reverse((d, i))) = heap.pop() {
        let i = i as usize;
        if known[i] || d != dist[i] {
            continue;
        }
        let px = estimate(&out, &known, w, h, i);
        out[i * 4..i * 4 + 4].copy_from_slice(&px);
        known[i] = true;
        for (j, cost) in neighbours(i).into_iter().flatten() {
            if mask[j] && !known[j] && d + cost < dist[j] {
                dist[j] = d + cost;
                heap.push(Reverse((dist[j], j as u32)));
            }
        }
    }
    Ok(out)
}

fn estimate(px: &[u8], known: &[bool], w: usize, h: usize, i: usize) -> [u8; 4] {
    let (cx, cy) = ((i % w) as i64, (i / w) as i64);
    for r in RADII {
        let (mut sum, mut total) = ([0.0f64; 3], 0.0f64);
        for y in (cy - r).max(0)..=(cy + r).min(h as i64 - 1) {
            for x in (cx - r).max(0)..=(cx + r).min(w as i64 - 1) {
                let j = y as usize * w + x as usize;
                let d2 = ((x - cx).pow(2) + (y - cy).pow(2)) as f64;
                if known[j] && d2 > 0.0 && d2 <= (r * r) as f64 {
                    let wgt = 1.0 / d2;
                    for k in 0..3 {
                        sum[k] += wgt * f64::from(px[j * 4 + k]);
                    }
                    total += wgt;
                }
            }
        }
        if total > 0.0 {
            let c = |k: usize| (sum[k] / total).round().clamp(0.0, 255.0) as u8;
            return [c(0), c(1), c(2), 255];
        }
    }
    [128, 128, 128, 255]
}
