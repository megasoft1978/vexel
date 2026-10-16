//! Pixel parity of pattern paint against C `svgworkbench` PPM goldens
//! (`rust/fixtures/render-pattern`, read-only inputs). Same conversion and
//! tolerances as `parity.rs`: the golden is the native-size tile over white.
mod common;
#[path = "common/pattern_data.rs"]
mod pattern_data;
#[path = "common/sha256.rs"]
mod sha256;

use common::parity_data::ShapeData;
use common::*;
use pattern_data::{PATTERN_CASES, PatternCase, PatternShape};
use sha256::sha256_hex;
use std::ops::Range;
use vexel_geometry::Affine;
use vexel_raster::Tile;
use vexel_render::{
    FlatEdge, Linecap, Linejoin, PatternPaint, PatternPaintShape, PatternTileSpec, PreserveAspect,
    RenderShape, StrokeStyle, ViewBox, ViewMap, render_pattern_shapes, render_pattern_tile,
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
    transform: Affine,
    edges: Vec<FlatEdge>,
    ranges: Vec<Range<usize>>,
    closed: Vec<bool>,
    data: &'static ShapeData,
    source: &'static PatternShape,
}

/// Forward element transform from C `shape_stroke_inverse`.
fn forward(m: [f64; 6]) -> Affine {
    let det = m[0] * m[3] - m[1] * m[2];
    let r = 1.0 / det;
    let (a, b, c, d) = (m[3] * r, -m[1] * r, -m[2] * r, m[0] * r);
    Affine {
        a,
        b,
        c,
        d,
        e: -(a * m[4] + c * m[5]),
        f: -(b * m[4] + d * m[5]),
    }
}

fn affine(m: [f64; 6]) -> Affine {
    Affine {
        a: m[0],
        b: m[1],
        c: m[2],
        d: m[3],
        e: m[4],
        f: m[5],
    }
}

fn build(shapes: &'static [PatternShape], shift_x: f64) -> Vec<Built> {
    shapes
        .iter()
        .map(|source| {
            let data = &source.shape;
            assert_eq!(data.dash_count, 0, "dashes are out of scope");
            Built {
                transform: forward(data.transform_inverse),
                edges: data
                    .edges
                    .iter()
                    .map(|e| FlatEdge {
                        x0: e[0] + shift_x,
                        y0: e[1],
                        x1: e[2] + shift_x,
                        y1: e[3],
                    })
                    .collect(),
                ranges: data.subpaths.iter().map(|s| s.0..s.1).collect(),
                closed: data.subpaths.iter().map(|s| s.2).collect(),
                data,
                source,
            }
        })
        .collect()
}

fn render_shape<'a>(
    b: &'a Built,
    fill: Option<PatternPaint<'a>>,
    stroke: Option<PatternPaint<'a>>,
) -> PatternPaintShape<'a> {
    let cap = [Linecap::Butt, Linecap::Square, Linecap::Round][b.data.cap as usize];
    let join = [Linejoin::Miter, Linejoin::Bevel, Linejoin::Round][b.data.join as usize];
    PatternPaintShape {
        shape: RenderShape {
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
                cap,
                join,
                crisp_edges: false,
            }),
            transform: b.transform,
        },
        fill,
        stroke,
    }
}

/// Rasterizes every pattern tile of `case` from its C-dumped children.
fn render_tiles(case: &'static PatternCase) -> Vec<Tile> {
    case.patterns
        .iter()
        .map(|pattern| {
            let built = build(pattern.children, 0.0);
            let children: Vec<_> = built.iter().map(|b| render_shape(b, None, None)).collect();
            let spec = PatternTileSpec {
                width: pattern.width,
                height: pattern.height,
                tile_width: pattern.tile_width,
                tile_height: pattern.tile_height,
            };
            render_pattern_tile(spec, &children, NEVER)
                .expect("tile")
                .expect("non-empty tile")
        })
        .collect()
}

