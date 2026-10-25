//! Differential and timing tests: the optimised `render_core` against a naive
//! reference that rescans every edge per pixel row and tests every stroke
//! primitive per sample (the pre-optimisation algorithm). Byte equality only.
//!
//! `VEXEL_PERF_PRINT=1 cargo test -p vexel-render --lib scene_tests -- --nocapture`
//! prints the timings and an FNV-1a hash of each output.

use crate::bounds::shape_bounds;
use crate::clip::ClipList;
use crate::composite::{SAMPLE_COUNT, resolve_samples, source_over};
use crate::fill::inside;
use crate::render::{render_core, render_shapes_clipped};
use crate::stroke::{self, StrokePrims};
use crate::validate::validate;
use crate::{
    ClipShape, FlatEdge, Linecap, Linejoin, RenderClip, RenderShape, SAMPLES_PER_AXIS, StrokeStyle,
    ViewBox, ViewMap,
};
use std::time::Instant;
use vexel_geometry::{Affine, Point, affine_identity, affine_map_point};
use vexel_raster::Tile;

const LOCATIONS: [f64; SAMPLES_PER_AXIS] = [0.125, 0.375, 0.625, 0.875];
const NEVER: &dyn Fn() -> bool = &|| false;

fn view(size: u32) -> ViewMap {
    let side = f64::from(size);
    let vb = ViewBox { x: 0.0, y: 0.0, width: side, height: side };
    ViewMap::from_viewbox(vb, size, size, None).unwrap()
}

fn tile(size: u32) -> Tile {
    Tile::from_owned_rgba(size, size, vec![0; size as usize * size as usize * 4]).unwrap()
}

fn fnv(bytes: &[u8]) -> u64 {
    bytes
        .iter()
        .fold(0xcbf2_9ce4_8422_2325, |h, b| (h ^ u64::from(*b)).wrapping_mul(0x0100_0000_01b3))
}

/// The original algorithm: all edges scanned per row, linear stroke tests.
fn reference(shapes: &[RenderShape<'_>], clips: &[ClipList<'_>], view: ViewMap, tile: &mut Tile) {
    let mut budget = validate(shapes, &[], &view, tile).unwrap();
    let width = tile.width() as usize;
    for (index, shape) in shapes.iter().enumerate() {
        let style = shape.stroke.filter(|s| stroke::is_visible(s) && !shape.edges.is_empty());
        let inverse = if style.is_some() { stroke::invert(shape.transform) } else { None };
        let Some(bounds) = shape_bounds(shape, &view, inverse, false).unwrap() else {
            continue;
        };
        let prims = match (style.as_ref(), inverse) {
            (Some(style), Some(inverse)) => {
                Some(StrokePrims::build(shape, style, inverse, None, &mut budget).unwrap())
            }
            _ => None,
        };
        let clip_list: ClipList<'_> = clips.get(index).copied().unwrap_or(&[]);
        let fill_visible = shape.filled && shape.fill_rgba[3] != 0;
        for y in bounds.y_first..=bounds.y_last {
            let low = view.to_user(0.0, f64::from(y)).1;
            let high = view.to_user(0.0, f64::from(y) + 1.0).1;
            let active: Vec<usize> = (0..shape.edges.len())
                .filter(|&i| {
                    let e = &shape.edges[i];
                    e.y0.min(e.y1) <= high && e.y0.max(e.y1) >= low
                })
                .collect();
            for x in bounds.x_first..=bounds.x_last {
                let mut samples = [[0u8; 4]; SAMPLE_COUNT];
                for (sy, ly) in LOCATIONS.iter().enumerate() {
                    for (sx, lx) in LOCATIONS.iter().enumerate() {
                        let (dx, dy) = (f64::from(x) + lx, f64::from(y) + ly);
                        if !view.sample_in_view(dx, dy) {
                            continue;
                        }
                        let (px, py) = view.to_user(dx, dy);
                        if !clip_list.iter().all(|c| c.contains(px, py)) {
                            continue;
                        }
                        let out = &mut samples[sy * SAMPLES_PER_AXIS + sx];
                        let local =
                            inverse.and_then(|i| affine_map_point(i, Point { x: px, y: py }));
                        if fill_visible && inside(shape.edges, &active, shape.evenodd, px, py) {
                            source_over(out, shape.fill_rgba);
                        }
                        if let (Some(prims), Some(style), Some(local)) =
                            (prims.as_ref(), shape.stroke.as_ref(), local)
                            && prims.contains_linear(local.x, local.y)
                        {
                            source_over(out, style.rgba);
                        }
                    }
                }
                let offset = (y as usize * width + x as usize) * 4;
                let pixel: &mut [u8; 4] =
                    (&mut tile.rgba_mut()[offset..offset + 4]).try_into().unwrap();
                source_over(pixel, resolve_samples(&samples));
            }
        }
    }
}

