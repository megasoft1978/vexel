//! Differential test for wide, long and dashed strokes (round and miter joins,
//! all caps, scaled and diagonal paths) at two sizes: the SHA-256 digests were
//! recorded from the rasterizer before the stroke culling work.
//! Set `VEXEL_PRINT_HASHES=1` to print the current digests.
#[path = "common/raster_scene.rs"]
mod raster_scene;
#[path = "common/sha256.rs"]
mod sha256;

use raster_scene::{Rng, view};
use vexel_geometry::Affine;
use vexel_raster::Tile;
use vexel_render::{
    DashedShape, FlatEdge, Linecap, Linejoin, RenderNode, RenderShape, StrokeStyle,
    render_dashed_shapes, render_groups,
};

#[derive(Clone, Copy, Debug)]
enum Case {
    /// Small closed blobs and open polylines with 20-40 px strokes.
    Wide,
    /// Long diagonal polylines crossing the canvas.
    Diagonal,
    /// Wide dashed strokes with short dashes.
    Dashed,
}

fn tile(size: u32) -> Tile {
    Tile::from_owned_rgba(size, size, vec![0; size as usize * size as usize * 4]).unwrap()
}

fn style(rng: &mut Rng, join: Linejoin, lo: f64, hi: f64) -> StrokeStyle {
    let a = if rng.below(4) == 0 { 255 } else { 60 + rng.below(196) as u8 };
    StrokeStyle {
        rgba: [rng.below(256) as u8, rng.below(256) as u8, rng.below(256) as u8, a],
        width: rng.range(lo, hi),
        miter_limit: 4.0,
        dash_offset: rng.range(0.0, 6.0),
        cap: [Linecap::Butt, Linecap::Square, Linecap::Round][rng.below(3)],
        join,
        crisp_edges: false,
    }
}

fn leak<T>(v: Vec<T>) -> &'static [T] {
    Box::leak(v.into_boxed_slice())
}

fn edges(points: &[(f64, f64)], closed: bool) -> &'static [FlatEdge] {
    let count = if closed { points.len() } else { points.len() - 1 };
    leak(
        (0..count)
            .map(|i| {
                let (a, b) = (points[i], points[(i + 1) % points.len()]);
                FlatEdge { x0: a.0, y0: a.1, x1: b.0, y1: b.1 }
            })
            .collect(),
    )
}

fn shape(points: &[(f64, f64)], closed: bool, stroke: StrokeStyle) -> RenderShape<'static> {
    RenderShape {
        edges: edges(points, closed),
        subpaths: &[],
        subpath_closed: &[],
        stroke_closed: closed,
        evenodd: false,
        filled: false,
        fill_rgba: [0; 4],
        stroke: Some(stroke),
        transform: Affine { a: 1.0, b: 0.0, c: 0.0, d: 1.0, e: 0.0, f: 0.0 },
        fill_gradient: None,
        stroke_gradient: None,
    }
}

fn points(rng: &mut Rng, case: Case, size: f64) -> Vec<(f64, f64)> {
    match case {
        Case::Diagonal => {
            let n = 4 + rng.below(8);
            let (mut x, mut y) = (rng.range(0.0, size * 0.3), rng.range(0.0, size * 0.3));
            let mut out = vec![(x, y)];
            for _ in 1..n {
                x += rng.range(size * 0.02, size * 0.2);
                y += rng.range(size * 0.02, size * 0.2);
                out.push((x, y));
            }
            out
        }
        Case::Wide | Case::Dashed => {
            let (cx, cy) = (rng.range(0.0, size), rng.range(0.0, size));
            let r = rng.range(size * 0.03, size * 0.15);
            let n = 3 + rng.below(8);
            (0..n)
                .map(|i| {
                    let a = (i as f64 + rng.range(0.0, 0.8)) / n as f64 * 6.2831853;
                    let d = r * rng.range(0.4, 1.0);
                    (cx + d * a.cos(), cy + d * a.sin())
                })
                .collect()
        }
    }
}

fn digest(size: u32, count: usize, case: Case, join: Linejoin) -> String {
    let mut rng = Rng(0xA5A5_1234 ^ u64::from(size) ^ ((case as u64) << 40));
    let fsize = f64::from(size);
    let mut out = tile(size);
    let mut plain = Vec::new();
    let mut dashes = Vec::new();
    for _ in 0..count {
        let pts = points(&mut rng, case, fsize);
        let closed = !matches!(case, Case::Diagonal) && rng.below(3) != 0;
        let s = shape(&pts, closed, style(&mut rng, join, 20.0 * fsize / 256.0 + 2.0, 40.0 * fsize / 256.0 + 4.0));
        match case {
            Case::Dashed => {
                let pattern = leak(vec![rng.range(2.0, 12.0), rng.range(2.0, 9.0)]);
                dashes.push(DashedShape { shape: s, dashes: pattern });
            }
            _ => plain.push(RenderNode::Shape { shape: s, clip: None }),
        }
    }
    match case {
        Case::Dashed => render_dashed_shapes(&dashes, view(size), &mut out, &|| false).unwrap(),
        _ => render_groups(leak(plain), view(size), &mut out, &|| false).unwrap(),
    }
    sha256::sha256_hex(out.rgba())
}

fn current() -> Vec<(String, String)> {
    let mut rows = Vec::new();
    for (size, count) in [(97u32, 40usize), (256, 90)] {
        for case in [Case::Wide, Case::Diagonal, Case::Dashed] {
            for join in [Linejoin::Round, Linejoin::Miter, Linejoin::Bevel] {
                rows.push((format!("{case:?}-{join:?}-{size}"), digest(size, count, case, join)));
            }
        }
    }
    rows
}

#[test]
fn wide_stroke_scenes_match_the_recorded_digests() {
    let rows = current();
    if std::env::var_os("VEXEL_PRINT_HASHES").is_some() {
        for (name, hash) in &rows {
            println!("{name} {hash}");
        }
    }
    let recorded = include_str!("raster_diff_wide_hashes.txt");
    let expected: Vec<(&str, &str)> = recorded.lines().filter(|l| !l.starts_with('#')).filter_map(|l| l.split_once(' ')).collect();
    assert_eq!(expected.len(), rows.len(), "recorded digest count");
    for ((name, hash), (want_name, want_hash)) in rows.iter().zip(expected) {
        assert_eq!(name, want_name);
        assert_eq!(hash, want_hash, "{name} changed");
    }
}
