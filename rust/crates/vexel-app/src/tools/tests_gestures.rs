//! Handle grab size at several zoom levels and arrow-key nudging.

use super::tests_support::*;
use super::{CanvasMap, Key, Outcome, PointerInput, ScaleMap, ToolController, tool_key};
use vexel_platform_apple::Modifiers;

const RECT: &str = "<rect id=\"r\" x=\"60\" y=\"30\" width=\"60\" height=\"40\" fill=\"#000000\"/>";

/// Presses `px` canvas pixels right of the bottom-right handle (120, 70),
/// drags 30 more pixels right and releases. True when the width changed.
fn resize_from(zoom: f64, px: f64) -> bool {
    let mut doc = with(RECT);
    let mut c = ToolController::new();
    c.select_id(&doc, "r").unwrap();
    let m = ScaleMap::new((10.0, 20.0), zoom, (200.0, 100.0)).unwrap();
    let (x, y) = (10.0 + 120.0 * zoom + px, 20.0 + 70.0 * zoom);
    assert!(m.to_doc(x + 30.0, y).is_some());
    let before = doc.source().unwrap();
    for input in [
        PointerInput::Down { x, y },
        PointerInput::Move { x: x + 30.0, y },
        PointerInput::Up { x: x + 30.0, y },
    ] {
        c.pointer(input, &mut doc, &m).unwrap();
    }
    doc.source().unwrap() != before
}

#[test]
fn resize_handles_grab_within_six_screen_pixels_at_any_zoom() {
    for zoom in [0.5, 1.0, 4.0] {
        assert!(resize_from(zoom, 0.0), "on the handle at {zoom}");
        assert!(resize_from(zoom, 5.0), "5 px away at {zoom}");
        assert!(!resize_from(zoom, 9.0), "9 px away at {zoom}");
    }
}

#[test]
fn nudge_moves_one_unit_or_ten_as_one_undoable_edit() {
    let mut doc = with(RECT);
    let mut c = ToolController::new();
    assert_eq!(c.key(Key::Nudge(1, 0), &mut doc), Ok(Outcome::Ignored));
    c.select_id(&doc, "r").unwrap();
    let plain = tool_key(124, Modifiers(0), true);
    assert_eq!(plain, Some(Key::Nudge(1, 0)));
    let shifted = tool_key(125, Modifiers(Modifiers::SHIFT), true);
    assert_eq!(shifted, Some(Key::Nudge(0, 10)));
    assert_eq!(tool_key(123, Modifiers(0), true), Some(Key::Nudge(-1, 0)));
    assert_eq!(tool_key(126, Modifiers(0), true), Some(Key::Nudge(0, -1)));
    assert_eq!(tool_key(124, Modifiers(0), false), None);
    assert_eq!(tool_key(124, Modifiers(Modifiers::COMMAND), true), None);
    let bounds = |d: &vexel_v1_document::V1Document| d.geometry_bounds("r").unwrap();
    assert_eq!(bounds(&doc), (60.0, 30.0, 120.0, 70.0));
    let count = doc.history_count();
    assert_eq!(c.key(plain.unwrap(), &mut doc), Ok(Outcome::Committed));
    assert_eq!(c.key(shifted.unwrap(), &mut doc), Ok(Outcome::Committed));
    assert_eq!(bounds(&doc), (61.0, 40.0, 121.0, 80.0));
    assert_eq!(doc.history_count(), count + 2);
    doc.undo().unwrap();
    assert_eq!(bounds(&doc), (61.0, 30.0, 121.0, 70.0));
}
