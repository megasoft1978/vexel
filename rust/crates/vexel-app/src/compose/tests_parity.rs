//! AC2: the DECODE+RENDER pipeline against C `svgworkbench` PPMs for the six
//! shared first-party fixtures (copied unchanged from vexel-render's goldens).
//!
//! Thresholds are those of WO-RUST-V1-RENDER-01 (`tests/parity.rs`): max
//! channel delta 1, no delta above 2, at most 0.5% differing pixels. The
//! RENDER-01 clause "pixels farther than 1 px from any edge must be exactly
//! equal" needs hand-dumped edge data and is replaced here by a stricter one:
//! the pipeline output must be byte-exact in every pixel the golden paints
//! white (no stray coverage).
use super::tests::{fixture, ppm_over_white};
use super::*;

const MAX_CHANNEL_DELTA: i32 = 1;
const MAX_ANY_DELTA: i32 = 2;
const MAX_DIFFERING_FRACTION: f64 = 0.005;

struct Report {
    total: usize,
    max_delta: i32,
    differing: usize,
    over_two: usize,
    stray: usize,
}

impl Report {
    fn passes(&self) -> bool {
        self.max_delta <= MAX_CHANNEL_DELTA
            && self.over_two == 0
            && self.stray == 0
            && (self.differing as f64) <= MAX_DIFFERING_FRACTION * self.total as f64
    }
}

fn compare(actual: &[u8], golden: &[u8]) -> Report {
    assert_eq!(actual.len(), golden.len());
    let total = golden.len() / 3;
    let mut report = Report {
        total,
        max_delta: 0,
        differing: 0,
        over_two: 0,
        stray: 0,
    };
    for i in 0..total {
        let delta = (0..3)
            .map(|c| (i32::from(actual[i * 3 + c]) - i32::from(golden[i * 3 + c])).abs())
            .max()
            .unwrap();
        report.max_delta = report.max_delta.max(delta);
        if delta > 0 {
            report.differing += 1;
            if golden[i * 3..i * 3 + 3] == [255, 255, 255] {
                report.stray += 1;
            }
        }
        if delta > MAX_ANY_DELTA {
            report.over_two += 1;
        }
    }
    report
}

fn run(id: &str) -> Report {
    let svg = fixture(&format!("c-reference/{id}.svg"));
    let golden = fixture(&format!("c-reference/{id}.ppm"));
    let header = b"P6\n128 96\n255\n";
    assert!(golden.starts_with(header), "{id}: golden header");
    let frame = compose_svg(&svg, 128, 96).unwrap();
    assert!(
        frame.disclosures.is_empty(),
        "{id}: {:?}",
        frame.disclosures
    );
    let actual = ppm_over_white(&frame);
    let report = compare(&actual[header.len()..], &golden[header.len()..]);
    println!(
        "PARITY {id}: pixels={} max_delta={} differing={} over_two={} stray={}",
        report.total, report.max_delta, report.differing, report.over_two, report.stray
    );
    report
}

macro_rules! parity {
    ($name:ident, $id:expr) => {
        #[test]
        fn $name() {
            let report = run($id);
            assert!(
                report.passes(),
                "{}: max_delta={} differing={}",
                $id,
                report.max_delta,
                report.differing
            );
        }
    };
}

parity!(parity_shapes_basic, "shapes-basic");
parity!(parity_shapes_rounded, "shapes-rounded");
parity!(parity_shapes_lines_polygons, "shapes-lines-polygons");
parity!(parity_path_line_curves, "path-line-curves");
parity!(parity_path_relative_commands, "path-relative-commands");
parity!(
    parity_style_presentation_attributes,
    "style-presentation-attributes"
);

#[test]
fn parity_harness_rejects_a_shifted_render() {
    let svg = fixture("c-reference/shapes-basic.svg");
    let golden = fixture("c-reference/shapes-basic.ppm");
    let header = b"P6\n128 96\n255\n".len();
    let shifted = String::from_utf8(svg)
        .unwrap()
        .replace("x=\"4\"", "x=\"4.5\"")
        .replace("cx=\"62\"", "cx=\"62.5\"");
    let frame = compose_svg(shifted.as_bytes(), 128, 96).unwrap();
    let report = compare(&ppm_over_white(&frame)[header..], &golden[header..]);
    assert!(!report.passes(), "a half-pixel shift must fail the harness");
}
