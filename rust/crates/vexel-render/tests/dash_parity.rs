//! Dashed-stroke pixel parity against C `svgworkbench` PPM goldens
//! (`rust/fixtures/render-dash/`), the empty-dash identity with
//! `render_shapes`, and the golden manifest checks.
//!
//! The goldens are the native-size frame written by the C `write_tile_ppm`
//! (composited over white as `min(255, rgb + 255 - alpha)`); the same
//! conversion is applied to the Rust tile. Tolerances are those of `parity.rs`.
mod common;
use common::dash_data::{CASES, DashCase, DashShape};
use common::parity_data::{CASES as PARITY_CASES, EXTRA};
use common::*;
use std::ops::Range;
use vexel_raster::Tile;
use vexel_render::{
    DashedShape, FlatEdge, Linecap, Linejoin, PreserveAspect, RenderShape, StrokeStyle, ViewBox,
    ViewMap, render_dashed_shapes, render_shapes,
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
    data: &'static DashShape,
}

/// Forward element transform from C `shape_stroke_inverse`.
fn forward(m: [f64; 6]) -> vexel_geometry::Affine {
    let det = m[0] * m[3] - m[1] * m[2];
    let r = 1.0 / det;
    let (a, b, c, d) = (m[3] * r, -m[1] * r, -m[2] * r, m[0] * r);
    vexel_geometry::Affine {
        a,
        b,
        c,
        d,
        e: -(a * m[4] + c * m[5]),
        f: -(b * m[4] + d * m[5]),
    }
}

fn cap(value: i32) -> Linecap {
    [Linecap::Butt, Linecap::Square, Linecap::Round][value as usize]
}

fn join(value: i32) -> Linejoin {
    [Linejoin::Miter, Linejoin::Bevel, Linejoin::Round][value as usize]
}

fn build(case: &'static DashCase, shift_x: f64) -> Vec<Built> {
    case.shapes
        .iter()
        .map(|data| Built {
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
        })
        .collect()
}

fn dashed(built: &[Built], use_dashes: bool) -> Vec<DashedShape<'_>> {
    built
        .iter()
        .map(|b| DashedShape {
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
                    dash_offset: b.data.dash_offset,
                    cap: cap(b.data.cap),
                    join: join(b.data.join),
                    crisp_edges: false,
                }),
                transform: b.transform,
            },
            dashes: if use_dashes { b.data.dashes } else { &[] },
        })
        .collect()
}

