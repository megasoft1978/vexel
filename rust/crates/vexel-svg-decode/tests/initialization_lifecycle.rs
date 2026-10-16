#[path = "support/production.rs"]
mod production;
mod support;
use production::{ProductionFixture, output};
use std::sync::Arc;
use support::Run;
use vexel_scene::*;
use vexel_svg_decode::{Decoder, DecoderState, TraversalDecodeEvent as Event};
fn options<'a>() -> DecodeOptions<'a> {
    DecodeOptions {
        limits: Limits::default(),
        system_language: b"en",
        media: None,
    }
}
fn document(input: &str, c: &mut Run) -> LexicalDocument {
    LexicalDocument::parse(Arc::new(input.as_bytes().to_vec()), 1 << 24, c).unwrap()
}
#[test]
fn actual_hidden_outcome_has_no_viewport_and_retains_actual_late_animation() {
    let input = "<svg display='none' width='broken' transform='broken'><rect><animate attributeName='opacity' values='0;1' dur='1s'/></rect></svg>";
    let mut c = Run::default();
    let doc = document(input, &mut c);
    let mut d = Decoder::new(&doc, 17, options(), &mut c).unwrap();
    assert_eq!(
        d.builder().unwrap().initialization(),
        DecodeInitialization::Hidden
    );
    let read = d
        .builder()
        .unwrap()
        .traversal_scene(d.builder().unwrap().root_id())
        .unwrap();
    assert!(read.viewport.is_none());
    assert!(read.parsed_animation().is_none());
    while d.state() == DecoderState::Running {
        d.step(1, &mut support::RejectOperations, &mut c).unwrap();
    }
    assert_eq!(
        d.builder().unwrap().initialization(),
        DecodeInitialization::HiddenFinalized
    );
    let DecodedScene::Hidden(hidden) = d.finish_decoded(&mut c).unwrap() else {
        panic!("hidden outcome")
    };
    assert!(hidden.view().viewport.is_none());
    assert_eq!(hidden.animation().tracks.len(), 1);
    assert!(Arc::ptr_eq(hidden.source_owner(), doc.source_owner()));
    assert_eq!(hidden.generation(), 17);
    assert!(hidden.view().shapes.is_empty());
}
#[test]
fn real_pattern_ticket_precedes_parent_animation_and_child_failure_wins() {
    let tracks = "<animate attributeName='opacity' values='0;1' dur='1s'/>".repeat(129);
    let input = format!(
        "<svg width='8' height='8'><defs><pattern id='p'><rect width='1' height='1'/></pattern></defs><rect width='2' height='2' fill='url(#p)'>{tracks}</rect></svg>"
    );
    let mut c = Run::default();
    let doc = document(&input, &mut c);
    let mut d = Decoder::new(&doc, 17, options(), &mut c).unwrap();
    let mut job =
        RasterJob::new(ChargeLedger::new(doc.source_owner(), 1 << 24, 65536).unwrap()).unwrap();
    let mut ops = ProductionFixture::default();
    let ticket = loop {
        match d.step_production(1, &mut ops, &job, &mut c).unwrap() {
            Event::NeedPattern(r) => {
                assert_eq!(r.child.shapes.len(), 1);
                assert!(r.child.parsed_animation().is_none());
                assert!(r.child.viewport.is_some());
                break r.ticket;
            }
            Event::Progress { charged_units } => assert!(charged_units > 0),
            Event::Complete => panic!("must request actual sealed child first"),
        }
    };
    let error = d
        .resume_pattern_production(
            ticket,
            &mut job,
            Err(Diagnostic {
                kind: ErrorKind::Canceled,
                offset: 37,
            }),
            &mut c,
        )
        .unwrap_err();
    assert_eq!(
        error,
        Diagnostic {
            kind: ErrorKind::Canceled,
            offset: 37
        }
    );
    assert_eq!(d.state(), DecoderState::Failed);
    assert_eq!(job.stats().output_bytes, 0);
    assert!(d.finish_decoded(&mut c).is_err());
}
#[test]
fn completed_pattern_then_actual_late_animation_limit_and_failed_finish() {
    let tracks = "<animate attributeName='opacity' values='0;1' dur='1s'/>".repeat(129);
    let input = format!(
        "<svg width='8' height='8'><defs><pattern id='p'><rect width='1' height='1'/></pattern></defs><rect width='2' height='2' fill='url(#p)'>{tracks}</rect></svg>"
    );
    let mut c = Run::default();
    let doc = document(&input, &mut c);
    let mut d = Decoder::new(&doc, 17, options(), &mut c).unwrap();
    let mut job =
        RasterJob::new(ChargeLedger::new(doc.source_owner(), 1 << 24, 65536).unwrap()).unwrap();
    let mut ops = ProductionFixture::default();
    let mut requests = 0;
    loop {
        match d.step_production(1, &mut ops, &job, &mut c) {
            Ok(Event::NeedPattern(r)) => {
                assert!(r.child.parsed_animation().is_none());
                let t = r.ticket;
                let pixels = output(&mut job, 17, &mut c); // Explicit fixture render result only.
                d.resume_pattern_production(t, &mut job, Ok(pixels), &mut c)
                    .unwrap();
                requests += 1;
            }
            Ok(Event::Progress { charged_units }) => assert!(charged_units > 0),
            Err(e) => {
                assert_eq!(e.kind, ErrorKind::Limit(LimitKind::Animation));
                break;
            }
            _ => panic!("actual late animation must reject track 129"),
        }
    }
    assert_eq!(requests, 1);
    assert_eq!(job.stats().output_bytes, 0);
    assert!(d.finish_decoded(&mut c).is_err());
}
#[test]
fn finalized_once_repeated_complete_is_read_only_and_actual_source_retained() {
    let mut c = Run::default();
    let doc = document(
        "<svg width='8' height='8'><rect width='1' height='1'/></svg>",
        &mut c,
    );
    let mut d = Decoder::new(&doc, 17, options(), &mut c).unwrap();
    assert_eq!(
        d.builder().unwrap().initialization(),
        DecodeInitialization::Traversing
    );
    while d.state() == DecoderState::Running {
        d.step(1, &mut support::RejectOperations, &mut c).unwrap();
    }
    let units = c.units;
    assert!(matches!(
        d.step(1, &mut support::RejectOperations, &mut c).unwrap(),
        DecodeEvent::Complete
    ));
    assert_eq!(c.units, units);
    assert_eq!(
        d.builder().unwrap().initialization(),
        DecodeInitialization::Finalized
    );
    let DecodedScene::Drawable(g) = d.finish_decoded(&mut c).unwrap() else {
        panic!("drawable")
    };
    assert_eq!(g.root().shapes.len(), 1);
    assert!(Arc::ptr_eq(g.source_owner(), doc.source_owner()));
}
