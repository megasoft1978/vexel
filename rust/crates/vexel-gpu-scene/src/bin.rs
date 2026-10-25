//! CPU tile binning with per-tile edge ranges (ADR-0035 addendum). For every
//! 16x16 tile a shape touches, the tile list gets a [`GpuTileEntry`] whose
//! edge range holds only what can change a sample of that tile, in tile-local
//! fixed point:
//!
//! * an edge entirely LEFT of the tile (its crossing can never be right of a
//!   sample) is dropped;
//! * an edge that straddles the tile's columns is kept verbatim (translated
//!   by the tile origin, so no rounding);
//! * edges entirely RIGHT of the tile cross every sample row they span, so
//!   they are collapsed into a per-sample-row winding carry, re-emitted as the
//!   fewest vertical unit edges on the tile's right border (`x = TILE_SIZE`).
//!
//! The sum of crossings per sample, hence non-zero winding and even-odd
//! parity, equals the unclipped result exactly. A shape whose clipped range
//! is empty adds no entry. Entries of a tile stay in ascending z. The sweep
//! along a tile row moves each edge right -> kept -> left once, so the cost is
//! the output size plus the edges of the row.

use crate::error::UnsupportedReason;
use crate::layout::{
    EDGE_FIXED_SHIFT, GpuEdge, GpuTile, GpuTileEntry, MAX_EDGES, MAX_TILE_LIST, SAMPLES_PER_AXIS,
    TILE_SIZE,
};

/// Fixed-point tile width.
const T: i64 = (TILE_SIZE as i64) << EDGE_FIXED_SHIFT;
/// Fixed-point distance between neighbouring samples (a quarter pixel).
const U: i64 = 1 << (EDGE_FIXED_SHIFT - 2);
/// Sample rows (and columns) per tile.
const ROWS: usize = (TILE_SIZE * SAMPLES_PER_AXIS) as usize;

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

/// True when the edge spans at least one sample row (`y0 <= py < y1` or
/// reversed for some global sample row); other edges never count.
pub fn spans_sample_row(e: &GpuEdge) -> bool {
    let (ylo, yhi) = (i64::from(e.y0.min(e.y1)), i64::from(e.y0.max(e.y1)));
    ceil_shr(ylo - U / 2, U_BITS) < ceil_shr(yhi - U / 2, U_BITS)
}

/// Result of binning: tile headers (row-major), tile entries in CSR order
/// (ascending z within a tile) and the per-tile edge pool they index.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Binned {
    pub tiles: Vec<GpuTile>,
    pub entries: Vec<GpuTileEntry>,
    pub edges: Vec<GpuEdge>,
}

/// `ceil(n / 2^bits)` for any sign of `n`.
fn ceil_shr(n: i64, bits: u32) -> i64 {
    (n + (1 << bits) - 1) >> bits
}

const T_BITS: u32 = EDGE_FIXED_SHIFT + TILE_SIZE.trailing_zeros();
const U_BITS: u32 = EDGE_FIXED_SHIFT - 2;
const ROW_BITS: u32 = ROWS.trailing_zeros();

/// Per source edge, computed once per shape.
struct Info {
    /// Tile rows `[row0, row1]` holding a sample row the edge spans.
    row0: i32,
    row1: i32,
    /// First global sample row spanned, and one past the last.
    k0: i32,
    k1: i32,
    /// Tile columns left of `a` see the edge entirely right (carry); columns
    /// from `b` on see it entirely left (dropped); in between it straddles.
    a: i32,
    b: i32,
    /// `+1` for `y1 > y0`.
    sign: i32,
}

/// One edge of the current tile row.
#[derive(Clone, Copy)]
struct Item {
    edge: u32,
    a: i32,
    b: i32,
    r0: u8,
    r1: u8,
    sign: i32,
}

/// Incremental binner: add shapes in z order, then [`Binner::finish`].
#[derive(Default)]
pub struct Binner {
    tiles_x: u32,
    tiles_y: u32,
    clip: bool,
    pending: Vec<(u32, GpuTileEntry)>,
    edges: Vec<GpuEdge>,
    infos: Vec<Info>,
    items: Vec<Item>,
    order: Vec<u32>,
    active: Vec<u32>,
    carry: Vec<GpuEdge>,
    bps: Vec<(u8, i32)>,
    segs: Vec<(u8, u8, i32)>,
}

