//! Minimal timeline strip: play/pause button, scrub bar, time label.
//!
//! Layout, hit-testing and the scrub drag are pure (the drag uses the
//! `vexel-ui` [`Slider`]); [`draw`] paints into an RGBA frame with the
//! shell's existing fill and text helpers. The strip overlays the bottom of
//! the stage and exists only while the document has a loop.

use super::driver::Driver;
use super::playback::LoopClock;
use crate::compose::fill;
use vexel_platform_apple::{RgbaFrame, TextEmphasis, TextFace, draw_text_rgba};
use vexel_ui::{PointerEvent as UiPointer, Rect, Slider, Theme, theme_tokens};

pub const STRIP_HEIGHT: u32 = 28;
const BUTTON_WIDTH: u32 = 64;
const LABEL_WIDTH: u32 = 150;
const PAD: u32 = 8;
const THUMB_WIDTH: u32 = 8;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StripLayout {
    pub bar: Rect,
    pub button: Rect,
    pub scrub: Rect,
    pub label: Rect,
}

/// Strip geometry for a stage; `None` when the stage is too small.
pub fn layout(stage: Rect) -> Option<StripLayout> {
    let fixed = BUTTON_WIDTH + LABEL_WIDTH + PAD * 4;
    if stage.height < STRIP_HEIGHT * 3 || stage.width < fixed + 40 {
        return None;
    }
    let bar = Rect {
        x: stage.x,
        y: stage.y + stage.height - STRIP_HEIGHT,
        width: stage.width,
        height: STRIP_HEIGHT,
    };
    let inner_h = STRIP_HEIGHT - 8;
    let button = Rect {
        x: bar.x + PAD,
        y: bar.y + 4,
        width: BUTTON_WIDTH,
        height: inner_h,
    };
    let label = Rect {
        x: bar.x + bar.width - PAD - LABEL_WIDTH,
        y: bar.y + 4,
        width: LABEL_WIDTH,
        height: inner_h,
    };
    let scrub_x = button.x + button.width + PAD;
    let scrub = Rect {
        x: scrub_x,
        y: bar.y + 4,
        width: label.x - PAD - scrub_x,
        height: inner_h,
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
            driver.seek(slider.value);
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
                    driver.seek(slider.value);
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
    let tokens = theme_tokens(if dark { Theme::Dark } else { Theme::Light }, false);
    fill(&mut pixels, width, scale, layout.bar, tokens.panel_raised);
    fill(&mut pixels, width, scale, layout.button, tokens.accent);
    let loop_seconds = driver.loop_seconds();
    let track = Rect {
        y: layout.scrub.y + layout.scrub.height / 2 - 1,
        height: 3,
        ..layout.scrub
    };
    fill(&mut pixels, width, scale, track, tokens.separator);
    let inner = layout.scrub.width.saturating_sub(THUMB_WIDTH);
    let ratio = if loop_seconds > 0.0 {
        (driver.time() / loop_seconds).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let thumb_x = layout.scrub.x + (f64::from(inner) * ratio).round() as u32;
    let progress = Rect {
        width: thumb_x - layout.scrub.x,
        ..track
    };
    fill(&mut pixels, width, scale, progress, tokens.accent);
    let thumb = Rect {
        x: thumb_x,
        width: THUMB_WIDTH,
        ..layout.scrub
    };
    fill(&mut pixels, width, scale, thumb, tokens.text_primary);
    let mut text = |rect: Rect, value: &str, color: u32| {
        let clip = (
            rect.x,
            rect.y + 3,
            rect.width,
            rect.height.saturating_sub(3),
        );
        let _ = draw_text_rgba(
            &mut pixels,
            width,
            height,
            width as usize * 4,
            scale,
            value,
            clip,
            color,
            TextFace::System,
            TextEmphasis::Regular,
            11,
        );
    };
    let name = if driver.is_playing() { "Pause" } else { "Play" };
    let button = Rect {
        x: layout.button.x + 12,
        width: layout.button.width.saturating_sub(12),
        ..layout.button
    };
    text(button, name, tokens.window);
    text(
        layout.label,
        &label(driver.time(), loop_seconds),
        tokens.text_secondary,
    );
    RgbaFrame::new(width, height, f64::from(scale), pixels).unwrap_or(frame)
}
