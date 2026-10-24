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

mod compose {
    use super::super::frame::{apply, shape_frame};
    use super::*;
    use vexel_v1_document::{TransformSpec, V1Document};

    /// Presses outside the top-left corner of the selection frame, turns the
    /// pointer by `delta` degrees about the pivot and releases.
    /// Returns the angle the gesture committed.
    fn rotate_by(c: &mut ToolController, doc: &mut V1Document, delta: f64) -> f64 {
        c.select_id(doc, "r").unwrap();
        let frame = shape_frame(doc, "r").unwrap();
        let (cx, cy) = frame.centre;
        let down_at = (frame.bounds.0 - 6.0, frame.bounds.1 - 6.0);
        let initial = (down_at.1 - cy).atan2(down_at.0 - cx);
        let radius = (down_at.0 - cx).hypot(down_at.1 - cy);
        let end_angle = initial + delta.to_radians();
        let end = (cx + radius * end_angle.cos(), cy + radius * end_angle.sin());
        let m = map();
        down(c, doc, &m, down_at).unwrap();
        mv(c, doc, &m, end).unwrap();
        let angle = c.rotate_angle().expect("rotate gesture is live");
        assert_eq!(up(c, doc, &m, end), Ok(Outcome::Committed));
        angle
    }

    fn matrix_angle(m: &[f64; 6]) -> f64 {
        m[1].atan2(m[0]).to_degrees()
    }

    fn assert_near(a: f64, b: f64) {
        assert!((a - b).abs() < 0.05, "{a} != {b}");
    }

    #[test]
    fn rotating_twice_adds_the_angles_and_keeps_the_centre() {
        let mut doc = with(RECT);
        let mut c = ToolController::new();
        let first = rotate_by(&mut c, &mut doc, 30.0);
        let one = doc.get_transform_matrix("r").unwrap();
        assert_near(matrix_angle(&one), first);
        let count = doc.history_count();
        let second = rotate_by(&mut c, &mut doc, 25.0);
        assert_eq!(doc.history_count(), count + 1, "one undoable edit");
        let two = doc.get_transform_matrix("r").unwrap();
        assert_near(matrix_angle(&two), first + second);
        assert_near(two[0].hypot(two[1]), 1.0);
        let centre = apply(&two, (30.0, 30.0));
        assert_near(centre.0, 30.0);
        assert_near(centre.1, 30.0);
        doc.undo().unwrap();
        assert_eq!(doc.get_transform_matrix("r").unwrap(), one);
    }

    #[test]
    fn rotating_a_scaled_shape_keeps_its_scale_and_centre() {
        let mut doc = with(RECT);
        doc.set_transform(
            "r",
            &TransformSpec {
                scale: (2.0, 2.0),
                ..TransformSpec::default()
            },
        )
        .unwrap();
        let mut c = ToolController::new();
        c.set_shift(true);
        let angle = rotate_by(&mut c, &mut doc, 40.0);
        assert_eq!(angle % 15.0, 0.0, "shift snaps to 15 degrees");
        let m = doc.get_transform_matrix("r").unwrap();
        assert_near(m[0].hypot(m[1]), 2.0);
        assert_near(m[2].hypot(m[3]), 2.0);
        assert_near(matrix_angle(&m), angle);
        // The unscaled centre (30, 30) sits at (60, 60) and stays there.
        let centre = apply(&m, (30.0, 30.0));
        assert_near(centre.0, 60.0);
        assert_near(centre.1, 60.0);
    }

    #[test]
    fn rotating_a_translated_shape_pivots_on_its_visible_centre() {
        let mut doc = with(
            "<rect id=\"r\" x=\"20\" y=\"20\" width=\"20\" height=\"20\" transform=\"translate(40 10)\"/>",
        );
        let mut c = ToolController::new();
        rotate_by(&mut c, &mut doc, 45.0);
        let m = doc.get_transform_matrix("r").unwrap();
        let centre = apply(&m, (30.0, 30.0));
        assert_near(centre.0, 70.0);
        assert_near(centre.1, 40.0);
    }
}
