//! Pure timeline geometry: transport row, ruler, name column, track rows, the
//! time <-> x mapping and hit-testing. Geometry is all-or-nothing: every rect
//! is inside the panel and the three bands never overlap; bands that do not
//! fit are zero-height rects at the bottom edge.

use super::model::{KeyRef, Row, TimelineModel};
use vexel_ui::Rect;

pub const TRANSPORT_H: u32 = 32;
pub const RULER_H: u32 = 24;
pub const ROW_H: u32 = 28;
pub const NAME_W: u32 = 160;
pub const BUTTON: u32 = 28;
const PLAY_W: u32 = 40;
const GAP: u32 = 2;
const PAD: u32 = 12;
/// Space between the name column edge and time 0 (room for a key half-width).
const LANE_INSET: u32 = 14;
const LANE_RIGHT: u32 = 16;
/// Half the clickable width of a key diamond.
pub const KEY_HIT: i32 = 8;
const READOUT_W: u32 = 132;
const LABEL_CHAR_W: u32 = 8;
const MAJOR_STEPS: [f64; 10] = [1.0, 2.0, 5.0, 10.0, 15.0, 30.0, 60.0, 120.0, 300.0, 600.0];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Hit {
    SkipStart,
    StepBack,
    Play,
    StepForward,
    SkipEnd,
    Loop,
    /// Drawn disabled: the easing menu is not available yet.
    Ease,
    /// Drawn disabled: adding keys is not available yet.
    AddKey,
    Ruler,
    Key(KeyRef),
    /// Chevron of an element row.
    Chevron(usize),
    /// Empty track-lane space (scrubs, never selects).
    Lane,
    /// Name column or other inert panel space.
    Panel,
}

