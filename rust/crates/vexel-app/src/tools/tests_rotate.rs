use super::tests_support::*;
use super::{Key, Outcome, PointerInput, ToolController};

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

fn start_rotate(c: &mut ToolController, doc: &mut vexel_v1_document::V1Document) {
    c.select_id(doc, "r").unwrap();
    down(c, doc, &map(), (14.0, 14.0)).unwrap();
    mv(c, doc, &map(), (34.0, 14.0)).unwrap();
}

const RECT: &str = "<rect id=\"r\" x=\"20\" y=\"20\" width=\"20\" height=\"20\"/>";

#[test]
fn in_progress_rotate_exposes_the_interim_angle_and_follows_the_pointer() {
    let mut doc = with(RECT);
    let mut c = ToolController::new();
    assert_eq!(c.rotate_angle(), None);
    start_rotate(&mut c, &mut doc);
    let first = c.rotate_angle().unwrap();
    assert!(first.abs() > 1.0);
    mv(&mut c, &mut doc, &map(), (30.0, 6.0)).unwrap();
    assert_ne!(c.rotate_angle().unwrap(), first);
}

#[test]
fn escape_cancels_rotate_without_touching_the_document() {
    let mut doc = with(RECT);
    let mut c = ToolController::new();
    let (before, count) = (doc.source().unwrap(), doc.history_count());
    start_rotate(&mut c, &mut doc);
    assert_eq!(c.key(Key::Escape, &mut doc), Ok(Outcome::Cancelled));
    assert_eq!(c.rotate_angle(), None);
    assert_eq!(
        up(&mut c, &mut doc, &map(), (34.0, 14.0)),
        Ok(Outcome::Ignored)
    );
    assert_eq!(
        (doc.source().unwrap(), doc.history_count()),
        (before, count)
    );
}

#[test]
fn release_outside_the_artboard_cancels_rotate() {
    let mut doc = with(RECT);
    let mut c = ToolController::new();
    let (before, count) = (doc.source().unwrap(), doc.history_count());
    start_rotate(&mut c, &mut doc);
    let outcome = c.pointer(
        PointerInput::Up {
            x: -500.0,
            y: -500.0,
        },
        &mut doc,
        &map(),
    );
    assert_eq!(outcome, Ok(Outcome::Cancelled));
    assert_eq!(c.rotate_angle(), None);
    assert_eq!(
        (doc.source().unwrap(), doc.history_count()),
        (before, count)
    );
}

#[test]
fn stage_paint_during_rotate_differs_from_idle() {
    use crate::canvas::{StagePaint, View, paint_stage};
    use vexel_ui::{Rect, Theme, theme_tokens};
    let stage = Rect {
        x: 0,
        y: 0,
        width: 200,
        height: 160,
    };
    let tokens = theme_tokens(Theme::Light, false);
    let shot = |rotation: Option<f64>| {
        let mut px = vec![0u8; 200 * 160 * 4];
        let paint = StagePaint {
            view: View::IDENTITY,
            artboard: (120.0, 100.0),
            frame: None,
            selection: Some((40.0, 30.0, 80.0, 70.0)),
            hover: None,
            rotation,
        };
        paint_stage(&mut px, 200, 1, stage, &tokens, Some(&paint));
        px
    };
    let idle = shot(None);
    let turned = shot(Some(30.0));
    assert_ne!(idle, turned);
    assert_eq!(turned, shot(Some(30.0)), "deterministic");
    assert_ne!(turned, shot(Some(60.0)));
}