impl Binner {
    /// `clip == false` keeps every shape edge in every tile entry (the
    /// unclipped reference variant used by tests and the bench).
    pub fn new(tiles_x: u32, tiles_y: u32, clip: bool) -> Self {
        let mut binner = Self::default();
        binner.reset(tiles_x, tiles_y, clip);
        binner
    }

    /// Starts a new frame, keeping every buffer's capacity.
    pub fn reset(&mut self, tiles_x: u32, tiles_y: u32, clip: bool) {
        self.tiles_x = tiles_x;
        self.tiles_y = tiles_y;
        self.clip = clip;
        self.pending.clear();
        self.edges.clear();
    }

    /// Reserves room for about `entries` tile entries and `edges` pool edges.
    pub fn reserve(&mut self, entries: usize, edges: usize) {
        self.pending.reserve(entries.min(MAX_TILE_LIST as usize));
        self.edges.reserve(edges.min(MAX_EDGES as usize));
    }

    /// Adds shape `shape` with canvas-space fixed-point edges `src` (each
    /// spans at least one sample row) over the inclusive tile `range`.
    pub fn add_shape(
        &mut self,
        shape: u32,
        src: &[GpuEdge],
        range: [u32; 4],
    ) -> Result<(), UnsupportedReason> {
        if !self.clip {
            return self.add_unclipped(shape, src, range);
        }
        let rows_per_tile = ROWS as i32;
        self.infos.clear();
        for e in src {
            let (ylo, yhi) = (i64::from(e.y0.min(e.y1)), i64::from(e.y0.max(e.y1)));
            let (xmin, xmax) = (i64::from(e.x0.min(e.x1)), i64::from(e.x0.max(e.x1)));
            let k0 = ceil_shr(ylo - U / 2, U_BITS) as i32;
            let k1 = ceil_shr(yhi - U / 2, U_BITS) as i32;
            self.infos.push(Info {
                row0: k0 >> ROW_BITS,
                row1: (k1 - 1) >> ROW_BITS,
                k0,
                k1,
                a: (ceil_shr(xmin + U / 2, T_BITS) - 1) as i32,
                b: ceil_shr(xmax - U / 2, T_BITS) as i32,
                sign: if e.y1 > e.y0 { 1 } else { -1 },
            });
        }
        let (tx0, tx1) = (range[0] as i32, range[2] as i32);
        for ty in range[1] as i32..=range[3] as i32 {
            self.items.clear();
            for (i, info) in self.infos.iter().enumerate() {
                if info.row0 > ty || info.row1 < ty {
                    continue;
                }
                let base = ty * rows_per_tile;
                self.items.push(Item {
                    edge: i as u32,
                    a: info.a,
                    b: info.b,
                    r0: (info.k0 - base).clamp(0, rows_per_tile) as u8,
                    r1: (info.k1 - base).clamp(0, rows_per_tile) as u8,
                    sign: info.sign,
                });
            }
            if self.items.is_empty() {
                continue;
            }
            self.sweep_row(shape, src, ty, tx0, tx1)?;
        }
        Ok(())
    }

    fn sweep_row(
        &mut self,
        shape: u32,
        src: &[GpuEdge],
        ty: i32,
        tx0: i32,
        tx1: i32,
    ) -> Result<(), UnsupportedReason> {
        let items = std::mem::take(&mut self.items);
        self.active.clear();
        self.order.clear();
        for (n, it) in items.iter().enumerate() {
            if it.a > tx0 {
                if it.a <= tx1 {
                    self.order.push(n as u32);
                }
            } else if it.b > tx0 {
                self.active.push(n as u32);
            }
        }
        if self.order.len() > 1 {
            self.order
                .sort_unstable_by_key(|n| (items[*n as usize].a, *n));
        }
        let mut next = 0usize;
        let mut dirty = true;
        for tx in tx0..=tx1 {
            while next < self.order.len() && items[self.order[next] as usize].a == tx {
                let n = self.order[next];
                dirty = true;
                if items[n as usize].b > tx {
                    self.active.push(n);
                }
                next += 1;
            }
            self.active.retain(|n| items[*n as usize].b > tx);
            if dirty {
                // The carry (edges still entirely right of the tile) only
                // changes when an edge is released.
                self.bps.clear();
                for it in items.iter().filter(|it| it.a > tx) {
                    self.bps.push((it.r0, it.sign));
                    self.bps.push((it.r1, -it.sign));
                }
                self.carry.clear();
                emit_carry(&mut self.bps, &mut self.segs, &mut self.carry);
                dirty = false;
            }
            if self.active.is_empty() && self.carry.is_empty() {
                continue;
            }
            let start = self.edges.len();
            let origin = (i64::from(tx) * T, i64::from(ty) * T);
            for n in &self.active {
                self.edges
                    .push(translate(&src[items[*n as usize].edge as usize], origin));
            }
            self.edges.extend_from_slice(&self.carry);
            self.push_entry(tx, ty, shape, start)?;
        }
        self.items = items;
        Ok(())
    }

