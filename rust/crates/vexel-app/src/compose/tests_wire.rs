//! WO-RUST-SVG-WIRE-01: mask, group, pattern, filter, text, dash and a combined
//! document composed through the app pipeline and compared with PPM goldens
//! made by the frozen legacy C `svgworkbench` (`rust/fixtures/v1/wire`).
//!
//! Tolerances are those of the renderer orders, copied unchanged: max channel
//! delta 1, no delta above 2, at most 0.5% differing pixels, and no differing
//! pixel farther than one pixel from painted golden content. Blur-class
//! fixtures (`feGaussianBlur`/`feDropShadow`) use the filter order's bound:
//! max delta 2 and no delta above 3.
use super::*;
use crate::compose::tests::ppm_over_white;
use std::sync::atomic::{AtomicUsize, Ordering};

#[path = "../../../vexel-render/tests/common/sha256.rs"]
mod sha256;

const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/v1/wire");

/// (fixture id, width, height, blur class, needs fonts)
const FIXTURES: [(&str, u32, u32, bool, bool); 9] = [
    ("wire-mask", 64, 48, false, false),
    ("wire-group", 64, 48, false, false),
    ("wire-pattern", 96, 64, false, false),
    ("wire-pattern-group", 96, 64, false, false),
    ("wire-filter", 96, 64, true, false),
    ("wire-text", 160, 80, false, true),
    ("wire-dash", 64, 48, false, false),
    ("wire-filter-mask", 64, 48, true, false),
    ("wire-combined", 128, 96, true, true),
];

fn read(name: &str) -> Vec<u8> {
    std::fs::read(format!("{DIR}/{name}")).unwrap_or_else(|e| panic!("{name}: {e}"))
}

#[derive(Debug)]
struct Report {
    total: usize,
    max_delta: i32,
    differing: usize,
    over_limit: usize,
    outside_dilation: usize,
}

fn near_content(golden: &[u8], width: usize, height: usize, x: usize, y: usize) -> bool {
    for ny in y.saturating_sub(1)..=(y + 1).min(height - 1) {
        for nx in x.saturating_sub(1)..=(x + 1).min(width - 1) {
            let at = (ny * width + nx) * 3;
            if golden[at..at + 3] != [255, 255, 255] {
                return true;
            }
        }
    }
    false
}

fn compare(actual: &[u8], golden: &[u8], width: usize, blur: bool) -> Report {
    assert_eq!(actual.len(), golden.len());
    let height = golden.len() / 3 / width;
    let any_limit = if blur { 3 } else { 2 };
    let mut report = Report {
        total: golden.len() / 3,
        max_delta: 0,
        differing: 0,
        over_limit: 0,
        outside_dilation: 0,
    };
    for index in 0..report.total {
        let delta = (0..3)
            .map(|c| (i32::from(actual[index * 3 + c]) - i32::from(golden[index * 3 + c])).abs())
            .max()
            .unwrap();
        report.max_delta = report.max_delta.max(delta);
        if delta == 0 {
            continue;
        }
        report.differing += 1;
        if delta > any_limit {
            report.over_limit += 1;
        }
        if !near_content(golden, width, height, index % width, index / width) {
            report.outside_dilation += 1;
        }
    }
    report
}

fn passes(report: &Report, blur: bool) -> bool {
    let channel = if blur { 2 } else { 1 };
    report.max_delta <= channel
        && report.over_limit == 0
        && report.outside_dilation == 0
        && (blur || (report.differing as f64) <= 0.005 * report.total as f64)
}

/// `Some(reason)` when a font recorded in the manifest is not installed.
fn font_precondition() -> Option<String> {
    let manifest = String::from_utf8(read("manifest.json")).unwrap();
    for line in manifest
        .lines()
        .filter(|l| l.trim_start().starts_with("\"fonts\""))
    {
        for part in line.split('"').filter(|p| p.starts_with("/System/")) {
            if !std::path::Path::new(part).exists() {
                return Some(format!("recorded font file {part} is not installed"));
            }
        }
    }
    None
}

