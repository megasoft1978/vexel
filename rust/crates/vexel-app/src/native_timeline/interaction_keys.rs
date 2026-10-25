//! Keyboard, zoom and viewport handling for `InteractionState`.

use super::*;
use crate::native_timeline::NativeTimelineTrack;
use vexel_ui::Rect;

impl InteractionState {
    pub fn key(&mut self, key: Key, model: &TimelineModel) -> Vec<NativeTimelineAction> {
        self.focus = true;
        let tick = model
            .current_tick
            .min(model.duration_ticks.saturating_sub(1));
        let end = model.duration_ticks.saturating_sub(1);
        match key {
            Key::Escape => {
                self.bezier.open = false;
                self.press = None;
                self.drag_preview = None;
                Vec::new()
            }
            Key::Tab | Key::Return => Vec::new(),
            Key::Space => vec![NativeTimelineAction::TogglePlayback],
            Key::ToggleLoop => vec![NativeTimelineAction::SetLoop {
                enabled: !model.loop_enabled,
            }],
            Key::Home => vec![NativeTimelineAction::Seek { tick: 0 }],
            Key::End => vec![NativeTimelineAction::Seek { tick: end }],
            Key::Left => vec![NativeTimelineAction::Seek {
                tick: tick.saturating_sub(1),
            }],
            Key::Right => vec![NativeTimelineAction::Seek {
                tick: tick.saturating_add(1).min(end),
            }],
            Key::Up => self.select_adjacent(model, false),
            Key::Down => self.select_adjacent(model, true),
            Key::Delete if model.can_edit_selected_item => {
                self.selected_edit(model).into_iter().collect()
            }
            Key::Delete => Vec::new(),
            Key::DuplicateTrack | Key::DeleteTrack => Vec::new(),
            Key::AddDrawingTrack if model.can_add_drawing_track => self
                .target_action(model, |target_id| {
                    NativeTimelineAction::RequestAddDrawingTrack { target_id }
                }),
            Key::AddDrawingTrack => Vec::new(),
            Key::ImportDrawing if model.can_import_asset => {
                self.target_action(model, |target_id| NativeTimelineAction::RequestImportSvg {
                    target_id,
                    tick,
                })
            }
            Key::ImportDrawing => Vec::new(),
            Key::AddKey => self.add_key_action(model),
            Key::ZoomIn | Key::ZoomOut => self.zoom(key == Key::ZoomIn, tick, model),
            Key::PageLeft | Key::PageRight => self.page(key == Key::PageRight, model),
        }
    }

    pub fn selected_cel(&self, model: &TimelineModel) -> Option<(String, u32)> {
        let (track, item) = self
            .selected_item
            .or_else(|| model.selected_item().map(|(ti, ii, _, _)| (ti, ii)))?;
        let track = model.tracks.get(track)?;
        if !matches!(track.kind, crate::native_timeline::NativeTrackKind::Drawing) {
            return None;
        }
        Some((track.id.clone(), track.items.get(item)?.tick))
    }

    pub(super) fn selected_edit(&self, model: &TimelineModel) -> Option<NativeTimelineAction> {
        let (track_index, item_index) = self
            .selected_item
            .or_else(|| model.selected_item().map(|(ti, ii, _, _)| (ti, ii)))?;
        let track = model.tracks.get(track_index)?;
        let item = track.items.get(item_index)?;
        if item.tick == 0 {
            return None;
        }
        Some(
            if matches!(
                track.kind,
                crate::native_timeline::NativeTrackKind::Property
            ) {
                NativeTimelineAction::RemovePropertyKey {
                    track_id: track.id.clone(),
                    tick: item.tick,
                }
            } else {
                NativeTimelineAction::RemoveCel {
                    track_id: track.id.clone(),
                    tick: item.tick,
                }
            },
        )
    }

