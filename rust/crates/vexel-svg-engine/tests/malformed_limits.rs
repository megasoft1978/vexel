//! Malformed input, numeric limits and cancellation through the engine and the
//! real Decoder (AC5). Every case returns a typed error: no panic, no scene.
mod common;
use common::*;
use std::sync::Arc;
use vexel_scene::*;
use vexel_svg_decode::{DecoderState, TraversalDecodeEvent};
use vexel_svg_engine::SvgEngineSession;

const VALID: &str = "<svg width='16' height='16'><g transform='translate(1 2)'>\
<path d='M0 0L8 0L8 8Z' fill='red'/><rect width='4' height='4' stroke='blue'/></g></svg>";

#[test]
fn every_truncation_of_a_valid_document_is_a_typed_error() {
    decode_ok(VALID);
    for end in 0..VALID.len() {
        // Cut on char boundaries only (the document is ASCII).
        let result = try_decode(&VALID.as_bytes()[..end], &mut Control::default());
        match result {
            Err(error) => assert!(
                matches!(error.kind, ErrorKind::Xml(_) | ErrorKind::InvalidArgument),
                "prefix {end}: unexpected kind {error:?}"
            ),
            Ok(_) => panic!("prefix of length {end} unexpectedly decoded"),
        }
    }
}

#[test]
fn truncated_open_tag_reports_xml_error_at_the_unterminated_element() {
    let svg = "<svg width='16' height='16'><rect width='8' height='8'";
    let error = decode_err(svg);
    assert_eq!(
        error.kind,
        ErrorKind::Xml(vexel_xml::XmlError::MalformedSyntax)
    );
    assert_eq!(error.offset, svg.find("<rect").unwrap());
    let mismatched =
        decode_err("<svg width='16' height='16'><g><rect width='8' height='8'/></svg>");
    assert!(matches!(mismatched.kind, ErrorKind::Xml(_)));
}

#[test]
fn non_numeric_path_data_is_a_path_error_inside_the_d_value() {
    for bad in [
        "M 0 0 L x y",
        "M 0 0 L 1",
        "L 1 2",
        "M 0 0 Q 1",
        "M 0 0 A 1 1 0 2 0 1 1",
    ] {
        let svg = format!("<svg width='16' height='16'><path d='{bad}'/></svg>");
        let error = decode_err(&svg);
        assert_eq!(error.kind, ErrorKind::Path, "{bad}: {error:?}");
        let d = svg.find(&format!("'{bad}'")).unwrap() + 1;
        assert!(
            (d..=d + bad.len()).contains(&error.offset),
            "{bad}: offset {} outside d value {d}..",
            error.offset
        );
    }
}

#[test]
fn huge_coordinates_are_rejected_with_a_typed_error() {
    let cases = [
        "<path d='M 0 0 L 1e308 1e308 L -1e308 5'/>",
        "<path d='M 0 0 L 1e7 0 L 0 1e7 Z'/>",
        "<rect x='1e300' width='1' height='1'/>",
        "<circle cx='0' cy='0' r='1e300'/>",
        "<polygon points='0,0 1e308,0 0,1e308'/>",
        "<line x1='0' y1='0' x2='1e999' y2='1'/>",
    ];
    for body in cases {
        let svg = format!("<svg width='16' height='16'>{body}</svg>");
        let error = decode_err(&svg);
        assert!(
            matches!(
                error.kind,
                ErrorKind::Path | ErrorKind::Number | ErrorKind::Limit(_)
            ),
            "{body}: {error:?}"
        );
        assert!(error.offset < svg.len(), "{body}: offset in source");
    }
}

#[test]
fn nan_and_infinite_lengths_are_number_errors_at_the_value() {
    for value in ["NaN", "nan", "inf", "-inf", "Infinity", "1e999", "-1e999"] {
        for (attr, other) in [("width", "height"), ("height", "width")] {
            let svg =
                format!("<svg width='16' height='16'><rect {attr}='{value}' {other}='8'/></svg>");
            let error = decode_err(&svg);
            assert_eq!(error.kind, ErrorKind::Number, "{attr}={value}: {error:?}");
            assert!(error.offset < svg.len(), "{attr}={value}: offset in source");
        }
    }
    let root = decode_err("<svg width='NaN' height='16'><rect width='8' height='8'/></svg>");
    assert_eq!(root.kind, ErrorKind::Number);
    let circle =
        decode_err("<svg width='16' height='16'><circle cx='0' cy='0' r='Infinity'/></svg>");
    assert_eq!(circle.kind, ErrorKind::Number);
    let polygon = decode_err("<svg width='16' height='16'><polygon points='0,0 nan,5 1,1'/></svg>");
    assert!(
        matches!(polygon.kind, ErrorKind::Number | ErrorKind::Path),
        "{polygon:?}"
    );
}

#[test]
fn cancel_at_every_work_point_returns_canceled_and_never_a_scene() {
    let mut probe = Control::default();
    try_decode(VALID.as_bytes(), &mut probe).unwrap();
    let total = probe.calls;
    assert!(total > 10, "decode performs checkpoints: {total}");
    for at in 1..=total {
        let mut control = Control::canceling_at(at);
        match try_decode(VALID.as_bytes(), &mut control) {
            Err(error) => assert_eq!(
                error.kind,
                ErrorKind::Canceled,
                "cancel at call {at}: {error:?}"
            ),
            Ok(_) => panic!("cancel at call {at} of {total} still produced a scene"),
        }
    }
}

#[test]
fn cancel_mid_step_fails_the_decoder_and_finish_cannot_publish() {
    let owner = Arc::new(VALID.as_bytes().to_vec());
    let mut control = Control::default();
    let document = LexicalDocument::parse(owner.clone(), 1 << 20, &mut control).unwrap();
    let mut source = DecodeSource::new(&document, &mut control).unwrap();
    let job = RasterJob::new(ChargeLedger::new(&owner, 1 << 24, 65536).unwrap())
        .unwrap_or_else(|_| panic!("job"));
    let mut session = SvgEngineSession::from_source(
        &document,
        &mut source,
        8,
        DecodeOptions {
            limits: Limits::default(),
            system_language: b"en",
            media: None,
        },
        &mut control,
    )
    .unwrap();
    // One quantum of progress, then cancel at the next work point.
    assert!(matches!(
        session.step(1, &job, &mut control).unwrap(),
        TraversalDecodeEvent::Progress { .. }
    ));
    assert_eq!(session.state(), DecoderState::Running);
    control.cancel_at_call = Some(control.calls + 1);
    let error = session
        .step(64, &job, &mut control)
        .err()
        .expect("canceled step");
    assert_eq!(error.kind, ErrorKind::Canceled);
    assert_eq!(session.state(), DecoderState::Failed);
    // A failed decoder never yields a scene, even with a fresh control.
    let mut fresh = Control::default();
    assert!(session.step(64, &job, &mut fresh).is_err());
    assert!(session.finish(&mut fresh).is_err());
}

#[test]
fn rejected_documents_leave_no_partial_state_for_a_following_decode() {
    // A failing decode must not poison independent sessions or leak into them.
    let bad =
        "<svg width='16' height='16'><rect width='8' height='8'/><path d='M 0 0 L x y'/></svg>";
    assert_eq!(decode_err(bad).kind, ErrorKind::Path);
    let graph = decode_ok(VALID);
    assert_eq!(graph.root().shapes.len(), 2);
}