fn run(id: &str, width: u32, height: u32, blur: bool, fonts: bool) -> Option<Report> {
    if fonts && let Some(reason) = font_precondition() {
        println!("SKIP {id}: {reason}");
        return None;
    }
    let composed = compose_svg(&read(&format!("{id}.svg")), width, height)
        .unwrap_or_else(|e| panic!("{id}: {e}"));
    assert!(
        composed.disclosures.is_empty(),
        "{id}: {:?}",
        composed.disclosures
    );
    let golden = read(&format!("{id}.ppm"));
    let header = format!("P6\n{width} {height}\n255\n").into_bytes();
    assert!(golden.starts_with(&header), "{id}: golden header");
    let actual = ppm_over_white(&composed);
    let report = compare(
        &actual[header.len()..],
        &golden[header.len()..],
        width as usize,
        blur,
    );
    println!(
        "PARITY {id}: pixels={} max_delta={} differing={} over_limit={} outside_dilation={}",
        report.total,
        report.max_delta,
        report.differing,
        report.over_limit,
        report.outside_dilation
    );
    Some(report)
}

#[test]
fn every_wire_fixture_composes_and_matches_its_c_golden() {
    let mut failures = Vec::new();
    for (id, width, height, blur, fonts) in FIXTURES {
        if let Some(report) = run(id, width, height, blur, fonts)
            && !passes(&report, blur)
        {
            failures.push(format!("{id}: {report:?}"));
        }
    }
    assert!(
        failures.is_empty(),
        "parity failures:\n{}",
        failures.join("\n")
    );
}

#[test]
fn the_parity_harness_rejects_a_shifted_render() {
    let svg = String::from_utf8(read("wire-pattern.svg")).unwrap();
    let shifted = svg.replace(
        "x=\"6\" y=\"20\" width=\"40\"",
        "x=\"9\" y=\"20\" width=\"40\"",
    );
    assert_ne!(shifted, svg);
    let composed = compose_svg(shifted.as_bytes(), 96, 64).unwrap();
    let golden = read("wire-pattern.ppm");
    let header = b"P6\n96 64\n255\n".len();
    let actual = ppm_over_white(&composed);
    let report = compare(&actual[header..], &golden[header..], 96, false);
    assert!(!passes(&report, false), "{report:?}");
}

#[test]
fn manifest_hashes_match_the_files() {
    let manifest = String::from_utf8(read("manifest.json")).unwrap();
    for field in [
        "legacy_tag_commit",
        "binary_sha256",
        "cmake_version",
        "compiler",
        "macos",
    ] {
        assert!(manifest.contains(&format!("\"{field}\"")), "{field}");
    }
    for (id, ..) in FIXTURES {
        let start = manifest
            .find(&format!("\"fixture_id\": \"{id}\""))
            .unwrap_or_else(|| panic!("{id} missing from manifest"));
        let entry = &manifest[start..];
        let entry = &entry[..entry.find('}').unwrap()];
        for (key, extension) in [("source_sha256", "svg"), ("ppm_sha256", "ppm")] {
            let marker = format!("\"{key}\": \"");
            let at = entry.find(&marker).unwrap() + marker.len();
            let expected = &entry[at..at + 64];
            let bytes = read(&format!("{id}.{extension}"));
            assert_eq!(sha256::sha256_hex(&bytes), expected, "{id}.{extension}");
        }
    }
}

fn composed(svg: &str, width: u32, height: u32) -> Result<Frame, ComposeError> {
    compose_svg(svg.as_bytes(), width, height)
}

fn wrap(body: &str) -> String {
    format!("<svg xmlns='http://www.w3.org/2000/svg' width='40' height='40'>{body}</svg>")
}

#[test]
fn unsupported_paint_on_clip_child_names_its_element_and_offset() {
    // A paint on a clipPath child is deferred; the offset falls inside the
    // attribute value, so the element must be recovered from the enclosing tag.
    let body = "<defs><clipPath id='c'><path d='M0 0H10V10Z' fill='#000000'/></clipPath></defs>\
         <rect width='30' height='30' clip-path='url(#c)'/>";
    let svg = wrap(body);
    let start = svg.find("<path").unwrap();
    let end = start + svg[start..].find("/>").unwrap();
    match composed(&svg, 40, 40) {
        Err(ComposeError::Unsupported {
            offset: Some(offset),
            element,
            ..
        }) => {
            assert!(
                (start..end).contains(&offset),
                "offset {offset} not in <path>"
            );
            assert_eq!(element.as_deref(), Some("path"));
        }
        other => panic!("expected Unsupported with offset and element, got {other:?}"),
    }
}

