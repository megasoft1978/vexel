//! Shared helpers for the `ui_timeline_*` tests (included with `#[path]`).
#![allow(dead_code)]

use vexel_app::animation::driver::Driver;
use vexel_app::animation::playback::ManualClock;
use vexel_app::model::{Mode, Model};
use vexel_app::render::render;
use vexel_app::timeline::Timeline;
use vexel_ui::{Rect, Theme, Tokens, theme_tokens};

pub const LOOP: &[u8] = include_bytes!("../../../fixtures/v1/anim/loop.svg");
pub const LOOP2: &[u8] = include_bytes!("../../../fixtures/v1/playback/loop2.svg");
pub const STATIC: &[u8] = include_bytes!("../../../fixtures/v1/compose/compose-v1.svg");
/// Five animated elements (more rows than a 200 px footer shows).
pub const LONG: &[u8] = include_bytes!("../../../fixtures/ui/timeline/many.svg");

pub fn tokens(dark: bool) -> Tokens {
    theme_tokens(if dark { Theme::Dark } else { Theme::Light }, false)
}

pub fn driver_with(source: &[u8]) -> Driver<ManualClock> {
    let mut driver = Driver::new(ManualClock::default());
    driver.sync(1, source);
    driver
}

/// A timeline loaded from `source`, with playback copied from `driver`.
pub fn timeline_of(source: &[u8], driver: &Driver<ManualClock>) -> Timeline {
    let mut tl = Timeline::default();
    tl.set_document(1, source, driver.loop_seconds());
    tl.model.sync_playback(driver);
    tl
}

pub fn rect(x: u32, y: u32, width: u32, height: u32) -> Rect {
    Rect {
        x,
        y,
        width,
        height,
    }
}

/// Full Animate window frame (RGBA, width, height) with `tl` in the footer.
pub fn frame(size: (u32, u32), dark: bool, scale: u32, tl: Timeline) -> (u32, u32, Vec<u8>) {
    let mut model = Model::new(dark);
    model.mode = Mode::Animate;
    model.timeline = tl;
    let out = render(size.0, size.1, scale, &model).expect("renders");
    (out.width(), out.height(), out.pixels().to_vec())
}

/// Footer rect of `size` in Animate mode.
pub fn footer(size: (u32, u32)) -> Rect {
    Model::new(true)
        .layout(size.0, size.1)
        .map(|_| ())
        .and(
            vexel_app::model::Layout::new(size.0, size.1, Mode::Animate)
                .unwrap()
                .footer,
        )
        .expect("animate layout has a footer")
}

pub fn pixel(frame: &(u32, u32, Vec<u8>), scale: u32, x: u32, y: u32) -> [u8; 3] {
    let (w, _, data) = frame;
    let at = ((y * scale * w + x * scale) * 4) as usize;
    [data[at], data[at + 1], data[at + 2]]
}

pub fn rgb(color: u32) -> [u8; 3] {
    let [r, g, b, _] = color.to_be_bytes();
    [r, g, b]
}
