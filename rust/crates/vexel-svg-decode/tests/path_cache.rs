//! The path cache returns exactly what a fresh parse would, including source
//! provenance rebased to the requesting element, and stays within its bound.
use vexel_base::Span;
use vexel_geometry::Affine;
use vexel_svg_decode::{element_geometry, path_cache::cached_bytes, source::attribute};
use vexel_xml::{XmlLimits, tokenize};

const D: &str = "M10 10 L 60.5 12 C70 20 80 30 90 40 q 3 4 5 6 z m 1 1 l 2 2";

/// `D` padded past the cache's minimum size.
fn long_d() -> String {
    format!("{D} {}", "l 1.5 2.5 ".repeat(60))
}

fn geometry_of(
    source: &str,
    t: Affine,
) -> (vexel_svg_path::PathDocument, vexel_svg_path::PathDocument) {
    let bytes = source.as_bytes();
    let (tokens, result) = tokenize(bytes, XmlLimits::default());
    let _ = result;
    let element = (0..tokens.len())
        .find(|&i| tokens[i].local_name.slice(bytes) == Some(b"path".as_slice()))
        .unwrap();
    let got = element_geometry(bytes, &tokens, element, t, [100.0, 100.0]).unwrap();
    let d = attribute(bytes, &tokens, element, b"d", None).unwrap();
    let span = Span {
        start: d.offset,
        end: d.end().unwrap(),
    };
    let fresh =
        vexel_svg_path::parse_path(bytes, span, t, vexel_svg_decode::path_options()).unwrap();
    (got.path, fresh)
}

#[test]
fn hit_is_identical_to_a_fresh_parse_at_a_shifted_offset() {
    let t = Affine {
        a: 1.25,
        b: 0.0,
        c: 0.0,
        d: 1.25,
        e: 3.0,
        f: -2.0,
    };
    let a = format!("<svg><path d=\"{}\"/></svg>", long_d());
    let b = format!(
        "<svg><g id=\"padding-moves-the-path\"/><path fill=\"red\" d=\"{}\"/></svg>",
        long_d()
    );
    let (miss, fresh_a) = geometry_of(&a, t);
    assert_eq!(miss, fresh_a);
    let before = cached_bytes();
    assert!(before > 0);
    let (hit, fresh_b) = geometry_of(&b, t);
    assert_eq!(hit, fresh_b, "rebased provenance must match a fresh parse");
    assert_ne!(hit.numbers[0].source, miss.numbers[0].source);
}

#[test]
fn transform_is_part_of_the_key() {
    let src = format!("<svg><path d=\"{} h 7\"/></svg>", long_d());
    let one = Affine {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: 1.0,
        e: 0.0,
        f: 0.0,
    };
    let two = Affine { a: 2.0, ..one };
    let (a, fa) = geometry_of(&src, one);
    let (b, fb) = geometry_of(&src, two);
    assert_eq!((a.clone(), b.clone()), (fa, fb));
    assert_ne!(a.edges, b.edges);
}

#[test]
fn cache_stays_bounded() {
    let t = Affine {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: 1.0,
        e: 0.0,
        f: 0.0,
    };
    for i in 0..4000 {
        let d: String = (0..200).map(|k| format!("L{} {} ", i + k, k * 3)).collect();
        let src = format!("<svg><path d=\"M0 0 {d}\"/></svg>");
        geometry_of(&src, t);
    }
    assert!(cached_bytes() <= 24 * 1024 * 1024);
}
