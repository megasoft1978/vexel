use super::*;

const BLANK: &[u8] = b"<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"100\" height=\"80\" viewBox=\"0 0 100 80\"/>";
const DRAWN: &[u8] = b"<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"100\" height=\"80\" viewBox=\"0 0 100 80\"><rect x=\"1e1\" y=\"2e1\" width=\"3e1\" height=\"4e1\" fill=\"#a1B2c3\" id=\"vexel-object-1\"/></svg>";

#[test]
fn blank_svg_is_valid_and_exact() {
    let doc = V1Document::new(100, 80).unwrap();
    assert_eq!(doc.width(), 100);
    assert_eq!(doc.height(), 80);
    assert_eq!(doc.source().unwrap(), BLANK);
    assert!(!doc.is_dirty());
    assert_eq!(doc.history_count(), 0);
}

#[test]
fn all_drag_directions_insert_identical_one_step_rectangle() {
    for (sx, sy, ex, ey) in [
        (10.0, 20.0, 40.0, 60.0),
        (40.0, 20.0, 10.0, 60.0),
        (10.0, 60.0, 40.0, 20.0),
        (40.0, 60.0, 10.0, 20.0),
    ] {
        let mut doc = V1Document::new(100, 80).unwrap();
        doc.add_rectangle(sx, sy, ex, ey, "#a1B2c3").unwrap();
        assert_eq!(doc.source().unwrap(), DRAWN);
        assert_eq!(doc.history_count(), 1);
        assert_eq!(doc.history_cursor(), 1);
        assert!(doc.is_dirty());
    }
}

#[test]
fn undo_redo_restore_byte_identical_sources_and_saved_state() {
    let mut doc = V1Document::new(100, 80).unwrap();
    let blank = doc.source().unwrap();
    doc.add_rectangle(10.0, 20.0, 40.0, 60.0, "#a1B2c3")
        .unwrap();
    doc.undo().unwrap();
    assert_eq!(doc.source().unwrap(), blank);
    assert!(!doc.is_dirty());
    doc.redo().unwrap();
    assert_eq!(doc.source().unwrap(), DRAWN);
    assert!(doc.is_dirty());
    doc.mark_saved();
    assert!(!doc.is_dirty());
    doc.undo().unwrap();
    assert!(doc.is_dirty());
}

#[test]
fn invalid_inputs_do_not_change_source_history_or_dirty_state() {
    let cases = [
        (
            f64::NAN,
            0.0,
            1.0,
            1.0,
            "#ffffff",
            V1DocumentError::InvalidGeometry,
        ),
        (
            0.0,
            0.0,
            0.0,
            1.0,
            "#ffffff",
            V1DocumentError::InvalidGeometry,
        ),
        (
            0.0,
            0.0,
            101.0,
            1.0,
            "#ffffff",
            V1DocumentError::InvalidGeometry,
        ),
        (0.0, 0.0, 1.0, 1.0, "red", V1DocumentError::InvalidColor),
        (0.0, 0.0, 1.0, 1.0, "#gggggg", V1DocumentError::InvalidColor),
    ];
    for (sx, sy, ex, ey, fill, expected) in cases {
        let mut doc = V1Document::new(100, 80).unwrap();
        let before = doc.source().unwrap();
        assert_eq!(doc.add_rectangle(sx, sy, ex, ey, fill), Err(expected));
        assert_eq!(doc.source().unwrap(), before);
        assert_eq!(doc.history_count(), 0);
        assert_eq!(doc.history_cursor(), 0);
        assert!(!doc.is_dirty());
    }
}

#[test]
fn dimensions_are_bounded() {
    assert_eq!(
        V1Document::new(0, 2).unwrap_err(),
        V1DocumentError::InvalidDimensions
    );
    assert_eq!(
        V1Document::new(2, 4097).unwrap_err(),
        V1DocumentError::InvalidDimensions
    );
    assert!(V1Document::new(4096, 4096).is_ok());
}

