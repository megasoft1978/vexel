//! Pixel parity of `render_groups` against C `svgworkbench` PPM goldens in
//! rust/fixtures/render-mask (opacity groups, clip groups, masks), plus the
//! manifest hash check and AC3 (trivial groups equal the ungrouped path).
//! Tolerances are copied unchanged from tests/parity.rs.
mod common;
#[path = "common/group_build.rs"]
mod group_build;
#[path = "common/group_cases_a.rs"]
mod group_cases_a;
#[path = "common/group_cases_b.rs"]
mod group_cases_b;
#[path = "common/group_cases_c.rs"]
mod group_cases_c;
#[path = "common/group_data.rs"]
mod group_data;

use common::*;
use group_build::{affine, forward, tree};
use group_data::{CASES, Case, DocData};
use vexel_raster::Tile;
use vexel_render::{RenderGroup, RenderNode, ViewBox, ViewMap, render_groups, render_shapes};

mod tolerance {
    pub const MAX_CHANNEL_DELTA: i32 = 1;
    pub const MAX_DIFFERING_FRACTION: f64 = 0.005;
    pub const MAX_ANY_DELTA: i32 = 2;
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

fn fixtures_dir() -> String {
    format!("{}/../../fixtures/render-mask", env!("CARGO_MANIFEST_DIR"))
}

fn view(case: &Case) -> ViewMap {
    ViewMap::from_viewbox(
        ViewBox {
            x: case.view[0],
            y: case.view[1],
            width: case.view[2],
            height: case.view[3],
        },
        case.width,
        case.height,
        None,
    )
    .unwrap()
}

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
    let path = format!("{}/{id}.ppm", fixtures_dir());
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

type Segment = ((f64, f64), (f64, f64), f64);

/// Device-space boundary segments with the stroke half-width that may add coverage.
fn segments(case: &Case) -> Vec<Segment> {
    let v = view(case);
    let scale = f64::from(case.width) / case.view[2];
    let scale = scale.min(f64::from(case.height) / case.view[3]);
    let _ = v;
    let offset = (
        (f64::from(case.width) - case.view[2] * scale) * 0.5 - case.view[0] * scale,
        (f64::from(case.height) - case.view[3] * scale) * 0.5 - case.view[1] * scale,
    );
    let dev = move |x: f64, y: f64| (offset.0 + x * scale, offset.1 + y * scale);
    let mut out = Vec::new();
    doc_segments(&case.doc, &dev, &mut out);
    for clip in case.clips {
        let to_doc = forward(clip.user_inverse);
        for e in clip.edges {
            let (mut a, mut b) = ((e[0], e[1]), (e[2], e[3]));
            if clip.object_bbox {
                let map = |p: (f64, f64)| {
                    (
                        clip.bounds[0] + p.0 * (clip.bounds[2] - clip.bounds[0]),
                        clip.bounds[1] + p.1 * (clip.bounds[3] - clip.bounds[1]),
                    )
                };
                (a, b) = (map(a), map(b));
            }
            let m = |p: (f64, f64)| {
                dev(
                    to_doc.a * p.0 + to_doc.c * p.1 + to_doc.e,
                    to_doc.b * p.0 + to_doc.d * p.1 + to_doc.f,
                )
            };
            out.push((m(a), m(b), 0.0));
        }
    }
    out
}

fn doc_segments(doc: &DocData, dev: &dyn Fn(f64, f64) -> (f64, f64), out: &mut Vec<Segment>) {
    for s in doc.shapes {
        let extent = if s.stroke[3] != 0 {
            s.stroke_width * 0.5 * s.miter_limit.max(1.5)
        } else {
            0.0
        };
        for e in s.edges {
            out.push((dev(e[0], e[1]), dev(e[2], e[3]), extent));
        }
    }
    for m in doc.masks {
        let [x, y, w, h] = m.region;
        let t = affine(m.region_transform);
        let pts = [(x, y), (x + w, y), (x + w, y + h), (x, y + h)]
            .map(|(px, py)| dev(t.a * px + t.c * py + t.e, t.b * px + t.d * py + t.f));
        for i in 0..4 {
            out.push((pts[i], pts[(i + 1) % 4], 0.0));
        }
        doc_segments(&m.content, dev, out);
    }
}

fn near_edge(segs: &[Segment], x: u32, y: u32) -> bool {
    let (cx, cy) = (f64::from(x) + 0.5, f64::from(y) + 0.5);
    segs.iter().any(|(a, b, extent)| {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let len2 = dx * dx + dy * dy;
        let t = if len2 == 0.0 {
            0.0
        } else {
            (((cx - a.0) * dx + (cy - a.1) * dy) / len2).clamp(0.0, 1.0)
        };
        (cx - (a.0 + t * dx)).hypot(cy - (a.1 + t * dy))
            <= extent + tolerance::DILATION_PIXELS + 0.75
    })
}

fn compare(case: &Case, actual: &[u8], golden: &[u8]) -> Report {
    let segs = segments(case);
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
            if delta > tolerance::MAX_ANY_DELTA {
                report.over_two += 1;
            }
            let (x, y) = (
                (index % case.width as usize) as u32,
                (index / case.width as usize) as u32,
            );
            if !near_edge(&segs, x, y) {
                report.outside_dilation += 1;
            }
        }
    }
    report
}

