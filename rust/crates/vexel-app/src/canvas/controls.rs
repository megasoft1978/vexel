//! The zoom controls cluster (minus, label, plus, Fit) at the bottom-left of
//! the well, as a rect list with hit targets and tooltip text. Rects are in
//! stage-local logical points.

use vexel_platform_apple::{draw_text_rgba, TextEmphasis, TextFace};
use vexel_ui::{FontFamily, FontWeight, Rect, Theme, Tokens, WidgetCanvas};

pub const MARGIN: u32 = 8;
pub const TARGET: u32 = 28;
const PAD: u32 = 2;
const LABEL_WIDTH: u32 = 52;
const FIT_WIDTH: u32 = 40;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlKind {
    ZoomOut,
    /// Click resets to 100%.
    Label,
    ZoomIn,
    Fit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Control {
    pub kind: ControlKind,
    pub rect: Rect,
    pub tooltip: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Controls {
    pub container: Rect,
    pub items: [Control; 4],
    /// `NNN%`, no decimals.
    pub label: String,
}

/// The cluster for a stage of `width` x `height` points, or `None` when the
/// stage is too small to hold it.
pub fn controls(width: u32, height: u32, percent: u32) -> Option<Controls> {
    let inner = TARGET + LABEL_WIDTH + TARGET + FIT_WIDTH;
    let (w, h) = (inner + 2 * PAD, TARGET + 2 * PAD);
    if width < w + 2 * MARGIN || height < h + 2 * MARGIN {
        return None;
    }
    let container = Rect {
        x: MARGIN,
        y: height - MARGIN - h,
        width: w,
        height: h,
    };
    let mut x = container.x + PAD;
    let mut item = |kind, width, tooltip| {
        let rect = Rect {
            x,
            y: container.y + PAD,
            width,
            height: TARGET,
        };
        x += width;
        Control {
            kind,
            rect,
            tooltip,
        }
    };
    let items = [
        item(ControlKind::ZoomOut, TARGET, "Zoom out"),
        item(ControlKind::Label, LABEL_WIDTH, "Actual size (Cmd+1)"),
        item(ControlKind::ZoomIn, TARGET, "Zoom in"),
        item(ControlKind::Fit, FIT_WIDTH, "Fit to window (Cmd+0)"),
    ];
    Some(Controls {
        container,
        items,
        label: super::view::percent_label(percent),
    })
}

impl Controls {
    /// The control under a stage-local point.
    pub fn hit(&self, x: f64, y: f64) -> Option<ControlKind> {
        let inside = |r: Rect| {
            x >= f64::from(r.x)
                && y >= f64::from(r.y)
                && x < f64::from(r.x + r.width)
                && y < f64::from(r.y + r.height)
        };
        self.items.iter().find(|c| inside(c.rect)).map(|c| c.kind)
    }

    /// True when the point is anywhere on the cluster (including padding).
    pub fn contains(&self, x: f64, y: f64) -> bool {
        let r = self.container;
        x >= f64::from(r.x)
            && y >= f64::from(r.y)
            && x < f64::from(r.x + r.width)
            && y < f64::from(r.y + r.height)
    }
}

fn shifted(r: Rect, stage: Rect) -> Rect {
    Rect {
        x: r.x + stage.x,
        y: r.y + stage.y,
        ..r
    }
}

/// Draws the cluster over the stage of a window buffer. With `native_text`
/// the label and "Fit" use CoreText; otherwise the software glyph path.
#[allow(clippy::too_many_arguments)]
pub fn paint_cluster(
    pixels: &mut [u8],
    (width, height): (u32, u32),
    scale: u32,
    stage: Rect,
    dark: bool,
    percent: u32,
    native_text: bool,
) {
    let Some(cluster) = controls(stage.width, stage.height, percent) else {
        return;
    };
    let (len, stride) = (pixels.len(), width as usize * 4);
    let theme = if dark { Theme::Dark } else { Theme::Light };
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
        let face = if family == FontFamily::Monospaced {
            TextFace::Monospaced
        } else {
            TextFace::System
        };
        let emphasis = if weight == FontWeight::Semibold {
            TextEmphasis::Semibold
        } else {
            TextEmphasis::Regular
        };
        let area = (rect.x, rect.y, rect.width, rect.height);
        draw_text_rgba(
            rgba, w, h, stride, scale, s, area, color, face, emphasis, size,
        )
        .is_ok()
    };
    let canvas = if native_text {
        WidgetCanvas::new(
            pixels,
            len,
            width,
            height,
            stride,
            scale,
            theme,
            false,
            Some(&mut text),
        )
    } else {
        WidgetCanvas::new(
            pixels, len, width, height, stride, scale, theme, false, None,
        )
    };
    if let Some(mut canvas) = canvas {
        paint_controls(&mut canvas, stage, &cluster);
    }
}

/// Draws the cluster over the stage region of `canvas`.
pub fn paint_controls(canvas: &mut WidgetCanvas<'_>, stage: Rect, controls: &Controls) {
    let t: Tokens = canvas.tokens;
    canvas.paint_round(
        shifted(controls.container, stage),
        t.radius_card,
        t.surface[2],
        t.control_edge,
        1,
    );
    for control in &controls.items {
        let r = shifted(control.rect, stage);
        let (cx, cy) = (r.x + r.width / 2, r.y + r.height / 2);
        let bar = |x, y, w, h| Rect {
            x,
            y,
            width: w,
            height: h,
        };
        match control.kind {
            ControlKind::ZoomOut => canvas.paint_rect(bar(cx - 5, cy, 10, 1), t.text_primary),
            ControlKind::ZoomIn => {
                canvas.paint_rect(bar(cx - 5, cy, 10, 1), t.text_primary);
                canvas.paint_rect(bar(cx, cy - 5, 1, 10), t.text_primary);
            }
            ControlKind::Label => {
                canvas.paint_text(
                    &controls.label,
                    r,
                    t.text_secondary,
                    FontFamily::Monospaced,
                    FontWeight::Regular,
                    12,
                );
            }
            ControlKind::Fit => {
                canvas.paint_text(
                    "Fit",
                    r,
                    t.text_primary,
                    FontFamily::System,
                    FontWeight::Regular,
                    12,
                );
            }
        }
    }
}