fn render_case(case: &'static PatternCase, built: &[Built], tiles: &[Tile]) -> Tile {
    let aspect = case.aspect_set.then_some(PreserveAspect {
        align_x: case.align[0],
        align_y: case.align[1],
        none: case.aspect_none,
        slice: case.aspect_slice,
    });
    let view = ViewMap::from_viewbox(
        ViewBox {
            x: case.view[0],
            y: case.view[1],
            width: case.view[2],
            height: case.view[3],
        },
        case.width,
        case.height,
        aspect,
    )
    .unwrap();
    let paint = |data: &Option<pattern_data::PaintData>| {
        data.as_ref().map(|p| PatternPaint {
            tile: Some(&tiles[p.pattern]),
            inverse: affine(p.inverse),
            opacity: p.opacity,
        })
    };
    let shapes: Vec<_> = built
        .iter()
        .map(|b| {
            render_shape(
                b,
                paint(&b.source.fill_pattern),
                paint(&b.source.stroke_pattern),
            )
        })
        .collect();
    let mut tile = empty_tile(case.width, case.height);
    render_pattern_shapes(&shapes, view, &mut tile, NEVER).expect("render");
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

fn fixture_dir() -> String {
    format!(
        "{}/../../fixtures/render-pattern",
        env!("CARGO_MANIFEST_DIR")
    )
}

fn load_ppm(id: &str, width: u32, height: u32) -> Vec<u8> {
    let path = format!("{}/{id}.ppm", fixture_dir());
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("golden {path} missing: {e}"));
    let header = format!("P6\n{width} {height}\n255\n");
    assert!(
        bytes.starts_with(header.as_bytes()),
        "{id}: unexpected PPM header"
    );
    let body = bytes[header.len()..].to_vec();
    assert_eq!(body.len(), width as usize * height as usize * 3);
    body
}

/// True when a pixel centre lies within the dilation of a parent shape edge.
fn near_edge(built: &[Built], x: u32, y: u32) -> bool {
    let (cx, cy) = (f64::from(x) + 0.5, f64::from(y) + 0.5);
    built.iter().any(|b| {
        let extent = b.data.stroke.get(3).filter(|a| **a != 0).map_or(0.0, |_| {
            b.data.stroke_width * 0.5 * b.data.miter_limit.max(1.5)
        });
        b.edges.iter().any(|e| {
            let (dx, dy) = (e.x1 - e.x0, e.y1 - e.y0);
            let len2 = dx * dx + dy * dy;
            let t = if len2 == 0.0 {
                0.0
            } else {
                (((cx - e.x0) * dx + (cy - e.y0) * dy) / len2).clamp(0.0, 1.0)
            };
            (cx - (e.x0 + t * dx)).hypot(cy - (e.y0 + t * dy))
                <= extent + tolerance::DILATION_PIXELS + 0.75
        })
    })
}