#[test]
fn fractional_geometry_has_fixed_scientific_values() {
    let mut doc = V1Document::new(100, 80).unwrap();
    doc.add_rectangle(1.25, 2.5, 3.75, 7.5, "#123456").unwrap();
    assert_eq!(
        doc.source().unwrap(),
        b"<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"100\" height=\"80\" viewBox=\"0 0 100 80\"><rect x=\"1.25e0\" y=\"2.5e0\" width=\"2.5e0\" height=\"5e0\" fill=\"#123456\" id=\"vexel-object-1\"/></svg>"
    );
    assert_eq!(doc.history_count(), 1);
    assert!(doc.is_dirty());
}

#[test]
fn smallest_positive_subnormal_formats_bounded_and_round_trips() {
    let tiny = f64::from_bits(1);
    let mut doc = V1Document::new(1, 1).unwrap();
    doc.add_rectangle(0.0, 0.0, tiny, 1.0, "#abcdef").unwrap();
    assert_eq!(
        doc.source().unwrap(),
        b"<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"1\" height=\"1\" viewBox=\"0 0 1 1\"><rect x=\"0e0\" y=\"0e0\" width=\"5e-324\" height=\"1e0\" fill=\"#abcdef\" id=\"vexel-object-1\"/></svg>"
    );
    let parsed: f64 = "5e-324".parse().unwrap();
    assert_eq!(parsed.to_bits(), tiny.to_bits());
    assert_eq!(doc.history_count(), 1);
    doc.undo().unwrap();
    assert_eq!(
        doc.source().unwrap(),
        b"<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"1\" height=\"1\" viewBox=\"0 0 1 1\"/>"
    );
    doc.redo().unwrap();
    assert_eq!(
        doc.source().unwrap().ends_with(
            b"width=\"5e-324\" height=\"1e0\" fill=\"#abcdef\" id=\"vexel-object-1\"/></svg>"
        ),
        true
    );
}

#[test]
fn editor_allocation_failures_are_atomic() {
    use vexel_document_edit::allocation_gate;
    let mut saw_failure = false;
    for permitted in 0..64 {
        let mut doc = V1Document::new(100, 80).unwrap();
        let before = doc.source().unwrap();
        allocation_gate(Some(permitted));
        let result = doc.add_rectangle(10.0, 20.0, 40.0, 60.0, "#ffffff");
        allocation_gate(None);
        match result {
            Err(V1DocumentError::Edit(EditError::OutOfMemory)) => {
                saw_failure = true;
                assert_eq!(doc.source().unwrap(), before);
                assert_eq!(doc.history_count(), 0);
                assert_eq!(doc.history_cursor(), 0);
                assert!(!doc.is_dirty());
            }
            Ok(()) => {
                assert!(saw_failure);
                return;
            }
            other => panic!("unexpected insertion result: {other:?}"),
        }
    }
    panic!("allocation gate did not permit the insertion within 64 steps");
}

const ELLIPSE: &[u8] = b"<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"100\" height=\"80\" viewBox=\"0 0 100 80\"><ellipse cx=\"2.5e1\" cy=\"4e1\" rx=\"1.5e1\" ry=\"2e1\" fill=\"#a1B2c3\" id=\"vexel-object-1\"/></svg>";

#[test]
fn all_drag_directions_insert_identical_one_step_ellipse() {
    for (sx, sy, ex, ey) in [
        (10.0, 20.0, 40.0, 60.0),
        (40.0, 20.0, 10.0, 60.0),
        (10.0, 60.0, 40.0, 20.0),
        (40.0, 60.0, 10.0, 20.0),
    ] {
        let mut doc = V1Document::new(100, 80).unwrap();
        doc.add_ellipse(sx, sy, ex, ey, "#a1B2c3").unwrap();
        assert_eq!(doc.source().unwrap(), ELLIPSE);
        assert_eq!(doc.history_count(), 1);
        assert_eq!(doc.history_cursor(), 1);
        assert!(doc.is_dirty());
    }
}

