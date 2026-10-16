use super::{route::RouteState, types::*};
use objc2::msg_send;
use objc2_app_kit::{NSEvent, NSView};
pub(crate) fn modifiers(native_flags: u64) -> Modifiers {
    Modifiers::from_native_bits(native_flags)
}
pub(crate) fn finite_point(x: f64, y: f64) -> Option<Point> {
    (x.is_finite() && y.is_finite() && x.abs() <= 1.0e6 && y.abs() <= 1.0e6)
        .then_some(Point { x, y })
}
fn location(route: &RouteState, view: &NSView, event: &NSEvent) -> Option<(Point, f64, f64)> {
    let p = view.convertPoint_fromView(event.locationInWindow(), None);
    let point = super::geometry::top_left(Point { x: p.x, y: p.y }, view.bounds().size.height)?;
    let point = finite_point(point.x, point.y)?;
    let (window, _) = route.native()?;
    let scale = super::geometry::scale(&window)?;
    Some((point, scale, route.timestamp(event.timestamp())?))
}
pub(crate) fn key(route: &RouteState, event: &NSEvent, down: bool) {
    let Some(timestamp_seconds) = route.timestamp(event.timestamp()) else {
        return;
    };
    let code = event.keyCode();
    let modifiers = modifiers(event.modifierFlags().bits() as u64);
    route.emit(
        if down {
            WindowEvent::KeyDown {
                code,
                modifiers,
                timestamp_seconds,
            }
        } else {
            WindowEvent::KeyUp {
                code,
                modifiers,
                timestamp_seconds,
            }
        },
        false,
    );
}
pub(crate) fn pointer(route: &RouteState, view: &NSView, event: &NSEvent, kind: PointerKind) {
    let Some((at, scale, timestamp_seconds)) = location(route, view, event) else {
        route.fail(QueueFaultReason::InputOverflow);
        return;
    };
    let button = if kind == PointerKind::Move {
        None
    } else {
        let Ok(button) = u8::try_from(event.buttonNumber()) else {
            route.fail(QueueFaultReason::InputOverflow);
            return;
        };
        Some(button)
    };
    let click_count = event.clickCount().clamp(0, 255) as u8;
    route.emit(
        WindowEvent::Pointer(PointerEvent {
            kind,
            scale,
            at,
            logical: (
                at.x.round().clamp(i32::MIN as f64, i32::MAX as f64) as i32,
                at.y.round().clamp(i32::MIN as f64, i32::MAX as f64) as i32,
            ),
            button,
            click_count,
            modifiers: modifiers(event.modifierFlags().bits() as u64),
            timestamp_seconds,
        }),
        false,
    );
}
pub(crate) fn scroll(route: &RouteState, view: &NSView, event: &NSEvent) {
    let Some((at, scale, timestamp_seconds)) = location(route, view, event) else {
        route.fail(QueueFaultReason::InputOverflow);
        return;
    };
    // SAFETY: baseline NSEvent CGFloat getters have f64 ABI on Apple Silicon.
    let delta = unsafe {
        Point {
            x: msg_send![event, scrollingDeltaX],
            y: msg_send![event, scrollingDeltaY],
        }
    };
    if !delta.x.is_finite()
        || !delta.y.is_finite()
        || delta.x.abs() > 10000.0
        || delta.y.abs() > 10000.0
    {
        route.fail(QueueFaultReason::InputOverflow);
        return;
    }
    route.emit(
        WindowEvent::Scroll(ScrollEvent {
            logical: (at.x.round() as i32, at.y.round() as i32),
            at,
            delta,
            scale,
            modifiers: modifiers(event.modifierFlags().bits() as u64),
            timestamp_seconds,
        }),
        false,
    );
}
pub(crate) fn magnify(route: &RouteState, view: &NSView, event: &NSEvent) {
    let Some((at, scale, timestamp_seconds)) = location(route, view, event) else {
        route.fail(QueueFaultReason::InputOverflow);
        return;
    }; // SAFETY: baseline NSEvent magnification getter returns CGFloat=f64.
    let amount: f64 = unsafe { msg_send![event, magnification] };
    if !amount.is_finite() || amount.abs() > 100.0 {
        route.fail(QueueFaultReason::InputOverflow);
        return;
    }
    route.emit(
        WindowEvent::Magnify {
            logical: (at.x.round() as i32, at.y.round() as i32),
            at,
            amount,
            scale,
            modifiers: modifiers(event.modifierFlags().bits() as u64),
            timestamp_seconds,
        },
        false,
    );
}