fn render_case(case: &'static Case) -> Tile {
    let mut tile = empty_tile(case.width, case.height);
    render_groups(tree(case), view(case), &mut tile, NEVER).expect("render_groups");
    tile
}

fn run(id: &str) -> Report {
    let case = CASES.iter().copied().find(|c| c.id == id).expect("case");
    let golden = load_ppm(id, case.width, case.height);
    let report = compare(case, &over_white(&render_case(case)), &golden);
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

parity_case!(group_opacity, "group-opacity");
parity_case!(nested_opacity, "nested-opacity");
parity_case!(clip_group_object_bbox, "clip-group-object-bbox");
parity_case!(clip_group_userspace, "clip-group-userspace");
parity_case!(mask_luminance, "mask-luminance");
parity_case!(mask_alpha, "mask-alpha");
parity_case!(mask_userspace, "mask-userspace");
parity_case!(mask_transformed_group, "mask-transformed-group");
parity_case!(mask_opacity_combined, "mask-opacity-combined");

#[test]
fn harness_rejects_a_shifted_mask_region() {
    let case = CASES
        .iter()
        .copied()
        .find(|c| c.id == "mask-userspace")
        .unwrap();
    let golden = load_ppm(case.id, case.width, case.height);
    let good = compare(case, &over_white(&render_case(case)), &golden);
    assert!(good.passes(), "{good:?}");
    // Render the golden of a different fixture against this case: must fail.
    let other = load_ppm("mask-alpha", case.width, case.height);
    let bad = compare(case, &over_white(&render_case(case)), &other);
    assert!(!bad.passes(), "{bad:?}");
}

/// AC3: wrapping every existing parity case in groups of opacity 1 (no clip,
/// no mask), nested, is byte-identical to the ungrouped `render_shapes` path.
#[test]
fn trivial_groups_equal_ungrouped_render_for_every_existing_parity_case() {
    use common::parity_data::{CASES as OLD, EXTRA};
    for case in OLD.iter().chain(EXTRA) {
        let built: Vec<_> = case
            .shapes
            .iter()
            .map(|d| {
                let edges: Vec<vexel_render::FlatEdge> = d
                    .edges
                    .iter()
                    .map(|e| vexel_render::FlatEdge {
                        x0: e[0],
                        y0: e[1],
                        x1: e[2],
                        y1: e[3],
                    })
                    .collect();
                let ranges: Vec<_> = d.subpaths.iter().map(|s| s.0..s.1).collect();
                let closed: Vec<bool> = d.subpaths.iter().map(|s| s.2).collect();
                (d, edges, ranges, closed)
            })
            .collect();
        let shapes: Vec<_> = built
            .iter()
            .map(|(d, edges, ranges, closed)| vexel_render::RenderShape {
                edges,
                subpaths: ranges,
                subpath_closed: closed,
                stroke_closed: d.stroke_closed,
                evenodd: d.evenodd,
                filled: d.filled,
                fill_rgba: d.fill,
                stroke: Some(vexel_render::StrokeStyle {
                    rgba: d.stroke,
                    width: d.stroke_width,
                    miter_limit: d.miter_limit,
                    dash_offset: 0.0,
                    cap: [
                        vexel_render::Linecap::Butt,
                        vexel_render::Linecap::Square,
                        vexel_render::Linecap::Round,
                    ][d.cap as usize],
                    join: [
                        vexel_render::Linejoin::Miter,
                        vexel_render::Linejoin::Bevel,
                        vexel_render::Linejoin::Round,
                    ][d.join as usize],
                    crisp_edges: false,
                }),
                transform: forward(d.transform_inverse),
            })
            .collect();
        let aspect = case.aspect_set.then_some(vexel_render::PreserveAspect {
            align_x: case.align[0],
            align_y: case.align[1],
            none: case.aspect_none,
            slice: case.aspect_slice,
        });
        let vm = ViewMap::from_viewbox(
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
        let mut plain = empty_tile(case.width, case.height);
        render_shapes(&shapes, vm, &mut plain, NEVER).unwrap();

        let nodes: Vec<RenderNode<'_>> = shapes
            .iter()
            .map(|shape| RenderNode::Shape {
                shape: *shape,
                clip: None,
            })
            .collect();
        let (head, tail) = nodes.split_at(nodes.len() / 2);
        let inner = [RenderNode::Group(RenderGroup {
            opacity: 1.0,
            clip: None,
            mask: None,
            children: tail,
        })];
        let outer = [
            RenderNode::Group(RenderGroup {
                opacity: 1.0,
                clip: None,
                mask: None,
                children: head,
            }),
            RenderNode::Group(RenderGroup {
                opacity: 1.0,
                clip: None,
                mask: None,
                children: &inner,
            }),
        ];
        let mut grouped = empty_tile(case.width, case.height);
        render_groups(&outer, vm, &mut grouped, NEVER).unwrap();
        assert_eq!(plain.rgba(), grouped.rgba(), "{}", case.id);
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    // Dependency-free SHA-256 (FIPS 180-4) so the manifest hashes can be verified offline.
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let mut data = bytes.to_vec();
    data.push(0x80);
    while data.len() % 64 != 56 {
        data.push(0);
    }
    data.extend_from_slice(&((bytes.len() as u64) * 8).to_be_bytes());
    for block in data.chunks_exact(64) {
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
        for i in 0..8 {
            h[i] = h[i].wrapping_add(v[i]);
        }
    }
    h.iter().map(|x| format!("{x:08x}")).collect()
}

#[test]
fn sha256_helper_matches_known_vector() {
    assert_eq!(
        sha256_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn manifest_records_provenance_and_every_hash_matches_its_file() {
    let dir = fixtures_dir();
    let manifest = std::fs::read_to_string(format!("{dir}/manifest.json")).unwrap();
    for key in [
        "legacy_tag_ref",
        "binary_sha256",
        "build_commands",
        "toolchain",
        "\"cmake\"",
        "\"macos\"",
    ] {
        assert!(manifest.contains(key), "manifest lacks {key}");
    }
    for case in CASES {
        let entry = manifest
            .split("\"fixture_id\": \"")
            .find(|part| part.starts_with(case.id) && part[case.id.len()..].starts_with('"'))
            .unwrap_or_else(|| panic!("{} not in manifest", case.id));
        let field = |name: &str| {
            let at = entry.find(&format!("\"{name}\": \"")).unwrap() + name.len() + 5;
            entry[at..].split('"').next().unwrap().to_owned()
        };
        let svg = std::fs::read(format!("{dir}/{}.svg", case.id)).unwrap();
        let ppm = std::fs::read(format!("{dir}/{}.ppm", case.id)).unwrap();
        assert_eq!(
            sha256_hex(&svg),
            field("source_sha256"),
            "{} source",
            case.id
        );
        assert_eq!(sha256_hex(&ppm), field("ppm_sha256"), "{} ppm", case.id);
    }
}
