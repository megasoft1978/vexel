//! Pure geometry and hit testing for the native animation panel.

use super::model::TimelineModel;
use vexel_ui::Rect;

pub const TRANSPORT_HEIGHT: u32 = 32;
pub const RULER_HEIGHT: u32 = 24;
pub const TRACK_HEIGHT: u32 = 28;
pub const MIN_NAME_WIDTH: u32 = 112;
pub const MAX_NAME_WIDTH: u32 = 184;
pub const MODE_SELECTOR_WIDTH: u32 = 148;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Viewport {
    pub first_tick: u32,
    pub last_tick: u32,
}

impl Viewport {
    pub fn full(model: &TimelineModel) -> Self {
        Self {
            first_tick: 0,
            last_tick: model.duration_ticks.max(1) - 1,
        }
    }

    pub fn bounded(self, model: &TimelineModel) -> Self {
        let end = model.duration_ticks.max(1) - 1;
        let first = self.first_tick.min(end);
        let last = self
            .last_tick
            .max(first.saturating_add(1).min(end))
            .min(end);
        Self {
            first_tick: first,
            last_tick: last,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Hit {
    ModeNative,
    ModeSmil,
    Play,
    Loop,
    OnionBefore,
    OnionAfter,
    AddDrawingTrack,
    ImportDrawing,
    Ruler,
    Track(usize),
    Visibility(usize),
    Item { track: usize, item: usize },
    Lane(usize),
    Empty,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimelineLayout {
    pub panel: Rect,
    pub transport: Rect,
    pub ruler: Rect,
    pub rows: Rect,
    pub name_width: u32,
    pub lane_left: f64,
    pub lane_right: f64,
    pub viewport: Viewport,
}

fn rect(x: u32, y: u32, width: u32, height: u32) -> Rect {
    Rect {
        x,
        y,
        width,
        height,
    }
}

impl TimelineLayout {
    pub fn new(panel: Rect, model: &TimelineModel, viewport: Viewport) -> Self {
        let transport_h = panel.height.min(TRANSPORT_HEIGHT);
        let ruler_h = panel.height.saturating_sub(transport_h).min(RULER_HEIGHT);
        let name_width = (panel.width.saturating_mul(2) / 5).clamp(
            MIN_NAME_WIDTH.min(panel.width),
            MAX_NAME_WIDTH.min(panel.width),
        );
        let panel_right = panel.x.saturating_add(panel.width);
        let lane_left = f64::from(
            panel
                .x
                .saturating_add(name_width)
                .saturating_add(10)
                .min(panel_right),
        );
        let lane_right = f64::from(panel_right.saturating_sub(12).max(panel.x)).max(lane_left);
        let viewport = viewport.bounded(model);
        Self {
            panel,
            transport: rect(panel.x, panel.y, panel.width, transport_h),
            ruler: rect(
                panel.x,
                panel.y.saturating_add(transport_h),
                panel.width,
                ruler_h,
            ),
            rows: rect(
                panel.x,
                panel.y.saturating_add(transport_h).saturating_add(ruler_h),
                panel.width,
                panel
                    .height
                    .saturating_sub(transport_h)
                    .saturating_sub(ruler_h),
            ),
            name_width,
            lane_left,
            lane_right,
            viewport,
        }
    }

    pub fn lane_x(&self, tick: u32) -> f64 {
        let span = self
            .viewport
            .last_tick
            .saturating_sub(self.viewport.first_tick)
            .max(1);
        let tick = tick.clamp(self.viewport.first_tick, self.viewport.last_tick);
        self.lane_left
            + (self.lane_right - self.lane_left)
                * f64::from(tick.saturating_sub(self.viewport.first_tick))
                / f64::from(span)
    }

    pub fn mode_selector(&self) -> Rect {
        rect(
            self.transport.x.saturating_add(8),
            self.transport
                .y
                .saturating_add(self.transport.height.saturating_sub(24) / 2),
            MODE_SELECTOR_WIDTH.min(self.transport.width.saturating_sub(16)),
            24.min(self.transport.height),
        )
    }

    pub fn tick_at(&self, x: i32) -> u32 {
        let span = self
            .viewport
            .last_tick
            .saturating_sub(self.viewport.first_tick);
        if span == 0 || self.lane_right <= self.lane_left {
            return self.viewport.first_tick;
        }
        let fraction = (f64::from(x) - self.lane_left) / (self.lane_right - self.lane_left);
        let offset = (fraction.clamp(0.0, 1.0) * f64::from(span)).round() as u32;
        self.viewport.first_tick.saturating_add(offset)
    }

    pub fn row_rect(&self, index: usize) -> Rect {
        let y = self
            .rows
            .y
            .saturating_add((index as u32).saturating_mul(TRACK_HEIGHT));
        let height = self
            .rows
            .y
            .saturating_add(self.rows.height)
            .saturating_sub(y)
            .min(TRACK_HEIGHT);
        rect(
            self.rows.x,
            y.min(self.rows.y.saturating_add(self.rows.height)),
            self.rows.width,
            height,
        )
    }

    pub fn item_rect(&self, model: &TimelineModel, track: usize, item: usize) -> Option<Rect> {
        let row = self.row_rect(track);
        let tr = model.tracks.get(track)?;
        let it = tr.items.get(item)?;
        let x = self.lane_x(it.tick).round().max(0.0) as u32;
        let next_tick = tr
            .items
            .get(item + 1)
            .map_or(self.viewport.last_tick, |next| next.tick);
        let right = self.lane_x(next_tick).round().max(0.0) as u32;
        let left = self.lane_left.round().max(0.0) as u32;
        let right_bound = self.lane_right.round().max(0.0) as u32;
        let x = x.clamp(left, right_bound);
        let width = match tr.kind {
            super::NativeTrackKind::Drawing => right
                .clamp(x.saturating_add(8).min(right_bound), right_bound)
                .saturating_sub(x),
            super::NativeTrackKind::Property => 12.min(right_bound.saturating_sub(x)),
        };
        Some(rect(
            x,
            row.y.saturating_add(5),
            width,
            row.height.saturating_sub(10),
        ))
    }

    pub fn hit_test(&self, model: &TimelineModel, x: i32, y: i32) -> Hit {
        if !inside(self.panel, x, y) {
            return Hit::Empty;
        }
        if inside(self.transport, x, y) {
            let rel = x.saturating_sub(self.transport.x as i32).max(0) as u32;
            let selector = self.mode_selector();
            if inside(selector, x, y) {
                let local_x = x.saturating_sub(selector.x as i32).max(0) as u32;
                if local_x < selector.width / 2 {
                    return Hit::ModeNative;
                }
                return Hit::ModeSmil;
            }
            if rel < MODE_SELECTOR_WIDTH {
                return Hit::Empty;
            }
            let rel = rel.saturating_sub(MODE_SELECTOR_WIDTH);
            if rel < 52 {
                return Hit::Play;
            }
            if rel < 114 {
                return Hit::Loop;
            }
            if rel < 180 {
                return Hit::Empty;
            }
            if rel < 246 {
                return Hit::Empty;
            }
            if rel < 308 {
                return Hit::AddDrawingTrack;
            }
            if rel < 370 {
                return Hit::ImportDrawing;
            }
            return Hit::Empty;
        }
        if inside(self.ruler, x, y) {
            return Hit::Ruler;
        }
        if inside(self.rows, x, y) {
            let index = ((y as u32).saturating_sub(self.rows.y) / TRACK_HEIGHT) as usize;
            let Some(track) = model.tracks.get(index) else {
                return Hit::Empty;
            };
            if x < (self
                .panel
                .x
                .saturating_add(self.name_width)
                .saturating_sub(26)) as i32
            {
                return Hit::Track(index);
            }
            if x < self.lane_left as i32 {
                return Hit::Visibility(index);
            }
            for item in 0..track.items.len() {
                if self
                    .item_rect(model, index, item)
                    .is_some_and(|r| inside(r, x, y))
                {
                    return Hit::Item { track: index, item };
                }
            }
            return Hit::Lane(index);
        }
        Hit::Empty
    }
}

fn inside(r: Rect, x: i32, y: i32) -> bool {
    x >= r.x as i32
        && y >= r.y as i32
        && x < r.x.saturating_add(r.width) as i32
        && y < r.y.saturating_add(r.height) as i32
}