    /// The add-key intent for the selected property track, else for the
    /// selected shape (a drawing track counts as its shape). Empty when
    /// nothing is selected.
    pub(super) fn add_key_action(&self, model: &TimelineModel) -> Vec<NativeTimelineAction> {
        let tick = model
            .current_tick
            .min(model.duration_ticks.saturating_sub(1));
        let track = self
            .selected_track
            .and_then(|index| model.tracks.get(index))
            .or_else(|| model.selected_track().map(|(_, track)| track));
        if let Some(track) = track {
            if matches!(
                track.kind,
                crate::native_timeline::NativeTrackKind::Property
            ) {
                return vec![NativeTimelineAction::AddKey {
                    track_id: Some(track.id.clone()),
                    target_id: None,
                    tick,
                }];
            }
            if !track.target.is_empty() {
                return vec![NativeTimelineAction::AddKey {
                    track_id: None,
                    target_id: Some(track.target.clone()),
                    tick,
                }];
            }
        }
        model
            .selected_target
            .iter()
            .filter(|target| !target.is_empty())
            .map(|target| NativeTimelineAction::AddKey {
                track_id: None,
                target_id: Some(target.clone()),
                tick,
            })
            .collect()
    }

    pub(super) fn target_action(
        &self,
        model: &TimelineModel,
        make: impl FnOnce(String) -> NativeTimelineAction,
    ) -> Vec<NativeTimelineAction> {
        let Some(index) = self
            .selected_track
            .or_else(|| model.selected_track().map(|(index, _)| index))
        else {
            return model
                .selected_target
                .as_ref()
                .map(|target| vec![make(target.clone())])
                .unwrap_or_default();
        };
        let target = model
            .tracks
            .get(index)
            .map(|track| track.target.as_str())
            .filter(|target| !target.is_empty())
            .or_else(|| model.selected_target.as_deref());
        target
            .map(|target| vec![make(target.to_owned())])
            .unwrap_or_default()
    }

    pub(super) fn select_adjacent(
        &mut self,
        model: &TimelineModel,
        forward: bool,
    ) -> Vec<NativeTimelineAction> {
        let items: Vec<(usize, usize)> = model
            .tracks
            .iter()
            .enumerate()
            .flat_map(|(track, row)| (0..row.items.len()).map(move |item| (track, item)))
            .collect();
        if items.is_empty() {
            return Vec::new();
        }
        let current = self.selected_item.or_else(|| {
            model
                .selected_item()
                .map(|(track, item, _, _)| (track, item))
        });
        let selected_index =
            current.and_then(|selected| items.iter().position(|item| *item == selected));
        let next = match (selected_index, forward) {
            (None, true) => 0,
            (None, false) => items.len() - 1,
            (Some(index), true) => (index + 1) % items.len(),
            (Some(index), false) => index.checked_sub(1).unwrap_or(items.len() - 1),
        };
        let (track_index, item_index) = items[next];
        let track = &model.tracks[track_index];
        let Some(item) = track.items.get(item_index) else {
            return Vec::new();
        };
        self.selected_track = Some(track_index);
        self.selected_item = Some((track_index, item_index));
        vec![
            NativeTimelineAction::SelectTrack {
                track_id: track.id.clone(),
            },
            NativeTimelineAction::SelectItem {
                track_id: track.id.clone(),
                tick: item.tick,
            },
        ]
    }

    pub(super) fn zoom(
        &mut self,
        inward: bool,
        focus_tick: u32,
        model: &TimelineModel,
    ) -> Vec<NativeTimelineAction> {
        let current = self.viewport(model);
        let old_span = current.last_tick.saturating_sub(current.first_tick).max(1);
        let new_span = if inward {
            (old_span / 2).max(1)
        } else {
            old_span
                .saturating_mul(2)
                .min(model.duration_ticks.saturating_sub(1).max(1))
        };
        self.set_centered_view(focus_tick, new_span, model)
    }

    pub(super) fn page(
        &mut self,
        forward: bool,
        model: &TimelineModel,
    ) -> Vec<NativeTimelineAction> {
        let current = self.viewport(model);
        let span = current.last_tick.saturating_sub(current.first_tick).max(1);
        let shift = span.saturating_mul(4) / 5;
        let delta = if forward {
            shift as i64
        } else {
            -(shift as i64)
        };
        let first = (current.first_tick as i64 + delta)
            .clamp(0, model.duration_ticks.saturating_sub(1) as i64) as u32;
        let last = first
            .saturating_add(span)
            .min(model.duration_ticks.saturating_sub(1));
        self.set_view(
            Viewport {
                first_tick: first,
                last_tick: last,
            },
            model,
        )
    }

