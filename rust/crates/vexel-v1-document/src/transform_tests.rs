use super::*;

#[test]
fn spec_matrix_composes_pivot_and_rotation() {
    let m = TransformSpec {
        rotate_deg: 90.0,
        ..Default::default()
    }
    .to_matrix();
    assert!((m[0]).abs() < 1e-12 && (m[1] - 1.0).abs() < 1e-12);
    let m = TransformSpec {
        translate: (4.0, 5.0),
        pivot: (7.0, 9.0),
        ..Default::default()
    }
    .to_matrix();
    assert_eq!(
        (
            m[0] * 7.0 + m[2] * 9.0 + m[4],
            m[1] * 7.0 + m[3] * 9.0 + m[5]
        ),
        (11.0, 14.0)
    );
}

#[test]
fn set_transform_is_one_undoable_edit_and_identity_removes_it() {
    let mut d=V1Document::from_source(b"<svg width=\"20\" height=\"20\"><rect id=\"r\" x=\"0\" y=\"0\" width=\"2\" height=\"2\"/></svg>").unwrap();
    let before = d.source().unwrap();
    let count = d.history_count();
    d.set_transform(
        "r",
        &TransformSpec {
            translate: (2.0, 3.0),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(d.history_count(), count + 1);
    assert_eq!(
        d.get_transform_matrix("r").unwrap(),
        [1., 0., 0., 1., 2., 3.]
    );
    let after = d.source().unwrap();
    d.undo().unwrap();
    assert_eq!(d.source().unwrap(), before);
    d.redo().unwrap();
    assert_eq!(d.source().unwrap(), after);
    d.set_transform("r", &TransformSpec::default()).unwrap();
    assert!(
        !String::from_utf8(d.source().unwrap())
            .unwrap()
            .contains("transform=")
    );
}

#[test]
fn parses_supported_transform_functions_and_rejects_others() {
    for (raw, expected) in [
        ("translate(2,3)", [1., 0., 0., 1., 2., 3.]),
        ("scale(2)", [2., 0., 0., 2., 0., 0.]),
        ("rotate(90)", [0., 1., -1., 0., 0., 0.]),
        ("matrix(1 0 0 1 4 5)", [1., 0., 0., 1., 4., 5.]),
    ] {
        let svg =
            format!("<svg width=\"20\" height=\"20\"><g id=\"g\" transform=\"{raw}\"/></svg>");
        let d = V1Document::from_source(svg.as_bytes()).unwrap();
        let got = d.get_transform_matrix("g").unwrap();
        for i in 0..6 {
            assert!((got[i] - expected[i]).abs() < 1e-12);
        }
    }
    let d = V1Document::from_source(
        b"<svg width=\"20\" height=\"20\"><g id=\"g\" transform=\"skewX(2)\"/></svg>",
    )
    .unwrap();
    assert_eq!(
        d.get_transform_matrix("g"),
        Err(V1DocumentError::UnsupportedTransform)
    );
}
