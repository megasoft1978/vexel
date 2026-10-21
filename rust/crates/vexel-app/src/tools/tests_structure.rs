use super::tests_support::*;
use super::{EditKey, Outcome, ToolController};
use vexel_v1_document::ElementKind;

#[test]
fn duplicate_offsets_and_round_trips_two_entries() {
    let mut doc = with("<rect id=\"a\" x=\"10\" y=\"10\" width=\"20\" height=\"20\"/>");
    let mut c = ToolController::new();
    c.select_id(&doc, "a").unwrap();
    let before = doc.source().unwrap();
    assert_eq!(
        c.edit_key(EditKey::Duplicate, &mut doc),
        Ok(Outcome::Committed)
    );
    assert_eq!(doc.history_count(), 2);
    let duplicate = c.selection().unwrap().to_owned();
    assert_eq!(
        doc.geometry_bounds(&duplicate).unwrap(),
        (22.0, 22.0, 42.0, 42.0)
    );
    doc.undo().unwrap();
    doc.undo().unwrap();
    assert_eq!(doc.source().unwrap(), before);
    doc.redo().unwrap();
    doc.redo().unwrap();
    assert_eq!(
        doc.geometry_bounds(&duplicate).unwrap(),
        (22.0, 22.0, 42.0, 42.0)
    );
}

#[test]
fn group_selection_is_unit_and_second_click_enters_child() {
    let mut doc = with("<g id=\"g\"><rect id=\"child\" x=\"10\" y=\"10\" width=\"20\" height=\"20\"/><rect id=\"child2\" x=\"40\" y=\"10\" width=\"20\" height=\"20\"/></g>");
    let mut c = ToolController::new();
    assert_eq!(click(&mut c, &mut doc, (15.0, 15.0)), Ok(Outcome::Updated));
    assert_eq!(c.selection(), Some("g"));
    assert_eq!(click(&mut c, &mut doc, (15.0, 15.0)), Ok(Outcome::Updated));
    assert_eq!(c.selection(), Some("child"));
    assert_eq!(c.key(super::Key::Escape, &mut doc), Ok(Outcome::Updated));
    assert_eq!(c.selection(), None);
}

#[test]
fn group_shortcut_round_trips_and_root_group_is_ignored() {
    let mut doc = with("<rect id=\"a\" x=\"1\" y=\"1\" width=\"5\" height=\"5\"/><rect id=\"b\" x=\"10\" y=\"1\" width=\"5\" height=\"5\"/>");
    let mut c = ToolController::new();
    c.select_id(&doc, "a").unwrap();
    let before = doc.source().unwrap();
    assert_eq!(c.edit_key(EditKey::Group, &mut doc), Ok(Outcome::Ignored));
    assert_eq!(doc.source().unwrap(), before);
    assert_eq!(doc.element_get("a").unwrap().kind, ElementKind::Rect);
}
