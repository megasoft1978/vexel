use super::tests_support::*;
use super::{Outcome, ToolController};

#[test]
fn rotate_preview_and_release_create_one_undoable_entry() {
    let mut doc = with("<rect id=\"r\" x=\"20\" y=\"20\" width=\"20\" height=\"20\"/>");
    let mut c = ToolController::new();
    c.select_id(&doc, "r").unwrap();
    let map = map();
    down(&mut c, &mut doc, &map, (14.0, 14.0)).unwrap();
    mv(&mut c, &mut doc, &map, (34.0, 14.0)).unwrap();
    assert!(c.rotate_preview().is_some());
    doc.mark_saved();
    let before = doc.source().unwrap();
    let count = doc.history_count();
    assert_eq!(
        up(&mut c, &mut doc, &map, (34.0, 14.0)),
        Ok(Outcome::Committed)
    );
    assert_eq!(doc.history_count(), count + 1);
    let after = doc.source().unwrap();
    doc.undo().unwrap();
    assert_eq!(doc.source().unwrap(), before);
    doc.redo().unwrap();
    assert_eq!(doc.source().unwrap(), after);
}