    fn add_unclipped(
        &mut self,
        shape: u32,
        src: &[GpuEdge],
        range: [u32; 4],
    ) -> Result<(), UnsupportedReason> {
        for ty in range[1] as i32..=range[3] as i32 {
            for tx in range[0] as i32..=range[2] as i32 {
                let start = self.edges.len();
                let origin = (i64::from(tx) * T, i64::from(ty) * T);
                self.edges.extend(src.iter().map(|e| translate(e, origin)));
                self.push_entry(tx, ty, shape, start)?;
            }
        }
        Ok(())
    }

    fn push_entry(
        &mut self,
        tx: i32,
        ty: i32,
        shape: u32,
        start: usize,
    ) -> Result<(), UnsupportedReason> {
        if self.edges.len() > MAX_EDGES as usize {
            return Err(UnsupportedReason::TooManyEdges);
        }
        if self.pending.len() >= MAX_TILE_LIST as usize {
            return Err(UnsupportedReason::TooManyTileEntries);
        }
        self.pending.push((
            (ty as u32 * self.tiles_x + tx as u32),
            GpuTileEntry {
                shape,
                edge_start: start as u32,
                edge_count: (self.edges.len() - start) as u32,
                prim_start: 0,
                prim_count: 0,
            },
        ));
        Ok(())
    }

    /// Number of entries added so far.
    pub(crate) fn pending_len(&self) -> usize {
        self.pending.len()
    }

    /// Gives the shape's tile `tile` the primitive range `start, count`:
    /// attaches it to the shape's fill entry among `pending[from..to]`
    /// (ascending tiles) or adds a stroke-only entry.
    pub(crate) fn add_prims(
        &mut self,
        shape: u32,
        fill_entries: std::ops::Range<usize>,
        tile: u32,
        start: u32,
        count: u32,
    ) -> Result<(), UnsupportedReason> {
        let found = self.pending[fill_entries.clone()].binary_search_by_key(&tile, |(t, _)| *t);
        match found {
            Ok(i) => {
                let entry = &mut self.pending[fill_entries.start + i].1;
                entry.prim_start = start;
                entry.prim_count = count;
            }
            Err(_) => {
                if self.pending.len() >= MAX_TILE_LIST as usize {
                    return Err(UnsupportedReason::TooManyTileEntries);
                }
                self.pending.push((
                    tile,
                    GpuTileEntry {
                        shape,
                        edge_start: 0,
                        edge_count: 0,
                        prim_start: start,
                        prim_count: count,
                    },
                ));
            }
        }
        Ok(())
    }

    /// Groups the entries per tile (stable, so ascending z).
    pub fn finish(mut self) -> Binned {
        let mut out = Binned::default();
        self.finish_into(&mut out);
        out
    }

    /// [`Binner::finish`] into `out`, reusing its buffers; the binner takes
    /// over `out`'s old edge pool as its next scratch.
    pub fn finish_into(&mut self, out: &mut Binned) {
        let tile_total = self.tiles_x as usize * self.tiles_y as usize;
        out.tiles.clear();
        out.tiles.resize(tile_total, GpuTile::default());
        for (t, _) in &self.pending {
            out.tiles[*t as usize].list_count += 1;
        }
        let mut start = 0u32;
        for tile in &mut out.tiles {
            tile.list_start = start;
            start += tile.list_count;
            tile.list_count = 0;
        }
        out.entries.clear();
        out.entries
            .resize(self.pending.len(), GpuTileEntry::default());
        for (t, entry) in &self.pending {
            let tile = &mut out.tiles[*t as usize];
            out.entries[(tile.list_start + tile.list_count) as usize] = *entry;
            tile.list_count += 1;
        }
        std::mem::swap(&mut out.edges, &mut self.edges);
    }
}