fn rect(x: f64, y: f64, w: f64, h: f64) -> Vec<FlatEdge> {
    let p = [(x, y), (x + w, y), (x + w, y + h), (x, y + h)];
    (0..4)
        .map(|i| FlatEdge { x0: p[i].0, y0: p[i].1, x1: p[(i + 1) % 4].0, y1: p[(i + 1) % 4].1 })
        .collect()
}

fn fill_shape(edges: &[FlatEdge], rgba: [u8; 4], evenodd: bool) -> RenderShape<'_> {
    RenderShape {
        fill_gradient: None,
        stroke_gradient: None,
        edges,
        subpaths: &[],
        subpath_closed: &[],
        stroke_closed: true,
        evenodd,
        filled: true,
        fill_rgba: rgba,
        stroke: None,
        transform: affine_identity(),
    }
}

fn stroke_style(rgba: [u8; 4], width: f64, cap: Linecap, join: Linejoin) -> StrokeStyle {
    StrokeStyle { rgba, width, miter_limit: 4.0, dash_offset: 0.0, cap, join, crisp_edges: false }
}

/// 512x512 scene of the `perf_gate` workload: `count` opaque rectangles.
fn rect_scene(count: usize) -> Vec<Vec<FlatEdge>> {
    (0..count)
        .map(|n| {
            let (x, y) = (((n * 37) % 480) as f64, ((n * 91) % 480) as f64);
            rect(x, y, (8 + n % 24) as f64, (8 + n % 17) as f64)
        })
        .collect()
}

fn rect_color(n: usize) -> [u8; 4] {
    let v = (n * 2_654_435_761) & 0xff_ffff;
    [(v >> 16) as u8, (v >> 8) as u8, v as u8, 255]
}

/// Deterministic closed random-walk polygon of `n` segments inside `size`.
fn walk(n: usize, size: f64) -> Vec<FlatEdge> {
    let mut state = 0x2545_f491_4f6c_dd1d_u64;
    let mut next = || {
        state =
            state.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        ((state >> 33) as f64) / f64::from(1u32 << 31)
    };
    let points: Vec<(f64, f64)> = (0..n).map(|_| (next() * size, next() * size)).collect();
    (0..n)
        .map(|i| {
            let (a, b) = (points[i], points[(i + 1) % n]);
            FlatEdge { x0: a.0, y0: a.1, x1: b.0, y1: b.1 }
        })
        .collect()
}

/// A small-step closed walk so segments stay short (like a flattened curve).
fn crawl(n: usize, size: f64) -> Vec<FlatEdge> {
    let (cx, cy) = (size / 2.0, size / 2.0);
    let point = |i: usize| {
        let t = i as f64 / n as f64 * std::f64::consts::TAU;
        let r = size * 0.4 * (1.0 + 0.1 * (t * 37.0).sin());
        (cx + r * t.cos(), cy + r * t.sin())
    };
    (0..n)
        .map(|i| {
            let (a, b) = (point(i), point((i + 1) % n));
            FlatEdge { x0: a.0, y0: a.1, x1: b.0, y1: b.1 }
        })
        .collect()
}

fn run_both(shapes: &[RenderShape<'_>], clips: &[ClipList<'_>], size: u32, name: &str) {
    run_view(shapes, clips, view(size), size, name);
}

