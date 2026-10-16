use vexel_svg_decode::TraversalDecodeEvent as DecodeEvent;
#[path = "support/production.rs"]
mod production;
mod support;
use production::{ProductionFixture, output};
use std::sync::Arc;
use support::Run;
use vexel_scene::*;
use vexel_svg_decode::{Decoder, DecoderState};
const SOURCE: &str = "<svg width=\"8\" height=\"8\"><defs><pattern id=\"p\"><rect width=\"2\" height=\"2\" fill=\"red\"/></pattern></defs><rect width=\"4\" height=\"4\" fill=\"url(#p)\"/></svg>";
fn lexical(s: &str, c: &mut Run) -> LexicalDocument {
    LexicalDocument::parse(Arc::new(s.as_bytes().to_vec()), 1 << 20, c).unwrap()
}
fn job(d: &LexicalDocument) -> RasterJob {
    RasterJob::new(ChargeLedger::new(d.source_owner(), 1 << 24, 65536).unwrap()).unwrap()
}
fn options<'a>() -> DecodeOptions<'a> {
    DecodeOptions {
        limits: Limits::default(),
        system_language: b"en",
        media: None,
    }
}
fn ticket(d: &mut Decoder<'_, '_>, job: &RasterJob, c: &mut Run) -> PatternTicket {
    loop {
        match d
            .step_production(1, &mut ProductionFixture::default(), job, c)
            .unwrap()
        {
            DecodeEvent::NeedPattern(r) => {
                assert_eq!(r.child.shapes.len(), 1);
                return r.ticket;
            }
            DecodeEvent::Progress { charged_units } => assert!(charged_units > 0),
            _ => panic!("missing request"),
        }
    }
}
#[test]
fn wrong_ticket_aborts_and_reclaims_only_transferred_job_output() {
    let mut c = Run::default();
    let doc = lexical(SOURCE, &mut c);
    let mut s1 = DecodeSource::new(&doc, &mut c).unwrap();
    let mut s2 = DecodeSource::new(&doc, &mut c).unwrap();
    let mut j1 = job(&doc);
    let mut j2 = job(&doc);
    let mut d1 = Decoder::from_source(&doc, &mut s1, 17, options(), &mut c).unwrap();
    let mut d2 = Decoder::from_source(&doc, &mut s2, 17, options(), &mut c).unwrap();
    let _a = ticket(&mut d1, &j1, &mut c);
    let b = ticket(&mut d2, &j2, &mut c);
    let pixels = output(&mut j1, 17, &mut c);
    let other = output(&mut j2, 17, &mut c);
    assert_eq!(
        d1.resume_pattern_production(b, &mut j1, Ok(pixels), &mut c)
            .unwrap_err()
            .kind,
        ErrorKind::InvalidArgument
    );
    assert_eq!(d1.state(), DecoderState::Failed);
    assert_eq!(j1.stats().output_bytes, 0);
    assert!(j2.slice(other.pixels).is_ok());
    assert!(d1.finish(&mut c).is_err());
    assert_eq!(
        d2.resume_pattern_production(
            b,
            &mut j2,
            Err(Diagnostic {
                kind: ErrorKind::Canceled,
                offset: 11
            }),
            &mut c
        )
        .unwrap_err()
        .offset,
        11
    );
    assert_eq!(d2.state(), DecoderState::Failed);
    j2.release(other.pixels).unwrap();
}
#[test]
fn cancellation_at_pattern_completion_releases_pixels_and_cannot_finish() {
    let mut c = Run::default();
    let doc = lexical(SOURCE, &mut c);
    let mut shared = DecodeSource::new(&doc, &mut c).unwrap();
    let mut j = job(&doc);
    let mut d = Decoder::from_source(&doc, &mut shared, 17, options(), &mut c).unwrap();
    let t = ticket(&mut d, &j, &mut c);
    let o = output(&mut j, 17, &mut c);
    c.stop_after = Some(c.units);
    assert_eq!(
        d.resume_pattern_production(t, &mut j, Ok(o), &mut c)
            .unwrap_err()
            .kind,
        ErrorKind::Canceled
    );
    assert_eq!(j.stats().output_bytes, 0);
    assert!(d.finish(&mut Run::default()).is_err());
}

