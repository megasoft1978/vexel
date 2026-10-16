//! Frozen, owned icon taxonomy and SVG path data from the in-repository UI source.
use crate::icons::{parse_path, IconBounds, IconKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IconDefinition { pub icon: IconKind, pub name: &'static str, pub path: &'static str }

#[rustfmt::skip]
static DEFINITIONS: &[IconDefinition] = &[
    IconDefinition { icon: IconKind::SELECT, name: "select", path: "M5 2.5 L5 20.5 L9.7 15.7 L13.1 22 L16 20.4 L12.6 14.1 L19.6 14.1 Z" },
    IconDefinition { icon: IconKind::PAN, name: "pan", path: "M12 3 L12 21 M3 12 L21 12 M12 3 L9 6 M12 3 L15 6 M12 21 L9 18 M12 21 L15 18 M3 12 L6 9 M3 12 L6 15 M21 12 L18 9 M21 12 L18 15" },
    IconDefinition { icon: IconKind::ZOOM_IN, name: "zoom_in", path: "M14.8 14.8 L21 21 M10.8 17 C7.4 17 4.6 14.2 4.6 10.8 C4.6 7.4 7.4 4.6 10.8 4.6 C14.2 4.6 17 7.4 17 10.8 C17 14.2 14.2 17 10.8 17 Z M10.8 7.5 L10.8 14.1 M7.5 10.8 L14.1 10.8" },
    IconDefinition { icon: IconKind::ZOOM_OUT, name: "zoom_out", path: "M14.8 14.8 L21 21 M10.8 17 C7.4 17 4.6 14.2 4.6 10.8 C4.6 7.4 7.4 4.6 10.8 4.6 C14.2 4.6 17 7.4 17 10.8 C17 14.2 14.2 17 10.8 17 Z M7.5 10.8 L14.1 10.8" },
    IconDefinition { icon: IconKind::RECTANGLE, name: "rectangle", path: "M4 5 L20 5 L20 19 L4 19 Z" },
    IconDefinition { icon: IconKind::ELLIPSE, name: "ellipse", path: "M12 6 C16.4 6 20 8.7 20 12 C20 15.3 16.4 18 12 18 C7.6 18 4 15.3 4 12 C4 8.7 7.6 6 12 6 Z" },
    IconDefinition { icon: IconKind::LINE, name: "line", path: "M4 20 L20 4" },
    IconDefinition { icon: IconKind::PEN, name: "pen", path: "M12 2.5 L20 10.5 L12 21.5 L4 10.5 Z M4 10.5 L20 10.5 M12 10.5 L12 21.5 M9.5 7.5 L12 10.5 L14.5 7.5" },
    IconDefinition { icon: IconKind::NODE_EDIT, name: "node_edit", path: "M4 17 C7 17 8 7 12 7 C16 7 17 17 20 17 M4 14 L4 20 M1 17 L7 17 M12 4 L12 10 M9 7 L15 7 M20 14 L20 20 M17 17 L23 17" },
    IconDefinition { icon: IconKind::PENCIL, name: "pencil", path: "M4 20 L5.5 15 L16.5 4 L20 7.5 L9 18.5 Z M14 6.5 L17.5 10" },
    IconDefinition { icon: IconKind::TEXT, name: "text", path: "M5 5 L19 5 M12 5 L12 20" },
    IconDefinition { icon: IconKind::EYEDROPPER, name: "eyedropper", path: "M14 3 L21 10 L18 13 L11 6 Z M12 7 L5 14 L5 19 L10 19 L17 12 M5 19 L10 14" },
    IconDefinition { icon: IconKind::GRADIENT, name: "gradient", path: "M4 4 L20 4 L20 20 L4 20 Z M4 8 L20 8 M4 12 L20 12 M4 16 L20 16" },
    IconDefinition { icon: IconKind::FILL, name: "fill", path: "M8 4 L17 13 L12 18 L3 9 Z M14 15 L18 15 L21 20 M4 20 L13 20" },
    IconDefinition { icon: IconKind::STROKE, name: "stroke", path: "M4 12 L20 12 M7 6 L7 18 M17 6 L17 18 M7 6 L17 6 M7 18 L17 18" },
    IconDefinition { icon: IconKind::LAYERS, name: "layers", path: "M12 3 L21 8 L12 13 L3 8 Z M3 12 L12 17 L21 12 M3 16 L12 21 L21 16" },
    IconDefinition { icon: IconKind::EYE, name: "eye", path: "M2.5 12 C5 7.5 8 5.5 12 5.5 C16 5.5 19 7.5 21.5 12 C19 16.5 16 18.5 12 18.5 C8 18.5 5 16.5 2.5 12 Z M12 9 C13.7 9 15 10.3 15 12 C15 13.7 13.7 15 12 15 C10.3 15 9 13.7 9 12 C9 10.3 10.3 9 12 9 Z" },
    IconDefinition { icon: IconKind::EYE_OFF, name: "eye_off", path: "M3 12 C5.5 7.5 8.5 5.5 12 5.5 C15.5 5.5 18.5 7.5 21 12 C18.5 16.5 15.5 18.5 12 18.5 C8.5 18.5 5.5 16.5 3 12 Z M4 4 L20 20" },
    IconDefinition { icon: IconKind::LOCK, name: "lock", path: "M5 10 L19 10 L19 21 L5 21 Z M8 10 L8 7 C8 4.8 9.8 3 12 3 C14.2 3 16 4.8 16 7 L16 10" },
    IconDefinition { icon: IconKind::UNLOCK, name: "unlock", path: "M5 10 L19 10 L19 21 L5 21 Z M8 10 L8 7 C8 4.8 9.8 3 12 3 C13.4 3 14.8 3.7 15.5 5" },
    IconDefinition { icon: IconKind::PLUS, name: "plus", path: "M12 4 L12 20 M4 12 L20 12" },
    IconDefinition { icon: IconKind::MINUS, name: "minus", path: "M4 12 L20 12" },
    IconDefinition { icon: IconKind::CLOSE, name: "close", path: "M5 5 L19 19 M19 5 L5 19" },
    IconDefinition { icon: IconKind::CHEVRON_RIGHT, name: "chevron_right", path: "M9 5 L16 12 L9 19" },
    IconDefinition { icon: IconKind::CHEVRON_DOWN, name: "chevron_down", path: "M5 9 L12 16 L19 9" },
    IconDefinition { icon: IconKind::MORE, name: "more", path: "M6 10 C7.1 10 8 10.9 8 12 C8 13.1 7.1 14 6 14 C4.9 14 4 13.1 4 12 C4 10.9 4.9 10 6 10 Z M12 10 C13.1 10 14 10.9 14 12 C14 13.1 13.1 14 12 14 C10.9 14 10 13.1 10 12 C10 10.9 10.9 10 12 10 Z M18 10 C19.1 10 20 10.9 20 12 C20 13.1 19.1 14 18 14 C16.9 14 16 13.1 16 12 C16 10.9 16.9 10 18 10 Z" },
    IconDefinition { icon: IconKind::SEARCH, name: "search", path: "M14.5 14.5 L21 21 M10 17 C6.1 17 3 13.9 3 10 C3 6.1 6.1 3 10 3 C13.9 3 17 6.1 17 10 C17 13.9 13.9 17 10 17 Z" },
    IconDefinition { icon: IconKind::SAVE, name: "save", path: "M4 3 L17 3 L21 7 L21 21 L3 21 L3 3 Z M7 3 L7 9 L16 9 L16 3 M7 21 L7 14 L17 14 L17 21" },
    IconDefinition { icon: IconKind::UNDO, name: "undo", path: "M9 7 L4 12 L9 17 M4 12 L14 12 C18 12 20 14 20 18" },
    IconDefinition { icon: IconKind::REDO, name: "redo", path: "M15 7 L20 12 L15 17 M20 12 L10 12 C6 12 4 14 4 18" },
    IconDefinition { icon: IconKind::DUPLICATE, name: "duplicate", path: "M8 8 L20 8 L20 20 L8 20 Z M4 16 L4 4 L16 4" },
    IconDefinition { icon: IconKind::TRASH, name: "trash", path: "M4 7 L20 7 M9 7 L9 4 L15 4 L15 7 M6 7 L7 20 L17 20 L18 7 M10 10 L10 17 M14 10 L14 17" },
    IconDefinition { icon: IconKind::GROUP, name: "group", path: "M4 6 L13 6 L13 15 L4 15 Z M11 9 L20 9 L20 18 L11 18 Z" },
    IconDefinition { icon: IconKind::UNGROUP, name: "ungroup", path: "M4 4 L8 4 L8 8 L4 8 Z M16 4 L20 4 L20 8 L16 8 Z M4 16 L8 16 L8 20 L4 20 Z M16 16 L20 16 L20 20 L16 20 Z M10 12 L6 8 M6 8 L6 11 M6 8 L9 8 M14 12 L18 8 M18 8 L15 8 M18 8 L18 11 M10 12 L6 16 M6 16 L9 16 M6 16 L6 13 M14 12 L18 16 M18 16 L15 16 M18 16 L18 13" },
    IconDefinition { icon: IconKind::FIT, name: "fit", path: "M3 9 L3 3 L9 3 M15 3 L21 3 L21 9 M21 15 L21 21 L15 21 M9 21 L3 21 L3 15" },
    IconDefinition { icon: IconKind::ACTUAL_SIZE, name: "actual_size", path: "M4 4 L9 4 M4 4 L4 9 M20 4 L15 4 M20 4 L20 9 M4 20 L4 15 M4 20 L9 20 M20 20 L15 20 M20 20 L20 15 M9 12 L15 12" },
    IconDefinition { icon: IconKind::RULER, name: "ruler", path: "M3 17 L17 3 L21 7 L7 21 Z M8 12 L11 15 M11 9 L14 12 M14 6 L17 9" },
    IconDefinition { icon: IconKind::GRID, name: "grid", path: "M4 4 L20 4 L20 20 L4 20 Z M9.3 4 L9.3 20 M14.7 4 L14.7 20 M4 9.3 L20 9.3 M4 14.7 L20 14.7" },
    IconDefinition { icon: IconKind::SNAP, name: "snap", path: "M5 3 L5 12 C5 16 8 19 12 19 C16 19 19 16 19 12 L19 3 L14 3 L14 11 L10 11 L10 3 Z" },
    IconDefinition { icon: IconKind::ALIGN_LEFT, name: "align_left", path: "M5 3 L5 21 M9 6 L20 6 M9 12 L16 12 M9 18 L20 18" },
    IconDefinition { icon: IconKind::ALIGN_CENTER, name: "align_center", path: "M12 3 L12 21 M5 6 L19 6 M8 12 L16 12 M5 18 L19 18" },
    IconDefinition { icon: IconKind::ALIGN_RIGHT, name: "align_right", path: "M19 3 L19 21 M4 6 L15 6 M8 12 L15 12 M4 18 L15 18" },
    IconDefinition { icon: IconKind::ALIGN_TOP, name: "align_top", path: "M3 5 L21 5 M6 9 L6 20 M12 9 L12 16 M18 9 L18 20" },
    IconDefinition { icon: IconKind::ALIGN_MIDDLE, name: "align_middle", path: "M3 12 L21 12 M6 5 L6 19 M12 8 L12 16 M18 5 L18 19" },
    IconDefinition { icon: IconKind::ALIGN_BOTTOM, name: "align_bottom", path: "M3 19 L21 19 M6 4 L6 15 M12 8 L12 15 M18 4 L18 15" },
    IconDefinition { icon: IconKind::CIRCLE, name: "circle", path: "M12 4 C16.4 4 20 7.6 20 12 C20 16.4 16.4 20 12 20 C7.6 20 4 16.4 4 12 C4 7.6 7.6 4 12 4 Z" },
    IconDefinition { icon: IconKind::DOCUMENT, name: "document", path: "M6 3 L15 3 L20 8 L20 21 L4 21 L4 3 Z M14 3 L14 9 L20 9 M8 13 L16 13 M8 17 L16 17" },
    IconDefinition { icon: IconKind::STOP, name: "stop", path: "M4 12 L20 12 M8 8 L8 16 M16 8 L16 16 M12 8 C14.2 8 16 9.8 16 12 C16 14.2 14.2 16 12 16 C9.8 16 8 14.2 8 12 C8 9.8 9.8 8 12 8 Z" },
    IconDefinition { icon: IconKind::PATH, name: "path", path: "M4 18 C6 18 6 6 12 6 C18 6 18 18 20 18 M4 15 L4 21 M1 18 L7 18 M20 15 L20 21 M17 18 L23 18" },
    IconDefinition { icon: IconKind::POLYLINE, name: "polyline", path: "M4 18 L9 7 L15 13 L20 5 M2 18 L6 18 M9 5 L9 9 M13 13 L17 13 M18 5 L22 5" },
    IconDefinition { icon: IconKind::IMAGE, name: "image", path: "M3 5 L21 5 L21 19 L3 19 Z M4 17 L10 11 L14 15 L17 12 L21 16 M7 9 C8.1 9 9 8.1 9 7 C9 5.9 8.1 5 7 5 C5.9 5 5 5.9 5 7 C5 8.1 5.9 9 7 9 Z" },
    IconDefinition { icon: IconKind::SIDEBAR_LEFT, name: "sidebar_left", path: "M4 4 L20 4 L20 20 L4 20 Z M10 4 L10 20 M7 9 L4 12 L7 15" },
    IconDefinition { icon: IconKind::SIDEBAR_RIGHT, name: "sidebar_right", path: "M4 4 L20 4 L20 20 L4 20 Z M14 4 L14 20 M17 9 L20 12 L17 15" },
];

pub fn icon_definitions() -> &'static [IconDefinition] { DEFINITIONS }
pub fn icon_path_count() -> usize { DEFINITIONS.len() }
pub fn icon_path_at(index: usize) -> Option<&'static IconDefinition> { DEFINITIONS.get(index) }
pub fn icon_path_validate(path: &str) -> Result<IconBounds, crate::IconError> { parse_path(path) }
pub fn icon_validate_all() -> bool {
    DEFINITIONS.len() == IconKind::COUNT as usize - 1 && DEFINITIONS.iter().enumerate().all(|(i,d)| {
        d.icon as usize == i+1 && !d.name.is_empty() && icon_path_validate(d.path).is_ok() &&
        DEFINITIONS[..i].iter().all(|p| p.name != d.name && p.icon != d.icon)
    })
}
