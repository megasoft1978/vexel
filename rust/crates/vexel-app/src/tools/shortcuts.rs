use super::{EditKey, Key};
use vexel_platform_apple::Modifiers;

/// Maps native Command editing chords to document structure operations.
pub fn edit_tool_key(code: u16, modifiers: Modifiers) -> Option<EditKey> {
    let m = modifiers.0;
    if m & Modifiers::COMMAND == 0 || m & (Modifiers::CONTROL | Modifiers::OPTION) != 0 {
        return None;
    }
    let shift = m & Modifiers::SHIFT != 0;
    match (code, shift) {
        (5, false) => Some(EditKey::Group),
        (5, true) => Some(EditKey::Ungroup),
        (2, false) => Some(EditKey::Duplicate),
        (30, false) => Some(EditKey::Forward),
        (33, false) => Some(EditKey::Backward),
        (30, true) => Some(EditKey::Front),
        (33, true) => Some(EditKey::Back),
        _ => None,
    }
}

/// Escape, Delete and Return for the tools, and (with `nudge`) the arrow
/// keys: 1 document unit, 10 with Shift. Command/Control/Option chords are
/// not tool keys.
pub fn tool_key(code: u16, modifiers: Modifiers, nudge: bool) -> Option<Key> {
    if modifiers.0 & (Modifiers::COMMAND | Modifiers::CONTROL | Modifiers::OPTION) != 0 {
        return None;
    }
    let step = if modifiers.0 & Modifiers::SHIFT != 0 {
        10
    } else {
        1
    };
    match code {
        53 => Some(Key::Escape),
        51 | 117 => Some(Key::Delete),
        36 | 76 => Some(Key::Enter),
        123 if nudge => Some(Key::Nudge(-step, 0)),
        124 if nudge => Some(Key::Nudge(step, 0)),
        126 if nudge => Some(Key::Nudge(0, -step)),
        125 if nudge => Some(Key::Nudge(0, step)),
        _ => None,
    }
}
