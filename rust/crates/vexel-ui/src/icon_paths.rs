//! Frozen, owned icon taxonomy and path data on the 16-unit grid (keyline 1..15, stroke 1.5).
use crate::icons::{IconBounds, IconKind, parse_path};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IconDefinition {
    pub icon: IconKind,
    pub name: &'static str,
    pub path: &'static str,
}

#[rustfmt::skip]
static DEFINITIONS: &[IconDefinition] = &[
    IconDefinition { icon: IconKind::SELECT, name: "select", path: "M4 2 L4 13 L7 10.2 L9.2 14 L11 13 L8.8 9.2 L12.5 9 Z" },
    IconDefinition { icon: IconKind::PAN, name: "pan", path: "M8 2 L8 14 M2 8 L14 8 M8 2 L6 4 M8 2 L10 4 M8 14 L6 12 M8 14 L10 12 M2 8 L4 6 M2 8 L4 10 M14 8 L12 6 M14 8 L12 10" },
    IconDefinition { icon: IconKind::ZOOM_IN, name: "zoom_in", path: "M12 7 A5 5 0 1 1 2 7 A5 5 0 1 1 12 7 Z M10.6 10.6 L14 14 M5 7 L9 7 M7 5 L7 9" },
    IconDefinition { icon: IconKind::ZOOM_OUT, name: "zoom_out", path: "M12 7 A5 5 0 1 1 2 7 A5 5 0 1 1 12 7 Z M10.6 10.6 L14 14 M5 7 L9 7" },
    IconDefinition { icon: IconKind::RECTANGLE, name: "rectangle", path: "M3 3 L13 3 Q14 3 14 4 L14 12 Q14 13 13 13 L3 13 Q2 13 2 12 L2 4 Q2 3 3 3 Z" },
    IconDefinition { icon: IconKind::ELLIPSE, name: "ellipse", path: "M14 8 A6 4.5 0 1 1 2 8 A6 4.5 0 1 1 14 8 Z" },
    IconDefinition { icon: IconKind::LINE, name: "line", path: "M3 13 L13 3" },
    IconDefinition { icon: IconKind::PEN, name: "pen", path: "M8 14 L4 8.5 L6 5 L6 2.5 L10 2.5 L10 5 L12 8.5 Z M8 14 L8 10" },
    IconDefinition { icon: IconKind::NODE_EDIT, name: "node_edit", path: "M2 13.5 C5 13.5 6.5 10 9 7 M9 2 L14 2 L14 7 L9 7 Z" },
    IconDefinition { icon: IconKind::PENCIL, name: "pencil", path: "M2.5 13.5 L3.5 10 L11 2.5 L13.5 5 L6 12.5 Z M9 4.5 L11.5 7" },
    IconDefinition { icon: IconKind::TEXT, name: "text", path: "M3 3 L13 3 M8 3 L8 13 M6 13 L10 13" },
    IconDefinition { icon: IconKind::EYEDROPPER, name: "eyedropper", path: "M3.7 9.7 L8.7 4.7 L11.3 7.3 L6.3 12.3 L2.5 13.5 Z M13.2 2.8 A1 1 0 1 1 11.2 2.8 A1 1 0 1 1 13.2 2.8 Z" },
    IconDefinition { icon: IconKind::GRADIENT, name: "gradient", path: "M3 2 L13 2 Q14 2 14 3 L14 13 Q14 14 13 14 L3 14 Q2 14 2 13 L2 3 Q2 2 3 2 Z M4 5 L5 4 M4 9.5 L9.5 4 M6 12 L12 6" },
    IconDefinition { icon: IconKind::FILL, name: "fill", path: "M2.5 8 L7.5 3 L13 8.5 L8.5 13.5 Z M2.5 8 L13 8.5 M13.5 11 Q14.5 12.5 13.5 13.5 Q12.5 12.5 13.5 11 Z" },
    IconDefinition { icon: IconKind::STROKE, name: "stroke", path: "M2 11 C4 4 6 4 8 8 C10 12 12 12 14 5" },
    IconDefinition { icon: IconKind::LAYERS, name: "layers", path: "M8 2 L14 5 L8 8 L2 5 Z M2 8 L8 11 L14 8 M2 11 L8 14 L14 11" },
    IconDefinition { icon: IconKind::EYE, name: "eye", path: "M1.5 8 C3.5 4.5 5.5 3.5 8 3.5 C10.5 3.5 12.5 4.5 14.5 8 C12.5 11.5 10.5 12.5 8 12.5 C5.5 12.5 3.5 11.5 1.5 8 Z M10 8 A2 2 0 1 1 6 8 A2 2 0 1 1 10 8 Z" },
    IconDefinition { icon: IconKind::EYE_OFF, name: "eye_off", path: "M1.5 8 C3.5 4.5 5.5 3.5 8 3.5 C10.5 3.5 12.5 4.5 14.5 8 C12.5 11.5 10.5 12.5 8 12.5 C5.5 12.5 3.5 11.5 1.5 8 Z M2.5 13.5 L13.5 2.5" },
    IconDefinition { icon: IconKind::LOCK, name: "lock", path: "M3 7 L13 7 L13 14 L3 14 Z M5 7 L5 5 C5 3.3 6.3 2 8 2 C9.7 2 11 3.3 11 5 L11 7" },
    IconDefinition { icon: IconKind::UNLOCK, name: "unlock", path: "M3 7 L13 7 L13 14 L3 14 Z M5 7 L5 5 C5 3.3 6.3 2 8 2 C9.4 2 10.5 2.9 10.9 4" },
    IconDefinition { icon: IconKind::PLUS, name: "plus", path: "M8 3 L8 13 M3 8 L13 8" },
    IconDefinition { icon: IconKind::MINUS, name: "minus", path: "M3 8 L13 8" },
    IconDefinition { icon: IconKind::CLOSE, name: "close", path: "M4 4 L12 12 M12 4 L4 12" },
    IconDefinition { icon: IconKind::CHEVRON_RIGHT, name: "chevron_right", path: "M5.5 3.2 L10.5 8 L5.5 12.8" },
    IconDefinition { icon: IconKind::CHEVRON_DOWN, name: "chevron_down", path: "M3.2 5.5 L8 10.5 L12.8 5.5" },
    IconDefinition { icon: IconKind::MORE, name: "more", path: "M4.5 8 A1 1 0 1 1 2.5 8 A1 1 0 1 1 4.5 8 Z M9 8 A1 1 0 1 1 7 8 A1 1 0 1 1 9 8 Z M13.5 8 A1 1 0 1 1 11.5 8 A1 1 0 1 1 13.5 8 Z" },
    IconDefinition { icon: IconKind::SEARCH, name: "search", path: "M11.5 7 A4.5 4.5 0 1 1 2.5 7 A4.5 4.5 0 1 1 11.5 7 Z M10.5 10.5 L14 14" },
    IconDefinition { icon: IconKind::SAVE, name: "save", path: "M2 2 L11 2 L14 5 L14 14 L2 14 Z M5 2 L5 6 L10 6 L10 2 M5 14 L5 9 L11 9 L11 14" },
    IconDefinition { icon: IconKind::UNDO, name: "undo", path: "M5 2 L2 5 L5 8 M2 5 L9 5 C12 5 14 7 14 9.5 C14 12 12 13.5 9 13.5 L6 13.5" },
    IconDefinition { icon: IconKind::REDO, name: "redo", path: "M11 2 L14 5 L11 8 M14 5 L7 5 C4 5 2 7 2 9.5 C2 12 4 13.5 7 13.5 L10 13.5" },
    IconDefinition { icon: IconKind::DUPLICATE, name: "duplicate", path: "M6 6 L13 6 L13 13 L6 13 Z M3 10 L3 3 L10 3" },
    IconDefinition { icon: IconKind::TRASH, name: "trash", path: "M2.5 4 L13.5 4 M6 4 L6 2 L10 2 L10 4 M4 4 L4.5 14 L11.5 14 L12 4 M8 7 L8 11" },
    IconDefinition { icon: IconKind::GROUP, name: "group", path: "M5 2 L11 2 M14 5 L14 11 M11 14 L5 14 M2 11 L2 5 M5 5 L6 5 L6 6 L5 6 Z M10 10 L11 10 L11 11 L10 11 Z" },
    IconDefinition { icon: IconKind::UNGROUP, name: "ungroup", path: "M2 2 L6 2 L6 6 L2 6 Z M10 2 L14 2 L14 6 L10 6 Z M2 10 L6 10 L6 14 L2 14 Z M10 10 L14 10 L14 14 L10 14 Z" },
    IconDefinition { icon: IconKind::FIT, name: "fit", path: "M2 6 L2 2 L6 2 M10 2 L14 2 L14 6 M14 10 L14 14 L10 14 M6 14 L2 14 L2 10" },
    IconDefinition { icon: IconKind::ACTUAL_SIZE, name: "actual_size", path: "M2 5.5 L2 2 L5.5 2 M10.5 2 L14 2 L14 5.5 M14 10.5 L14 14 L10.5 14 M5.5 14 L2 14 L2 10.5 M6.5 6 L8 4.5 L8 11.5" },
    IconDefinition { icon: IconKind::RULER, name: "ruler", path: "M2 5 L14 5 L14 10 L2 10 Z M5 5 L5 7.5 M8 5 L8 7.5 M11 5 L11 7.5" },
    IconDefinition { icon: IconKind::GRID, name: "grid", path: "M2 2 L14 2 L14 14 L2 14 Z M8 2 L8 14 M2 8 L14 8" },
    IconDefinition { icon: IconKind::SNAP, name: "snap", path: "M3 2 L3 8 C3 11 5 13 8 13 C11 13 13 11 13 8 L13 2 L10 2 L10 8 C10 9.5 9 10.5 8 10.5 C7 10.5 6 9.5 6 8 L6 2 Z" },
    IconDefinition { icon: IconKind::ALIGN_LEFT, name: "align_left", path: "M2 2 L2 14 M5 4 L14 4 M5 8 L11 8 M5 12 L14 12" },
    IconDefinition { icon: IconKind::ALIGN_CENTER, name: "align_center", path: "M8 2 L8 14 M3 4 L13 4 M5 8 L11 8 M3 12 L13 12" },
    IconDefinition { icon: IconKind::ALIGN_RIGHT, name: "align_right", path: "M14 2 L14 14 M2 4 L11 4 M5 8 L11 8 M2 12 L11 12" },
    IconDefinition { icon: IconKind::ALIGN_TOP, name: "align_top", path: "M2 2 L14 2 M4 5 L4 14 M8 5 L8 11 M12 5 L12 14" },
    IconDefinition { icon: IconKind::ALIGN_MIDDLE, name: "align_middle", path: "M2 8 L14 8 M4 2 L4 14 M8 5 L8 11 M12 2 L12 14" },
    IconDefinition { icon: IconKind::ALIGN_BOTTOM, name: "align_bottom", path: "M2 14 L14 14 M4 2 L4 11 M8 5 L8 11 M12 2 L12 11" },
    IconDefinition { icon: IconKind::CIRCLE, name: "circle", path: "M14 8 A6 6 0 1 1 2 8 A6 6 0 1 1 14 8 Z" },
    IconDefinition { icon: IconKind::DOCUMENT, name: "document", path: "M3 2 L9 2 L13 6 L13 14 L3 14 Z M9 2 L9 6 L13 6 M5.5 9 L10.5 9 M5.5 11.5 L10.5 11.5" },
    IconDefinition { icon: IconKind::STOP, name: "stop", path: "M5 2 L11 2 L11 10 L8 14 L5 10 Z" },
    IconDefinition { icon: IconKind::PATH, name: "path", path: "M3 13 C3 7 6 4 13 3 M2 12 L4 12 L4 14 L2 14 Z M12 2 L14 2 L14 4 L12 4 Z" },
    IconDefinition { icon: IconKind::POLYLINE, name: "polyline", path: "M2 13 L6 6 L10 11 L14 3" },
    IconDefinition { icon: IconKind::IMAGE, name: "image", path: "M3 3 L13 3 Q14 3 14 4 L14 12 Q14 13 13 13 L3 13 Q2 13 2 12 L2 4 Q2 3 3 3 Z M2 11 L6 7 L9.5 10.5 L11.5 8.5 L14 11 M12 6 A1 1 0 1 1 10 6 A1 1 0 1 1 12 6 Z" },
    IconDefinition { icon: IconKind::SIDEBAR_LEFT, name: "sidebar_left", path: "M3 3 L13 3 Q14 3 14 4 L14 12 Q14 13 13 13 L3 13 Q2 13 2 12 L2 4 Q2 3 3 3 Z M6 3 L6 13" },
    IconDefinition { icon: IconKind::SIDEBAR_RIGHT, name: "sidebar_right", path: "M3 3 L13 3 Q14 3 14 4 L14 12 Q14 13 13 13 L3 13 Q2 13 2 12 L2 4 Q2 3 3 3 Z M10 3 L10 13" },
    IconDefinition { icon: IconKind::EXPORT, name: "export", path: "M8 10 L8 2 M5 5 L8 2 L11 5 M3 9 L3 14 L13 14 L13 9" },
    IconDefinition { icon: IconKind::PLAY, name: "play", path: "M4.5 3 L12.5 8 L4.5 13 Z" },
    IconDefinition { icon: IconKind::PAUSE, name: "pause", path: "M6 3 L6 13 M10 3 L10 13" },
    IconDefinition { icon: IconKind::STEP_BACK, name: "step_back", path: "M3 3 L3 13 M13 3 L6 8 L13 13 Z" },
    IconDefinition { icon: IconKind::STEP_FORWARD, name: "step_forward", path: "M13 3 L13 13 M3 3 L10 8 L3 13 Z" },
    IconDefinition { icon: IconKind::SKIP_START, name: "skip_start", path: "M3 3 L3 13 M8.5 3 L5 8 L8.5 13 M13.5 3 L10 8 L13.5 13" },
    IconDefinition { icon: IconKind::SKIP_END, name: "skip_end", path: "M13 3 L13 13 M7.5 3 L11 8 L7.5 13 M2.5 3 L6 8 L2.5 13" },
    IconDefinition { icon: IconKind::LOOP, name: "loop", path: "M2 7 C2 4.5 4 3 6 3 L13 3 M11 1 L13 3 L11 5 M14 9 C14 11.5 12 13 10 13 L3 13 M5 11 L3 13 L5 15" },
    IconDefinition { icon: IconKind::KEYFRAME, name: "keyframe", path: "M8 2.2 L13.5 8 L8 13.8 L2.5 8 Z" },
    IconDefinition { icon: IconKind::KEYFRAME_FILLED, name: "keyframe_filled", path: "M8 2.2 L13.5 8 L8 13.8 L2.5 8 Z" },
    IconDefinition { icon: IconKind::EASING, name: "easing", path: "M2 13 C8 13 8 3 14 3 M1.5 11.5 L4.5 11.5 L4.5 14.5 L1.5 14.5 Z" },
    IconDefinition { icon: IconKind::CURVE, name: "curve", path: "M2 2 L2 14 L14 14 M4 11 C7 11 8 5 13 4" },
    IconDefinition { icon: IconKind::TIMELINE, name: "timeline", path: "M8 3 L10 5 L8 7 L6 5 Z M2 13 L14 13" },
];

