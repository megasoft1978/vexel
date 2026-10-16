//! AC-3: pixel parity against C `svgworkbench` PPM goldens (read-only inputs).
//!
//! The goldens are the native-size tile written by the C `write_tile_ppm`
//! (frame composed over white as `min(255, rgb + 255 - alpha)`); the same
//! conversion is applied to the Rust tile before comparing.
mod common;
use common::parity_data::{CASES, Case, EXTRA, ShapeData};
use common::*;
use std::ops::Range;
use vexel_raster::Tile;
use vexel_render::{
    FlatEdge, Linecap, Linejoin, PreserveAspect, RenderShape, StrokeStyle, ViewBox, ViewMap,
    render_shapes,
};

/// Thresholds of the work order. They are fixed here and must not be raised.
mod tolerance {
    pub const MAX_CHANNEL_DELTA: i32 = 1;
    pub const MAX_DIFFERING_FRACTION: f64 = 0.005;
    pub const MAX_ANY_DELTA: i32 = 2;
    /// Pixels farther than this from any shape edge must be exactly equal.
    pub const DILATION_PIXELS: f64 = 1.0;
}

#[derive(Debug)]
struct Report {
    total: usize,
    max_delta: i32,
    differing: usize,
    over_two: usize,
    outside_dilation: usize,
}

impl Report {
    fn passes(&self) -> bool {
        self.max_delta <= tolerance::MAX_CHANNEL_DELTA
            && self.over_two == 0
            && self.outside_dilation == 0
            && (self.differing as f64) <= tolerance::MAX_DIFFERING_FRACTION * self.total as f64
    }
}

struct Built {
    transform: vexel_geometry::Affine,
    edges: Vec<FlatEdge>,
    ranges: Vec<Range<usize>>,
    closed: Vec<bool>,
    data: &'static ShapeData,
}

/// Forward element transform from C `shape_stroke_inverse` (the same matrix
/// inversion formula as C `svg_shape_affine_inverse`).
fn forward(m: [f64; 6]) -> vexel_geometry::Affine {
    let det = m[0] * m[3] - m[1] * m[2];
    let r = 1.0 / det;
    let (a, b, c, d) = (m[3] * r, -m[1] * r, -m[2] * r, m[0] * r);
    vexel_geometry::Affine { a, b, c, d, e: -(a * m[4] + c * m[5]), f: -(b * m[4] + d * m[5]) }
}

fn cap(value: i32) -> Linecap {
    [Linecap::Butt, Linecap::Square, Linecap::Round][value as usize]
}

fn join(value: i32) -> Linejoin {
    [Linejoin::Miter, Linejoin::Bevel, Linejoin::Round][value as usize]
}

fn build(case: &'static Case, shift_x: f64) -> Vec<Built> {
    case.shapes
        .iter()
        .map(|data| {
            assert_eq!(data.dash_count, 0, "{}: dashes are out of scope", case.id);
            Built {
                transform: forward(data.transform_inverse),
                edges: data
                    .edges
                    .iter()
                    .map(|e| FlatEdge { x0: e[0] + shift_x, y0: e[1], x1: e[2] + shift_x, y1: e[3] })
                    .collect(),
                ranges: data.subpaths.iter().map(|s| s.0..s.1).collect(),
                closed: data.subpaths.iter().map(|s| s.2).collect(),
                data,
            }
        })
        .collect()
}

fn shapes(built: &[Built]) -> Vec<RenderShape<'_>> {
    built
        .iter()
        .map(|b| RenderShape {
            edges: &b.edges,
            subpaths: &b.ranges,
            subpath_closed: &b.closed,
            stroke_closed: b.data.stroke_closed,
            evenodd: b.data.evenodd,
            filled: b.data.filled,
            fill_rgba: b.data.fill,
            stroke: Some(StrokeStyle {
                rgba: b.data.stroke,
                width: b.data.stroke_width,
                miter_limit: b.data.miter_limit,
                dash_offset: 0.0,
                cap: cap(b.data.cap),
                join: join(b.data.join),
                crisp_edges: false,
            }),
            transform: b.transform,
        })
        .collect()
}

fn render_case(case: &'static Case, built: &[Built]) -> Tile {
    let aspect = case.aspect_set.then_some(PreserveAspect {
        align_x: case.align[0],
        align_y: case.align[1],
        none: case.aspect_none,
        slice: case.aspect_slice,
    });
    let view = ViewMap::from_viewbox(
        ViewBox { x: case.view[0], y: case.view[1], width: case.view[2], height: case.view[3] },
        case.width,
        case.height,
        aspect,
    )
    .unwrap();
    let mut tile = empty_tile(case.width, case.height);
    render_shapes(&shapes(built), view, &mut tile, NEVER).expect("render");
    tile
}

/// C `write_tile_ppm` colour conversion.
fn over_white(tile: &Tile) -> Vec<u8> {
    tile.rgba()
        .chunks_exact(4)
        .flat_map(|p| {
            let a = u32::from(p[3]);
            (0..3).map(move |c| (u32::from(p[c]) + 255 - a).min(255) as u8)
        })
        .collect()
}