fn translate(e: &GpuEdge, origin: (i64, i64)) -> GpuEdge {
    let f = |v: i32, o: i64| (i64::from(v) - o) as i32;
    GpuEdge {
        x0: f(e.x0, origin.0),
        y0: f(e.y0, origin.1),
        x1: f(e.x1, origin.0),
        y1: f(e.y1, origin.1),
    }
}

/// Fewest-run vertical edges on the tile's right border reproducing the
/// per-row winding given as `(row, delta)` breakpoints (rows `0..=ROWS`):
/// maximal runs of equal-sign adjacent rows, peeled one level at a time.
/// `bps` is scratch and left sorted.
fn emit_carry(bps: &mut [(u8, i32)], segs: &mut Vec<(u8, u8, i32)>, out: &mut Vec<GpuEdge>) {
    if bps.is_empty() {
        return;
    }
    let x = T as i32;
    let edge = |from: u8, to: u8, up: bool| {
        let (ya, yb) = ((i64::from(from) * U) as i32, (i64::from(to) * U) as i32);
        if up {
            GpuEdge {
                x0: x,
                y0: yb,
                x1: x,
                y1: ya,
            }
        } else {
            GpuEdge {
                x0: x,
                y0: ya,
                x1: x,
                y1: yb,
            }
        }
    };
    if let [(r0, d0), (r1, _)] = *bps {
        // One source edge (`r0 < r1`): the common case for small shapes.
        if r0 < r1 {
            out.push(edge(r0, r1, d0 < 0));
        }
        return;
    }
    bps.sort_unstable();
    segs.clear();
    let (mut level, mut last) = (0i32, 0u8);
    for (row, delta) in bps.iter() {
        if *row > last && level != 0 {
            segs.push((last, *row, level));
        }
        level += delta;
        last = *row;
    }
    loop {
        let mut any = false;
        let mut i = 0;
        while i < segs.len() {
            let sign = segs[i].2.signum();
            if sign == 0 {
                i += 1;
                continue;
            }
            any = true;
            let (first, mut end) = (segs[i].0, segs[i].1);
            segs[i].2 -= sign;
            while i + 1 < segs.len() && segs[i + 1].0 == end && segs[i + 1].2.signum() == sign {
                i += 1;
                end = segs[i].1;
                segs[i].2 -= sign;
            }
            let (ya, yb) = ((first as i64 * U) as i32, (end as i64 * U) as i32);
            out.push(if sign > 0 {
                GpuEdge {
                    x0: x,
                    y0: ya,
                    x1: x,
                    y1: yb,
                }
            } else {
                GpuEdge {
                    x0: x,
                    y0: yb,
                    x1: x,
                    y1: ya,
                }
            });
            i += 1;
        }
        if !any {
            break;
        }
    }
}

