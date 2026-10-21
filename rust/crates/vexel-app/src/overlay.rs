//! The tooltip layer, painted over the finished window frame (after the stage
//! blit, so a rail tooltip can extend over the canvas well).

use crate::layout::{Layout, draw_tooltip};
use crate::model::{Model, Target};
use std::cell::Cell;
use vexel_platform_apple::{RgbaFrame, TextEmphasis, TextFace, draw_text_rgba};
use vexel_ui::{FontFamily, FontWeight, Rect, Theme, WidgetCanvas};

/// `frame` with the tooltip of `target` painted; the frame unchanged when
/// there is nothing to paint (MCP popover open, no tooltip for the target).
pub fn overlay_tooltip(
    frame: RgbaFrame,
    model: &Model,
    layout: &Layout,
    target: Option<Target>,
) -> RgbaFrame {
    let Some(target) = target.filter(|_| !model.ui.mcp_open) else {
        return frame;
    };
    let (width, height, scale) = (frame.width(), frame.height(), frame.backing_scale() as u32);
    let mut pixels = frame.pixels().to_vec();
    let (len, stride) = (pixels.len(), width as usize * 4);
    let theme = if model.dark {
        Theme::Dark
    } else {
        Theme::Light
    };
    let failed = Cell::new(false);
    let mut text = |s: &str,
                    rect: Rect,
                    _theme: Theme,
                    size: u32,
                    color: u32,
                    family: FontFamily,
                    weight: FontWeight,
                    rgba: &mut [u8],
                    w: u32,
                    h: u32,
                    stride: usize| {
        let result = draw_text_rgba(
            rgba,
            w,
            h,
            stride,
            scale,
            s,
            (rect.x, rect.y, rect.width, rect.height),
            color,
            if family == FontFamily::Monospaced {
                TextFace::Monospaced
            } else {
                TextFace::System
            },
            if weight == FontWeight::Semibold {
                TextEmphasis::Semibold
            } else {
                TextEmphasis::Regular
            },
            size,
        );
        failed.set(failed.get() || result.is_err());
        result.is_ok()
    };
    let canvas = WidgetCanvas::new(
        &mut pixels,
        len,
        width,
        height,
        stride,
        scale,
        theme,
        false,
        Some(&mut text),
    );
    let Some(mut canvas) = canvas else {
        return frame;
    };
    if matches!(target, Target::NativeTimeline(_)) {
        draw_native_timeline_tooltip(&mut canvas, layout, target);
    } else {
        draw_tooltip(&mut canvas, layout, target);
    }
    RgbaFrame::new(width, height, f64::from(scale), pixels).unwrap_or(frame)
}

fn draw_native_timeline_tooltip(canvas: &mut WidgetCanvas<'_>, layout: &Layout, target: Target) {
    let Some((name, shortcut)) = crate::layout::tooltip(target) else {
        return;
    };
    let Some(rect) = crate::layout::tooltip_rect(layout, target) else {
        return;
    };
    crate::layout::paint::paint_tip(canvas, rect, name, shortcut);
}
