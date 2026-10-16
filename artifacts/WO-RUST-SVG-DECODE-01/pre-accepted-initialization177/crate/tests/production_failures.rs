#[path = "support/production.rs"]
mod production;
mod support;
use production::{output, ProductionFixture};
use std::sync::Arc;
use support::Run;
use vexel_scene::*;
use vexel_svg_decode::{ContractBlocker, Decoder, DecoderState};
const SOURCE:&str="<svg width=\"8\" height=\"8\"><defs><pattern id=\"p\"><rect width=\"2\" height=\"2\" fill=\"red\"/></pattern></defs><rect width=\"4\" height=\"4\" fill=\"url(#p)\"/></svg>";
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
fn missing_definition_channel_contract_is_named_without_equality_inference() {
    let text = SOURCE.replace(
        "fill=\"url(#p)\"/>",
        "fill=\"url(#p)\" stroke=\"url(#p)\" fill-opacity=\"0.25\" stroke-opacity=\"0.75\"/>",
    );
    let mut c = Run::default();
    let doc = lexical(&text, &mut c);
    let mut shared = DecodeSource::new(&doc, &mut c).unwrap();
    let j = job(&doc);
    let mut d = Decoder::from_source(&doc, &mut shared, 17, options(), &mut c).unwrap();
    loop {
        match d.step_production(64, &mut ProductionFixture::default(), &j, &mut c) {
            Ok(DecodeEvent::Progress { .. }) => {}
            Err(e) => {
                assert_eq!(e.kind, ErrorKind::InvalidArgument);
                break;
            }
            _ => panic!("must not choose a mapping for ambiguous channels"),
        }
    }
    assert_eq!(
        d.blocked_contract(),
        Some(ContractBlocker::DefinitionPaintChannel)
    );
    assert_eq!(d.state(), DecoderState::Failed);
}
