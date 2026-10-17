//! Footer timeline of the Animate workspace (design direction 5.2): transport
//! row, time ruler, track rows with key diamonds and a scrubbable playhead.
//!
//! `model` reads tracks from the document read-only, `layout` is pure
//! geometry and hit-testing, `paint` draws into a [`vexel_ui::WidgetCanvas`],
//! and this module holds the interaction state. Playback is only ever
//! steered through the existing [`Driver`] API; nothing here edits the
//! document. Key editing, easing and "+ key" are intentionally disabled.

pub mod layout;
pub mod model;
pub mod paint;

use crate::animation::driver::Driver;
use crate::animation::playback::LoopClock;
use layout::{Hit, ROW_H, TimelineLayout};
use model::{FPS, KeyRef, Row, TimelineModel};
use vexel_platform_apple::Modifiers;

pub use paint::paint;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Phase {
    Down,
    Move,
    Up,
}

/// Result of routing one pointer event.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Outcome {
    /// The timeline owns the event (the stage and tools must not see it).
    pub consumed: bool,
    pub redraw: bool,
}

/// Timeline data plus interaction state (selection, collapse, scroll, hover).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Timeline {
    pub model: TimelineModel,
    pub selection: Option<KeyRef>,
    pub collapsed: Vec<bool>,
    /// First drawn row.
    pub scroll: usize,
    pub hover: Option<Hit>,
    pub pressed: Option<Hit>,
    pub scrubbing: bool,
    scroll_acc: f64,
}

impl Timeline {
    /// Loads the tracks for a new document revision and resets the view state.
    pub fn set_document(&mut self, revision: u64, source: &[u8], loop_seconds: f64) {
        self.model.set_document(revision, source, loop_seconds);
        self.selection = None;
        self.collapsed = vec![false; self.model.elements.len()];
        self.scroll = 0;
    }

    pub fn clear(&mut self) {
        self.model.clear();
        self.selection = None;
        self.collapsed.clear();
        self.scroll = 0;
    }

    pub fn rows(&self) -> Vec<Row> {
        self.model.rows(&self.collapsed)
    }

    /// Scrolls the track rows by `delta` rows (clamped); true when it moved.
    pub fn scroll_rows(&mut self, delta: i64, layout: &TimelineLayout) -> bool {
        let max = layout.max_scroll(self.rows().len());
        let next = (self.scroll as i64 + delta).clamp(0, max as i64) as usize;
        let moved = next != self.scroll;
        self.scroll = next;
        moved
    }

    /// Wheel/trackpad delta in points; accumulates to whole rows.
    pub fn scroll_by_points(&mut self, dy: f64, layout: &TimelineLayout) -> bool {
        self.scroll_acc += dy;
        let rows = (self.scroll_acc / f64::from(ROW_H)).trunc();
        self.scroll_acc -= rows * f64::from(ROW_H);
        rows != 0.0 && self.scroll_rows(-(rows as i64), layout)
    }

    fn hit(&self, layout: &TimelineLayout, x: i32, y: i32) -> Hit {
        layout.hit_test(&self.model, &self.rows(), self.scroll, x, y)
    }

    fn scrub<C: LoopClock>(&self, layout: &TimelineLayout, x: i32, driver: &mut Driver<C>) {
        driver.seek(layout.x_to_time(f64::from(x), driver.loop_seconds()));
    }