#[test]
fn ellipse_undo_redo_and_saved_state_restore_exact_source() {
    let mut doc = V1Document::new(100, 80).unwrap();
    let blank = doc.source().unwrap();
    doc.add_ellipse(10.0, 20.0, 40.0, 60.0, "#a1B2c3").unwrap();
    doc.mark_saved();
    assert!(!doc.is_dirty());
    doc.undo().unwrap();
    assert_eq!(doc.source().unwrap(), blank);
    assert!(doc.is_dirty());
    doc.redo().unwrap();
    assert_eq!(doc.source().unwrap(), ELLIPSE);
    assert!(!doc.is_dirty());
}

#[test]
fn invalid_ellipse_inputs_are_atomic() {
    for (sx, sy, ex, ey, fill, expected) in [
        (
            f64::NAN,
            0.0,
            1.0,
            1.0,
            "#ffffff",
            V1DocumentError::InvalidGeometry,
        ),
        (
            0.0,
            0.0,
            0.0,
            1.0,
            "#ffffff",
            V1DocumentError::InvalidGeometry,
        ),
        (
            0.0,
            0.0,
            101.0,
            1.0,
            "#ffffff",
            V1DocumentError::InvalidGeometry,
        ),
        (
            0.0,
            0.0,
            f64::from_bits(1),
            1.0,
            "#ffffff",
            V1DocumentError::InvalidGeometry,
        ),
        (
            0.0,
            0.0,
            1.0,
            f64::from_bits(1),
            "#ffffff",
            V1DocumentError::InvalidGeometry,
        ),
        (0.0, 0.0, 1.0, 1.0, "red", V1DocumentError::InvalidColor),
        (0.0, 0.0, 1.0, 1.0, "#gggggg", V1DocumentError::InvalidColor),
    ] {
        let mut doc = V1Document::new(100, 80).unwrap();
        let before = doc.source().unwrap();
        assert_eq!(doc.add_ellipse(sx, sy, ex, ey, fill), Err(expected));
        assert_eq!(doc.source().unwrap(), before);
        assert_eq!(doc.history_count(), 0);
        assert_eq!(doc.history_cursor(), 0);
        assert!(!doc.is_dirty());
    }
}

#[test]
fn animation_fragment_inserts_under_stable_shape_id_and_undoes_redoes_exactly() {
    let mut doc = V1Document::new(100, 80).unwrap();
    doc.add_rectangle(10.0, 20.0, 40.0, 60.0, "#a1B2c3")
        .unwrap();
    let shape = doc.source().unwrap();
    doc.add_animation_fragment(
        "vexel-object-1",
        b"<animate attributeName=\"x\" values=\"10;20\" dur=\"1s\"/>",
    )
    .unwrap();
    let animated = b"<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"100\" height=\"80\" viewBox=\"0 0 100 80\"><rect x=\"1e1\" y=\"2e1\" width=\"3e1\" height=\"4e1\" fill=\"#a1B2c3\" id=\"vexel-object-1\"><animate attributeName=\"x\" values=\"10;20\" dur=\"1s\"/></rect></svg>";
    assert_eq!(doc.source().unwrap(), animated);
    assert_eq!(doc.history_count(), 2);
    assert_eq!(doc.history_cursor(), 2);
    doc.undo().unwrap();
    assert_eq!(doc.source().unwrap(), shape);
    doc.redo().unwrap();
    assert_eq!(doc.source().unwrap(), animated);
}

