mod support;
use std::sync::Arc;
use support::Run;
use vexel_scene::*;
use vexel_svg_decode::{Decoder, DecoderState};
#[test]
fn original_post_close_metadata_and_authored_numbers_survive_actual_use_instances() {
    let input="<svg width=\"16\" height=\"16\"><defs><path id=\"p\" d=\"M1 1 C2 2 3 3 4 4 Z S6 6 7 7\"/></defs><use href=\"#p\"/><use href=\"#p\"/></svg>";
    let mut c = Run::default();
    let doc = LexicalDocument::parse(Arc::new(input.as_bytes().to_vec()), 1 << 20, &mut c).unwrap();
    let t = doc
        .tokens()
        .iter()
        .position(|v| v.local_name.slice(doc.source()) == Some(b"path"))
        .unwrap();
    let span =
        vexel_svg_decode::source::attribute(doc.source(), doc.tokens(), t, b"d", None).unwrap();
    let original = vexel_svg_path::parse_path(
        doc.source(),
        Span {
            start: span.offset,
            end: span.end().unwrap(),
        },
        vexel_geometry::affine_identity(),
        Default::default(),
    )
    .unwrap();
    let mut shared = DecodeSource::new(&doc, &mut c).unwrap();
    let mut d = Decoder::from_source(
        &doc,
        &mut shared,
        1,
        DecodeOptions {
            limits: Limits::default(),
            system_language: b"en",
            media: None,
        },
        &mut c,
    )
    .unwrap();
    while d.state() == DecoderState::Running {
        d.step(1, &mut support::RejectOperations, &mut c).unwrap();
    }
    let graph = d.finish(&mut c).unwrap();
    let r = graph.root();
    assert_eq!(r.shapes.len(), 2);
    assert_eq!(&r.path_numbers[..original.numbers.len()], &original.numbers);
    assert_eq!(&r.path_numbers[original.numbers.len()..], &original.numbers);
    for k in 0..2 {
        for (i, expected) in original.subpaths.iter().enumerate() {
            let actual = r.path_subpaths[k * original.subpaths.len() + i];
            assert_eq!(
                actual.first_edge,
                expected.first_edge + k * original.edges.len()
            );
            assert_eq!(actual.edge_count, expected.edge_count);
            assert_eq!(actual.closed, expected.closed);
        }
        for (i, expected) in original.segments.iter().enumerate() {
            let actual = &r.path_segments[k * original.segments.len() + i];
            let mut expected = *expected;
            expected.first_edge += k * original.edges.len();
            expected.first_number += k * original.numbers.len();
            assert_eq!(actual.path, expected);
        }
    }
}
