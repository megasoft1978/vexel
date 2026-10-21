//! Pure geometry and hit testing for the native animation panel.
//!
//! Bands from top to bottom: transport (mode, buttons, readouts), ruler, track
//! rows and, only while a property key is selected, the key bar (easing of
//! that key). The key bar sits at the bottom edge so rows never move when a
//! key is picked. The control sizes and
//! gaps follow the inspector (28 px fields and buttons, 6 px segment gaps).

use super::model::TimelineModel;
use vexel_ui::Rect;

pub const TRANSPORT_HEIGHT: u32 = 36;
pub const KEYBAR_HEIGHT: u32 = 36;
pub const RULER_HEIGHT: u32 = 28;
pub const TRACK_HEIGHT: u32 = 28;
pub const MIN_NAME_WIDTH: u32 = 168;
pub const MAX_NAME_WIDTH: u32 = 248;
pub const MODE_SELECTOR_WIDTH: u32 = 176;
pub const CONTROL_SIZE: u32 = 28;
const CONTROL_GAP: u32 = 4;
/// Room kept at the right end of each row for the duplicate and delete icons.
pub const ROW_GUTTER: u32 = 56;
/// The key bar needs this much panel width and height to be shown.
pub const KEYBAR_MIN_WIDTH: u32 = 460;
pub const KEYBAR_MIN_HEIGHT: u32 = 150;
pub const BEZIER_EDITOR_W: u32 = 366;
pub const BEZIER_EDITOR_H: u32 = 124;

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
    Item {
        track: usize,
        item: usize,
    },
    Ease(&'static str),
    /// Opens or closes the custom cubic-bezier editor.
    EaseCustom,
    /// One stepper button of the bezier editor: field 0..4 = x1,y1,x2,y2.
    BezierStep {
        field: u8,
        up: bool,
    },
    BezierApply,
    /// Closes the editor without an edit.
    BezierCancel,
    /// Inert area of the open editor; swallows pointer input.
    BezierPanel,
    DuplicateTrack(usize),
    DeleteTrack(usize),
    Lane(usize),
    Empty,
}

/// The easing presets in key bar order, then the custom editor toggle.
pub const EASES: [(&str, &str); 5] = [
    ("linear", "Linear"),
    ("ease_in", "In"),
    ("ease_out", "Out"),
    ("ease_in_out", "In-Out"),
    ("step", "Step"),
];
const EASE_WIDTHS: [u32; 6] = [52, 40, 40, 54, 46, 58];
/// Left edge of the first easing cell, after the "Easing" caption.
const EASE_X: u32 = 68;
pub const EASE_GAP: u32 = 6;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Readout {
    Tick,
    Rate,
    Zoom,
}

/// The four bands of a panel; each rect is inside the panel and zero height
/// when it does not fit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bands {
    pub transport: Rect,
    pub ruler: Rect,
    pub rows: Rect,
    pub keybar: Rect,
}

/// `with_keybar` reserves the bottom strip when the panel is big enough.
pub fn bands(panel: Rect, with_keybar: bool) -> Bands {
    let transport_h = panel.height.min(TRANSPORT_HEIGHT);
    let left = panel.height.saturating_sub(transport_h);
    let keybar_h =
        if with_keybar && panel.width >= KEYBAR_MIN_WIDTH && panel.height >= KEYBAR_MIN_HEIGHT {
            left.min(KEYBAR_HEIGHT)
        } else {
            0
        };
    let ruler_h = left.saturating_sub(keybar_h).min(RULER_HEIGHT);
    let ruler_y = panel.y.saturating_add(transport_h);
    let rows_y = ruler_y.saturating_add(ruler_h);
    let rows_h = left.saturating_sub(keybar_h + ruler_h);
    Bands {
        transport: rect(panel.x, panel.y, panel.width, transport_h),
        ruler: rect(panel.x, ruler_y, panel.width, ruler_h),
        rows: rect(panel.x, rows_y, panel.width, rows_h),
        keybar: rect(
            panel.x,
            panel
                .y
                .saturating_add(panel.height.saturating_sub(keybar_h)),
            panel.width,
            keybar_h,
        ),
    }
}

/// Transport button `hit` (0..5 in transport order) inside `panel`.
pub fn control_rect_in(panel: Rect, hit: Hit) -> Option<Rect> {
    let transport = bands(panel, false).transport;
    let index = match hit {
        Hit::Play => 0,
        Hit::Loop => 1,
        Hit::OnionBefore => 2,
        Hit::AddDrawingTrack => 3,
        Hit::ImportDrawing => 4,
        _ => return None,
    };
    let first = transport.x.saturating_add(8 + MODE_SELECTOR_WIDTH + 12);
    // A wider gap separates playback (play, loop) from the edit buttons.
    let group = if index >= 2 { 8 } else { 0 };
    Some(rect(
        first.saturating_add(index * (CONTROL_SIZE + CONTROL_GAP) + group),
        transport
            .y
            .saturating_add(transport.height.saturating_sub(CONTROL_SIZE) / 2),
        CONTROL_SIZE.min(transport.width),
        CONTROL_SIZE.min(transport.height),
    ))
}

