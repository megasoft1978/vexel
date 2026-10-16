//! Pixel parity of the filter module against C `svgworkbench` goldens
//! (`rust/fixtures/render-filter`, generated outside the repo from the frozen
//! legacy binary; see that directory's `manifest.json`).
//!
//! Each fixture is a filtered group of three solid shapes. The Rust layer is
//! the `render_shapes` output (not filtered), the filter chain is applied to it
//! and the layer is composited over white exactly as the C `write_tile_ppm`.
mod common;
#[path = "common/filter_data.rs"]
mod filter_data;
#[path = "common/sha256.rs"]
mod sha256;

use common::{NEVER, empty_tile, fill_shape};
use filter_data::{Case, HEIGHT, SCENE, WIDTH, cases, edges};
use vexel_render::filter::apply_filter_chain;
use vexel_render::{ViewBox, ViewMap, render_shapes};
use vexel_scene::{Filter, TableRange};

/// Work-order bounds; they are fixed here and must not be raised.
mod tolerance {
    pub const MAX_CHANNEL_DELTA: i32 = 1;
    pub const MAX_DIFFERING_FRACTION: f64 = 0.005;
    /// `feGaussianBlur` / `feDropShadow`.
    pub const BLUR_MAX_DELTA: i32 = 2;
    pub const BLUR_NO_PIXEL_OVER: i32 = 3;
}

fn fixture_dir() -> String {
    format!(
        "{}/../../fixtures/render-filter",
        env!("CARGO_MANIFEST_DIR")
    )
}

fn view() -> ViewMap {
    ViewMap::from_viewbox(
        ViewBox {
            x: 0.0,
            y: 0.0,
            width: f64::from(WIDTH),
            height: f64::from(HEIGHT),
        },
        WIDTH,
        HEIGHT,
        None,
    )
    .unwrap()
}

fn filter_of(case: &Case) -> Filter {
    Filter {
        primitives: TableRange {
            first: 0,
            count: case.primitives.len(),
        },
        linear_rgb: true,
        filter_units_object_bounding_box: case.units_bbox,
        primitive_units_object_bounding_box: case.primitive_units_bbox,
        custom_region: case.custom_region,
        region: case.region,
    }
}

/// The unfiltered group layer (straight RGBA8).
fn scene_layer() -> Vec<u8> {
    let edge_sets: Vec<_> = SCENE.iter().map(edges).collect();
    let shapes: Vec<_> = SCENE
        .iter()
        .zip(&edge_sets)
        .map(|(s, e)| fill_shape(e, s.fill, false))
        .collect();
    let mut tile = empty_tile(WIDTH, HEIGHT);
    render_shapes(&shapes, view(), &mut tile, NEVER).expect("render");
    tile.rgba().to_vec()
}

/// C `write_tile_ppm`: `min(255, rgb + 255 - alpha)`.
fn over_white(rgba: &[u8]) -> Vec<u8> {
    rgba.chunks_exact(4)
        .flat_map(|p| {
            let a = i32::from(p[3]);
            (0..3).map(move |c| (i32::from(p[c]) + 255 - a).min(255) as u8)
        })
        .collect()
}

fn load_ppm(id: &str) -> Vec<u8> {
    let path = format!("{}/{id}.ppm", fixture_dir());
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("golden {path}: {e}"));
    let header = format!("P6\n{WIDTH} {HEIGHT}\n255\n");
    assert!(
        bytes.starts_with(header.as_bytes()),
        "{id}: unexpected PPM header"
    );
    bytes[header.len()..].to_vec()
}

#[derive(Debug)]
struct Report {
    total: usize,
    max_delta: i32,
    differing: usize,
    over_three: usize,
    outside_dilation: usize,
}

/// True when pixel `(x, y)` is within one pixel of a non-white golden pixel.
fn near_content(golden: &[u8], x: i32, y: i32) -> bool {
    for dy in -1..=1 {
        for dx in -1..=1 {
            let (nx, ny) = (x + dx, y + dy);
            if nx < 0 || ny < 0 || nx >= WIDTH as i32 || ny >= HEIGHT as i32 {
                continue;
            }
            let at = (ny as usize * WIDTH as usize + nx as usize) * 3;
            if golden[at..at + 3] != [255, 255, 255] {
                return true;
            }
        }
    }
    false
}

