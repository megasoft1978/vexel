use crate::inspector::draw_inspector;
use crate::layout::paint::centred;
use crate::layout::{draw_chrome_with, draw_popover};
use crate::model::{Model, checked_pixel_geometry};
use crate::motion::MotionFrame;
use std::cell::Cell;
use vexel_platform_apple::{RgbaFrame, TextEmphasis, TextFace, draw_text_rgba};
use vexel_ui::{FontFamily, FontWeight, Rect, Theme, WidgetCanvas};

const MAX_FRAME_BYTES: usize = 64 * 1024 * 1024;

pub fn render(
    width_points: u32,
    height_points: u32,
    scale: u32,
    model: &Model,
) -> Result<RgbaFrame, &'static str> {
    render_with(
        width_points,
        height_points,
        scale,
        model,
        &MotionFrame::rest(model.dark, model.mode),
    )
}

/// [`render`] with the interpolated hover/press/focus visuals of the UI
/// micro-transitions (the rest frame reproduces the static look exactly).
pub fn render_with(
    width_points: u32,
    height_points: u32,
    scale: u32,
    model: &Model,
    motion: &MotionFrame,
) -> Result<RgbaFrame, &'static str> {
    let (width, height) = checked_pixel_geometry((width_points, height_points), scale)
        .ok_or("window pixels exceed checked scale or frame limits")?;
    let stride = (width as usize).checked_mul(4).ok_or("row byte overflow")?;
    let len = stride
        .checked_mul(height as usize)
        .filter(|n| *n <= MAX_FRAME_BYTES)
        .ok_or("window pixels exceed the 64 MiB frame limit")?;
    let mut pixels = Vec::new();
    pixels
        .try_reserve_exact(len)
        .map_err(|_| "frame allocation failed")?;
    pixels.resize(len, 0);
    let layout = model
        .layout(width_points, height_points)
        .ok_or("window is too small for the shell layout")?;
    let theme = if model.dark {
        Theme::Dark
    } else {
        Theme::Light
    };
    let text_failed = Cell::new(false);
    {
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
            if result.is_err() {
                text_failed.set(true);
            }
            result.is_ok()
        };
        let mut c = WidgetCanvas::new(
            &mut pixels,
            len,
            width,
            height,
            stride,
            scale,
            theme,
            false,
            Some(&mut text),
        )
        .ok_or("UI painter rejected checked frame geometry")?;
        let tokens = c.tokens;
        c.rgba.fill(0);
        c.paint_rect(
            Rect {
                x: 0,
                y: 0,
                width: width_points,
                height: height_points,
            },
            tokens.window,
        );
        draw_chrome_with(&mut c, &layout, model, Some(motion));
        draw_stage_empty(&mut c, layout.stage);
        if let Some(footer) = layout.footer {
            crate::timeline::paint(&mut c, footer, &model.timeline);
        }
        draw_inspector(&mut c, &layout, model, motion);
        draw_popover(&mut c, &layout, model);
    }
    if text_failed.get() {
        return Err("CoreText rejected bounded workbench text");
    }
    RgbaFrame::new(width, height, scale as f64, pixels)
        .map_err(|_| "native frame constructor rejected geometry")
}
pub(crate) fn label(
    c: &mut WidgetCanvas<'_>,
    text: &str,
    bounds: Rect,
    color: u32,
    size: u32,
    weight: FontWeight,
) {
    let _ = c.paint_text(text, bounds, color, FontFamily::System, weight, size);
}
/// Stage placeholder; a composed document is blitted over it by the shell.
fn draw_stage_empty(c: &mut WidgetCanvas<'_>, stage: Rect) {
    let tokens = c.tokens;
    c.paint_rect(stage, crate::canvas::well_fill(&tokens));
    let mut edge = |x, y, width, height| {
        c.paint_rect(
            Rect {
                x,
                y,
                width,
                height,
            },
            tokens.border,
        )
    };
    edge(stage.x, stage.y, stage.width, 1);
    edge(stage.x, stage.y + stage.height - 1, stage.width, 1);
    edge(stage.x, stage.y, 1, stage.height);
    edge(stage.x + stage.width - 1, stage.y, 1, stage.height);
    let w = stage.width.saturating_sub(24).min(480).max(1);
    let x = stage.x + stage.width.saturating_sub(w) / 2;
    let mid = stage.y + stage.height / 2;
    let row = |y: u32, h: u32| Rect {
        x,
        y,
        width: w,
        height: h,
    };
    centred(
        c,
        "No document open",
        row(mid.saturating_sub(40), 32),
        tokens.text_primary,
        20,
        FontWeight::Semibold,
    );
    centred(
        c,
        "Use File > Open to load an SVG or project.",
        row(mid.saturating_sub(4), 24),
        tokens.text_secondary,
        13,
        FontWeight::Regular,
    );
}
