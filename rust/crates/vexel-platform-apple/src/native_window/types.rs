use std::ops::Range;

use crate::NativeFault;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct WindowId {
    pub slot: u32,
    pub generation: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QueueFaultReason {
    InputOverflow,
    TextInputStateMissing,
    InvalidTextRange,
    AllocationFailure,
    NativeCallbackFailure,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QueueFault {
    pub reason: QueueFaultReason,
    pub coalesced_or_dropped: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowError {
    ExpiredOwner,
    ExpiredWindow,
    InvalidConfig,
    InvalidTextInputState,
    PresentationUnavailable,
    QueueFault,
    Native(NativeFault),
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Size {
    pub width: f64,
    pub height: f64,
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub origin: Point,
    pub size: Size,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SizeU32 {
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cursor {
    Arrow,
    OpenHand,
    ClosedHand,
    ResizeNorthSouth,
    ResizeEastWest,
    ResizeNorthwestSoutheast,
    ResizeNortheastSouthwest,
    Rotate,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Modifiers(pub u32);
impl Modifiers {
    pub const SHIFT: u32 = 1;
    pub const COMMAND: u32 = 2;
    pub const OPTION: u32 = 4;
    pub const CONTROL: u32 = 8;
    pub const fn from_native_bits(bits: u64) -> Self {
        let mut out = 0;
        if bits & (1 << 17) != 0 {
            out |= Self::SHIFT;
        }
        if bits & (1 << 20) != 0 {
            out |= Self::COMMAND;
        }
        if bits & (1 << 19) != 0 {
            out |= Self::OPTION;
        }
        if bits & (1 << 18) != 0 {
            out |= Self::CONTROL;
        }
        Self(out)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointerKind {
    Move,
    Down,
    Up,
}
#[derive(Clone, Debug, PartialEq)]
pub struct PointerEvent {
    pub kind: PointerKind,
    pub scale: f64,
    pub logical: (i32, i32),
    pub at: Point,
    pub button: Option<u8>,
    pub click_count: u8,
    pub modifiers: Modifiers,
    pub timestamp_seconds: f64,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ScrollEvent {
    pub logical: (i32, i32),
    pub at: Point,
    pub delta: Point,
    pub scale: f64,
    pub modifiers: Modifiers,
    pub timestamp_seconds: f64,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Composition {
    pub text: String,
    pub selection_utf8: Range<usize>,
    pub replacement_utf8: Option<Range<usize>>,
    pub attributes: Vec<TextAttributeSpan>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextAttributeSpan {
    pub range_utf8: Range<usize>,
    pub kind: TextAttributeKind,
    pub value: i64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextAttributeKind {
    Underline,
    ClauseSegment,
    MarkedClause,
}
#[derive(Clone, Debug, PartialEq)]
pub struct TextInputState {
    pub source_window: WindowId,
    pub text: String,
    pub selection_utf8: Range<usize>,
    pub marked_utf8: Option<Range<usize>>,
    pub marked_attributes: Vec<TextAttributeSpan>,
    pub candidate_rect_points: Rect,
}

#[derive(Clone, Debug, PartialEq)]
pub enum WindowEvent {
    Open,
    Focus(bool),
    AppearanceChanged {
        dark: bool,
    },
    Resize {
        points: Size,
        pixels: SizeU32,
    },
    BackingScaleChanged {
        scale: f64,
        pixels: SizeU32,
    },
    KeyDown {
        code: u16,
        modifiers: Modifiers,
        timestamp_seconds: f64,
    },
    KeyUp {
        code: u16,
        modifiers: Modifiers,
        timestamp_seconds: f64,
    },
    Pointer(PointerEvent),
    Scroll(ScrollEvent),
    Magnify {
        logical: (i32, i32),
        at: Point,
        amount: f64,
        scale: f64,
        modifiers: Modifiers,
        timestamp_seconds: f64,
    },
    TextCommitted(String),
    TextCommittedWithReplacement {
        text: String,
        replacement_utf8: Range<usize>,
    },
    TextCommand(String),
    Composition(Composition),
    CompositionEnded,
    InputReset {
        sequence: u64,
        all_keys_up: bool,
        all_buttons_up: bool,
    },
    CloseRequested,
    /// File menu item activated; the value is the application-chosen tag.
    MenuCommand(u32),
    AnimationTick {
        monotonic_seconds: f64,
    },
    QueueFault(QueueFault),
}
#[derive(Clone, Debug, PartialEq)]
pub struct EventBatch {
    pub events: Vec<WindowEvent>,
    pub pending: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WindowConfig {
    pub title: String,
    pub width_points: u32,
    pub height_points: u32,
    pub resizable: bool,
    pub initially_hidden: bool,
}
impl WindowConfig {
    pub(crate) fn valid(&self) -> bool {
        !self.title.is_empty()
            && self.title.len() <= 1024
            && !self.title.contains('\0')
            && (1..=4096).contains(&self.width_points)
            && (1..=4096).contains(&self.height_points)
    }
}
pub(crate) fn validate_text_state(id: WindowId, state: &TextInputState) -> bool {
    fn range_ok(text: &str, range: &Range<usize>) -> bool {
        range.start <= range.end
            && range.end <= text.len()
            && text.is_char_boundary(range.start)
            && text.is_char_boundary(range.end)
    }
    state.source_window == id
        && state.text.len() <= 64 * 1024
        && range_ok(&state.text, &state.selection_utf8)
        && state
            .marked_utf8
            .as_ref()
            .is_none_or(|r| range_ok(&state.text, r))
        && state
            .text
            .len()
            .checked_add(
                state
                    .marked_attributes
                    .len()
                    .saturating_mul(std::mem::size_of::<TextAttributeSpan>()),
            )
            .is_some_and(|n| n <= 64 * 1024)
        && state.marked_attributes.len() <= 256
        && state.marked_attributes.iter().all(|a| {
            range_ok(&state.text, &a.range_utf8)
                && (0..=65535).contains(&a.value)
                && state
                    .marked_utf8
                    .as_ref()
                    .is_some_and(|m| m.start <= a.range_utf8.start && a.range_utf8.end <= m.end)
        })
        && state.candidate_rect_points.origin.x.is_finite()
        && state.candidate_rect_points.origin.y.is_finite()
        && state.candidate_rect_points.origin.x.abs() <= 1.0e6
        && state.candidate_rect_points.origin.y.abs() <= 1.0e6
        && state.candidate_rect_points.size.width.is_finite()
        && state.candidate_rect_points.size.height.is_finite()
        && state.candidate_rect_points.size.width >= 0.0
        && state.candidate_rect_points.size.height >= 0.0
        && state.candidate_rect_points.size.width <= 1.0e6
        && state.candidate_rect_points.size.height <= 1.0e6
}