fn compare(actual: &[u8], golden: &[u8]) -> Report {
    let mut report = Report {
        total: golden.len() / 3,
        max_delta: 0,
        differing: 0,
        over_three: 0,
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
            if delta > tolerance::BLUR_NO_PIXEL_OVER {
                report.over_three += 1;
            }
            let (x, y) = (
                (index % WIDTH as usize) as i32,
                (index / WIDTH as usize) as i32,
            );
            if !near_content(golden, x, y) {
                report.outside_dilation += 1;
            }
        }
    }
    report
}

fn is_blur_class(case: &Case) -> bool {
    use vexel_scene::FilterKind::{DropShadow, GaussianBlur};
    case.primitives
        .iter()
        .any(|p| matches!(p.kind, GaussianBlur | DropShadow))
}

fn run(case: &Case) -> Report {
    let mut layer = scene_layer();
    let filter = filter_of(case);
    apply_filter_chain(
        &mut layer,
        &view(),
        &filter,
        &case.primitives,
        64 << 20,
        NEVER,
    )
    .unwrap_or_else(|e| panic!("{}: {e:?}", case.id));
    let golden = load_ppm(case.id);
    let report = compare(&over_white(&layer), &golden);
    println!(
        "{}: max_delta={} differing={}/{} over_three={} outside_dilation={} exact={}",
        case.id,
        report.max_delta,
        report.differing,
        report.total,
        report.over_three,
        report.outside_dilation,
        report.differing == 0
    );
    report
}

#[test]
fn every_fixture_matches_the_c_golden() {
    let mut failures = Vec::new();
    let mut exact = 0;
    let all = cases();
    for case in &all {
        let report = run(case);
        if report.differing == 0 {
            exact += 1;
        }
        let ok = if is_blur_class(case) {
            report.max_delta <= tolerance::BLUR_MAX_DELTA && report.over_three == 0
        } else {
            report.max_delta <= tolerance::MAX_CHANNEL_DELTA
                && (report.differing as f64)
                    <= tolerance::MAX_DIFFERING_FRACTION * report.total as f64
        };
        if !ok || report.outside_dilation != 0 {
            failures.push(format!("{}: {report:?}", case.id));
        }
    }
    println!("exact equality: {exact}/{} fixtures", all.len());
    assert!(
        failures.is_empty(),
        "parity failures:\n{}",
        failures.join("\n")
    );
}

#[test]
fn manifest_hashes_match_the_files() {
    let dir = fixture_dir();
    let manifest = std::fs::read_to_string(format!("{dir}/manifest.json")).expect("manifest");
    let mut checked = 0;
    for case in cases() {
        let entry_start = manifest
            .find(&format!("\"fixture_id\": \"{}\"", case.id))
            .unwrap_or_else(|| panic!("{} missing from manifest", case.id));
        let entry = &manifest[entry_start..];
        let entry = &entry[..entry.find('}').expect("entry end")];
        for (key, file) in [("source_sha256", "svg"), ("ppm_sha256", "ppm")] {
            let marker = format!("\"{key}\": \"");
            let at = entry
                .find(&marker)
                .unwrap_or_else(|| panic!("{} lacks {key}", case.id))
                + marker.len();
            let expected = &entry[at..at + 64];
            let bytes = std::fs::read(format!("{dir}/{}.{file}", case.id)).expect("fixture file");
            assert_eq!(sha256::sha256_hex(&bytes), expected, "{}.{file}", case.id);
        }
        checked += 1;
    }
    assert_eq!(checked, cases().len());
    for field in [
        "legacy_tag_commit",
        "binary_sha256",
        "cmake_version",
        "compiler",
        "macos",
    ] {
        assert!(
            manifest.contains(&format!("\"{field}\"")),
            "manifest lacks {field}"
        );
    }
}

#[test]
fn sha256_known_answer() {
    assert_eq!(
        sha256::sha256_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}
