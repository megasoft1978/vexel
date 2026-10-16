use vexel_geometry::{Affine, Point, affine_identity};
use vexel_scene::ErrorKind;
use vexel_svg_decode::*;
use vexel_xml::XmlSpan;

#[test]
fn span_point_and_transformed_tolerance_boundaries() {
    let src = b"x-coordinate";
    assert!(span_eq(
        src,
        XmlSpan {
            offset: 2,
            length: 10
        },
        b"coordinate"
    ));
    assert!(!span_eq(
        src,
        XmlSpan {
            offset: usize::MAX,
            length: 8
        },
        b"coordinate"
    ));
    assert!(point_bounded(Point {
        x: 1_000_000.0,
        y: -1_000_000.0
    }));
    assert!(!point_bounded(Point {
        x: f64::INFINITY,
        y: 0.0
    }));
    assert!((transformed_flatness_tolerance(affine_identity()) - 0.25).abs() < 1e-12);
    let scaled = Affine {
        a: 2.0,
        b: 0.0,
        c: 0.0,
        d: 2.0,
        e: 0.0,
        f: 0.0,
    };
    assert!((transformed_flatness_tolerance(scaled) - 0.125).abs() < 1e-12);
}
#[test]
fn original_path_number_and_arc_flag_boundaries() {
    assert_eq!(read_arc_flag(b"01", 0), Ok((false, 1)));
    assert_eq!(read_arc_flag(b"01", 1), Ok((true, 2)));
    assert_eq!(read_arc_flag(b"2", 0).unwrap_err().kind, ErrorKind::Path);
    assert_eq!(
        read_number(
            b"-.4",
            XmlSpan {
                offset: 0,
                length: 3
            }
        )
        .unwrap(),
        -0.4
    );
    assert_eq!(
        read_number(
            b"xx.4..6",
            XmlSpan {
                offset: 2,
                length: 5
            }
        )
        .unwrap_err()
        .offset,
        4
    );
    assert_eq!(
        read_number_list(
            b"1, 2 3 4",
            XmlSpan {
                offset: 0,
                length: 8
            },
            4
        )
        .unwrap(),
        [1.0, 2.0, 3.0, 4.0]
    );
    assert_eq!(
        read_length(
            b"50%",
            XmlSpan {
                offset: 0,
                length: 3
            },
            20.0
        )
        .unwrap(),
        10.0
    );
    assert!(
        read_dimension(
            b"50%",
            XmlSpan {
                offset: 0,
                length: 3
            }
        )
        .is_err()
    );
    assert_eq!(skip_number_space(b" \t\n,2", 0), 3);
    assert!(next_number_separator(b"1,2", 1));
    assert!(!next_number_separator(b"12", 1));
}
#[test]
fn preserve_aspect_ratio_and_viewbox_original_fingerprints() {
    let meet = parse_preserve_aspect(
        b"xMidYMid meet",
        XmlSpan {
            offset: 0,
            length: 13,
        },
    )
    .unwrap();
    let matrix = viewbox_transform(0.0, 0.0, 10.0, 5.0, 0.0, 0.0, 20.0, 20.0, meet).unwrap();
    assert_eq!(
        (matrix.a, matrix.d, matrix.e, matrix.f),
        (2.0, 2.0, 0.0, 5.0)
    );
    let slice = parse_preserve_aspect(
        b"xMaxYMin slice",
        XmlSpan {
            offset: 0,
            length: 14,
        },
    )
    .unwrap();
    let matrix = viewbox_transform(0.0, 0.0, 10.0, 5.0, 0.0, 0.0, 20.0, 20.0, slice).unwrap();
    assert_eq!((matrix.a, matrix.e, matrix.f), (4.0, -20.0, 0.0));
    assert!(viewbox_transform(0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 2.0, 2.0, meet).is_err());
}
