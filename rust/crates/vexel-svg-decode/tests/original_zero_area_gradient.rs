#[path = "support/production.rs"]
mod production;
mod support;

use production::ProductionFixture;
use std::sync::Arc;
use support::Run;
use vexel_scene::*;
use vexel_svg_decode::{Decoder, DecoderState, TraversalDecodeEvent as DecodeEvent};

const INPUT: &str = "<svg width=\"8\" height=\"8\"><defs><linearGradient id=\"g\"><stop stop-color=\"red\"/><stop offset=\"1\" stop-color=\"blue\"/></linearGradient></defs><rect width=\"0\" height=\"8\" fill=\"url(#g)\"/><rect width=\"4\" height=\"4\" fill=\"red\"/></svg>";

fn run_error(
    input: &str,
    operations: &mut ProductionFixture,
) -> (Diagnostic, (usize, usize, usize, usize), u64) {
    let source = Arc::new(input.as_bytes().to_vec());
    let mut control = Run::default();
    let document = LexicalDocument::parse(source.clone(), 1 << 20, &mut control).unwrap();
    let mut shared = DecodeSource::new(&document, &mut control).unwrap();
    let mut decoder = Decoder::from_source(
        &document,
        &mut shared,
        17,
        DecodeOptions {
            limits: Limits::default(),
            system_language: b"en",
            media: None,
        },
        &mut control,
    )
    .unwrap();
    let job = RasterJob::new(ChargeLedger::new(&source, 1 << 20, 65_536).unwrap()).unwrap();
    let error = loop {
        assert_eq!(decoder.state(), DecoderState::Running);
        match decoder.step_production(64, operations, &job, &mut control) {
            Ok(DecodeEvent::Progress { .. }) => {}
            Ok(DecodeEvent::NeedPattern(_)) => panic!("unexpected pattern request"),
            Ok(DecodeEvent::Complete) => panic!("zero-area gradient unexpectedly completed"),
            Err(error) => break error,
        }
    };
    let root = decoder.builder().unwrap().root_id();
    let scene = decoder.builder().unwrap().traversal_scene(root).unwrap();
    (
        error,
        (
            scene.edges.len(),
            scene.shapes.len(),
            scene.fill_paints.len(),
            scene.stops.len(),
        ),
        control.units,
    )
}

#[test]
fn original_zero_area_gradient_fails_at_value_offset_without_partial_geometry() {
    let mut operations = ProductionFixture::default();
    let (error, counts, _) = run_error(INPUT, &mut operations);
    assert_eq!(error.kind, ErrorKind::Unsupported);
    assert_eq!(error.offset, INPUT.find("url(#g)").unwrap());
    assert_eq!(operations.resolves, 1);
    assert_eq!(operations.paint_edge_counts, [0]);
    assert_eq!(counts, (0, 0, 0, 0));
}

#[test]
fn zero_area_gradient_resolver_malformed_diagnostic_precedes_fallback() {
    let malformed_offset = INPUT.find("stop-color").unwrap();
    let mut operations = ProductionFixture::default();
    operations.paint_error = Some(Diagnostic {
        kind: ErrorKind::Unsupported,
        offset: malformed_offset,
    });
    let (error, counts, units) = run_error(INPUT, &mut operations);
    assert_eq!(error.kind, ErrorKind::Unsupported);
    assert_eq!(error.offset, malformed_offset);
    assert_eq!(operations.resolves, 1);
    assert_eq!(operations.paint_edge_counts, [0]);
    assert_eq!(counts, (0, 0, 0, 0));
    assert!(units > 0);
}

#[test]
fn zero_area_gradient_resolver_limit_precedes_fallback() {
    let mut operations = ProductionFixture::default();
    operations.paint_error = Some(Diagnostic {
        kind: ErrorKind::Limit(LimitKind::PaintStops),
        offset: 0,
    });
    let (error, counts, units) = run_error(INPUT, &mut operations);
    assert_eq!(error.kind, ErrorKind::Limit(LimitKind::PaintStops));
    assert_eq!(error.offset, 0);
    assert_eq!(operations.resolves, 1);
    assert_eq!(operations.paint_edge_counts, [0]);
    assert_eq!(counts, (0, 0, 0, 0));
    assert!(units > 0);
}
