//! WO-RUST-CORPUS-UNSUPPORTED-01: markers and data-URI PNG images composed
//! through the app pipeline and compared with PPM goldens made by the frozen
//! legacy C `svgworkbench` (`rust/fixtures/unsupported`).
//!
//! Tolerances are those of the renderer orders, copied unchanged: max channel
//! delta 1, no delta above 2, at most 0.5% differing pixels, and no differing
//! pixel farther than one pixel from painted golden content. The marker
//! goldens and `image-png-rgba` are byte-exact; `image-png-types` differs by 1
//! in a few tie-rounding pixels (the pattern sampler the images reuse rounds
//! x.5 channel values differently from the C image sampler).
use vexel_app::compose::{ComposeError, Frame, compose_document, compose_svg};

#[path = "../../vexel-render/tests/common/sha256.rs"]
mod sha256;

const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/unsupported");

/// (fixture id, width, height)
const GOLDENS: [(&str, u32, u32); 4] = [
    ("marker-end", 64, 48),
    ("marker-mid-end", 96, 64),
    ("image-png-rgba", 64, 48),
    ("image-png-types", 96, 64),
];

fn read(name: &str) -> Vec<u8> {
    std::fs::read(format!("{DIR}/{name}")).unwrap_or_else(|e| panic!("{name}: {e}"))
}

fn over_white(frame: &Frame) -> Vec<u8> {
    let mut out = Vec::new();
    for p in frame.rgba.chunks_exact(4) {
        let a = u32::from(p[3]);
        out.extend((0..3).map(|c| (u32::from(p[c]) + 255 - a).min(255) as u8));
    }
    out
}

fn near_content(golden: &[u8], width: usize, height: usize, x: usize, y: usize) -> bool {
    (y.saturating_sub(1)..=(y + 1).min(height - 1)).any(|ny| {
        (x.saturating_sub(1)..=(x + 1).min(width - 1))
            .any(|nx| golden[(ny * width + nx) * 3..][..3] != [255, 255, 255])
    })
}

#[derive(Debug)]
struct Report {
    total: usize,
    max_delta: i32,
    differing: usize,
    over_two: usize,
    outside_dilation: usize,
}

fn compare(actual: &[u8], golden: &[u8], width: usize) -> Report {
    assert_eq!(actual.len(), golden.len());
    let height = golden.len() / 3 / width;
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
        if delta == 0 {
            continue;
        }
        report.differing += 1;
        report.over_two += usize::from(delta > 2);
        if !near_content(golden, width, height, index % width, index / width) {
            report.outside_dilation += 1;
        }
    }
    report
}

fn passes(report: &Report) -> bool {
    report.max_delta <= 1
        && report.over_two == 0
        && report.outside_dilation == 0
        && (report.differing as f64) <= 0.005 * report.total as f64
}

fn run(id: &str, width: u32, height: u32) -> Report {
    let frame = compose_svg(&read(&format!("{id}.svg")), width, height)
        .unwrap_or_else(|e| panic!("{id}: {e}"));
    assert!(
        frame.disclosures.is_empty(),
        "{id}: {:?}",
        frame.disclosures
    );
    let golden = read(&format!("{id}.ppm"));
    let header = format!("P6\n{width} {height}\n255\n").into_bytes();
    assert!(golden.starts_with(&header), "{id}: golden header");
    let report = compare(&over_white(&frame), &golden[header.len()..], width as usize);
    println!("PARITY {id}: {report:?}");
    report
}

#[test]
fn every_marker_and_image_fixture_composes_and_matches_its_c_golden() {
    let failures: Vec<_> = GOLDENS
        .iter()
        .filter_map(|(id, w, h)| {
            let report = run(id, *w, *h);
            (!passes(&report)).then(|| format!("{id}: {report:?}"))
        })
        .collect();
    assert!(
        failures.is_empty(),
        "parity failures:\n{}",
        failures.join("\n")
    );
}

#[test]
fn marker_and_image_goldens_are_byte_exact_where_documented() {
    for id in ["marker-end", "marker-mid-end", "image-png-rgba"] {
        let (w, h) = GOLDENS
            .iter()
            .find(|g| g.0 == id)
            .map(|g| (g.1, g.2))
            .unwrap();
        let report = run(id, w, h);
        assert_eq!(report.differing, 0, "{id}: {report:?}");
    }
}

#[test]
fn the_parity_harness_rejects_a_shifted_marker_render() {
    let svg = String::from_utf8(read("marker-end.svg")).unwrap();
    let shifted = svg.replace("L40 12", "L41 12");
    assert_ne!(shifted, svg);
    let frame = compose_svg(shifted.as_bytes(), 64, 48).unwrap();
    let golden = read("marker-end.ppm");
    let header = b"P6\n64 48\n255\n".len();
    assert!(!passes(&compare(
        &over_white(&frame),
        &golden[header..],
        64
    )));
}

