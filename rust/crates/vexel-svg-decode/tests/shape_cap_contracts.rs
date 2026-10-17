//! WO-RUST-SCENE-SHAPES-01: the shape cap and the `<use>` instance cap are one named
//! constant each, `MAX_SHAPES` and `MAX_USE_INSTANCES`, and nested expansion cannot
//! multiply shapes past them.
mod support;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use support::*;
use vexel_base::BudgetControl;
use vexel_scene::*;
use vexel_svg_decode::*;

const SVG_OPEN: &str = "<svg width=\"128\" height=\"128\" viewBox=\"0 0 128 128\">";

// Measured need of the external corpus file generated-use_t2 (WO-RUST-SCENE-LIMITS-01).
const CORPUS_USE_T2_SHAPES: usize = 178_589;

// One attribute per shape: the XML layer caps total attributes at 1,000,000
// (vexel-xml MAX_TOTAL_ATTRIBUTES), so four-attribute rects cannot reach the shape cap.
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
    // A 28-byte target keeps 524,288 instances under the 16 MiB reference-bytes budget
    // (a 36-byte rect target would be refused there first; WO-RUST-LIMITS-03).
    let mut s = format!("{SVG_OPEN}<defs><path id=\"r\" d=\"M0 0h1v1z\"/></defs>");
    s.push_str(&"<use href=\"#r\"/>".repeat(n));
    s.push_str("</svg>");
    s
}

/// Same steps as `support::decode_outcome`, with a caller-supplied control.
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
fn corpus_use_t2_shape_need_decodes() {
    assert!(MAX_SHAPES > CORPUS_USE_T2_SHAPES);
    assert_eq!(
        root_shapes(&use_rects(CORPUS_USE_T2_SHAPES)),
        CORPUS_USE_T2_SHAPES
    );
}

#[test]
fn use_instance_cap_never_exceeds_the_shape_cap() {
    // WO-RUST-LIMITS-03 decoupled the two: shapes 524,288, use instances 262,144 (the
    // `<use>`-heavy worst case dominates peak memory).
    assert!(MAX_USE_INSTANCES <= MAX_SHAPES);
    assert_eq!(MAX_USE_INSTANCES, 262_144);
}

#[test]
fn flat_shape_boundary_accepts_max_and_refuses_next() {
    assert_eq!(root_shapes(&flat_shapes(MAX_SHAPES)), MAX_SHAPES);
    assert_eq!(
        refusal(&flat_shapes(MAX_SHAPES + 1)),
        ErrorKind::Limit(LimitKind::Shapes)
    );
}

#[test]
fn use_instance_boundary_accepts_max_and_refuses_next() {
    assert_eq!(
        root_shapes(&use_rects(MAX_USE_INSTANCES)),
        MAX_USE_INSTANCES
    );
    assert_eq!(
        refusal(&use_rects(MAX_USE_INSTANCES + 1)),
        ErrorKind::Limit(LimitKind::References)
    );
}

#[test]
fn nested_use_product_is_refused_at_the_instance_cap() {
    // Three levels of fan-out 81: 81 + 81^2 + 81^3 = 538,083 expansions, above
    // MAX_USE_INSTANCES = 524,288 (WO-RUST-LIMITS-03 moved this from fan 65, which
    // is 274,490 and now under the cap), while the leaves alone 81^3 are above MAX_SHAPES.
    let fan = 81;
    let mut defs = String::from("<defs><g id=\"g0\"><rect width=\"1\" height=\"1\"/></g>");
    for level in 1..=3 {
        let uses = format!("<use href=\"#g{}\"/>", level - 1).repeat(fan);
        defs.push_str(&format!("<g id=\"g{level}\">{uses}</g>"));
    }
    defs.push_str("</defs>");
    let source = format!("{SVG_OPEN}{defs}<use href=\"#g3\"/></svg>");
    let start = Instant::now();
    let kind = refusal(&source);
    let elapsed = start.elapsed();
    assert_eq!(kind, ErrorKind::Limit(LimitKind::References));
    eprintln!("MEASURE nested_refusal_elapsed_ms={}", elapsed.as_millis());
    assert!(
        elapsed < Duration::from_secs(60),
        "nested refusal took {elapsed:?}"
    );
}

#[test]
fn cancel_during_max_shape_decode_is_prompt() {
    let source = flat_shapes(MAX_SHAPES);
    let flag = AtomicBool::new(false);
    let (result, raised, returned) = std::thread::scope(|scope| {
        let flag_ref = &flag;
        let raiser = scope.spawn(move || {
            std::thread::sleep(Duration::from_millis(50));
            let raised = Instant::now();
            flag_ref.store(true, Ordering::Release);
            raised
        });
        let mut control = BudgetControl::new(&flag, u64::MAX);
        let result = decode_controlled(&source, &mut control);
        let returned = Instant::now();
        (result, raiser.join().unwrap(), returned)
    });
    match result {
        Err(error) => assert_eq!(error.kind, ErrorKind::Canceled),
        Ok(_) => panic!("decode finished before the cancel signal"),
    }
    let latency = returned.duration_since(raised);
    eprintln!("MEASURE cancel_latency_us={}", latency.as_micros());
    assert!(
        latency <= Duration::from_millis(250),
        "cancel latency {latency:?}"
    );
}
