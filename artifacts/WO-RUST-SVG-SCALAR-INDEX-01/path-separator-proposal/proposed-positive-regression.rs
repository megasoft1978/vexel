// Proposed additional regression; not installed or claimed executed.
use vexel_geometry::affine_identity;
use vexel_svg_path::{ParseOptions, Span, parse_path};

#[test]
fn compact_decimal_pair_and_implicit_line_repeat_match_spaced_path() {
    let compact = b"M0 0 L1.2.3 4 5";
    let spaced = b"M0 0 L1.2 .3 L4 5";
    let a = parse_path(compact, Span { start: 0, end: compact.len() },
        affine_identity(), ParseOptions::default()).unwrap();
    let b = parse_path(spaced, Span { start: 0, end: spaced.len() },
        affine_identity(), ParseOptions::default()).unwrap();
    assert_eq!(a.numbers.iter().map(|n| n.value).collect::<Vec<_>>(),
        vec![0., 0., 1.2, 0.3, 4., 5.]);
    assert_eq!(a.edges, b.edges);
    assert_eq!(a.subpaths, b.subpaths);
    let malformed = b"M0 0 L1.2..3 4 5";
    assert!(parse_path(malformed, Span { start: 0, end: malformed.len() },
        affine_identity(), ParseOptions::default()).is_err());
}
