use super::EditKey;
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