impl Hit {
    /// Tooltip for the disabled controls.
    pub fn disabled_hint(self) -> Option<&'static str> {
        match self {
            Self::Ease => Some("Easing editor is coming soon"),
            Self::AddKey => Some("Adding keyframes is coming soon"),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Tick {
    pub time: f64,
    pub x: f64,
    pub major: bool,
    /// Label text and its cell (inside the panel; absent when it does not fit).
    pub label: Option<(String, Rect)>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimelineLayout {
    pub rect: Rect,
    pub transport: Rect,
    pub ruler: Rect,
    /// Track rows band (name column plus lanes).
    pub tracks: Rect,
    pub name_w: u32,
    pub lane_x0: f64,
    pub lane_x1: f64,
    pub skip_start: Option<Rect>,
    pub step_back: Option<Rect>,
    pub play: Option<Rect>,
    pub step_forward: Option<Rect>,
    pub skip_end: Option<Rect>,
    pub readout: Option<Rect>,
    pub loop_btn: Option<Rect>,
    pub speed: Option<Rect>,
    pub ease: Option<Rect>,
    pub add_key: Option<Rect>,
}

fn r(x: u32, y: u32, width: u32, height: u32) -> Rect {
    Rect {
        x,
        y,
        width,
        height,
    }
}

fn inside(r: Rect, x: i32, y: i32) -> bool {
    x >= r.x as i32 && y >= r.y as i32 && x < (r.x + r.width) as i32 && y < (r.y + r.height) as i32
}

impl TimelineLayout {
    pub fn new(rect: Rect) -> Self {
        let bottom = rect.y + rect.height;
        let transport_h = rect.height.min(TRANSPORT_H);
        let transport = r(rect.x, rect.y, rect.width, transport_h);
        let after_transport = rect.height - transport_h;
        let ruler_h = if after_transport >= RULER_H {
            RULER_H
        } else {
            0
        };
        let ruler = r(rect.x, rect.y + transport_h, rect.width, ruler_h);
        let tracks_h = after_transport - ruler_h;
        let tracks_y = if ruler_h == 0 {
            bottom
        } else {
            ruler.y + ruler_h
        };
        let tracks = r(
            rect.x,
            tracks_y,
            rect.width,
            if ruler_h == 0 { 0 } else { tracks_h },
        );
        let name_w = NAME_W.min(rect.width * 2 / 5);
        let lane_x0 = f64::from(rect.x + name_w + LANE_INSET);
        let lane_x1 = f64::from(rect.x + rect.width.saturating_sub(LANE_RIGHT)).max(lane_x0 + 1.0);

        // Transport: left cluster, readout, loop; right cluster fills what is left.
        let by = rect.y + (transport_h.saturating_sub(BUTTON)) / 2;
        let limit = rect.x + rect.width.saturating_sub(PAD);
        let fit = |x: u32, w: u32, limit: u32| {
            (transport_h >= BUTTON && x + w <= limit).then(|| r(x, by, w, BUTTON))
        };
        let mut x = rect.x + PAD;
        let next = |w: u32, x: &mut u32| {
            let rc = fit(*x, w, limit);
            if rc.is_some() {
                *x += w + GAP;
            }
            rc
        };
        let skip_start = next(BUTTON, &mut x);
        let step_back = next(BUTTON, &mut x);
        let play = next(PLAY_W, &mut x);
        let step_forward = next(BUTTON, &mut x);
        let skip_end = next(BUTTON, &mut x);
        let all_buttons = skip_end.is_some();
        x += 10;
        let readout = if all_buttons {
            fit(x, READOUT_W, limit)
        } else {
            None
        };
        if readout.is_some() {
            x += READOUT_W + 6;
        }
        let loop_btn = if readout.is_some() {
            fit(x, BUTTON, limit)
        } else {
            None
        };
        let left_end = loop_btn.map_or(x, |l| l.x + l.width) + PAD;
        // Right cluster, right to left: "+ key", "Ease", speed label.
        let mut rx = limit;
        let mut place = |w: u32| {
            let start = rx.checked_sub(w)?;
            (loop_btn.is_some() && start >= left_end && transport_h >= BUTTON).then(|| {
                rx = start.saturating_sub(8);
                r(start, by, w, BUTTON)
            })
        };
        let add_key = place(64);
        let ease = if add_key.is_some() { place(64) } else { None };
        let speed = if ease.is_some() { place(36) } else { None };
        Self {
            rect,
            transport,
            ruler,
            tracks,
            name_w,
            lane_x0,
            lane_x1,
            skip_start,
            step_back,
            play,
            step_forward,
            skip_end,
            readout,
            loop_btn,
            speed,
            ease,
            add_key,
        }
    }

    /// Clickable transport controls in paint order.
    pub fn buttons(&self) -> Vec<(Hit, Rect)> {
        [
            (Hit::SkipStart, self.skip_start),
            (Hit::StepBack, self.step_back),
            (Hit::Play, self.play),
            (Hit::StepForward, self.step_forward),
            (Hit::SkipEnd, self.skip_end),
            (Hit::Loop, self.loop_btn),
            (Hit::Ease, self.ease),
            (Hit::AddKey, self.add_key),
        ]
        .into_iter()
        .filter_map(|(h, rc)| rc.map(|rc| (h, rc)))
        .collect()
    }

    // ---- time <-> x ------------------------------------------------------

    /// x of `seconds` (unclamped, sub-pixel) for a loop of `loop_seconds`.
    pub fn time_to_x(&self, seconds: f64, loop_seconds: f64) -> f64 {
        if loop_seconds <= 0.0 {
            return self.lane_x0;
        }
        self.lane_x0 + seconds / loop_seconds * (self.lane_x1 - self.lane_x0)
    }

    /// Time at `x`, clamped into the loop.
    pub fn x_to_time(&self, x: f64, loop_seconds: f64) -> f64 {
        if loop_seconds <= 0.0 {
            return 0.0;
        }
        ((x - self.lane_x0) / (self.lane_x1 - self.lane_x0) * loop_seconds).clamp(0.0, loop_seconds)
    }

    /// Playhead x: the mapped time, clamped to the ruler lane.
    pub fn playhead_x(&self, seconds: f64, loop_seconds: f64) -> f64 {
        self.time_to_x(seconds, loop_seconds)
            .clamp(self.lane_x0, self.lane_x1)
    }

    // ---- ruler -----------------------------------------------------------

    /// Ruler ticks: major every `step` seconds (1 s, thinned as the panel
    /// narrows), minor at a quarter of the step when there is room. Labels
    /// never overlap and always fit inside the panel.
    pub fn ticks(&self, loop_seconds: f64) -> Vec<Tick> {
        if loop_seconds <= 0.0 || self.ruler.height == 0 {
            return Vec::new();
        }
        let pps = (self.lane_x1 - self.lane_x0) / loop_seconds;
        let widest = label_text(loop_seconds.floor()).len() as u32 * LABEL_CHAR_W;
        let need = f64::from(widest + 16);
        let step = MAJOR_STEPS
            .iter()
            .copied()
            .find(|s| s * pps >= need)
            .unwrap_or(MAJOR_STEPS[MAJOR_STEPS.len() - 1]);
        let minor = step / 4.0;
        let draw_minor = minor * pps >= 6.0;
        let count = (loop_seconds / minor).floor() as usize;
        let right = self.rect.x + self.rect.width;
        let mut out: Vec<Tick> = Vec::new();
        let mut last_label_end = 0u32;
        for i in 0..=count.min(4096) {
            let time = i as f64 * minor;
            let major = i % 4 == 0;
            if !major && !draw_minor {
                continue;
            }
            let x = self.time_to_x(time, loop_seconds);
            let label = major.then(|| label_text(time)).and_then(|text| {
                let w = text.len() as u32 * LABEL_CHAR_W;
                let at = x.round() as u32;
                let cell = r(at + 4, self.ruler.y + 2, w, 16);
                let flipped = r(at.saturating_sub(4 + w), self.ruler.y + 2, w, 16);
                let cell = if cell.x + cell.width <= right {
                    cell
                } else if flipped.x >= last_label_end + 6 {
                    flipped
                } else {
                    return None;
                };
                last_label_end = cell.x + cell.width;
                Some((text, cell))
            });
            out.push(Tick {
                time,
                x,
                major,
                label,
            });
        }
        out
    }

    // ---- rows ------------------------------------------------------------

    /// Number of whole rows that fit in the track band.
    pub fn visible_rows(&self) -> usize {
        (self.tracks.height / ROW_H) as usize
    }

    /// Largest scroll offset (in rows) for `total` rows.
    pub fn max_scroll(&self, total: usize) -> usize {
        total.saturating_sub(self.visible_rows())
    }

    /// Rect of the `slot`-th visible row (0 = first drawn row).
    pub fn row_rect(&self, slot: usize) -> Rect {
        r(
            self.tracks.x,
            self.tracks.y + slot as u32 * ROW_H,
            self.tracks.width,
            ROW_H,
        )
    }

    /// Hit-test at `(x, y)` (points). `rows` are all visible rows, `scroll`
    /// the first drawn row.
    pub fn hit_test(
        &self,
        model: &TimelineModel,
        rows: &[Row],
        scroll: usize,
        x: i32,
        y: i32,
    ) -> Hit {
        if !inside(self.rect, x, y) {
            return Hit::Panel;
        }
        for (hit, rc) in self.buttons() {
            if inside(rc, x, y) {
                return hit;
            }
        }
        if inside(self.ruler, x, y) {
            return if f64::from(x) >= self.lane_x0 - 6.0 {
                Hit::Ruler
            } else {
                Hit::Panel
            };
        }
        if !inside(self.tracks, x, y) {
            return Hit::Panel;
        }
        let slot = (y as u32 - self.tracks.y) / ROW_H;
        let Some(row) = rows
            .get(scroll + slot as usize)
            .copied()
            .filter(|_| (slot as usize) < self.visible_rows())
        else {
            return if f64::from(x) >= self.lane_x0 - 6.0 {
                Hit::Lane
            } else {
                Hit::Panel
            };
        };
        if x < (self.rect.x + self.name_w) as i32 {
            if let Row::Element(e) = row {
                if x < (self.rect.x + 28) as i32 {
                    return Hit::Chevron(e);
                }
            }
            return Hit::Panel;
        }
        let keys = model.row_keys(row);
        let nearest = keys
            .iter()
            .enumerate()
            .map(|(i, t)| {
                (
                    i,
                    (self.time_to_x(*t, model.loop_seconds) - f64::from(x)).abs(),
                )
            })
            .filter(|(_, d)| *d <= f64::from(KEY_HIT))
            .min_by(|a, b| a.1.total_cmp(&b.1));
        match nearest {
            Some((index, _)) => Hit::Key(match row {
                Row::Element(element) => KeyRef {
                    element,
                    prop: None,
                    index,
                },
                Row::Prop(element, prop) => KeyRef {
                    element,
                    prop: Some(prop),
                    index,
                },
            }),
            None => Hit::Lane,
        }
    }
}

/// Ruler label: `Ns` below a minute, `m:ss` from a minute.
pub fn label_text(seconds: f64) -> String {
    let s = seconds.round() as u64;
    if s >= 60 {
        format!("{}:{:02}", s / 60, s % 60)
    } else {
        format!("{s}s")
    }
}