    pub(super) fn set_centered_view(
        &mut self,
        center: u32,
        span: u32,
        model: &TimelineModel,
    ) -> Vec<NativeTimelineAction> {
        let half = span / 2;
        let end = model.duration_ticks.saturating_sub(1);
        let first = center.saturating_sub(half);
        let first = first.min(end.saturating_sub(span.min(end)));
        let last = first.saturating_add(span).min(end);
        self.set_view(
            Viewport {
                first_tick: first,
                last_tick: last,
            },
            model,
        )
    }

    pub(super) fn set_view(
        &mut self,
        viewport: Viewport,
        model: &TimelineModel,
    ) -> Vec<NativeTimelineAction> {
        let viewport = viewport.bounded(model);
        self.viewport = Some(viewport);
        vec![NativeTimelineAction::SetViewport {
            first_tick: viewport.first_tick,
            last_tick: viewport.last_tick,
        }]
    }

    /// Horizontal trackpad or shift-wheel movement pans the visible tick range.
    /// The caller supplies already-normalized tick deltas from its input layer.
    pub fn wheel_viewport(
        &mut self,
        delta_ticks: i64,
        model: &TimelineModel,
    ) -> Vec<NativeTimelineAction> {
        if delta_ticks == 0 {
            return Vec::new();
        }
        let current = self.viewport(model);
        let span = current.last_tick.saturating_sub(current.first_tick);
        let end = model.duration_ticks.saturating_sub(1) as i64;
        let first = (current.first_tick as i64 + delta_ticks).clamp(0, end) as u32;
        let first = first.min(end.saturating_sub(span as i64).max(0) as u32);
        self.set_view(
            Viewport {
                first_tick: first,
                last_tick: first.saturating_add(span).min(end as u32),
            },
            model,
        )
    }

    /// First row drawn for a band of `visible` rows: the stored scroll, moved
    /// just far enough to show a pending reveal, clamped to the track count.
    pub fn first_row(&self, model: &TimelineModel, visible: usize) -> usize {
        let max = model.tracks.len().saturating_sub(visible);
        let mut first = self.scroll.min(max);
        let wanted = self
            .reveal
            .as_deref()
            .and_then(|id| model.tracks.iter().position(|track| track.id == id));
        if let (Some(index), true) = (wanted, visible > 0) {
            if index < first {
                first = index;
            } else if index >= first + visible {
                first = index + 1 - visible;
            }
        }
        first.min(max)
    }

    /// The layout for `panel` with this state's viewport and vertical scroll.
    pub fn layout(&self, panel: Rect, model: &TimelineModel) -> TimelineLayout {
        let base = TimelineLayout::new(panel, model, self.viewport(model));
        let first = self.first_row(model, base.visible_rows());
        base.scrolled(first, model.tracks.len())
    }

    /// Remembers the scroll `layout` shows and drops any pending reveal.
    pub(super) fn commit_scroll(&mut self, layout: &TimelineLayout) {
        self.scroll = layout.first_row;
        self.reveal = None;
    }

    /// Notes the document-selected track of a new snapshot; a change to a new
    /// track schedules a reveal so a canvas selection stays on screen.
    pub fn note_selection(&mut self, tracks: &[NativeTimelineTrack]) {
        let selected = tracks.iter().find(|t| t.selected).map(|t| t.id.as_str());
        if selected != self.seen_selected.as_deref() {
            self.seen_selected = selected.map(str::to_owned);
            if self.seen_selected.is_some() {
                self.reveal = self.seen_selected.clone();
            }
        }
    }

    /// Scrolls the rows by `delta_rows` (negative = up); true when they moved.
    pub fn scroll_rows(&mut self, delta_rows: i64, layout: &TimelineLayout, total: usize) -> bool {
        let max = layout.max_first_row(total) as i64;
        let next = (layout.first_row as i64 + delta_rows).clamp(0, max) as usize;
        let moved = next != layout.first_row;
        self.scroll = next;
        self.reveal = None;
        moved
    }

    /// Wheel/trackpad delta in points (positive = content moves down, as in
    /// the SMIL panel); accumulates to whole rows.
    pub fn scroll_points(&mut self, dy: f64, layout: &TimelineLayout, total: usize) -> bool {
        let row = f64::from(crate::native_timeline::layout::TRACK_HEIGHT);
        self.scroll_acc += dy;
        let rows = (self.scroll_acc / row).trunc();
        self.scroll_acc -= rows * row;
        rows != 0.0 && self.scroll_rows(-(rows as i64), layout, total)
    }
}
