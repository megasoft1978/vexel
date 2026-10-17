//! Playback keyboard map (ANSI virtual key codes, no modifiers).
//!
//! | Key   | Action                                  |
//! |-------|-----------------------------------------|
//! | Space | play / pause                            |
//! | Home  | seek to 0 (play state kept)             |
//! | Left  | step one 1/30 s frame back (pauses)     |
//! | Right | step one 1/30 s frame on (pauses)       |
//! | L     | toggle loop / play-once                 |
//!
//! Escape, Delete, Return (tools), D/A (modes) and every Command/Control/
//! Option/Shift chord (Cmd+Z, Cmd+S, ...) are not playback keys.

use super::driver::Driver;
use super::playback::LoopClock;
use vexel_platform_apple::Modifiers;

const KEY_L: u16 = 37;
const KEY_SPACE: u16 = 49;
const KEY_HOME: u16 = 115;
const KEY_LEFT: u16 = 123;
const KEY_RIGHT: u16 = 124;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlaybackKey {
    Toggle,
    Home,
    StepBack,
    StepForward,
    ToggleLoop,
}

pub fn playback_key(code: u16, modifiers: Modifiers) -> Option<PlaybackKey> {
    if modifiers.0 != 0 {
        return None;
    }
    match code {
        KEY_SPACE => Some(PlaybackKey::Toggle),
        KEY_HOME => Some(PlaybackKey::Home),
        KEY_LEFT => Some(PlaybackKey::StepBack),
        KEY_RIGHT => Some(PlaybackKey::StepForward),
        KEY_L => Some(PlaybackKey::ToggleLoop),
        _ => None,
    }
}

/// Applies `key` to the driver. Ignored (false) while a text field or a tool
/// owns the keyboard, or when the document has no animation.
pub fn apply<C: LoopClock>(
    driver: &mut Driver<C>,
    key: PlaybackKey,
    text_focus: bool,
    tool_owns_keys: bool,
) -> bool {
    if text_focus || tool_owns_keys || !driver.strip_visible() {
        return false;
    }
    match key {
        PlaybackKey::Toggle => driver.toggle(),
        PlaybackKey::Home => driver.seek(0.0),
        PlaybackKey::StepBack => driver.step(false),
        PlaybackKey::StepForward => driver.step(true),
        PlaybackKey::ToggleLoop => {
            driver.toggle_looping();
            true
        }
    }
}