#[test]
fn animation_fragment_rejects_missing_ids_and_non_animation_xml_atomically() {
    let mut doc = V1Document::new(100, 80).unwrap();
    doc.add_ellipse(10.0, 20.0, 40.0, 60.0, "#a1B2c3").unwrap();
    let before = doc.source().unwrap();
    let history = doc.history_count();
    assert_eq!(
        doc.add_animation_fragment("missing", b"<animate attributeName=\"x\"/>"),
        Err(V1DocumentError::MissingStableId)
    );
    assert_eq!(
        doc.add_animation_fragment("vexel-object-1", b"<circle/>"),
        Err(V1DocumentError::InvalidAnimationFragment)
    );
    assert_eq!(doc.source().unwrap(), before);
    assert_eq!(doc.history_count(), history);
    assert_eq!(doc.history_cursor(), history);
}

#[test]
fn explicit_shape_ids_are_preserved_and_can_be_animation_targets() {
    let mut doc = V1Document::new(100, 80).unwrap();
    doc.add_rectangle_with_id(10.0, 20.0, 40.0, 60.0, "#a1B2c3", "pip-body-1")
        .unwrap();
    doc.add_ellipse_with_id(50.0, 10.0, 70.0, 30.0, "#abcdef", "pip-eye-2")
        .unwrap();
    let source = doc.source().unwrap();
    assert!(
        source
            .windows(b"id=\"pip-body-1\"".len())
            .any(|window| window == b"id=\"pip-body-1\"")
    );
    assert!(
        source
            .windows(b"id=\"pip-eye-2\"".len())
            .any(|window| window == b"id=\"pip-eye-2\"")
    );
    doc.add_animation_fragment(
        "pip-body-1",
        b"<animate attributeName=\"x\" values=\"10;20\"/>",
    )
    .unwrap();
    doc.add_animation_fragment(
        "pip-eye-2",
        b"<animate attributeName=\"opacity\" values=\"1;0.5\"/>",
    )
    .unwrap();
    let animated = doc.source().unwrap();
    assert!(
        animated
            .windows(b"id=\"pip-body-1\"><animate".len())
            .any(|window| window == b"id=\"pip-body-1\"><animate")
    );
    assert!(
        animated
            .windows(b"id=\"pip-eye-2\"><animate".len())
            .any(|window| window == b"id=\"pip-eye-2\"><animate")
    );
}

#[test]
fn explicit_shape_ids_use_the_same_bounds_and_reject_duplicates() {
    let mut doc = V1Document::new(100, 80).unwrap();
    assert_eq!(
        doc.add_rectangle_with_id(1.0, 1.0, 2.0, 2.0, "#ffffff", "bad id"),
        Err(V1DocumentError::InvalidStableId)
    );
    doc.add_rectangle_with_id(1.0, 1.0, 2.0, 2.0, "#ffffff", "unique")
        .unwrap();
    assert_eq!(
        doc.add_ellipse_with_id(3.0, 3.0, 4.0, 4.0, "#ffffff", "unique"),
        Err(V1DocumentError::Edit(EditError::DuplicateId))
    );
    assert_eq!(doc.history_count(), 1);
}

#[test]
fn from_source_opens_exact_bytes_clean_with_empty_history() {
    let mut doc = V1Document::from_source(DRAWN).unwrap();
    assert_eq!((doc.width(), doc.height()), (100, 80));
    assert_eq!(doc.source().unwrap(), DRAWN);
    assert!(!doc.is_dirty());
    assert_eq!(doc.history_count(), 0);
    assert_eq!(
        doc.undo(),
        Err(V1DocumentError::Edit(EditError::InvalidOperation))
    );
}

#[test]
fn from_source_rejects_bad_roots_dimensions_and_xml() {
    for bad in [
        &b"<svg xmlns=\"http://www.w3.org/2000/svg\"/>"[..],
        b"<svg width=\"0\" height=\"5\"/>",
        b"<svg width=\"4097\" height=\"5\"/>",
        b"<svg width=\"1.5\" height=\"5\"/>",
        b"<g width=\"5\" height=\"5\"/>",
        b"<svg width=\"5\" height=\"5\">",
        b"",
    ] {
        assert!(V1Document::from_source(bad).is_err(), "{bad:?}");
    }
}
