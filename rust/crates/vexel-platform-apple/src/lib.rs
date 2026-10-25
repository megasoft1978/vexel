//! Safe boundary primitives for the native Apple implementation.
//!
//! This crate deliberately exposes no Objective-C pointer or fabricated
//! thread marker. Native calls and inbound callbacks should enter through
//! [`guard_native_call`], with callback adapters returning their ABI-valid
//! fallback after observing an error.

#[cfg(target_os = "macos")]
pub mod gpu_compute;
mod main_thread;
mod native_boundary;
mod native_window;
mod panels;
mod text_layout;
#[cfg(all(any(test, feature = "test-support"), target_os = "macos"))]
pub mod video_reader;
#[cfg(target_os = "macos")]
mod video_writer;

#[cfg(target_os = "macos")]
pub use text_layout::{
    MAX_FAMILIES, MAX_FAMILY_BYTES, MAX_GLYPHS, MAX_PATH_SEGMENTS, MAX_TEXT_BYTES, TextCommand,
    TextGlyph, TextLayout, TextLayoutError, TextRequest, TextStyle, TextVerb, text_layout,
};

#[cfg(target_os = "macos")]
pub use video_writer::{MAX_VIDEO_DIMENSION, VideoCodec, VideoError, VideoSpec, VideoWriter};

pub use panels::{
    MAX_PANEL_PATH, NativePanels, PROJECT_EXTENSION, PanelBackend, PanelResult, RecoveryChoice,
    RecoveryText, UnsavedChoice, UnsavedText, checked_path, confirm_recovery, confirm_unsaved,
    open_panel, save_panel, show_error, with_extension, with_project_extension,
};

pub use main_thread::{MainThread, MainThreadError};
pub use native_boundary::{NativeFault, NativeFaultKind, guard_native_call};
pub use native_window::{
    Composition, Cursor, EventBatch, MAX_MENU_ITEMS, MenuItemSpec, Modifiers, NativeApp, Point,
    PointerEvent, PointerKind, QueueFault, QueueFaultReason, Rect, RgbaFrame, RgbaFrameError,
    ScrollEvent, Size, SizeU32, TextAttributeKind, TextAttributeSpan, TextEmphasis, TextError,
    TextFace, TextInputState, Window, WindowConfig, WindowError, WindowEvent, WindowId,
    draw_text_rgba,
};
