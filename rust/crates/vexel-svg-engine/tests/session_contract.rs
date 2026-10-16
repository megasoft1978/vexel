//! Engine session lifecycle contract (AC6): finish only after `Complete`, bounded
//! steps, pending pattern work is never success, and the clip owner publishes.
mod common;
use common::*;
use std::sync::Arc;
use vexel_scene::*;
use vexel_svg_decode::{DecoderState, TraversalDecodeEvent};
use vexel_svg_engine::SvgEngineSession;

const CLIP: &str = "<svg width='16' height='16'><defs><clipPath id='c'><path d='M0 0L8 0L8 8Z'/></clipPath></defs>\
<rect id='owner' width='12' height='12' clip-path='url(#c)'/></svg>";
const PATTERN: &str = "<svg width='16' height='16'><defs><pattern id='p' width='4' height='4' patternUnits='userSpaceOnUse'>\
<rect width='2' height='2'/></pattern></defs><rect width='8' height='8' fill='url(#p)'/></svg>";
const SHAPES: &str = "<svg width='16' height='16'><rect width='4' height='4'/><circle cx='8' cy='8' r='3'/><path d='M0 0L5 5'/></svg>";

struct Fixture {
    owner: Arc<Vec<u8>>,
}
fn fixture(text: &str) -> Fixture {
    Fixture {
        owner: Arc::new(text.as_bytes().to_vec()),
    }
}
fn options<'a>() -> DecodeOptions<'a> {
    DecodeOptions {
        limits: Limits::default(),
        system_language: b"en",
        media: None,
    }
}

#[test]
fn finish_before_complete_is_rejected_at_every_step_boundary() {
    let fx = fixture(SHAPES);
    // Finish immediately after open, and after each Progress event.
    for steps_before_finish in 0..4 {
        let mut control = Control::default();
        let document = LexicalDocument::parse(fx.owner.clone(), 1 << 20, &mut control).unwrap();
        let mut source = DecodeSource::new(&document, &mut control).unwrap();
        let job = RasterJob::new(ChargeLedger::new(&fx.owner, 1 << 24, 65536).unwrap())
            .unwrap_or_else(|_| panic!("job"));
        let mut session =
            SvgEngineSession::from_source(&document, &mut source, 8, options(), &mut control)
                .unwrap();
        for _ in 0..steps_before_finish {
            let event = session.step(1, &job, &mut control).unwrap();
            assert!(
                matches!(event, TraversalDecodeEvent::Progress { .. }),
                "still running"
            );
        }
        assert_eq!(session.state(), DecoderState::Running);
        assert!(
            session.finish(&mut control).is_err(),
            "finish after {steps_before_finish} steps"
        );
    }
}

#[test]
fn finish_succeeds_only_after_complete() {
    let fx = fixture(SHAPES);
    let mut control = Control::default();
    let document = LexicalDocument::parse(fx.owner.clone(), 1 << 20, &mut control).unwrap();
    let mut source = DecodeSource::new(&document, &mut control).unwrap();
    let job = RasterJob::new(ChargeLedger::new(&fx.owner, 1 << 24, 65536).unwrap())
        .unwrap_or_else(|_| panic!("job"));
    let mut session =
        SvgEngineSession::from_source(&document, &mut source, 8, options(), &mut control).unwrap();
    while !matches!(
        session.step(64, &job, &mut control).unwrap(),
        TraversalDecodeEvent::Complete
    ) {}
    assert_eq!(session.state(), DecoderState::Complete);
    let DecodedScene::Drawable(graph) = session.finish(&mut control).unwrap() else {
        panic!("drawable")
    };
    assert_eq!(graph.root().shapes.len(), 3);
}

#[test]
fn step_is_bounded_by_the_quantum() {
    let fx = fixture(SHAPES);
    let steps_for = |quantum: u64| -> (u64, u64, u64) {
        let mut control = Control::default();
        let document = LexicalDocument::parse(fx.owner.clone(), 1 << 20, &mut control).unwrap();
        let mut source = DecodeSource::new(&document, &mut control).unwrap();
        let job = RasterJob::new(ChargeLedger::new(&fx.owner, 1 << 24, 65536).unwrap())
            .unwrap_or_else(|_| panic!("job"));
        let mut session =
            SvgEngineSession::from_source(&document, &mut source, 8, options(), &mut control)
                .unwrap();
        let (mut steps, mut max_charge, mut total) = (0u64, 0u64, 0u64);
        loop {
            steps += 1;
            assert!(steps < 100_000, "decode must terminate");
            match session.step(quantum, &job, &mut control).unwrap() {
                TraversalDecodeEvent::Progress { charged_units } => {
                    assert!(charged_units > 0, "progress is observable");
                    max_charge = max_charge.max(charged_units);
                    total += charged_units;
                }
                TraversalDecodeEvent::Complete => return (steps, max_charge, total),
                TraversalDecodeEvent::NeedPattern(_) => panic!("no pattern expected"),
            }
        }
    };
    let (fine_steps, fine_charge, fine_total) = steps_for(1);
    let (coarse_steps, _, _) = steps_for(1 << 20);
    assert!(
        fine_steps > coarse_steps,
        "small quantum yields more, smaller steps"
    );
    // A step overshoots the quantum by at most one indivisible task, so the
    // fine-grained maximum stays below the whole-document total.
    assert!(fine_charge < fine_total, "{fine_charge} < {fine_total}");
}