fn pixel(frame: &Frame, x: u32, y: u32) -> [u8; 4] {
    let at = (y as usize * frame.width as usize + x as usize) * 4;
    frame.rgba[at..at + 4].try_into().unwrap()
}

// ---- AC3: deferred features keep their typed disclosure ----------------------

#[test]
fn unsupported_filter_primitive_stays_a_typed_error() {
    let svg = wrap(
        "<defs><filter id='f'><feDisplacementMap scale='3'/></filter></defs>\
         <rect width='30' height='30' fill='red' filter='url(#f)'/>",
    );
    match composed(&svg, 40, 40) {
        Err(ComposeError::Unsupported {
            feature,
            offset: Some(_),
            element,
        }) => {
            assert_eq!(feature, "filter");
            assert_eq!(element.as_deref(), Some("feDisplacementMap"));
        }
        other => panic!("expected Unsupported(filter), got {other:?}"),
    }
}

#[test]
fn text_path_stays_a_typed_error() {
    let svg = wrap(
        "<defs><path id='p' d='M0 20 L40 20'/></defs>\
         <text font-size='10'><textPath href='#p'>Hi</textPath></text>",
    );
    match composed(&svg, 40, 40) {
        Err(ComposeError::Unsupported { feature, .. }) => assert_eq!(feature, "text"),
        other => panic!("expected Unsupported(text), got {other:?}"),
    }
}

#[test]
fn linear_gradient_fill_composes_to_a_gradient_pixel_run() {
    let svg = wrap(
        "<defs><linearGradient id='g'><stop offset='0' stop-color='red'/>\
         <stop offset='1' stop-color='blue'/></linearGradient></defs>\
         <rect width='10' height='10' fill='#00aa00'/>\
         <rect x='20' width='10' height='10' fill='url(#g)'/>",
    );
    let frame = composed(&svg, 40, 40).unwrap();
    assert!(
        frame.disclosures.is_empty(),
        "supported gradient is not disclosed"
    );
    assert_eq!(pixel(&frame, 5, 5), [0, 0xaa, 0, 255]);
    let run: Vec<[u8; 4]> = (20..30).map(|x| pixel(&frame, x, 5)).collect();
    assert!(run.iter().all(|p| p[3] == 255), "{run:?}");
    assert!(
        run.windows(2).any(|w| w[0] != w[1]),
        "gradient run is uniform: {run:?}"
    );
    // Red at the start of the box, blue at its end.
    assert!(run[0][0] > run[9][0] && run[0][2] < run[9][2], "{run:?}");
}

#[test]
fn image_stays_a_typed_error() {
    let svg = wrap("<image width='10' height='10' href='data:image/png;base64,iVBORw0KGgo='/>");
    match composed(&svg, 40, 40) {
        Err(ComposeError::Unsupported { feature, .. }) => assert_eq!(feature, "image"),
        other => panic!("expected Unsupported(image), got {other:?}"),
    }
}

#[test]
fn gradient_inside_a_pattern_tile_is_drawn_in_every_tile() {
    let svg = wrap(
        "<defs><linearGradient id='g'><stop offset='0' stop-color='red'/>\
         <stop offset='1' stop-color='blue'/></linearGradient>\
         <pattern id='p' patternUnits='userSpaceOnUse' width='8' height='8'>\
         <rect width='8' height='8' fill='url(#g)'/></pattern></defs>\
         <rect width='30' height='30' fill='url(#p)'/>",
    );
    let frame = composed(&svg, 40, 40).unwrap();
    assert!(frame.disclosures.is_empty(), "{:?}", frame.disclosures);
    // Each 8 px tile runs red to blue across its width, so the first and last
    // columns of a tile are the two ends of the ramp.
    for tile_x in [0, 8, 16] {
        let (start, end) = (pixel(&frame, tile_x, 4), pixel(&frame, tile_x + 7, 4));
        assert!(start[0] > 200 && start[2] < 40, "{start:?}");
        assert!(end[2] > 200 && end[0] < 40, "{end:?}");
    }
}

