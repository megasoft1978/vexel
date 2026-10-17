//! Draw-mode timeline strip: a 28 px rounded level-2 bar with a play/pause
//! button, a scrub rule and the time readout.
//!
//! Layout, hit-testing and the scrub drag are pure (the drag uses the
//! `vexel-ui` [`Slider`]); [`draw`] paints into an RGBA frame through the
//! shell's widget painter. The strip floats over the stage, 8 px above the
//! zoom cluster, and exists only while the document has a loop.

use super::driver::{Driver, quantize};
use super::playback::LoopClock;
use std::cell::Cell;
use vexel_platform_apple::{RgbaFrame, TextEmphasis, TextFace, draw_text_rgba};
use vexel_ui::{
    FontFamily, FontWeight, IconKind, PointerEvent as UiPointer, Rect, Slider, Theme, WidgetCanvas,
};

pub const STRIP_HEIGHT: u32 = 28;
/// Gap between the strip and the stage edge / the zoom cluster below it.
pub const STRIP_GAP: u32 = 8;
const BUTTON_SIZE: u32 = 24;
const LABEL_WIDTH: u32 = 150;
const PAD: u32 = 8;
/// Slider travel reserved for the thumb (the drawn rule is 2 px wide).
const THUMB_WIDTH: u32 = 8;
const RULE_WIDTH: u32 = 3;
/// Readout size: the body type size.
pub const READOUT_PT: u32 = vexel_ui::type_scale().body_pt as u32;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StripLayout {
    pub bar: Rect,
    pub button: Rect,
    pub scrub: Rect,
    pub label: Rect,
}

/// Strip geometry for a stage; `None` when the stage is too small. The bar
/// sits above the zoom cluster (or the stage margin when there is none).
pub fn layout(stage: Rect) -> Option<StripLayout> {
    let fixed = BUTTON_SIZE + LABEL_WIDTH + PAD * 4;
    let reserved = crate::canvas::controls(stage.width, stage.height, 100)
        .map_or(STRIP_GAP, |c| stage.height - c.container.y + STRIP_GAP);
    if stage.height < reserved + STRIP_HEIGHT + STRIP_GAP
        || stage.width < fixed + 40 + 2 * STRIP_GAP
    {
        return None;
    }
    let bar = Rect {
        x: stage.x + STRIP_GAP,
        y: stage.y + stage.height - reserved - STRIP_HEIGHT,
        width: stage.width - 2 * STRIP_GAP,
        height: STRIP_HEIGHT,
    };
    let button = Rect {
        x: bar.x + 2,
        y: bar.y + (STRIP_HEIGHT - BUTTON_SIZE) / 2,
        width: BUTTON_SIZE,
        height: BUTTON_SIZE,
    };
    let label = Rect {
        x: bar.x + bar.width - PAD - LABEL_WIDTH,
        y: bar.y + 4,
        width: LABEL_WIDTH,
        height: STRIP_HEIGHT - 8,
    };
    let scrub_x = button.x + button.width + PAD;
    let scrub = Rect {
        x: scrub_x,
        y: bar.y + 4,
        width: label.x - PAD - scrub_x,
        height: STRIP_HEIGHT - 8,
    };
    Some(StripLayout {
        bar,
        button,
        scrub,
        label,
    })
}