#[test]
fn pending_pattern_is_never_success_and_failed_render_rolls_back() {
    let fx = fixture(PATTERN);
    let mut control = Control::default();
    let document = LexicalDocument::parse(fx.owner.clone(), 1 << 20, &mut control).unwrap();
    let mut source = DecodeSource::new(&document, &mut control).unwrap();
    let mut job = RasterJob::new(ChargeLedger::new(&fx.owner, 1 << 24, 65536).unwrap())
        .unwrap_or_else(|_| panic!("job"));
    let mut session =
        SvgEngineSession::from_source(&document, &mut source, 8, options(), &mut control).unwrap();
    let ticket = loop {
        match session.step(64, &job, &mut control).unwrap() {
            TraversalDecodeEvent::Progress { .. } => {}
            TraversalDecodeEvent::NeedPattern(request) => break request.ticket,
            TraversalDecodeEvent::Complete => panic!("pattern work completed without a render"),
        }
    };
    assert_eq!(
        session.state(),
        DecoderState::Running,
        "suspended, not complete"
    );
    // Stepping again without resuming the exact ticket must not advance.
    assert!(session.step(64, &job, &mut control).is_err());
    assert_eq!(session.state(), DecoderState::Failed);
    // The same session cannot be finished into a scene afterwards.
    let mut fresh = Control::default();
    assert!(session.finish(&mut fresh).is_err());

    // Fresh session: finish while the pattern is pending is rejected, never a scene.
    let mut control = Control::default();
    let mut source = DecodeSource::new(&document, &mut control).unwrap();
    let mut session =
        SvgEngineSession::from_source(&document, &mut source, 8, options(), &mut control).unwrap();
    let ticket2 = loop {
        match session.step(64, &job, &mut control).unwrap() {
            TraversalDecodeEvent::Progress { .. } => {}
            TraversalDecodeEvent::NeedPattern(request) => break request.ticket,
            TraversalDecodeEvent::Complete => panic!("pattern work completed without a render"),
        }
    };
    let _ = ticket;
    // A failed external render resumes as a typed error and fails the decoder.
    let failure = Diagnostic {
        kind: ErrorKind::Unsupported,
        offset: PATTERN.find("<pattern").unwrap(),
    };
    let resumed = session.resume_pattern(ticket2, &mut job, Err(failure), &mut control);
    assert_eq!(resumed.err(), Some(failure));
    assert_eq!(session.state(), DecoderState::Failed);
    assert!(session.finish(&mut control).is_err());
}

#[test]
fn pending_pattern_finish_is_rejected_without_resume() {
    let fx = fixture(PATTERN);
    let mut control = Control::default();
    let document = LexicalDocument::parse(fx.owner.clone(), 1 << 20, &mut control).unwrap();
    let mut source = DecodeSource::new(&document, &mut control).unwrap();
    let job = RasterJob::new(ChargeLedger::new(&fx.owner, 1 << 24, 65536).unwrap())
        .unwrap_or_else(|_| panic!("job"));
    let mut session =
        SvgEngineSession::from_source(&document, &mut source, 8, options(), &mut control).unwrap();
    loop {
        match session.step(64, &job, &mut control).unwrap() {
            TraversalDecodeEvent::Progress { .. } => {}
            TraversalDecodeEvent::NeedPattern(_) => break,
            TraversalDecodeEvent::Complete => panic!("must suspend"),
        }
    }
    assert!(
        session.finish(&mut control).is_err(),
        "pending pattern is not a scene"
    );
}

#[test]
fn clip_owner_publishes_a_clip_with_geometry() {
    let graph = decode_ok(CLIP);
    let scene = graph.root();
    assert_eq!(scene.shapes.len(), 1);
    assert_eq!(scene.clips.len(), 1, "owner clip published");
    let clip = scene.shapes[0]
        .clip
        .expect("owner shape references its clip");
    let published = scene.clip(clip).expect("clip id resolves");
    assert!(!published.geometry.shapes.is_empty() && !published.geometry.edges.is_empty());
    assert!(!published.object_bounding_box);
    assert_eq!(published.bounds, [0.0, 0.0, 12.0, 12.0]);
}