fn compare(case: &PatternCase, built: &[Built], actual: &[u8], golden: &[u8]) -> Report {
    let mut report = Report {
        total: golden.len() / 3,
        max_delta: 0,
        differing: 0,
        over_two: 0,
        outside_dilation: 0,
    };
    for index in 0..report.total {
        let delta = (0..3)
            .map(|c| (i32::from(actual[index * 3 + c]) - i32::from(golden[index * 3 + c])).abs())
            .max()
            .unwrap();
        report.max_delta = report.max_delta.max(delta);
        if delta > 0 {
            report.differing += 1;
            let (x, y) = (
                (index % case.width as usize) as u32,
                (index / case.width as usize) as u32,
            );
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

fn case(id: &str) -> &'static PatternCase {
    PATTERN_CASES.iter().find(|c| c.id == id).expect("case")
}

fn run(id: &str) -> Report {
    let case = case(id);
    let built = build(case.shapes, 0.0);
    let tiles = render_tiles(case);
    let actual = over_white(&render_case(case, &built, &tiles));
    let golden = load_ppm(id, case.width, case.height);
    let report = compare(case, &built, &actual, &golden);
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

parity_case!(pattern_user_space, "pattern-user-space");
parity_case!(pattern_object_bounds, "pattern-object-bounds");
parity_case!(pattern_transform, "pattern-transform");
parity_case!(pattern_stroke, "pattern-stroke");
parity_case!(pattern_viewbox, "pattern-viewbox");

#[test]
fn harness_rejects_a_deliberately_shifted_pattern() {
    let case = case("pattern-user-space");
    let golden = load_ppm(case.id, case.width, case.height);
    let tiles = render_tiles(case);
    let good = build(case.shapes, 0.0);
    let ok = compare(
        case,
        &good,
        &over_white(&render_case(case, &good, &tiles)),
        &golden,
    );
    assert!(ok.passes(), "unshifted control must pass: {ok:?}");
    // Shifting the painted shape moves it relative to its pattern phase only
    // through the shape edge, so shift the pattern inverse instead: half a
    // tile pixel changes the sampled tile phase.
    let shifted_tiles = render_tiles(case);
    let built = build(case.shapes, 0.0);
    let aspect = None;
    let view = ViewMap::from_viewbox(
        ViewBox {
            x: case.view[0],
            y: case.view[1],
            width: case.view[2],
            height: case.view[3],
        },
        case.width,
        case.height,
        aspect,
    )
    .unwrap();
    let shapes: Vec<_> = built
        .iter()
        .map(|b| {
            let p = b.source.fill_pattern.as_ref().unwrap();
            let mut inverse = affine(p.inverse);
            inverse.e += 2.0;
            render_shape(
                b,
                Some(PatternPaint {
                    tile: Some(&shifted_tiles[0]),
                    inverse,
                    opacity: p.opacity,
                }),
                None,
            )
        })
        .collect();
    let mut tile = empty_tile(case.width, case.height);
    render_pattern_shapes(&shapes, view, &mut tile, NEVER).unwrap();
    let bad = compare(case, &built, &over_white(&tile), &golden);
    println!("PARITY negative (pattern-user-space phase shifted 2 tile px): {bad:?}");
    assert!(
        !bad.passes(),
        "a shifted pattern phase must fail the harness: {bad:?}"
    );
}

#[test]
fn manifest_records_provenance_and_every_file_hash() {
    let dir = fixture_dir();
    let manifest = std::fs::read_to_string(format!("{dir}/manifest.json")).expect("manifest");
    for key in [
        "legacy_tag_commit",
        "legacy_binary_sha256",
        "legacy_build_commands",
        "compiler",
        "macos",
    ] {
        assert!(
            manifest.contains(&format!("\"{key}\"")),
            "manifest lacks {key}"
        );
    }
    for case in PATTERN_CASES {
        let entry_start = manifest
            .find(&format!("\"fixture_id\": \"{}\"", case.id))
            .unwrap_or_else(|| panic!("{} not in manifest", case.id));
        let entry = &manifest[entry_start..];
        let entry = &entry[..entry.find('}').expect("entry end")];
        let value = |key: &str| -> String {
            let at = entry
                .find(&format!("\"{key}\": \""))
                .unwrap_or_else(|| panic!("{} lacks {key}", case.id));
            let rest = &entry[at + key.len() + 5..];
            rest[..rest.find('"').unwrap()].to_string()
        };
        let svg = std::fs::read(format!("{dir}/{}.svg", case.id)).unwrap();
        assert_eq!(
            sha256_hex(&svg),
            value("source_sha256"),
            "{} svg hash",
            case.id
        );
        let ppm = std::fs::read(format!("{dir}/{}.ppm", case.id)).unwrap();
        assert_eq!(
            sha256_hex(&ppm),
            value("ppm_sha256"),
            "{} ppm hash",
            case.id
        );
        assert!(entry.contains(&format!("\"width\": {}", case.width)));
        assert!(entry.contains(&format!("\"height\": {}", case.height)));
    }
}