#[test]
fn clipped_pattern_shape_is_disclosed_not_dropped() {
    let svg = wrap(
        "<defs><clipPath id='c'><rect width='20' height='20'/></clipPath>\
         <pattern id='p' patternUnits='userSpaceOnUse' width='8' height='8'>\
         <rect width='4' height='4' fill='blue'/></pattern></defs>\
         <rect width='30' height='30' fill='url(#p)' clip-path='url(#c)'/>",
    );
    let frame = composed(&svg, 40, 40).unwrap();
    let features: Vec<_> = frame.disclosures.iter().map(|d| d.feature).collect();
    assert_eq!(features, ["clip path"]);
}

// ---- AC5: bounds, cancellation, stale frames --------------------------------

#[test]
fn oversize_filter_blur_returns_the_typed_limit_error() {
    let svg = wrap(
        "<defs><filter id='f' x='-5' y='-5' width='11' height='11'>\
         <feGaussianBlur stdDeviation='100'/></filter></defs>\
         <g filter='url(#f)'><rect width='20' height='20' fill='red'/></g>",
    );
    // The decoder rejects the oversize deviation before rendering, so this is a decode limit.
    assert_eq!(composed(&svg, 40, 40), Err(ComposeError::LimitDecode));
}

#[test]
fn oversize_filter_surface_returns_the_typed_limit_error() {
    let svg = "<svg xmlns='http://www.w3.org/2000/svg' width='3000' height='3000'>\
         <defs><filter id='f'><feGaussianBlur stdDeviation='1'/></filter></defs>\
         <g filter='url(#f)'><rect width='20' height='20' fill='red'/></g></svg>";
    assert_eq!(
        compose_svg(svg.as_bytes(), 3000, 3000),
        Err(ComposeError::LimitRender)
    );
}

#[test]
fn layer_memory_is_bounded_by_the_scratch_limit() {
    // A masked group holds its layer and the mask content layer at once; on a
    // 3000x3000 canvas that is 72 MB, over the 64 MiB group scratch bound.
    let svg = "<svg xmlns='http://www.w3.org/2000/svg' width='3000' height='3000'>\
         <defs><mask id='m'><rect width='3000' height='3000' fill='white'/></mask></defs>\
         <g mask='url(#m)'><rect width='20' height='20' fill='red'/></g></svg>";
    assert_eq!(
        compose_svg(svg.as_bytes(), 3000, 3000),
        Err(ComposeError::LimitRender)
    );
    // The same group on a small canvas is fine.
    let small = svg.replace("3000", "100");
    assert!(compose_svg(small.as_bytes(), 100, 100).is_ok());
}

#[test]
fn every_wire_fixture_honours_cancellation() {
    for (id, width, height, ..) in FIXTURES {
        let result = compose_document(&read(&format!("{id}.svg")), width, height, &|| true);
        assert!(matches!(result, Err(ComposeError::Cancelled)), "{id}");
        // Cancel late, inside rendering rather than decode.
        let calls = AtomicUsize::new(0);
        let late = || calls.fetch_add(1, Ordering::SeqCst) > 40;
        let result = compose_document(&read(&format!("{id}.svg")), width, height, &late);
        assert!(
            matches!(result, Err(ComposeError::Cancelled)) || result.is_ok(),
            "{id}"
        );
    }
}

// ---- hit testing over grouped content ---------------------------------------

#[test]
fn hit_testing_still_finds_shapes_inside_groups_and_patterns() {
    let composed = compose_document(&read("wire-pattern.svg"), 96, 64, &|| false).unwrap();
    assert!(composed.hit_test(10.0, 10.0).is_some(), "pattern stroke");
    assert!(composed.hit_test(20.0, 30.0).is_some(), "pattern fill");
    assert!(composed.hit_test(90.0, 60.0).is_none(), "empty corner");
}