#[test]
fn manifest_hashes_match_the_files() {
    let manifest = String::from_utf8(read("manifest.json")).unwrap();
    for field in ["legacy_tag", "binary_sha256", "generator"] {
        assert!(manifest.contains(&format!("\"{field}\"")), "{field}");
    }
    for (id, ..) in GOLDENS {
        let start = manifest
            .find(&format!("\"fixture_id\": \"{id}\""))
            .unwrap_or_else(|| panic!("{id} missing from manifest"));
        let entry = &manifest[start..];
        let entry = &entry[..entry.find('}').unwrap()];
        for (key, extension) in [("source_sha256", "svg"), ("ppm_sha256", "ppm")] {
            let marker = format!("\"{key}\": \"");
            let at = entry.find(&marker).unwrap() + marker.len();
            let bytes = read(&format!("{id}.{extension}"));
            assert_eq!(
                sha256::sha256_hex(&bytes),
                &entry[at..at + 64],
                "{id}.{extension}"
            );
        }
    }
}

fn pixel(frame: &Frame, x: u32, y: u32) -> [u8; 4] {
    let at = (y as usize * frame.width as usize + x as usize) * 4;
    frame.rgba[at..at + 4].try_into().unwrap()
}

/// Hand-derived oracle (no C golden: the legacy C emits the start marker twice
/// per subpath). `marker-start.svg` has a 6x6 square marker, `refX=refY=3`,
/// stroke width 1 so scale 1, referenced by `marker-start` only: the square
/// covers exactly x 9..15, y 9..15 around the path start (12, 12), once.
#[test]
fn start_marker_is_drawn_once_at_the_path_start_only() {
    let composed = compose_document(&read("marker-start.svg"), 64, 48, &|| false).unwrap();
    assert!(composed.frame.disclosures.is_empty());
    assert_eq!(
        composed.shapes.len(),
        2,
        "host path plus one marker instance"
    );
    let red = [0xcc, 0, 0, 255];
    let count = (0..48)
        .flat_map(|y| (0..64).map(move |x| (x, y)))
        .filter(|&(x, y)| pixel(&composed.frame, x, y) == red)
        .count();
    assert_eq!(count, 36, "exactly one 6x6 marker square");
    for (x, y) in [(9, 9), (14, 9), (9, 14), (14, 14), (12, 12)] {
        assert_eq!(pixel(&composed.frame, x, y), red, "({x}, {y})");
    }
    for (x, y) in [(8, 8), (15, 15), (52, 12), (52, 36)] {
        assert_ne!(
            pixel(&composed.frame, x, y),
            red,
            "({x}, {y}) is outside the marker"
        );
    }
}

#[test]
fn an_image_inside_the_document_is_drawn_and_hit_testable() {
    let composed = compose_document(&read("image-png-rgba.svg"), 64, 48, &|| false).unwrap();
    assert!(composed.frame.disclosures.is_empty());
    // The 4x3 image is drawn into (7,5)-(54,42) with aspect meet: opaque inside.
    assert_eq!(pixel(&composed.frame, 30, 24)[3], 255);
    assert_eq!(
        pixel(&composed.frame, 2, 2),
        [0, 0, 0, 0],
        "outside the image"
    );
    assert!(composed.hit_test(30.0, 24.0).is_some());
    assert!(composed.hit_test(2.0, 2.0).is_none());
}

#[test]
fn thousands_of_identical_images_share_one_tile() {
    // 1x1 opaque red PNG repeated 1500 times: more images than the renderer's
    // limit of distinct pattern tiles per batch.
    let red = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/iZk9HQAAAABJRU5ErkJggg==";
    let mut svg = String::from("<svg xmlns='http://www.w3.org/2000/svg' width='50' height='40'>");
    for i in 0..1500 {
        svg.push_str(&format!(
            "<image x='{}' y='{}' width='1' height='1' href='data:image/png;base64,{red}'/>",
            i % 50,
            i / 50
        ));
    }
    svg.push_str("</svg>");
    let frame = compose_svg(svg.as_bytes(), 50, 40).unwrap();
    assert!(frame.disclosures.is_empty(), "{:?}", frame.disclosures);
    assert_eq!(pixel(&frame, 0, 0), [255, 0, 0, 255]);
    assert_eq!(pixel(&frame, 49, 29), [255, 0, 0, 255]);
    assert_eq!(pixel(&frame, 0, 30), [0, 0, 0, 0], "row 30 has no image");
}

#[test]
fn jpeg_and_malformed_images_stay_typed_unsupported_errors() {
    let wrap = |href: &str| {
        format!(
            "<svg xmlns='http://www.w3.org/2000/svg' width='20' height='20'>\
<image width='10' height='10' href='{href}'/></svg>"
        )
    };
    for href in [
        "data:image/jpeg;base64,/9j/4AAQSkZJRgABAQEASABIAAD/2wBDAP//////////////////////////////////////////////////////////////////////////////////////wAALCAABAAEBAREA/8QAFAABAAAAAAAAAAAAAAAAAAAAA//EABQQAQAAAAAAAAAAAAAAAAAAAAD/2gAIAQEAAD8AKn//2Q==",
        "data:image/png;base64,iVBORw0KGgo=",
        "https://example.invalid/a.png",
    ] {
        match compose_svg(wrap(href).as_bytes(), 20, 20) {
            Err(ComposeError::Unsupported { feature, .. }) => {
                assert_eq!(feature, "image", "{href}")
            }
            other => panic!("expected Unsupported(image) for {href}, got {other:?}"),
        }
    }
}