#[test]
fn nested_pattern_failure_and_cancellation_unwind_every_request_and_keep_source_identity() {
    let source = Arc::new(b"<svg width='8' height='8'><defs><pattern id='outer'><rect width='2' height='2' fill='url(#inner)'/></pattern><pattern id='inner'><rect width='1' height='1' fill='red'/></pattern></defs><rect width='4' height='4' fill='url(#outer)'/></svg>".to_vec());
    for cancel in [false, true] {
        let mut control = Run::default();
        let document = LexicalDocument::parse(source.clone(), 1 << 20, &mut control).unwrap();
        let mut shared = DecodeSource::new(&document, &mut control).unwrap();
        let mut job = job(&document);
        let mut decoder =
            Decoder::from_source(&document, &mut shared, 31, options(), &mut control).unwrap();
        let mut operations = ProductionFixture::default();
        let ticket = loop {
            match ScopedProductionDecodeSession::step(
                &mut decoder,
                1,
                &job,
                &mut operations,
                &mut control,
            )
            .unwrap()
            {
                ScopedDecodeEvent::Progress => {}
                ScopedDecodeEvent::Pattern(request) => {
                    assert!(request.child().input().static_request().is_some());
                    break request.ticket();
                }
                ScopedDecodeEvent::Complete => panic!("nested fixture must suspend"),
            }
        };
        if cancel {
            let pixels = output(&mut job, 31, &mut control);
            control.stop_after = Some(control.units);
            assert_eq!(
                decoder
                    .resume_pattern_production(ticket, &mut job, Ok(pixels), &mut control)
                    .unwrap_err()
                    .kind,
                ErrorKind::Canceled
            );
        } else {
            assert_eq!(
                decoder
                    .resume_pattern_production(
                        ticket,
                        &mut job,
                        Err(Diagnostic {
                            kind: ErrorKind::Unsupported,
                            offset: 47
                        }),
                        &mut control
                    )
                    .unwrap_err()
                    .kind,
                ErrorKind::Unsupported
            );
        }
        assert_eq!(decoder.state(), DecoderState::Failed);
        assert!(Arc::ptr_eq(document.source_owner(), &source));
        assert_eq!(job.stats().output_bytes, 0);
        assert!(decoder.finish(&mut Run::default()).is_err());
    }
}
#[test]
fn explicit_definition_channels_preserve_distinct_requested_opacities() {
    let text = SOURCE.replace(
        "fill=\"url(#p)\"/>",
        "fill=\"url(#p)\" stroke=\"url(#p)\" fill-opacity=\"0.25\" stroke-opacity=\"0.75\"/>",
    );
    let (graph, requests) =
        production::run_production(&text, &mut ProductionFixture::default()).unwrap();
    assert_eq!(requests, 2);
    let root = graph.root();
    let shape = root.shapes[0];
    let fill = root.fill_paints[shape.fill_paint.unwrap()];
    let stroke = root.stroke_paints[shape.stroke_paint.unwrap()];
    assert_eq!(fill.pattern_opacity, 0.25);
    assert!((stroke.pattern_opacity - 0.75).abs() < 1e-15);
    assert_ne!(fill.pattern, stroke.pattern);
}

#[test]
fn child_draft_cancellation_sweep_rolls_back_every_admitted_draft() {
    use std::sync::Arc;

    let input = Arc::new(
        b"<svg width='8' height='8'><defs><clipPath id='c'><rect width='1' height='1'/></clipPath></defs><rect width='2' height='2' clip-path='url(#c)'/></svg>".to_vec(),
    );
    for cutoff in 540..620 {
        let mut control = Run::default();
        let document = LexicalDocument::parse(input.clone(), 1 << 20, &mut control).unwrap();
        let mut source = DecodeSource::new(&document, &mut control).unwrap();
        let job = job(&document);
        let mut decoder =
            Decoder::from_source(&document, &mut source, 17, options(), &mut control).unwrap();
        control.stop_after = Some(control.units + cutoff);
        let mut fixture = ProductionFixture::default();
        loop {
            match decoder.step_production(1, &mut fixture, &job, &mut control) {
                Ok(DecodeEvent::Progress { charged_units }) => assert!(charged_units > 0),
                Ok(DecodeEvent::Complete) => break,
                Ok(DecodeEvent::NeedPattern(_)) => panic!("clip unexpectedly requested a pattern"),
                Err(error) => {
                    assert_eq!(error.kind, ErrorKind::Canceled, "cutoff={cutoff}");
                    assert_eq!(decoder.state(), DecoderState::Failed);
                    assert!(decoder.finish(&mut Run::default()).is_err());
                    break;
                }
            }
        }
    }
}