/// Easing cell `index` (0..5, 5 = custom) of the key bar of `panel`.
pub fn ease_cell_in(panel: Rect, index: usize) -> Option<Rect> {
    let bar = bands(panel, true).keybar;
    if bar.height == 0 || index > 5 {
        return None;
    }
    let mut x = bar.x.saturating_add(EASE_X);
    for w in EASE_WIDTHS.iter().take(index) {
        x = x.saturating_add(w + EASE_GAP);
    }
    Some(rect(
        x,
        bar.y.saturating_add(bar.height.saturating_sub(28) / 2),
        EASE_WIDTHS[index],
        28.min(bar.height),
    ))
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimelineLayout {
    pub panel: Rect,
    pub transport: Rect,
    pub ruler: Rect,
    pub rows: Rect,
    /// Zero height unless a property key is selected.
    pub keybar: Rect,
    pub name_width: u32,
    pub lane_left: f64,
    pub lane_right: f64,
    pub viewport: Viewport,
}

impl TimelineLayout {
    /// Compact, non-overlapping icon targets in transport order.
    pub fn control_rects(&self) -> [(Hit, Rect); 5] {
        [
            Hit::Play,
            Hit::Loop,
            Hit::OnionBefore,
            Hit::AddDrawingTrack,
            Hit::ImportDrawing,
        ]
        .map(|hit| {
            (
                hit,
                control_rect_in(self.panel, hit).unwrap_or_else(|| rect(0, 0, 0, 0)),
            )
        })
    }

    /// Easing cells in key bar order: the five presets, then custom.
    pub fn ease_cells(&self) -> [(Hit, Rect); 6] {
        std::array::from_fn(|i| {
            let hit = EASES
                .get(i)
                .map_or(Hit::EaseCustom, |(id, _)| Hit::Ease(id));
            (
                hit,
                ease_cell_in(self.panel, i).unwrap_or_else(|| rect(0, 0, 0, 0)),
            )
        })
    }

    /// Small display fields at the right end of the transport, right to left.
    pub fn readout(&self, which: Readout) -> Rect {
        let (w, slot) = match which {
            Readout::Zoom => (88, 0),
            Readout::Rate => (88, 1),
            Readout::Tick => (112, 2),
        };
        let right = self.transport.x + self.transport.width;
        let used: u32 = match slot {
            0 => 0,
            1 => 88 + 6,
            _ => 88 + 6 + 88 + 6,
        };
        let x = right.saturating_sub(8 + used + w);
        rect(
            x,
            self.transport
                .y
                .saturating_add(self.transport.height.saturating_sub(28) / 2),
            w.min(self.transport.width),
            28.min(self.transport.height),
        )
    }
}

pub(super) fn rect(x: u32, y: u32, width: u32, height: u32) -> Rect {
    Rect {
        x,
        y,
        width,
        height,
    }
}

pub(super) fn inside(r: Rect, x: i32, y: i32) -> bool {
    x >= r.x as i32
        && y >= r.y as i32
        && x < r.x.saturating_add(r.width) as i32
        && y < r.y.saturating_add(r.height) as i32
}

impl TimelineLayout {
    pub fn new(panel: Rect, model: &TimelineModel, viewport: Viewport) -> Self {
        let property_key = model
            .selected_item()
            .is_some_and(|(_, _, tr, _)| matches!(tr.kind, super::NativeTrackKind::Property));
        let b = bands(panel, property_key);
        let name_width = (panel.width.saturating_mul(2) / 5).clamp(
            MIN_NAME_WIDTH.min(panel.width),
            MAX_NAME_WIDTH.min(panel.width),
        );
        let panel_right = panel.x.saturating_add(panel.width);
        let lane_left = f64::from(
            panel
                .x
                .saturating_add(name_width)
                .saturating_add(12)
                .min(panel_right),
        );
        let lane_right =
            f64::from(panel_right.saturating_sub(ROW_GUTTER).max(panel.x)).max(lane_left);
        let viewport = viewport.bounded(model);
        Self {
            panel,
            transport: b.transport,
            keybar: b.keybar,
            ruler: b.ruler,
            rows: b.rows,
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
                .saturating_add(self.transport.height.saturating_sub(CONTROL_SIZE) / 2),
            MODE_SELECTOR_WIDTH.min(self.transport.width.saturating_sub(16)),
            CONTROL_SIZE.min(self.transport.height),
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

    /// Hit box of one item. Property keys are 14 px boxes centred on their
    /// tick; drawing cels span to the next cel.
    pub fn item_rect(&self, model: &TimelineModel, track: usize, item: usize) -> Option<Rect> {
        let row = self.row_rect(track);
        let tr = model.tracks.get(track)?;
        let it = tr.items.get(item)?;
        let x = self.lane_x(it.tick).round().max(0.0) as u32;
        let left = self.lane_left.round().max(0.0) as u32;
        let right_bound = self.lane_right.round().max(0.0) as u32;
        if matches!(tr.kind, super::NativeTrackKind::Property) {
            return Some(rect(
                x.saturating_sub(7),
                row.y.saturating_add(5),
                14,
                row.height.saturating_sub(10),
            ));
        }
        let next_tick = tr
            .items
            .get(item + 1)
            .map_or(self.viewport.last_tick, |next| next.tick);
        let right = self.lane_x(next_tick).round().max(0.0) as u32;
        let x = x.clamp(left, right_bound);
        let width = right
            .clamp(x.saturating_add(8).min(right_bound), right_bound)
            .saturating_sub(x);
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
            let selector = self.mode_selector();
            if inside(selector, x, y) {
                let local_x = x.saturating_sub(selector.x as i32).max(0) as u32;
                if local_x < selector.width / 2 {
                    return Hit::ModeNative;
                }
                return Hit::ModeSmil;
            }
            if let Some((hit, _)) = self
                .control_rects()
                .into_iter()
                .find(|(_, r)| inside(*r, x, y))
            {
                return hit;
            }
            return Hit::Empty;
        }
        if inside(self.keybar, x, y) {
            return self
                .ease_cells()
                .into_iter()
                .find(|(_, r)| inside(*r, x, y))
                .map_or(Hit::Empty, |(hit, _)| hit);
        }
        if inside(self.ruler, x, y) {
            return Hit::Ruler;
        }
        if inside(self.rows, x, y) {
            let index = ((y as u32).saturating_sub(self.rows.y) / TRACK_HEIGHT) as usize;
            let Some(track) = model.tracks.get(index) else {
                return Hit::Empty;
            };
            let row = self.row_rect(index);
            let right = row.x.saturating_add(row.width);
            if x >= right.saturating_sub(48) as i32 {
                return if x >= right.saturating_sub(24) as i32 {
                    Hit::DeleteTrack(index)
                } else {
                    Hit::DuplicateTrack(index)
                };
            }
            // A key on the first tick straddles the lane edge: keys win.
            if x >= (self.lane_left as i32).saturating_sub(7) {
                for item in 0..track.items.len() {
                    if self
                        .item_rect(model, index, item)
                        .is_some_and(|r| inside(r, x, y))
                    {
                        return Hit::Item { track: index, item };
                    }
                }
            }
            if x < (self.panel.x.saturating_add(34)) as i32 {
                return Hit::Visibility(index);
            }
            if x < self.lane_left as i32 {
                return Hit::Track(index);
            }
            return Hit::Lane(index);
        }
        Hit::Empty
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_timeline::NativeTimelineSnapshot;

    fn layout() -> (TimelineModel, TimelineLayout) {
        let snapshot = NativeTimelineSnapshot {
            ticks_per_second: 24,
            duration_ticks: 24,
            current_tick: 0,
            tracks: vec![],
            selected_target: None,
            playing: false,
            loop_enabled: false,
            onion_before: 0,
            onion_after: 0,
            onion_opacity: 0.25,
            can_add_drawing_track: true,
            can_import_asset: true,
            can_edit_selected_item: false,
            status: None,
        };
        let model = TimelineModel::from_snapshot(&snapshot);
        let panel = rect(0, 0, 720, 180);
        let layout = TimelineLayout::new(panel, &model, Viewport::full(&model));
        (model, layout)
    }

    #[test]
    fn icon_targets_are_at_least_24_square_non_overlapping_and_hit_their_actions() {
        let (model, layout) = layout();
        let controls = layout.control_rects();
        for (i, (hit, r)) in controls.iter().enumerate() {
            assert!(r.width >= 24 && r.height >= 24);
            for (_, next) in controls.iter().skip(i + 1) {
                assert!(r.x + r.width <= next.x || next.x + next.width <= r.x);
            }
            assert_eq!(
                layout.hit_test(
                    &model,
                    (r.x + r.width / 2) as i32,
                    (r.y + r.height / 2) as i32
                ),
                *hit
            );
        }
        assert_eq!(
            controls.map(|(h, _)| h),
            [
                Hit::Play,
                Hit::Loop,
                Hit::OnionBefore,
                Hit::AddDrawingTrack,
                Hit::ImportDrawing
            ]
        );
    }
}
