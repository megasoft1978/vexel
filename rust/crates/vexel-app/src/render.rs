use crate::chrome::{rail_glyph, status_label_rect, top_label_rect};
use crate::inspector::draw_inspector;
use crate::model::{Layout, Mode, Model, Tool, checked_pixel_geometry};
use std::cell::Cell;
use vexel_platform_apple::{RgbaFrame, TextEmphasis, TextFace, draw_text_rgba};
use vexel_ui::{
    ControlState, FontFamily, FontWeight, Rect, Theme, WidgetCanvas, WidgetDrawSpec, WidgetKind,
    widget_paint,
};

const MAX_FRAME_BYTES: usize = 64 * 1024 * 1024;

pub fn render(
    width_points: u32,
    height_points: u32,
    scale: u32,
    model: &Model,
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
    let layout = Layout::new(width_points, height_points, model.mode)
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
        c.paint_rect(layout.top, tokens.window);
        c.paint_rect(
            Rect {
                x: 0,
                y: layout.top.height.saturating_sub(1),
                width: width_points,
                height: 1,
            },
            tokens.separator,
        );
        label(
            &mut c,
            "Vexel",
            Rect {
                x: 18,
                y: 8,
                width: 86,
                height: 30,
            },
            tokens.text_primary,
            16,
            FontWeight::Semibold,
        );
        if width_points >= 1000 {
            label(
                &mut c,
                &model.chrome.top,
                top_label_rect(&layout),
                tokens.text_secondary,
                12,
                FontWeight::Regular,
            );
        }
        segment(&mut c, "Draw", layout.draw_mode, model.mode == Mode::Draw);
        segment(
            &mut c,
            "Animate",
            layout.animate_mode,
            model.mode == Mode::Animate,
        );
        if width_points >= 1000 {
            label(
                &mut c,
                "No history",
                Rect {
                    x: width_points.saturating_sub(368),
                    y: 10,
                    width: 62,
                    height: 26,
                },
                tokens.text_tertiary,
                10,
                FontWeight::Regular,
            );
            button(
                &mut c,
                "Undo",
                Rect {
                    x: width_points.saturating_sub(300),
                    y: 8,
                    width: 54,
                    height: 30,
                },
            );
            button(
                &mut c,
                "Redo",
                Rect {
                    x: width_points.saturating_sub(240),
                    y: 8,
                    width: 54,
                    height: 30,
                },
            );
            button(
                &mut c,
                "Save",
                Rect {
                    x: width_points.saturating_sub(176),
                    y: 8,
                    width: 54,
                    height: 30,
                },
            );
            button(
                &mut c,
                "Export",
                Rect {
                    x: width_points.saturating_sub(116),
                    y: 8,
                    width: 70,
                    height: 30,
                },
            );
        }

        c.paint_rect(layout.rail, tokens.panel);
        for (i, tool) in [
            Tool::Select,
            Tool::Pen,
            Tool::Rectangle,
            Tool::Ellipse,
            Tool::Line,
        ]
        .iter()
        .enumerate()
        {
            let r = layout.tools[i];
            c.paint_round(r, 5, tokens.panel_raised, tokens.separator, 1);
            let (glyph, caption) = rail_glyph(*tool);
            label(
                &mut c,
                glyph,
                Rect {
                    x: r.x + 2,
                    y: r.y + 6,
                    width: r.width.saturating_sub(4),
                    height: 22,
                },
                tokens.disabled_text,
                13,
                FontWeight::Regular,
            );
            if r.height >= 42 {
                label(
                    &mut c,
                    caption,
                    Rect {
                        x: r.x + 1,
                        y: r.y + 23,
                        width: r.width.saturating_sub(2),
                        height: 16,
                    },
                    tokens.disabled_text,
                    9,
                    FontWeight::Regular,
                );
            }
        }

        c.paint_rect(layout.stage, tokens.canvas_well);
        let stage = layout.stage;
        let mark_w = stage.width.saturating_sub(24).min(480).max(1);
        let mark_x = stage.x + stage.width.saturating_sub(mark_w) / 2;
        let center = stage.y + stage.height / 2;
        label(
            &mut c,
            "No document is open",
            Rect {
                x: mark_x,
                y: center.saturating_sub(56),
                width: mark_w,
                height: 32,
            },
            tokens.text_primary,
            21,
            FontWeight::Semibold,
        );
        label(
            &mut c,
            "Drawing and project actions are unavailable in this shell.",
            Rect {
                x: mark_x,
                y: center.saturating_sub(16),
                width: mark_w,
                height: 24,
            },
            tokens.text_secondary,
            12,
            FontWeight::Regular,
        );
        label(
            &mut c,
            "Use Draw or Animate above to switch workspaces.",
            Rect {
                x: mark_x,
                y: center + 8,
                width: mark_w,
                height: 24,
            },
            tokens.text_secondary,
            12,
            FontWeight::Regular,
        );
        if model.mode == Mode::Animate {
            draw_timeline(
                &mut c,
                layout.timeline.expect("animate layout has timeline"),
            );
        }
        if let Some(panel) = layout.inspector {
            draw_inspector(&mut c, panel, &model.chrome.inspector);
        }
        draw_activity(&mut c, layout.activity, model.status);
        c.paint_rect(layout.status, tokens.panel_raised);
        label(
            &mut c,
            "Ready",
            Rect {
                x: 14,
                y: layout.status.y + 3,
                width: 66,
                height: 22,
            },
            tokens.success,
            11,
            FontWeight::Semibold,
        );
        label(
            &mut c,
            &model.chrome.status,
            status_label_rect(&layout),
            tokens.text_secondary,
            11,
            FontWeight::Regular,
        );
        label(
            &mut c,
            if model.mode == Mode::Draw {
                "Draw workspace"
            } else {
                "Animate workspace"
            },
            Rect {
                x: width_points.saturating_sub(166),
                y: layout.status.y + 3,
                width: 152,
                height: 22,
            },
            tokens.text_secondary,
            11,
            FontWeight::Regular,
        );
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
fn segment(c: &mut WidgetCanvas<'_>, text: &str, bounds: Rect, selected: bool) {
    let mut s = WidgetDrawSpec::new(WidgetKind::ButtonSecondary, bounds);
    s.label = Some(text);
    s.state = if selected {
        ControlState::Selected
    } else {
        ControlState::Rest
    };
    let _ = widget_paint(c, &s);
}
fn button(c: &mut WidgetCanvas<'_>, text: &str, bounds: Rect) {
    let mut s = WidgetDrawSpec::new(WidgetKind::ButtonSecondary, bounds);
    s.label = Some(text);
    s.state = ControlState::Disabled;
    let _ = widget_paint(c, &s);
}

#[cfg(test)]
mod geometry_tests {
    use super::*;
    #[test]
    fn timeline_rows_are_omitted_until_their_full_bounds_fit() {
        for height in [51, 106] {
            let r = Rect {
                x: 54,
                y: 100,
                width: 386,
                height,
            };
            for row in timeline_content_rows(r).into_iter().flatten() {
                assert!(row.x >= r.x && row.y >= r.y);
                assert!(row.x + row.width <= r.x + r.width);
                assert!(row.y + row.height <= r.y + r.height);
            }
            if height == 51 {
                assert!(timeline_content_rows(r)[1].is_none());
                assert!(timeline_content_rows(r)[2].is_none());
            }
        }
    }
}
fn draw_timeline(c: &mut WidgetCanvas<'_>, r: Rect) {
    let tokens = c.tokens;
    c.paint_rect(r, tokens.panel);
    c.paint_rect(
        Rect {
            x: r.x,
            y: r.y,
            width: r.width,
            height: 1,
        },
        tokens.separator,
    );
    let rows = timeline_content_rows(r);
    if let Some(bounds) = rows[0] {
        label(
            c,
            "Timeline",
            bounds,
            tokens.text_primary,
            13,
            FontWeight::Semibold,
        );
    }
    if let Some(bounds) = rows[1] {
        label(
            c,
            "No document to animate",
            bounds,
            tokens.text_secondary,
            11,
            FontWeight::Regular,
        );
    }
    if let Some(track) = rows[2] {
        c.paint_round(track, 3, tokens.panel_raised, tokens.separator, 1);
    }
}

fn timeline_content_rows(r: Rect) -> [Option<Rect>; 3] {
    [
        (r.height >= 32).then_some(Rect {
            x: r.x + 14,
            y: r.y + 8,
            width: 94,
            height: 24,
        }),
        (r.height >= 64).then_some(Rect {
            x: r.x + 14,
            y: r.y + 42,
            width: 220,
            height: 22,
        }),
        (r.height >= 96).then_some(Rect {
            x: r.x + 14,
            y: r.y + 74,
            width: r.width.saturating_sub(28),
            height: 18,
        }),
    ]
}
fn draw_activity(c: &mut WidgetCanvas<'_>, r: Rect, status: &str) {
    let tokens = c.tokens;
    c.paint_rect(r, tokens.window);
    c.paint_rect(
        Rect {
            x: r.x,
            y: r.y,
            width: r.width,
            height: 1,
        },
        tokens.separator,
    );
    label(
        c,
        "MCP activity",
        Rect {
            x: 16,
            y: r.y + 10,
            width: 148,
            height: 22,
        },
        tokens.text_primary,
        12,
        FontWeight::Semibold,
    );
    label(
        c,
        "Unavailable in this shell",
        Rect {
            x: 16,
            y: r.y + 38,
            width: 230,
            height: 20,
        },
        tokens.disabled_text,
        11,
        FontWeight::Regular,
    );
    label(
        c,
        status,
        Rect {
            x: 250,
            y: r.y + 38,
            width: r.width.saturating_sub(390),
            height: 20,
        },
        tokens.text_secondary,
        11,
        FontWeight::Regular,
    );
    button(
        c,
        "Connect",
        Rect {
            x: r.width.saturating_sub(100),
            y: r.y + 18,
            width: 84,
            height: 30,
        },
    );
}
