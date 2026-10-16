use super::types::{Point, Size, SizeU32};

pub(crate) fn pixels(points: Size, scale: f64) -> Option<SizeU32> {
    if !points.width.is_finite() || !points.height.is_finite() || !scale.is_finite()
        || !(1.0..=3.0).contains(&scale) || scale.fract() != 0.0 || points.width < 1.0 || points.height < 1.0 { return None; }
    let w = (points.width * scale).round(); let h = (points.height * scale).round();
    if !w.is_finite() || !h.is_finite() || w < 1.0 || h < 1.0 || w > u32::MAX as f64 || h > u32::MAX as f64 { return None; }
    Some(SizeU32 { width: w as u32, height: h as u32 })
}
pub(crate) fn top_left(local_bottom_left: Point, height: f64) -> Option<Point> {
    let y = height - local_bottom_left.y;
    (local_bottom_left.x.is_finite() && local_bottom_left.y.is_finite() && height.is_finite() && y.is_finite()).then_some(Point { x: local_bottom_left.x, y })
}
pub(crate) fn checked_screen_origin(display: Rect, cocoa_screen: Rect, content: Rect) -> Option<Point> {
    let x = display.origin.x + content.origin.x - cocoa_screen.origin.x;
    let y = display.origin.y + cocoa_screen.origin.y + cocoa_screen.size.height - (content.origin.y + content.size.height);
    [x, y].iter().all(|v| v.is_finite() && v.abs() <= 1.0e9).then_some(Point { x, y })
}

use super::types::Rect;