pub fn icon_definitions() -> &'static [IconDefinition] {
    DEFINITIONS
}
pub fn icon_path_count() -> usize {
    DEFINITIONS.len()
}
pub fn icon_path_at(index: usize) -> Option<&'static IconDefinition> {
    DEFINITIONS.get(index)
}
pub fn icon_path_validate(path: &str) -> Result<IconBounds, crate::IconError> {
    parse_path(path)
}
/// Every kind has one definition in enum order, unique lowercase snake-case names, a valid path,
/// and geometry inside the 1..=15 keyline of the 16-unit grid. At most 70 icons.
pub fn icon_validate_all() -> bool {
    let snake = |n: &str| !n.is_empty() && n.bytes().all(|b| b.is_ascii_lowercase() || b == b'_');
    DEFINITIONS.len() == IconKind::COUNT as usize - 1
        && DEFINITIONS.len() <= 70
        && DEFINITIONS.iter().enumerate().all(|(i, d)| {
            d.icon as usize == i + 1
                && snake(d.name)
                && icon_path_validate(d.path).is_ok_and(|b| {
                    b.left >= 1.0 && b.top >= 1.0 && b.right <= 15.0 && b.bottom <= 15.0
                })
                && DEFINITIONS[..i]
                    .iter()
                    .all(|p| p.name != d.name && p.icon != d.icon)
        })
}