fn view_for(case: &DashCase) -> ViewMap {
    let aspect = case.aspect_set.then_some(PreserveAspect {
        align_x: case.align[0],
        align_y: case.align[1],
        none: case.aspect_none,
        slice: case.aspect_slice,
    });
    ViewMap::from_viewbox(
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
    .unwrap()
}

fn render_case(case: &'static DashCase, built: &[Built], use_dashes: bool) -> Tile {
    let mut tile = empty_tile(case.width, case.height);
    render_dashed_shapes(&dashed(built, use_dashes), view_for(case), &mut tile, NEVER)
        .expect("render");
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

fn fixture_path(name: &str) -> String {
    format!(
        "{}/../../fixtures/render-dash/{name}",
        env!("CARGO_MANIFEST_DIR")
    )
}

fn load_ppm(id: &str, width: u32, height: u32) -> Vec<u8> {
    let path = fixture_path(&format!("{id}.ppm"));
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

fn near_edge(built: &[Built], x: u32, y: u32) -> bool {
    let (cx, cy) = (f64::from(x) + 0.5, f64::from(y) + 0.5);
    built.iter().any(|b| {
        let extent = if b.data.stroke[3] != 0 {
            b.data.stroke_width * 0.5 * b.data.miter_limit.max(1.5)
        } else {
            0.0
        };
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

fn compare(case: &DashCase, built: &[Built], actual: &[u8], golden: &[u8]) -> Report {
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

fn run(id: &str) -> Report {
    let case = CASES.iter().find(|c| c.id == id).expect("case");
    let built = build(case, 0.0);
    let tile = render_case(case, &built, true);
    let golden = load_ppm(id, case.width, case.height);
    let report = compare(case, &built, &over_white(&tile), &golden);
    println!(
        "DASH PARITY {id}: pixels={} max_delta={} differing={} over_two={} outside_dilation={}",
        report.total, report.max_delta, report.differing, report.over_two, report.outside_dilation
    );
    report
}

macro_rules! dash_case {
    ($name:ident, $id:expr) => {
        #[test]
        fn $name() {
            let report = run($id);
            assert!(report.passes(), "{}: {report:?}", $id);
        }
    };
}

dash_case!(stroke_dashes, "stroke-dashes");
dash_case!(stroke_dash_join, "stroke-dash-join");
dash_case!(stroke_dash_closed_seam, "stroke-dash-closed-seam");
dash_case!(dashed_cubic, "dashed-cubic");
dash_case!(dash_dots, "dash-dots");

#[test]
fn dashes_change_the_picture_and_a_shift_fails_the_harness() {
    for case in CASES {
        let built = build(case, 0.0);
        let with = over_white(&render_case(case, &built, true));
        let without = over_white(&render_case(case, &built, false));
        assert_ne!(with, without, "{}: dashes had no effect", case.id);
        let golden = load_ppm(case.id, case.width, case.height);
        let solid = compare(case, &built, &without, &golden);
        assert!(
            !solid.passes(),
            "{}: the solid render must not match the dashed golden",
            case.id
        );
    }
    let case = CASES.iter().find(|c| c.id == "dashed-cubic").unwrap();
    let golden = load_ppm(case.id, case.width, case.height);
    let shifted = build(case, 0.5);
    let bad = compare(
        case,
        &shifted,
        &over_white(&render_case(case, &shifted, true)),
        &golden,
    );
    println!("DASH PARITY negative (dashed-cubic shifted +0.5px): {bad:?}");
    assert!(
        !bad.passes(),
        "a half-pixel shift must fail the harness: {bad:?}"
    );
}

/// AC3: empty dashes render byte-identically to `render_shapes` on every
/// existing parity case.
#[test]
fn empty_dashes_match_render_shapes_on_every_parity_case() {
    let mut checked = 0;
    for case in PARITY_CASES.iter().chain(EXTRA) {
        let built: Vec<_> = case
            .shapes
            .iter()
            .map(|data| {
                (
                    forward(data.transform_inverse),
                    data.edges
                        .iter()
                        .map(|e| FlatEdge {
                            x0: e[0],
                            y0: e[1],
                            x1: e[2],
                            y1: e[3],
                        })
                        .collect::<Vec<_>>(),
                    data.subpaths.iter().map(|s| s.0..s.1).collect::<Vec<_>>(),
                    data.subpaths.iter().map(|s| s.2).collect::<Vec<_>>(),
                    data,
                )
            })
            .collect();
        let shapes: Vec<RenderShape<'_>> = built
            .iter()
            .map(|(transform, edges, ranges, closed, data)| RenderShape {
                edges,
                subpaths: ranges,
                subpath_closed: closed,
                stroke_closed: data.stroke_closed,
                evenodd: data.evenodd,
                filled: data.filled,
                fill_rgba: data.fill,
                stroke: Some(StrokeStyle {
                    rgba: data.stroke,
                    width: data.stroke_width,
                    miter_limit: data.miter_limit,
                    dash_offset: 0.0,
                    cap: cap(data.cap),
                    join: join(data.join),
                    crisp_edges: false,
                }),
                transform: *transform,
            })
            .collect();
        let aspect = case.aspect_set.then_some(PreserveAspect {
            align_x: case.align[0],
            align_y: case.align[1],
            none: case.aspect_none,
            slice: case.aspect_slice,
        });
        let view = || {
            ViewMap::from_viewbox(
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
            .unwrap()
        };
        let mut plain = empty_tile(case.width, case.height);
        render_shapes(&shapes, view(), &mut plain, NEVER).expect("render_shapes");
        let items: Vec<DashedShape<'_>> = shapes
            .iter()
            .map(|shape| DashedShape {
                shape: *shape,
                dashes: &[],
            })
            .collect();
        let mut through_dashed = empty_tile(case.width, case.height);
        render_dashed_shapes(&items, view(), &mut through_dashed, NEVER).expect("render_dashed");
        assert_eq!(plain.rgba(), through_dashed.rgba(), "{}", case.id);
        checked += 1;
    }
    assert_eq!(checked, PARITY_CASES.len() + EXTRA.len());
    assert!(checked >= 7);
}

#[test]
fn dots_draw_with_round_cap_and_nothing_with_butt() {
    let edges = polyline(&[(2.0, 8.0), (30.0, 8.0)]);
    let render_dots = |cap: Linecap| {
        let shape = stroke_shape(
            &edges,
            false,
            style([0, 0, 0, 255], 4.0, cap, Linejoin::Miter, 4.0),
        );
        let mut tile = empty_tile(32, 16);
        let view = ViewMap::from_viewbox(
            ViewBox {
                x: 0.0,
                y: 0.0,
                width: 32.0,
                height: 16.0,
            },
            32,
            16,
            None,
        )
        .unwrap();
        let item = DashedShape {
            shape,
            dashes: &[0.0, 10.0],
        };
        render_dashed_shapes(&[item], view, &mut tile, NEVER).expect("render");
        tile
    };
    let round = render_dots(Linecap::Round);
    // Dots at x = 2, 12, 22 (and none at 32: the line ends at 30).
    for x in [2, 12, 22] {
        assert_eq!(pixel(&round, x, 8)[3], 255, "dot centre at {x}");
    }
    assert_eq!(pixel(&round, 7, 8)[3], 0, "gap between dots");
    assert_eq!(pixel(&round, 17, 8)[3], 0, "gap between dots");
    let butt = render_dots(Linecap::Butt);
    assert!(
        butt.rgba().iter().all(|b| *b == 0),
        "butt caps draw nothing for zero-length dashes"
    );
}

#[test]
fn seventeen_dash_values_is_a_limit_error_and_nan_renders_solid() {
    let edges = polyline(&[(2.0, 8.0), (30.0, 8.0)]);
    let shape = stroke_shape(
        &edges,
        false,
        style([0, 0, 0, 255], 4.0, Linecap::Butt, Linejoin::Miter, 4.0),
    );
    let try_dashes = |dashes: &[f64]| {
        let mut tile = empty_tile(32, 16);
        let view = ViewMap::from_viewbox(
            ViewBox {
                x: 0.0,
                y: 0.0,
                width: 32.0,
                height: 16.0,
            },
            32,
            16,
            None,
        )
        .unwrap();
        render_dashed_shapes(&[DashedShape { shape, dashes }], view, &mut tile, NEVER).map(|_| tile)
    };
    assert_eq!(
        try_dashes(&[1.0; 17]).map(|_| ()),
        Err(vexel_render::RenderError::LimitExceeded)
    );
    let solid = try_dashes(&[]).unwrap();
    assert_eq!(try_dashes(&[f64::NAN]).unwrap().rgba(), solid.rgba());
    assert_eq!(try_dashes(&[-1.0, 2.0]).unwrap().rgba(), solid.rgba());
    assert_eq!(try_dashes(&[0.0, 0.0]).unwrap().rgba(), solid.rgba());
    assert_ne!(try_dashes(&[4.0, 4.0]).unwrap().rgba(), solid.rgba());
}

// ---- manifest ----------------------------------------------------------------

const K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

fn sha256_hex(data: &[u8]) -> String {
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let mut message = data.to_vec();
    message.push(0x80);
    while message.len() % 64 != 56 {
        message.push(0);
    }
    message.extend_from_slice(&((data.len() as u64) * 8).to_be_bytes());
    for block in message.chunks_exact(64) {
        let mut w = [0u32; 64];
        for (i, word) in block.chunks_exact(4).enumerate() {
            w[i] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let mut v = h;
        for i in 0..64 {
            let s1 = v[4].rotate_right(6) ^ v[4].rotate_right(11) ^ v[4].rotate_right(25);
            let ch = (v[4] & v[5]) ^ (!v[4] & v[6]);
            let t1 = v[7]
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = v[0].rotate_right(2) ^ v[0].rotate_right(13) ^ v[0].rotate_right(22);
            let maj = (v[0] & v[1]) ^ (v[0] & v[2]) ^ (v[1] & v[2]);
            let t2 = s0.wrapping_add(maj);
            v = [
                t1.wrapping_add(t2),
                v[0],
                v[1],
                v[2],
                v[3].wrapping_add(t1),
                v[4],
                v[5],
                v[6],
            ];
        }
        for (slot, value) in h.iter_mut().zip(v) {
            *slot = slot.wrapping_add(value);
        }
    }
    h.iter().map(|word| format!("{word:08x}")).collect()
}

/// Value of `"key": "<value>"` at or after `from`.
fn string_after<'a>(text: &'a str, from: usize, key: &str) -> (&'a str, usize) {
    let marker = format!("\"{key}\": \"");
    let start = text[from..]
        .find(&marker)
        .unwrap_or_else(|| panic!("manifest key {key}"))
        + from;
    let value_start = start + marker.len();
    let end = text[value_start..].find('"').unwrap() + value_start;
    (&text[value_start..end], end)
}

#[test]
fn sha256_matches_the_known_vector() {
    assert_eq!(
        sha256_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn manifest_hashes_match_the_fixture_files() {
    let manifest = std::fs::read_to_string(fixture_path("manifest.json")).unwrap();
    for key in [
        "legacy_tag",
        "legacy_commit",
        "binary_sha256",
        "cmake_version",
        "compiler",
    ] {
        let (value, _) = string_after(&manifest, 0, key);
        assert!(!value.is_empty(), "{key}");
    }
    assert_eq!(string_after(&manifest, 0, "legacy_commit").0.len(), 40);
    assert_eq!(string_after(&manifest, 0, "binary_sha256").0.len(), 64);
    assert!(manifest.contains("--render-animation <fixture.svg> --fps 1 --duration 1 --out <dir>"));
    for case in CASES {
        let at = manifest
            .find(&format!("\"fixture_id\": \"{}\"", case.id))
            .unwrap_or_else(|| panic!("{} missing from the manifest", case.id));
        let (source, next) = string_after(&manifest, at, "source_sha256");
        let (ppm, _) = string_after(&manifest, next, "ppm_sha256");
        let svg = std::fs::read(fixture_path(&format!("{}.svg", case.id))).unwrap();
        let golden = std::fs::read(fixture_path(&format!("{}.ppm", case.id))).unwrap();
        assert_eq!(sha256_hex(&svg), source, "{} svg", case.id);
        assert_eq!(sha256_hex(&golden), ppm, "{} ppm", case.id);
        assert!(
            manifest[at..].contains(&format!("\"width\": {}", case.width)),
            "{}",
            case.id
        );
        assert!(
            manifest[at..].contains(&format!("\"height\": {}", case.height)),
            "{}",
            case.id
        );
    }
}
