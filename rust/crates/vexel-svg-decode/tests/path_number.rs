use vexel_base::Span;
use vexel_geometry::affine_identity;
use vexel_svg_decode::map_path_error;
use vexel_svg_path::{ParseOptions, parse_path};

fn span(source: &[u8], needle: &[u8]) -> Span {
    let at = source
        .windows(needle.len())
        .position(|v| v == needle)
        .unwrap();
    Span {
        start: at,
        end: at + needle.len(),
    }
}

#[test]
fn original_compact_and_separated_path_number_assertions() {
    let compact = b"<svg width='8' height='8'><path d='M-.4.6L1.2.4Z'/></svg>";
    let separated = b"<svg width='8' height='8'><path d='M-0.4 0.6 L1.2 0.4 Z'/></svg>";
    let a_span = span(compact, b"M-.4.6L1.2.4Z");
    let b_span = span(separated, b"M-0.4 0.6 L1.2 0.4 Z");
    let a = parse_path(compact, a_span, affine_identity(), ParseOptions::default()).unwrap();
    let b = parse_path(
        separated,
        b_span,
        affine_identity(),
        ParseOptions::default(),
    )
    .unwrap();
    let expected = [-0.4, 0.6, 1.2, 0.4];
    let literals: [&[u8]; 4] = [b"-.4", b".6", b"1.2", b".4"];
    assert_eq!(a.numbers.len(), 4);
    assert_eq!(b.numbers.len(), 4);
    assert_eq!(a.edges.len(), b.edges.len());
    let mut from = a_span.start;
    for (i, n) in a.numbers.iter().enumerate() {
        let at = compact[from..a_span.end]
            .windows(literals[i].len())
            .position(|v| v == literals[i])
            .unwrap()
            + from;
        assert_eq!(
            n.source,
            Span {
                start: at,
                end: at + literals[i].len()
            }
        );
        assert_eq!(&compact[n.source.start..n.source.end], literals[i]);
        assert!((n.value - expected[i]).abs() < 1e-12);
        assert!((b.numbers[i].value - n.value).abs() < 1e-12);
        from = at + literals[i].len();
    }
    for (x, y) in a.edges.iter().zip(&b.edges) {
        for (u, v) in [
            (x.a.x, y.a.x),
            (x.a.y, y.a.y),
            (x.b.x, y.b.x),
            (x.b.y, y.b.y),
        ] {
            assert!((u - v).abs() < 1e-12)
        }
    }
}

#[test]
fn original_malformed_path_offset_assertion() {
    let source = b"<svg width='8' height='8'><path d='M.4..6'/></svg>";
    let data = span(source, b"M.4..6");
    let expected = span(source, b".4..6").start;
    let error = parse_path(source, data, affine_identity(), ParseOptions::default()).unwrap_err();
    assert_eq!(map_path_error(error).offset, expected);
}