fn contains(r: Rect, x: i32, y: i32) -> bool {
    x >= r.x as i32 && y >= r.y as i32 && x < (r.x + r.width) as i32 && y < (r.y + r.height) as i32
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StripHit {
    Button,
    Scrub,
    /// Inside the strip but on neither control.
    Bar,
}

pub fn hit(layout: &StripLayout, x: i32, y: i32) -> Option<StripHit> {
    if !contains(layout.bar, x, y) {
        None
    } else if contains(layout.button, x, y) {
        Some(StripHit::Button)
    } else if contains(layout.scrub, x, y) {
        Some(StripHit::Scrub)
    } else {
        Some(StripHit::Bar)
    }
}

/// `m:ss.cs`.
pub fn format_time(seconds: f64) -> String {
    let centis = (seconds.max(0.0) * 100.0).round() as u64;
    format!(
        "{}:{:02}.{:02}",
        centis / 6000,
        centis / 100 % 60,
        centis % 100
    )
}

/// `0:01.50 / 0:02.00`.
pub fn label(time: f64, loop_seconds: f64) -> String {
    format!("{} / {}", format_time(time), format_time(loop_seconds))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StripPointer {
    Down,
    Move,
    Up,
}

/// Scrub drag state. A drag pauses playback and seeks; release stays paused.
#[derive(Debug, Default)]
pub struct Strip {
    drag: Option<Slider>,
}

impl Strip {
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn is_dragging(&self) -> bool {
        self.drag.is_some()
    }

    /// Routes a primary-pointer event. True when the strip consumed it (the
    /// tools must not see it).
    pub fn pointer<C: LoopClock>(
        &mut self,
        kind: StripPointer,
        (x, y): (i32, i32),
        layout: &StripLayout,
        driver: &mut Driver<C>,
    ) -> bool {
        let loop_seconds = driver.loop_seconds();
        if loop_seconds <= 0.0 {
            self.drag = None;
            return false;
        }
        let track_x = (layout.scrub.x + THUMB_WIDTH / 2) as i32;
        let width = layout.scrub.width.saturating_sub(THUMB_WIDTH).max(1);
        if let Some(slider) = self.drag.as_mut() {
            let event = match kind {
                StripPointer::Move => UiPointer::Move,
                StripPointer::Up => UiPointer::UpInside,
                StripPointer::Down => UiPointer::Down,
            };
            slider.pointer(event, x, track_x, width);
            driver.seek(quantize(slider.value, loop_seconds));
            if kind == StripPointer::Up {
                self.drag = None;
            }
            return true;
        }
        if kind != StripPointer::Down {
            return false;
        }
        match hit(layout, x, y) {
            None => false,
            Some(StripHit::Button) => {
                driver.toggle();
                true
            }
            Some(StripHit::Scrub) => {
                driver.pause();
                if let Some(mut slider) = Slider::new("Timeline", 0.0, 0.0, loop_seconds, 0.01) {
                    slider.pointer(UiPointer::Down, x, track_x, width);
                    driver.seek(quantize(slider.value, loop_seconds));
                    self.drag = Some(slider);
                }
                true
            }
            Some(StripHit::Bar) => true,
        }
    }
}

/// Paints the strip over `frame`'s stage area.
pub fn draw<C: LoopClock>(
    frame: RgbaFrame,
    layout: &StripLayout,
    driver: &Driver<C>,
    dark: bool,
) -> RgbaFrame {
    let (width, height, scale) = (frame.width(), frame.height(), frame.backing_scale() as u32);
    let mut pixels = frame.pixels().to_vec();
    let (len, stride) = (pixels.len(), width as usize * 4);
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
    let theme = if dark { Theme::Dark } else { Theme::Light };
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
    let Some(mut c) = canvas else { return frame };
    let t = c.tokens;
    c.paint_round(layout.bar, t.radius_card, t.surface[2], t.control_edge, 1);
    let b = layout.button;
    let icon = Rect {
        x: b.x + (b.width - 16) / 2,
        y: b.y + (b.height - 16) / 2,
        width: 16,
        height: 16,
    };
    let kind = if driver.is_playing() {
        IconKind::PAUSE
    } else {
        IconKind::PLAY
    };
    c.paint_icon_color(kind, icon, t.text_primary);
    // Scrub rule: a 2 px track, the elapsed part filled in accent, a 3 px accent thumb.
    let loop_seconds = driver.loop_seconds();
    let time = quantize(driver.time(), loop_seconds);
    let mid = layout.scrub.y + layout.scrub.height / 2;
    let track = Rect {
        y: mid - 1,
        height: 2,
        ..layout.scrub
    };
    c.paint_rect(track, t.control_edge);
    let inner = layout.scrub.width.saturating_sub(THUMB_WIDTH);
    let ratio = if loop_seconds > 0.0 {
        (time / loop_seconds).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let travel = (f64::from(inner) * ratio).round() as u32;
    let progress = Rect {
        width: travel + THUMB_WIDTH / 2,
        ..track
    };
    c.paint_rect(progress, t.accent);
    let thumb = Rect {
        x: layout.scrub.x + travel + (THUMB_WIDTH - RULE_WIDTH) / 2,
        y: layout.scrub.y + 2,
        width: RULE_WIDTH,
        height: layout.scrub.height - 4,
    };
    c.paint_rect(thumb, t.accent);
    let readout = Rect {
        y: layout.bar.y + 5,
        height: 18,
        ..layout.label
    };
    let _ = c.paint_text(
        &label(time, loop_seconds),
        readout,
        t.text_secondary,
        FontFamily::Monospaced,
        FontWeight::Regular,
        READOUT_PT,
    );
    drop(c);
    RgbaFrame::new(width, height, f64::from(scale), pixels).unwrap_or(frame)
}
