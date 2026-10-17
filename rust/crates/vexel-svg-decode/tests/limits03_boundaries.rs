//! WO-RUST-LIMITS-03 boundary pins at the literal targets: shapes 524,288, `<use>`
//! instances 524,288, and one `<use>` target of 1 MiB (`Limit(References)`).
mod support;
use std::sync::Arc;
use std::time::{Duration, Instant};
use support::*;
use vexel_scene::*;
use vexel_svg_decode::*;

const SVG_OPEN: &str = "<svg width=\"128\" height=\"128\" viewBox=\"0 0 128 128\">";
const MIB: usize = 1024 * 1024;

fn flat_shapes(n: usize) -> String {
    let mut s = String::with_capacity(n * 32 + 64);
    s.push_str(SVG_OPEN);
    for i in 0..n {
        s.push_str(&format!(
            "<path d=\"M{} {}h1v1z\"/>",
            i % 128,
            (i / 128) % 128
        ));
    }
    s.push_str("</svg>");
    s
}

fn use_rects(n: usize) -> String {
    // 28-byte target: 524,288 * 28 = 14.7 MB stays under the 16 MiB reference-bytes budget.
    let mut s = format!("{SVG_OPEN}<defs><path id=\"r\" d=\"M0 0h1v1z\"/></defs>");
    s.push_str(&"<use href=\"#r\"/>".repeat(n));
    s.push_str("</svg>");
    s
}

fn decode_controlled(input: &str, control: &mut dyn WorkControl) -> Result<DecodedScene> {
    let source = Arc::new(input.as_bytes().to_vec());
    let document = LexicalDocument::parse(source, 256 * 1024 * 1024, control)?;
    let mut shared = DecodeSource::new(&document, control)?;
    let mut decoder = Decoder::from_source(
        &document,
        &mut shared,
        17,
        DecodeOptions {
            limits: Limits::default(),
            system_language: b"en",
            media: None,
        },
        control,
    )?;
    while decoder.state() == DecoderState::Running {
        decoder.step(1024, &mut RejectOperations, control)?;
    }
    decoder.finish_decoded(control)
}

fn root_shapes(input: &str) -> usize {
    match decode_controlled(input, &mut Run::default()).expect("decode must succeed") {
        DecodedScene::Drawable(g) => g.root().shapes.len(),
        DecodedScene::Hidden(_) => panic!("fixture must be drawable"),
    }
}

fn refusal(input: &str) -> ErrorKind {
    match decode_controlled(input, &mut Run::default()) {
        Err(error) => error.kind,
        Ok(_) => panic!("fixture must be refused"),
    }
}

#[test]
fn limit03_shape_boundary_is_524288() {
    assert_eq!(MAX_SHAPES, 524_288);
    assert_eq!(root_shapes(&flat_shapes(524_288)), 524_288);
    assert_eq!(
        refusal(&flat_shapes(524_289)),
        ErrorKind::Limit(LimitKind::Shapes)
    );
}

#[test]
fn limit03_use_instance_boundary_is_524288() {
    assert_eq!(MAX_USE_INSTANCES, 524_288);
    assert_eq!(root_shapes(&use_rects(524_288)), 524_288);
    assert_eq!(
        refusal(&use_rects(524_289)),
        ErrorKind::Limit(LimitKind::References)
    );
}

/// `<use>` target `<g id="t">` with `pad` spaces of text. The target span runs from
/// `<g` to `</g>`: `<g id="t">` (10 bytes) + pad + `</g>` (4 bytes).
fn use_target(pad: usize) -> String {
    format!(
        "{SVG_OPEN}<defs><g id=\"t\">{}</g></defs><use href=\"#t\"/></svg>",
        " ".repeat(pad)
    )
}

#[test]
fn limit03_use_target_1_mib_is_accepted_and_the_next_byte_refused() {
    let at_limit = MIB - 14;
    let _ = root_shapes(&use_target(at_limit));
    assert_eq!(
        refusal(&use_target(at_limit + 1)),
        ErrorKind::Limit(LimitKind::References)
    );
}

#[test]
fn limit03_nested_use_product_is_refused_at_the_instance_cap() {
    // Fan-out 3 over 12 levels: 3^12 = 531,441 leaves and (3^13 - 1) / 2 = 797,161
    // instances in total, above MAX_USE_INSTANCES = 524,288. Depth 12 stays under
    // the 16-deep active-use bound, so the instance cap is the limit that fires.
    let fan = 3;
    let mut defs = String::from("<defs><g id=\"g0\"><rect width=\"1\" height=\"1\"/></g>");
    for level in 1..=12 {
        let uses = format!("<use href=\"#g{}\"/>", level - 1).repeat(fan);
        defs.push_str(&format!("<g id=\"g{level}\">{uses}</g>"));
    }
    defs.push_str("</defs>");
    let source = format!("{SVG_OPEN}{defs}<use href=\"#g12\"/></svg>");
    let start = Instant::now();
    let kind = refusal(&source);
    let elapsed = start.elapsed();
    eprintln!("MEASURE limit03_nested_refusal_ms={}", elapsed.as_millis());
    assert_eq!(kind, ErrorKind::Limit(LimitKind::References));
    assert!(
        elapsed < Duration::from_secs(60),
        "nested refusal took {elapsed:?}"
    );
}
