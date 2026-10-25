//! CPU tile binning: per 16x16 tile, the shapes whose bounding box overlaps
//! it, in ascending z (= shape index). Two counting passes over the shapes
//! (CSR layout), no sorting, deterministic.

use crate::error::UnsupportedReason;
use crate::layout::{EDGE_FIXED_SHIFT, GpuTile, MAX_TILE_LIST, TILE_SIZE};

/// Fixed-point bounding box `[min_x, min_y, max_x, max_y]` in 1/256 pixel.
pub type FixedBox = [i32; 4];

/// Inclusive tile rectangle `[tx0, ty0, tx1, ty1]` a box overlaps on a
/// `tiles_x * tiles_y` grid, or `None` when it misses the grid. A box edge
/// exactly on a tile border counts as touching both tiles (conservative).
pub fn tile_range(b: FixedBox, tiles_x: u32, tiles_y: u32) -> Option<[u32; 4]> {
    let shift = EDGE_FIXED_SHIFT + TILE_SIZE.trailing_zeros();
    let clamp = |v: i32, count: u32| -> i64 { i64::from(v >> shift).clamp(-1, i64::from(count)) };
    let (x0, x1) = (clamp(b[0], tiles_x), clamp(b[2], tiles_x));
    let (y0, y1) = (clamp(b[1], tiles_y), clamp(b[3], tiles_y));
    let (x0, y0) = (x0.max(0), y0.max(0));
    let (x1, y1) = (
        x1.min(i64::from(tiles_x) - 1),
        y1.min(i64::from(tiles_y) - 1),
    );
    (x0 <= x1 && y0 <= y1).then(|| [x0 as u32, y0 as u32, x1 as u32, y1 as u32])
}

/// Bins `boxes` (index = z) onto the grid. Returns the tile headers
/// (row-major) and the concatenated lists. Boxes missing the grid add
/// nothing. Errors with `TooManyTileEntries` past [`MAX_TILE_LIST`].
pub fn bin_tiles(
    boxes: &[FixedBox],
    tiles_x: u32,
    tiles_y: u32,
) -> Result<(Vec<GpuTile>, Vec<u32>), UnsupportedReason> {
    let tile_total = tiles_x as usize * tiles_y as usize;
    let mut tiles = vec![GpuTile::default(); tile_total];
    let ranges: Vec<Option<[u32; 4]>> = boxes
        .iter()
        .map(|b| tile_range(*b, tiles_x, tiles_y))
        .collect();
    let mut total = 0u64;
    for r in ranges.iter().flatten() {
        for ty in r[1]..=r[3] {
            let row = (ty * tiles_x) as usize;
            for tile in &mut tiles[row + r[0] as usize..=row + r[2] as usize] {
                tile.list_count += 1;
            }
        }
        total += u64::from(r[2] - r[0] + 1) * u64::from(r[3] - r[1] + 1);
        if total > u64::from(MAX_TILE_LIST) {
            return Err(UnsupportedReason::TooManyTileEntries);
        }
    }
    let mut start = 0u32;
    for tile in &mut tiles {
        tile.list_start = start;
        start += tile.list_count;
        tile.list_count = 0;
    }
    let mut list = vec![0u32; start as usize];
    for (z, r) in ranges.iter().enumerate() {
        let Some(r) = r else { continue };
        for ty in r[1]..=r[3] {
            let row = (ty * tiles_x) as usize;
            for tile in &mut tiles[row + r[0] as usize..=row + r[2] as usize] {
                list[(tile.list_start + tile.list_count) as usize] = z as u32;
                tile.list_count += 1;
            }
        }
    }
    Ok((tiles, list))
}

#[cfg(test)]
mod tests {
    use super::*;

    const PX: i32 = 1 << EDGE_FIXED_SHIFT;

    fn bx(x0: i32, y0: i32, x1: i32, y1: i32) -> FixedBox {
        [x0 * PX, y0 * PX, x1 * PX, y1 * PX]
    }

    #[test]
    fn ranges_and_culling() {
        // 4x3 tiles = 64x48 px.
        assert_eq!(tile_range(bx(0, 0, 1, 1), 4, 3), Some([0, 0, 0, 0]));
        assert_eq!(tile_range(bx(15, 15, 17, 17), 4, 3), Some([0, 0, 1, 1]));
        assert_eq!(tile_range(bx(16, 0, 16, 1), 4, 3), Some([1, 0, 1, 0]));
        assert_eq!(tile_range(bx(-50, -50, 500, 500), 4, 3), Some([0, 0, 3, 2]));
        assert_eq!(tile_range(bx(-50, 0, -1, 10), 4, 3), None);
        assert_eq!(tile_range(bx(64, 0, 80, 10), 4, 3), None);
        assert_eq!(tile_range(bx(0, 100, 10, 120), 4, 3), None);
    }

    #[test]
    fn lists_are_z_ordered_and_deterministic() {
        let boxes = [
            bx(0, 0, 40, 10),
            bx(20, 0, 30, 10),
            bx(0, 0, 5, 5),
            bx(900, 900, 910, 910),
        ];
        let (tiles, list) = bin_tiles(&boxes, 4, 3).unwrap();
        let of = |t: usize| &list[tiles[t].list_start as usize..][..tiles[t].list_count as usize];
        assert_eq!(of(0), &[0, 2]);
        assert_eq!(of(1), &[0, 1]);
        assert_eq!(of(2), &[0]);
        assert_eq!(of(3), &[] as &[u32]);
        assert_eq!(of(4), &[] as &[u32]);
        assert_eq!(list.len(), 5);
        assert_eq!(bin_tiles(&boxes, 4, 3).unwrap(), (tiles, list));
    }

    #[test]
    fn overflow_is_reported() {
        let full = bx(0, 0, 16384, 16384);
        let boxes = vec![full; 9];
        assert_eq!(
            bin_tiles(&boxes, 1024, 1024).unwrap_err(),
            UnsupportedReason::TooManyTileEntries
        );
    }
}