fn run_view(
    shapes: &[RenderShape<'_>],
    clips: &[ClipList<'_>],
    map: ViewMap,
    size: u32,
    name: &str,
) {
    let mut fast = tile(size);
    let t = Instant::now();
    if clips.is_empty() {
        render_core(shapes, &[], &[], &[], map, &mut fast, NEVER).unwrap();
    } else {
        render_shapes_clipped(shapes, clips, map, &mut fast, NEVER).unwrap();
    }
    let fast_ms = t.elapsed().as_secs_f64() * 1000.0;
    let mut naive = tile(size);
    let t = Instant::now();
    reference(shapes, clips, map, &mut naive);
    let naive_ms = t.elapsed().as_secs_f64() * 1000.0;
    if std::env::var("VEXEL_PERF_PRINT").is_ok_and(|v| v == "1") {
        println!(
            "scene {name}: optimised_ms={fast_ms:.1} naive_ms={naive_ms:.1} hash={:016x}",
            fnv(fast.rgba())
        );
    }
    assert!(
        fast.rgba() == naive.rgba(),
        "{name}: optimised output differs from the naive reference"
    );
    assert!(fast.rgba().iter().any(|b| *b != 0), "{name}: scene rendered nothing");
}

#[test]
fn rects_2000_match_reference() {
    let edges = rect_scene(2000);
    let shapes: Vec<_> =
        edges.iter().enumerate().map(|(n, e)| fill_shape(e, rect_color(n), false)).collect();
    run_both(&shapes, &[], 512, "rects_2000_512");
}

fn ring_clip<'a>(edges: &'a [FlatEdge], shapes: &'a [ClipShape]) -> RenderClip<'a> {
    RenderClip {
        edges,
        shapes,
        parent: None,
        object_bounding_box: false,
        bounds: [0.0; 4],
        user_inverse: affine_identity(),
    }
}

#[test]
fn overlapping_strokes_and_clips_match_reference() {
    let size = 160u32;
    let side = f64::from(size);
    let polylines: Vec<Vec<FlatEdge>> = (0..9)
        .map(|k| {
            let walk = walk(14 + k, side);
            walk[..walk.len() - 1].to_vec()
        })
        .collect();
    let caps = [Linecap::Butt, Linecap::Round, Linecap::Square];
    let joins = [Linejoin::Miter, Linejoin::Round, Linejoin::Bevel];
    let rotate = Affine { a: 0.8, b: 0.3, c: -0.3, d: 0.9, e: 12.0, f: -5.0 };
    let mut shapes: Vec<RenderShape<'_>> = Vec::new();
    for (k, edges) in polylines.iter().enumerate() {
        let mut shape = fill_shape(edges, [40 + 20 * k as u8, 90, 200, 120], k % 2 == 0);
        shape.filled = k % 3 != 0;
        shape.stroke_closed = k % 2 == 1;
        shape.stroke = Some(stroke_style(
            [200, 30 * k as u8, 50, 180],
            1.5 + k as f64,
            caps[k % 3],
            joins[(k / 3) % 3],
        ));
        if k == 4 {
            shape.transform = rotate;
        }
        shapes.push(shape);
    }
    let circle: Vec<FlatEdge> = crawl(40, side * 0.8);
    let circle: Vec<FlatEdge> = circle
        .iter()
        .map(|e| FlatEdge { x0: e.x0 + 16.0, y0: e.y0 + 16.0, x1: e.x1 + 16.0, y1: e.y1 + 16.0 })
        .collect();
    let child = [ClipShape { edges: 0..circle.len(), evenodd: false, filled: true }];
    let outer = ring_clip(&circle, &child);
    let square = rect(30.0, 30.0, 100.0, 100.0);
    let square_child = [ClipShape { edges: 0..4, evenodd: false, filled: true }];
    let mut inner = ring_clip(&square, &square_child);
    inner.parent = Some(&outer);
    let first: [&RenderClip<'_>; 1] = [&outer];
    let second: [&RenderClip<'_>; 2] = [&inner, &outer];
    let empty: [&RenderClip<'_>; 0] = [];
    let clips: Vec<ClipList<'_>> = (0..shapes.len())
        .map(|k| match k % 3 {
            0 => &first[..],
            1 => &second[..],
            _ => &empty[..],
        })
        .collect();
    run_both(&shapes, &clips, size, "overlap_strokes_clips_160");
}

#[test]
fn large_path_fill_matches_reference() {
    let edges = crawl(20_000, 256.0);
    let shapes = [fill_shape(&edges, [10, 120, 60, 255], true)];
    run_both(&shapes, &[], 256, "path_fill_20k_256");
    let edges = walk(3_000, 256.0);
    let shapes = [fill_shape(&edges, [10, 120, 60, 200], false)];
    run_both(&shapes, &[], 256, "walk_fill_3k_256");
}

#[test]
fn large_path_stroke_matches_reference() {
    let edges = crawl(600, 128.0);
    let mut shape = fill_shape(&edges, [0, 0, 0, 0], false);
    shape.filled = false;
    shape.stroke = Some(stroke_style([20, 20, 200, 255], 2.0, Linecap::Round, Linejoin::Round));
    run_both(&[shape], &[], 128, "path_stroke_600_128");
    let edges = walk(500, 128.0);
    let mut shape = fill_shape(&edges, [200, 0, 0, 90], true);
    shape.stroke = Some(stroke_style([20, 160, 20, 200], 1.2, Linecap::Square, Linejoin::Miter));
    run_both(&[shape], &[], 128, "walk_fillstroke_500_128");
}

/// Large single-path timings; the naive reference is skipped (too slow).
#[test]
fn large_path_timings() {
    let print = std::env::var("VEXEL_PERF_PRINT").is_ok_and(|v| v == "1");
    let edges = crawl(100_000, 512.0);
    let shapes = [fill_shape(&edges, [10, 120, 60, 255], true)];
    let mut out = tile(512);
    let t = Instant::now();
    render_core(&shapes, &[], &[], &[], view(512), &mut out, NEVER).unwrap();
    let fill_ms = t.elapsed().as_secs_f64() * 1000.0;
    let stroked = crawl(5_000, 512.0);
    let mut shape = fill_shape(&stroked, [0, 0, 0, 0], false);
    shape.filled = false;
    shape.stroke = Some(stroke_style([20, 20, 200, 255], 2.0, Linecap::Butt, Linejoin::Miter));
    let mut out2 = tile(512);
    let t = Instant::now();
    render_core(&[shape], &[], &[], &[], view(512), &mut out2, NEVER).unwrap();
    let stroke_ms = t.elapsed().as_secs_f64() * 1000.0;
    if print {
        println!(
            "scene path_fill_100k_512 / path_stroke_5k_512: fill_ms={fill_ms:.1} fill_hash={:016x} stroke_ms={stroke_ms:.1} stroke_hash={:016x}",
            fnv(out.rgba()),
            fnv(out2.rgba())
        );
    }
    assert!(out.rgba().iter().any(|b| *b != 0) && out2.rgba().iter().any(|b| *b != 0));
}

#[test]
fn letterboxed_and_scaled_views_match_reference() {
    let star: Vec<FlatEdge> = (0..5)
        .map(|k| {
            let point = |i: usize| {
                let t = (i * 2 % 5) as f64 * std::f64::consts::TAU / 5.0 - 1.0;
                (50.0 + 45.0 * t.cos(), 30.0 + 28.0 * t.sin())
            };
            let (a, b) = (point(k), point((k + 1) % 5));
            FlatEdge { x0: a.0, y0: a.1, x1: b.0, y1: b.1 }
        })
        .collect();
    let boxy = rect(-10.0, 5.5, 130.0, 20.25);
    let open = walk(40, 100.0);
    let mut stroked = fill_shape(&open[..39], [30, 30, 220, 255], false);
    stroked.filled = false;
    stroked.stroke = Some(stroke_style([220, 30, 30, 200], 3.0, Linecap::Round, Linejoin::Bevel));
    let shapes = [
        fill_shape(&star, [200, 100, 20, 255], true),
        fill_shape(&star, [20, 100, 200, 128], false),
        fill_shape(&boxy, [0, 200, 0, 90], false),
        stroked,
    ];
    let vb = ViewBox { x: 3.0, y: -2.0, width: 100.0, height: 60.0 };
    for (aspect, name) in [
        (None, "letterbox_meet"),
        (
            Some(crate::PreserveAspect { align_x: 1.0, align_y: 0.0, none: false, slice: true }),
            "slice",
        ),
        (
            Some(crate::PreserveAspect { align_x: 0.0, align_y: 0.0, none: true, slice: false }),
            "none",
        ),
    ] {
        let map = ViewMap::from_viewbox(vb, 131, 97, aspect).unwrap();
        let mut fast = Tile::from_owned_rgba(131, 97, vec![0; 131 * 97 * 4]).unwrap();
        render_core(&shapes, &[], &[], &[], map, &mut fast, NEVER).unwrap();
        let mut naive = Tile::from_owned_rgba(131, 97, vec![0; 131 * 97 * 4]).unwrap();
        reference(&shapes, &[], map, &mut naive);
        assert!(fast.rgba() == naive.rgba(), "{name}: differs from the naive reference");
        assert!(fast.rgba().iter().any(|b| *b != 0), "{name}: rendered nothing");
    }
}