fn load_ppm(id: &str, width: u32, height: u32) -> Vec<u8> {
    let path = format!("{}/tests/golden/{id}.ppm", env!("CARGO_MANIFEST_DIR"));
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("golden {path} missing: {e}"));
    let header = format!("P6\n{width} {height}\n255\n");
    assert!(bytes.starts_with(header.as_bytes()), "{id}: unexpected PPM header");
    let body = bytes[header.len()..].to_vec();
    assert_eq!(body.len(), width as usize * height as usize * 3);
    body
}

fn near_edge(built: &[Built], x: u32, y: u32) -> bool {
    let (cx, cy) = (f64::from(x) + 0.5, f64::from(y) + 0.5);
    built.iter().any(|b| {
        let extent = b
            .data
            .stroke
            .get(3)
            .filter(|a| **a != 0)
            .map_or(0.0, |_| b.data.stroke_width * 0.5 * b.data.miter_limit.max(1.5));
        b.edges.iter().any(|e| {
            let (dx, dy) = (e.x1 - e.x0, e.y1 - e.y0);
            let len2 = dx * dx + dy * dy;
            let t = if len2 == 0.0 { 0.0 } else { (((cx - e.x0) * dx + (cy - e.y0) * dy) / len2).clamp(0.0, 1.0) };
            (cx - (e.x0 + t * dx)).hypot(cy - (e.y0 + t * dy))
                <= extent + tolerance::DILATION_PIXELS + 0.75
        })
    })
}

fn compare(case: &Case, built: &[Built], actual: &[u8], golden: &[u8]) -> Report {
    let mut report = Report { total: golden.len() / 3, max_delta: 0, differing: 0, over_two: 0, outside_dilation: 0 };
    for index in 0..report.total {
        let delta = (0..3)
            .map(|c| (i32::from(actual[index * 3 + c]) - i32::from(golden[index * 3 + c])).abs())
            .max()
            .unwrap();
        report.max_delta = report.max_delta.max(delta);
        if delta > 0 {
            report.differing += 1;
            let (x, y) = ((index % case.width as usize) as u32, (index / case.width as usize) as u32);
            if delta > tolerance::MAX_ANY_DELTA {
                report.over_two += 1;
            }
            if !near_edge(built, x, y) {
                report.outside_dilation += 1;
            }
        }
    }
    report
}

fn run(id: &str) -> Report {
    let case = CASES.iter().chain(EXTRA).find(|c| c.id == id).expect("case");
    let built = build(case, 0.0);
    let tile = render_case(case, &built);
    let golden = load_ppm(id, case.width, case.height);
    let report = compare(case, &built, &over_white(&tile), &golden);
    println!(
        "PARITY {id}: pixels={} max_delta={} differing={} over_two={} outside_dilation={}",
        report.total, report.max_delta, report.differing, report.over_two, report.outside_dilation
    );
    report
}

macro_rules! parity_case {
    ($name:ident, $id:expr) => {
        #[test]
        fn $name() {
            let report = run($id);
            assert!(report.passes(), "{}: {report:?}", $id);
        }
    };
}

parity_case!(shapes_basic, "shapes-basic");
parity_case!(shapes_rounded, "shapes-rounded");
parity_case!(shapes_lines_polygons, "shapes-lines-polygons");
parity_case!(path_line_curves, "path-line-curves");
parity_case!(path_relative_commands, "path-relative-commands");
parity_case!(style_presentation_attributes, "style-presentation-attributes");
// Crate-local extra: caps, joins, miter fallback, rotation and non-uniform
// scale transforms, evenodd subpaths, and a scaled letterboxed view box.
parity_case!(render_extra, "render-extra");

#[test]
fn every_golden_is_listed_in_the_manifest_with_its_size() {
    let manifest = std::fs::read_to_string(format!("{}/tests/golden/manifest.json", env!("CARGO_MANIFEST_DIR"))).unwrap();
    for case in CASES {
        assert!(manifest.contains(&format!("\"fixture_id\": \"{}\"", case.id)), "{}", case.id);
        assert!(manifest.contains("\"ppm_sha256\""));
    }
    assert!(manifest.contains("\"width\": 128") && manifest.contains("\"height\": 96"));
}

#[test]
fn harness_rejects_a_deliberately_shifted_shape() {
    let case = CASES.iter().find(|c| c.id == "shapes-basic").unwrap();
    let golden = load_ppm(case.id, case.width, case.height);
    let good = build(case, 0.0);
    let ok = compare(case, &good, &over_white(&render_case(case, &good)), &golden);
    assert!(ok.passes(), "unshifted control must pass: {ok:?}");
    let shifted = build(case, 0.5);
    let bad = compare(case, &shifted, &over_white(&render_case(case, &shifted)), &golden);
    println!("PARITY negative (shapes-basic shifted +0.5px): {bad:?}");
    assert!(!bad.passes(), "a half-pixel shift must fail the harness: {bad:?}");
}
