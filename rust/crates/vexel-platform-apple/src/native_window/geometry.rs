use super::types::{Point, Size, SizeU32};

pub(crate) fn pixels(points: Size, scale: f64) -> Option<SizeU32> {
    if !points.width.is_finite()
        || !points.height.is_finite()
        || !scale.is_finite()
        || !(1.0..=3.0).contains(&scale)
        || scale.fract() != 0.0
        || points.width < 1.0
        || points.height < 1.0
    {
        return None;
    }
    let w = (points.width * scale).round();
    let h = (points.height * scale).round();
    if !w.is_finite()
        || !h.is_finite()
        || w < 1.0
        || h < 1.0
        || w > u32::MAX as f64
        || h > u32::MAX as f64
    {
        return None;
    }
    Some(SizeU32 {
        width: w as u32,
        height: h as u32,
    })
}
pub(crate) fn top_left(local_bottom_left: Point, height: f64) -> Option<Point> {
    let y = height - local_bottom_left.y;
    (local_bottom_left.x.is_finite()
        && local_bottom_left.y.is_finite()
        && height.is_finite()
        && y.is_finite())
    .then_some(Point {
        x: local_bottom_left.x,
        y,
    })
}
pub(crate) fn checked_screen_origin(
    display: Rect,
    cocoa_screen: Rect,
    content: Rect,
) -> Option<Point> {
    let x = display.origin.x + content.origin.x - cocoa_screen.origin.x;
    let y = display.origin.y + cocoa_screen.origin.y + cocoa_screen.size.height
        - (content.origin.y + content.size.height);
    [x, y]
        .iter()
        .all(|v| v.is_finite() && v.abs() <= 1.0e9)
        .then_some(Point { x, y })
}

use super::types::Rect;

use super::types::WindowError;
use objc2_app_kit::{
    NSAppearanceCustomization, NSAppearanceNameAqua, NSAppearanceNameDarkAqua, NSView, NSWindow,
};
use objc2_foundation::{NSArray, NSNumber, NSPoint, NSRect, NSSize, NSString};

pub(crate) fn ns_rect(rect: Rect) -> NSRect {
    NSRect::new(
        NSPoint::new(rect.origin.x, rect.origin.y),
        NSSize::new(rect.size.width, rect.size.height),
    )
}
pub(crate) fn rect(rect: NSRect) -> Rect {
    Rect {
        origin: Point {
            x: rect.origin.x,
            y: rect.origin.y,
        },
        size: Size {
            width: rect.size.width,
            height: rect.size.height,
        },
    }
}
pub(crate) fn scale(window: &NSWindow) -> Option<f64> {
    let b = window.convertRectToBacking(NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(1.0, 1.0)));
    (b.size.width == b.size.height
        && pixels(
            Size {
                width: 1.0,
                height: 1.0,
            },
            b.size.width,
        )
        .is_some())
    .then_some(b.size.width)
}
pub(crate) fn screen_origin(window: &NSWindow, view: &NSView) -> Result<Point, WindowError> {
    let screen = window.screen().ok_or(WindowError::InvalidConfig)?;
    let description = screen.deviceDescription();
    let key = NSString::from_str("NSScreenNumber");
    let number = description
        .objectForKey(&key)
        .ok_or(WindowError::InvalidConfig)?;
    let number = number
        .downcast::<NSNumber>()
        .map_err(|_| WindowError::InvalidConfig)?;
    let display = number.unsignedIntValue();
    if display == 0 {
        return Err(WindowError::InvalidConfig);
    }
    // Raw CoreGraphics call is not covered by objc2 catch-all; CGDisplayBounds
    // uses a value-only ABI and does not raise Objective-C exceptions.
    let quartz = objc2_core_graphics::CGDisplayBounds(display);
    if !quartz.size.width.is_finite()
        || !quartz.size.height.is_finite()
        || quartz.size.width <= 0.0
        || quartz.size.height <= 0.0
    {
        return Err(WindowError::InvalidConfig);
    }
    let content = window.convertRectToScreen(view.convertRect_toView(view.bounds(), None));
    display_origin(
        Some(Rect {
            origin: Point {
                x: quartz.origin.x,
                y: quartz.origin.y,
            },
            size: Size {
                width: quartz.size.width,
                height: quartz.size.height,
            },
        }),
        rect(screen.frame()),
        rect(content),
    )
    .ok_or(WindowError::InvalidConfig)
}
pub(crate) fn dark(window: &NSWindow) -> bool {
    // SAFETY: baseline AppKit process-lifetime NSString constants, available on macOS13.
    let names = unsafe { NSArray::from_slice(&[NSAppearanceNameAqua, NSAppearanceNameDarkAqua]) };
    window
        .effectiveAppearance()
        .bestMatchFromAppearancesWithNames(&names)
        .is_some_and(|name| unsafe { name.isEqualToString(NSAppearanceNameDarkAqua) })
}
pub(crate) fn candidate_to_screen(window: &NSWindow, view: &NSView, candidate: Rect) -> NSRect {
    let local = ns_rect(candidate_local(candidate, view.bounds().size.height));
    window.convertRectToScreen(view.convertRect_toView(local, None))
}
pub(crate) fn screen_to_content(window: &NSWindow, view: &NSView, point: NSPoint) -> Option<Point> {
    let local = view.convertPoint_fromView(window.convertPointFromScreen(point), None);
    top_left(
        Point {
            x: local.x,
            y: local.y,
        },
        view.bounds().size.height,
    )
    .filter(|p| p.x.abs() <= 1.0e6 && p.y.abs() <= 1.0e6)
}

pub(crate) fn display_origin(display: Option<Rect>, cocoa: Rect, content: Rect) -> Option<Point> {
    let display = display?;
    if !display.size.width.is_finite()
        || !display.size.height.is_finite()
        || display.size.width <= 0.0
        || display.size.height <= 0.0
    {
        return None;
    }
    checked_screen_origin(display, cocoa, content)
}

pub(crate) fn candidate_local(candidate: Rect, height: f64) -> Rect {
    Rect {
        origin: Point {
            x: candidate.origin.x,
            y: height - candidate.origin.y - candidate.size.height,
        },
        size: candidate.size,
    }
}
