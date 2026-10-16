//! Safe boundary primitives for the native Apple implementation.
//!
//! This crate deliberately exposes no Objective-C pointer or fabricated
//! thread marker. Native calls and inbound callbacks should enter through
//! [`guard_native_call`], with callback adapters returning their ABI-valid
//! fallback after observing an error.

mod main_thread;
mod native_boundary;
mod native_window;

pub use main_thread::{MainThread, MainThreadError};
pub use native_boundary::{NativeFault, NativeFaultKind, guard_native_call};
pub use native_window::{Composition, Cursor, EventBatch, Modifiers, NativeApp, Point, PointerKind, PointerEvent, QueueFault, QueueFaultReason, Rect, RgbaFrame, RgbaFrameError, ScrollEvent, Size, SizeU32, TextAttributeKind, TextAttributeSpan, TextInputState, Window, WindowConfig, WindowError, WindowEvent, WindowId, draw_text_rgba, TextEmphasis, TextError, TextFace};
