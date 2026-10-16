//! Owned port of src/platform/macos/platform_cursor.m (24 point artwork).
use super::types::Cursor;
use crate::MainThread;
use objc2::{AllocAnyThread, msg_send, rc::Retained};
use objc2_app_kit::{NSBezierPath, NSColor, NSCursor, NSImage};
use objc2_foundation::{NSPoint, NSSize};

pub(crate) fn arrow_geometry(x: f64, y: f64, dx: f64, dy: f64) -> [(f64, f64); 4] {
    let angle = dy.atan2(dx);
    [
        (x, y),
        (x + dx, y + dy),
        (
            x + (angle + 0.65).cos() * 4.0,
            y + (angle + 0.65).sin() * 4.0,
        ),
        (
            x + (angle - 0.65).cos() * 4.0,
            y + (angle - 0.65).sin() * 4.0,
        ),
    ]
}
fn arrow(descending: bool) -> Retained<NSBezierPath> {
    let p = arrow_geometry(
        4.0,
        if descending { 4.0 } else { 20.0 },
        16.0,
        if descending { 16.0 } else { -16.0 },
    );
    let path = NSBezierPath::bezierPath();
    path.moveToPoint(NSPoint::new(p[0].0, p[0].1));
    path.lineToPoint(NSPoint::new(p[1].0, p[1].1));
    for endpoint in &p[2..] {
        path.moveToPoint(NSPoint::new(p[0].0, p[0].1));
        path.lineToPoint(NSPoint::new(endpoint.0, endpoint.1));
    }
    path
}
#[allow(deprecated)] // Exact owned legacy drawing recipe; valid on the macOS13 baseline.
pub(crate) fn custom(_mt: MainThread, kind: Cursor) -> Retained<NSCursor> {
    let image = NSImage::initWithSize(NSImage::alloc(), NSSize::new(24.0, 24.0));
    image.lockFocus();
    // Always unlock the drawing focus, including an objc2-translated exception.
    struct Focus<'a> {
        image: &'a NSImage,
        armed: bool,
    }
    impl Drop for Focus<'_> {
        fn drop(&mut self) {
            if self.armed {
                let _ = crate::guard_native_call(|| self.image.unlockFocus());
            }
        }
    }
    let mut focus = Focus {
        image: &image,
        armed: true,
    };
    for (color, width) in [(NSColor::whiteColor(), 4.0), (NSColor::blackColor(), 1.8)] {
        color.setStroke();
        let path = if kind == Cursor::Rotate {
            let p = NSBezierPath::bezierPath(); // SAFETY: baseline selector, NSPoint plus CGFloat=f64 and BOOL ABIs.
            unsafe {
                let _: () = msg_send![&*p,appendBezierPathWithArcWithCenter:NSPoint::new(12.0,12.0),radius:7.0f64,startAngle:35.0f64,endAngle:305.0f64,clockwise:false];
            }
            p
        } else {
            arrow(kind == Cursor::ResizeNorthwestSoutheast)
        };
        // SAFETY: baseline setLineWidth: accepts CGFloat=f64 on Apple Silicon.
        unsafe {
            let _: () = msg_send![&*path,setLineWidth:width];
        }
        path.stroke();
    }
    if kind == Cursor::Rotate {
        NSColor::blackColor().setFill();
        let head = NSBezierPath::bezierPath();
        head.moveToPoint(NSPoint::new(20.0, 18.0));
        head.lineToPoint(NSPoint::new(15.0, 18.0));
        head.lineToPoint(NSPoint::new(18.0, 14.0));
        head.closePath();
        head.fill();
    }
    focus.armed = false;
    image.unlockFocus();
    drop(focus);
    NSCursor::initWithImage_hotSpot(NSCursor::alloc(), &image, NSPoint::new(12.0, 12.0))
}
#[allow(deprecated)] // Baseline macOS13 cursors; replacement selectors require newer macOS.
pub(crate) fn select(mt: MainThread, kind: Cursor) -> Retained<NSCursor> {
    match kind {
        Cursor::Arrow => NSCursor::arrowCursor(),
        Cursor::OpenHand => NSCursor::openHandCursor(),
        Cursor::ClosedHand => NSCursor::closedHandCursor(),
        Cursor::ResizeNorthSouth => NSCursor::resizeUpDownCursor(),
        Cursor::ResizeEastWest => NSCursor::resizeLeftRightCursor(),
        other => custom(mt, other),
    }
}