// ---- AC6: source preservation through the project path -----------------------

#[test]
fn open_and_save_keep_feature_documents_byte_identical() {
    use crate::session::AppDocument;
    use std::os::unix::ffi::OsStrExt;
    let dir = super::tests_open::TempDir::new();
    for id in [
        "wire-mask",
        "wire-group",
        "wire-filter",
        "wire-text",
        "wire-pattern",
        "wire-combined",
    ] {
        let source = read(&format!("{id}.svg"));
        let mut app = AppDocument::new(10, 10).unwrap();
        app.open_path(std::path::Path::new(&format!("{DIR}/{id}.svg")))
            .unwrap();
        assert_eq!(app.source_bytes().unwrap(), source, "{id}: imported source");
        let project = dir.0.join(format!("{id}.vexel"));
        app.save_as(project.as_os_str().as_bytes()).unwrap();
        let mut reopened = AppDocument::new(10, 10).unwrap();
        reopened.open_path(&project).unwrap();
        assert_eq!(
            reopened.source_bytes().unwrap(),
            source,
            "{id}: saved and reopened source"
        );
    }
}

// ---- animation frames use the same renderer ----------------------------------

#[test]
fn animation_frames_render_masks_groups_and_text() {
    use crate::animation::frame::decode;
    use std::sync::atomic::AtomicBool;
    let svg = "<svg xmlns='http://www.w3.org/2000/svg' width='64' height='64'>\
        <defs><mask id='m' maskUnits='userSpaceOnUse' x='0' y='0' width='64' height='64'>\
        <rect x='0' y='0' width='32' height='64' fill='white'/></mask></defs>\
        <rect x='4' y='42' width='10' height='6' fill='#0000ff'>\
        <animate attributeName='x' values='4;40;4' dur='1s' repeatCount='indefinite'/></rect>\
        <text x='4' y='62' font-size='12' fill='#000000'>Hi</text>\
        <g mask='url(#m)' opacity='0.5'>\
        <rect x='0' y='8' width='64' height='32' fill='#ff0000'/></g></svg>";
    let scene = decode(svg.as_bytes()).expect("animated scene with mask, group and text");
    let stop = AtomicBool::new(false);
    let first = scene.render(0.0, &stop).expect("t=0");
    let later = scene.render(0.5, &stop).expect("t=0.5");
    let at = |tile: &vexel_raster::Tile, x: usize, y: usize| {
        let o = (y * 64 + x) * 4;
        tile.rgba()[o..o + 4].to_vec()
    };
    // Half-opacity red inside the mask, nothing beyond x=32, at both times.
    for frame in [&first, &later] {
        let inside = at(frame, 10, 20);
        assert!(
            inside[3] > 100 && inside[3] < 160 && inside[0] > 200,
            "{inside:?}"
        );
        assert_eq!(at(frame, 40, 20)[3], 0, "masked away beyond x=32");
    }
    // The animated blue rect moves from x=4 (t=0) to x=40 (t=0.5).
    assert_eq!(at(&first, 8, 44), [0, 0, 255, 255]);
    assert_eq!(at(&first, 44, 44)[3], 0);
    assert_eq!(at(&later, 44, 44), [0, 0, 255, 255]);
    let glyphs = (52..64)
        .flat_map(|y| (4..24).map(move |x| (x, y)))
        .filter(|&(x, y)| at(&first, x, y)[3] > 128)
        .count();
    assert!(glyphs > 8, "text drawn in animation frames ({glyphs})");
}

#[test]
fn animation_frames_keep_dashes_deferred() {
    use crate::animation::frame::{AnimationError, decode};
    use std::sync::atomic::AtomicBool;
    let svg = "<svg xmlns='http://www.w3.org/2000/svg' width='8' height='8'>\
        <rect x='1' y='1' width='4' height='4' fill='none' stroke='red' stroke-dasharray='2 1'>\
        <animate attributeName='x' values='1;2' dur='1s'/></rect></svg>";
    let scene = decode(svg.as_bytes()).unwrap();
    assert!(matches!(
        scene.render(0.0, &AtomicBool::new(false)),
        Err(AnimationError::Render(_))
    ));
}