/// Edge visits a kernel makes: the sum of edge counts over all tile entries.
pub fn edge_visits(b: &Binned) -> u64 {
    b.entries.iter().map(|e| u64::from(e.edge_count)).sum()
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

    /// Closed fixed-point rectangle (px units), horizontals dropped.
    fn rect(x0: i32, y0: i32, x1: i32, y1: i32) -> Vec<GpuEdge> {
        let e = |a: i32, b: i32, c: i32, d: i32| GpuEdge {
            x0: a * PX,
            y0: b * PX,
            x1: c * PX,
            y1: d * PX,
        };
        [
            e(x0, y0, x1, y0),
            e(x1, y0, x1, y1),
            e(x1, y1, x0, y1),
            e(x0, y1, x0, y0),
        ]
        .into_iter()
        .filter(spans_sample_row)
        .collect()
    }

    fn bin(shapes: &[(Vec<GpuEdge>, [u32; 4])], clip: bool) -> Binned {
        let mut b = Binner::new(4, 3, clip);
        for (z, (edges, range)) in shapes.iter().enumerate() {
            b.add_shape(z as u32, edges, *range).unwrap();
        }
        b.finish()
    }

    fn tile_edges(b: &Binned, tile: usize) -> Vec<(u32, Vec<GpuEdge>)> {
        let t = b.tiles[tile];
        b.entries[t.list_start as usize..][..t.list_count as usize]
            .iter()
            .map(|e| {
                (
                    e.shape,
                    b.edges[e.edge_start as usize..][..e.edge_count as usize].to_vec(),
                )
            })
            .collect()
    }

    #[test]
    fn lists_are_z_ordered_and_deterministic() {
        let shapes = [
            (rect(0, 0, 40, 10), [0, 0, 2, 0]),
            (rect(20, 0, 30, 10), [1, 0, 1, 0]),
            (rect(0, 0, 5, 5), [0, 0, 0, 0]),
        ];
        let b = bin(&shapes, true);
        let ids = |t: usize| tile_edges(&b, t).iter().map(|e| e.0).collect::<Vec<_>>();
        assert_eq!(ids(0), [0, 2]);
        assert_eq!(ids(1), [0, 1]);
        assert_eq!(ids(2), [0]);
        assert!(ids(3).is_empty() && ids(4).is_empty());
        assert_eq!(bin(&shapes, true), b);
    }

    #[test]
    fn edges_are_kept_dropped_or_carried_per_tile() {
        let big = rect(2, 2, 60, 40);
        let b = bin(&[(big, [0, 0, 3, 2])], true);
        let t = |px: i32| px * PX;
        // Tile (0,0): left edge kept (tile-local), right edge collapsed into
        // one vertical carry edge on the right border for y = 2..16.
        let e = tile_edges(&b, 0)[0].1.clone();
        assert_eq!(e.len(), 2);
        assert!(e.contains(&GpuEdge {
            x0: t(2),
            y0: t(40),
            x1: t(2),
            y1: t(2)
        }));
        assert!(e.contains(&GpuEdge {
            x0: t(16),
            y0: t(2),
            x1: t(16),
            y1: t(16)
        }));
        // Interior tile (1,1): left edge dropped, one full-height carry edge.
        assert_eq!(
            tile_edges(&b, 5)[0].1,
            [GpuEdge {
                x0: t(16),
                y0: 0,
                x1: t(16),
                y1: t(16)
            }]
        );
        // Tile (3,0): the right edge straddles and is kept, translated.
        assert_eq!(
            tile_edges(&b, 3)[0].1,
            [GpuEdge {
                x0: t(12),
                y0: t(2),
                x1: t(12),
                y1: t(40)
            }]
        );
        // Work: 12 tiles x 2 source edges unclipped, far fewer clipped.
        let un = bin(&[(rect(2, 2, 60, 40), [0, 0, 3, 2])], false);
        assert_eq!(edge_visits(&un), 24);
        assert!(edge_visits(&b) <= 16, "{}", edge_visits(&b));
    }

    #[test]
    fn carry_edges_reproduce_the_row_winding() {
        // Winding +2 for rows 0..64, -1 for rows 10..20, 0 for 40..44.
        let mut w = [2i32; ROWS];
        w[10..20].iter_mut().for_each(|v| *v = -1);
        w[40..44].iter_mut().for_each(|v| *v = 0);
        let mut bps = vec![
            (0, 2),
            (ROWS as u8, -2),
            (10, -3),
            (20, 3),
            (40, -2),
            (44, 2),
        ];
        let mut out = Vec::new();
        emit_carry(&mut bps, &mut Vec::new(), &mut out);
        let mut back = [0i32; ROWS];
        for e in &out {
            assert_eq!(e.x0, T as i32);
            assert_eq!(e.x0, e.x1);
            let (lo, hi) = (e.y0.min(e.y1) as i64 / U, e.y0.max(e.y1) as i64 / U);
            for r in lo..hi {
                back[r as usize] += if e.y1 > e.y0 { 1 } else { -1 };
            }
        }
        assert_eq!(back, w);
        assert!(out.len() <= 8, "{}", out.len());
        let mut none = Vec::new();
        emit_carry(&mut [(5, 1), (5, -1)], &mut Vec::new(), &mut none);
        assert!(none.is_empty());
    }

    #[test]
    fn overflow_is_reported() {
        // Two nested full-canvas rectangles: the edge pool (8 Mi) fills before
        // the tile list (6 Mi entries) does.
        let mut full = rect(0, 0, 16384, 16384);
        full.extend(rect(0, 0, 16000, 16000));
        let mut b = Binner::new(1024, 1024, true);
        let mut result = Ok(());
        for z in 0..9 {
            result = b.add_shape(z, &full, [0, 0, 1023, 1023]);
        }
        assert_eq!(result.unwrap_err(), UnsupportedReason::TooManyEdges);
    }
}