    /// Routes a pointer event. Buttons fire on release over the pressed
    /// button; the ruler and empty lanes scrub; a diamond selects its key.
    pub fn pointer<C: LoopClock>(
        &mut self,
        phase: Phase,
        (x, y): (i32, i32),
        layout: &TimelineLayout,
        driver: &mut Driver<C>,
    ) -> Outcome {
        let before = (self.hover, self.pressed, self.selection, self.scrubbing);
        let hit = self.hit(layout, x, y);
        let in_panel = point_in(layout.rect, x, y);
        let mut consumed = false;
        match phase {
            Phase::Move => {
                self.hover = (hit != Hit::Panel).then_some(hit);
                if self.scrubbing {
                    self.scrub(layout, x, driver);
                    consumed = true;
                }
            }
            Phase::Down if in_panel => {
                consumed = true;
                match hit {
                    Hit::Ruler | Hit::Lane if driver.strip_visible() => {
                        driver.pause();
                        self.scrubbing = true;
                        self.scrub(layout, x, driver);
                    }
                    Hit::Key(key) => self.selection = Some(key),
                    Hit::Chevron(e) => {
                        if let Some(flag) = self.collapsed.get_mut(e) {
                            *flag = !*flag;
                        }
                        self.scroll_rows(0, layout);
                    }
                    Hit::Panel | Hit::Ruler | Hit::Lane | Hit::Ease | Hit::AddKey => {}
                    button => self.pressed = Some(button),
                }
            }
            Phase::Up => {
                if self.scrubbing {
                    self.scrub(layout, x, driver);
                    self.scrubbing = false;
                    consumed = true;
                }
                if let Some(pressed) = self.pressed.take() {
                    consumed = true;
                    if pressed == hit {
                        self.activate(pressed, driver);
                    }
                }
            }
            Phase::Down => {}
        }
        let after = (self.hover, self.pressed, self.selection, self.scrubbing);
        Outcome {
            consumed,
            redraw: before != after || consumed,
        }
    }

    /// Runs a transport button through the driver.
    pub fn activate<C: LoopClock>(&mut self, hit: Hit, driver: &mut Driver<C>) {
        match hit {
            Hit::SkipStart => {
                driver.seek(0.0);
            }
            Hit::SkipEnd => {
                driver.seek(driver.loop_seconds());
            }
            Hit::StepBack => {
                step(driver, false);
            }
            Hit::StepForward => {
                step(driver, true);
            }
            Hit::Play => {
                driver.toggle();
            }
            Hit::Loop => driver.toggle_looping(),
            _ => {}
        }
    }
}

fn point_in(r: vexel_ui::Rect, x: i32, y: i32) -> bool {
    x >= r.x as i32 && y >= r.y as i32 && x < (r.x + r.width) as i32 && y < (r.y + r.height) as i32
}

/// Steps one 24 fps frame, pausing first. Wraps when looping, clamps otherwise.
pub fn step<C: LoopClock>(driver: &mut Driver<C>, forward: bool) -> bool {
    let loop_seconds = driver.loop_seconds();
    if loop_seconds <= 0.0 {
        return false;
    }
    driver.pause();
    let frames = ((loop_seconds * FPS) - 1e-9).ceil().max(1.0) as i64;
    let current = (driver.time() * FPS).round() as i64;
    let next = current + if forward { 1 } else { -1 };
    let next = if driver.is_looping() {
        next.rem_euclid(frames)
    } else {
        next.clamp(0, frames)
    };
    driver.seek(next as f64 / FPS)
}

const KEY_L: u16 = 37;
const KEY_SPACE: u16 = 49;
const KEY_HOME: u16 = 115;
const KEY_END: u16 = 119;
const KEY_LEFT: u16 = 123;
const KEY_RIGHT: u16 = 124;

/// Playback keys of the Animate workspace: Space, Left/Right (one frame),
/// Home/End, L. True when the key was consumed.
pub fn key<C: LoopClock>(code: u16, modifiers: Modifiers, driver: &mut Driver<C>) -> bool {
    if modifiers.0 != 0 || !driver.strip_visible() {
        return false;
    }
    match code {
        KEY_SPACE => driver.toggle(),
        KEY_LEFT => step(driver, false),
        KEY_RIGHT => step(driver, true),
        KEY_HOME => driver.seek(0.0),
        KEY_END => driver.seek(driver.loop_seconds()),
        KEY_L => {
            driver.toggle_looping();
            true
        }
        _ => return false,
    }
}
