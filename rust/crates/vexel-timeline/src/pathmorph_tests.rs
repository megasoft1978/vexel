use super::*;

fn data(text: &str) -> Shape {
    parse(PathKind::Data, text.as_bytes()).unwrap()
}

#[test]
fn normalises_relative_and_shorthand_commands() {
    let s = data("m 10 10 h 5 v 5 l 1 1 z");
    assert_eq!(s.kinds, b"MLLLZ");
    assert_eq!(s.coords, [10.0, 10.0, 15.0, 10.0, 15.0, 15.0, 16.0, 16.0]);
    let s = data("M0 0 C 1 1 2 2 3 3 S 5 5 6 6");
    assert_eq!(s.kinds, b"MCC");
    assert_eq!(&s.coords[8..], [4.0, 4.0, 5.0, 5.0, 6.0, 6.0]);
    let s = data("M0 0 Q 2 2 4 0 T 8 0");
    assert_eq!(&s.coords[6..], [6.0, -2.0, 8.0, 0.0]);
    // Implicit repeated pairs after M are lines.
    assert_eq!(data("M0 0 1 1 2 2").kinds, b"MLL");
}

#[test]
fn rejects_arcs_and_bad_syntax() {
    assert_eq!(
        parse(PathKind::Data, b"M0 0 A 1 1 0 0 0 2 2").unwrap_err().kind,
        PathErrorKind::ArcUnsupported
    );
    assert!(parse(PathKind::Data, b"L 1 1").is_err());
    assert!(parse(PathKind::Data, b"M 1").is_err());
    assert!(parse(PathKind::Data, b"").is_err());
    assert!(parse(PathKind::Points, b"1 2 3").is_err());
}

#[test]
fn compare_names_first_mismatch() {
    let a = data("M0 0 L1 1 L2 2 Z");
    let b = data("M0 0 L1 1 C 1 1 2 2 3 3 Z");
    let e = compare(&a, &b).unwrap_err();
    assert_eq!(e.index, 2);
    assert_eq!(e.kind, PathErrorKind::CommandMismatch { expected: b'L', found: b'C' });
    assert!(e.message().contains("command 3"));
    let short = data("M0 0 L1 1");
    let e = compare(&a, &short).unwrap_err();
    assert_eq!(e.kind, PathErrorKind::CountMismatch { expected: 4, found: 2 });
}

#[test]
fn lerp_blends_every_coordinate_and_formats() {
    let a = data("M0 0 L10 0 Z");
    let b = data("M10 10 L30 0 Z");
    let mid = lerp(&a, &b, 0.5);
    assert_eq!(format(PathKind::Data, &mid), b"M 5 5 L 20 0 Z");
    let p = parse(PathKind::Points, b"0,0 10,0").unwrap();
    let q = parse(PathKind::Points, b"0 10 10 10").unwrap();
    assert_eq!(format(PathKind::Points, &lerp(&p, &q, 0.5)), b"0,5 10,5");
}
