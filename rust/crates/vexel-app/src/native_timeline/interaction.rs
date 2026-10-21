//! Transient input state for the native timeline. Every returned value is an
//! application intent; this module never mutates a project or playback engine.

use super::NativeTimelineAction;
use super::layout::{Hit, TimelineLayout, Viewport};
use super::model::TimelineModel;

const DRAG_SLOP: i32 = 3;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Phase {
    Down,
    Move,
    Up,
    Cancel,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Key {
    Tab,
    Return,
    Space,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    Delete,
    Escape,
    ToggleLoop,
    OnionBefore,
    OnionAfter,
    AddDrawingTrack,
    ImportDrawing,
    AddOpacityKey,
    DuplicateTrack,
    DeleteTrack,
    ZoomIn,
    ZoomOut,
    PageLeft,
    PageRight,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DragPreview {
    pub track_index: usize,
    pub from_tick: u32,
    pub to_tick: u32,
}

#[derive(Clone, Debug, PartialEq)]
enum Press {
    Button(Hit),
    Seek,
    Item {
        track: usize,
        tick: u32,
        origin_x: i32,
        property: bool,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct InteractionState {
    pub viewport: Option<Viewport>,
    pub focus: bool,
    pub selected_track: Option<usize>,
    pub selected_item: Option<(usize, usize)>,
    pub drag_preview: Option<DragPreview>,
    pub hover: Option<Hit>,
    press: Option<Press>,
}

impl Default for InteractionState {
    fn default() -> Self {
        Self {
            viewport: None,
            focus: false,
            selected_track: None,
            selected_item: None,
            drag_preview: None,
            hover: None,
            press: None,
        }
    }
}

impl InteractionState {
    pub fn pressed(&self) -> Option<Hit> {
        match self.press {
            Some(Press::Button(hit)) => Some(hit),
            _ => None,
        }
    }
    pub fn viewport(&self, model: &TimelineModel) -> Viewport {
        self.viewport
            .unwrap_or_else(|| Viewport::full(model))
            .bounded(model)
    }

    pub fn pointer(
        &mut self,
        phase: Phase,
        point: (i32, i32),
        layout: &TimelineLayout,
        model: &TimelineModel,
    ) -> Vec<NativeTimelineAction> {
        let hit = layout.hit_test(model, point.0, point.1);
        let mut actions = Vec::with_capacity(2);
        match phase {
            Phase::Down => {
                if hit == Hit::Empty {
                    return actions;
                }
                self.press = None;
                self.focus = true;
                self.hover = Some(hit);
                match hit {
                    Hit::ModeNative
                    | Hit::ModeSmil
                    | Hit::Play
                    | Hit::Loop
                    | Hit::OnionBefore
                    | Hit::OnionAfter
                    | Hit::AddDrawingTrack
                    | Hit::ImportDrawing => {
                        self.press = Some(Press::Button(hit));
                    }
                    Hit::Ease(_) => self.press = Some(Press::Button(hit)),
                    Hit::Ruler | Hit::Lane(_) => {
                        self.press = Some(Press::Seek);
                    }
                    Hit::Track(index) => {
                        if let Some(track) = model.tracks.get(index) {
                            self.selected_track = Some(index);
                            self.selected_item = None;
                            actions.push(NativeTimelineAction::SelectTrack {
                                track_id: track.id.clone(),
                            });
                        }
                    }
                    Hit::DuplicateTrack(_) | Hit::DeleteTrack(_) => {
                        self.press = Some(Press::Button(hit))
                    }
                    Hit::Visibility(index) => {
                        if let Some(track) = model.tracks.get(index) {
                            actions.push(NativeTimelineAction::SetTrackVisibility {
                                track_id: track.id.clone(),
                                visible: !track.visible,
                            });
                        }
                    }
                    Hit::Item { track, item } => {
                        if let (Some(tr), Some(it)) = (
                            model.tracks.get(track),
                            model.tracks.get(track).and_then(|t| t.items.get(item)),
                        ) {
                            self.selected_track = Some(track);
                            self.selected_item = Some((track, item));
                            self.press = Some(Press::Item {
                                track,
                                tick: it.tick,
                                origin_x: point.0,
                                property: matches!(tr.kind, super::NativeTrackKind::Property),
                            });
                            actions.push(NativeTimelineAction::SelectTrack {
                                track_id: tr.id.clone(),
                            });
                            actions.push(NativeTimelineAction::SelectItem {
                                track_id: tr.id.clone(),
                                tick: it.tick,
                            });
                        }
                    }
                    Hit::Empty => {}
                }
            }
            Phase::Move => {
                self.hover = (hit != Hit::Empty).then_some(hit);
                if let Some(Press::Item {
                    track,
                    tick,
                    origin_x,
                    ..
                }) = &self.press
                {
                    if point.0.abs_diff(*origin_x) >= DRAG_SLOP as u32 {
                        let mut to_tick = layout.tick_at(point.0);
                        if matches!(&self.press, Some(Press::Item { property: true, .. })) {
                            to_tick = to_tick.max(1);
                        }
                        self.drag_preview = Some(DragPreview {
                            track_index: *track,
                            from_tick: *tick,
                            to_tick,
                        });
                    }
                }
            }
            Phase::Up => {
                let press = self.press.take();
                match press {
                    Some(Press::Button(button)) if button == hit => match button {
                        Hit::DuplicateTrack(index) => {
                            if let Some(track) = model.tracks.get(index) {
                                actions.push(NativeTimelineAction::DuplicateTrack {
                                    track_id: track.id.clone(),
                                });
                            }
                        }
                        Hit::DeleteTrack(index) => {
                            if let Some(track) = model.tracks.get(index) {
                                actions.push(NativeTimelineAction::RemoveTrack {
                                    track_id: track.id.clone(),
                                });
                            }
                        }
                        Hit::Ease(ease) => {
                            if let Some((_, _, track, item)) = model.selected_item() {
                                actions.push(NativeTimelineAction::SetPropertyKeyEase {
                                    track_id: track.id.clone(),
                                    tick: item.tick,
                                    value: item.label.clone(),
                                    ease: ease.to_owned(),
                                });
                            }
                        }
                        Hit::Play => actions.push(NativeTimelineAction::TogglePlayback),
                        Hit::Loop => actions.push(NativeTimelineAction::SetLoop {
                            enabled: !model.loop_enabled,
                        }),
                        Hit::OnionBefore => actions.push(NativeTimelineAction::SetOnion {
                            before: model.onion_before.saturating_sub(1),
                            after: model.onion_after,
                            opacity: model.onion_opacity,
                        }),
                        Hit::OnionAfter => actions.push(NativeTimelineAction::SetOnion {
                            before: model.onion_before,
                            after: model.onion_after.saturating_add(1).min(8),
                            opacity: model.onion_opacity,
                        }),
                        Hit::AddDrawingTrack if model.can_add_drawing_track => {
                            actions.extend(self.target_action(model, |target_id| {
                                NativeTimelineAction::RequestAddDrawingTrack { target_id }
                            }));
                        }
                        Hit::ImportDrawing if model.can_import_asset => {
                            actions.extend(self.target_action(model, |target_id| {
                                NativeTimelineAction::RequestImportSvg {
                                    target_id,
                                    tick: model.current_tick,
                                }
                            }));
                        }
                        Hit::ModeNative | Hit::ModeSmil => {}
                        _ => {}
                    },
                    Some(Press::Seek) => actions.push(NativeTimelineAction::Seek {
                        tick: layout.tick_at(point.0),
                    }),
                    Some(Press::Item {
                        track,
                        tick,
                        origin_x,
                        property,
                    }) => {
                        if point.0.abs_diff(origin_x) >= DRAG_SLOP as u32 {
                            let mut to_tick = layout.tick_at(point.0);
                            if property {
                                to_tick = to_tick.max(1);
                            }
                            if to_tick != tick {
                                if let Some(tr) = model.tracks.get(track) {
                                    if model.can_edit_selected_item && property {
                                        actions.push(NativeTimelineAction::MovePropertyKey {
                                            track_id: tr.id.clone(),
                                            from_tick: tick,
                                            to_tick,
                                        });
                                    } else if model.can_edit_selected_item
                                        && matches!(tr.kind, super::NativeTrackKind::Drawing)
                                    {
                                        actions.push(NativeTimelineAction::MoveCel {
                                            track_id: tr.id.clone(),
                                            from_tick: tick,
                                            to_tick,
                                        });
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
                self.drag_preview = None;
            }
            Phase::Cancel => {
                self.press = None;
                self.drag_preview = None;
            }
        }
        if matches!(phase, Phase::Up | Phase::Cancel) {
            self.hover = (hit != Hit::Empty).then_some(hit);
        }
        actions
    }

    pub fn key(&mut self, key: Key, model: &TimelineModel) -> Vec<NativeTimelineAction> {
        self.focus = true;
        let tick = model
            .current_tick
            .min(model.duration_ticks.saturating_sub(1));
        let end = model.duration_ticks.saturating_sub(1);
        match key {
            Key::Escape => {
                self.press = None;
                self.drag_preview = None;
                Vec::new()
            }
            Key::Tab | Key::Return => Vec::new(),
            Key::Space => vec![NativeTimelineAction::TogglePlayback],
            Key::ToggleLoop => vec![NativeTimelineAction::SetLoop {
                enabled: !model.loop_enabled,
            }],
            Key::OnionBefore => vec![NativeTimelineAction::SetOnion {
                before: model.onion_before.saturating_sub(1),
                after: model.onion_after,
                opacity: model.onion_opacity,
            }],
            Key::OnionAfter => vec![NativeTimelineAction::SetOnion {
                before: model.onion_before,
                after: model.onion_after.saturating_add(1).min(8),
                opacity: model.onion_opacity,
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
            Key::AddOpacityKey if model.can_edit_selected_item => {
                self.target_action(model, |target_id| NativeTimelineAction::AddOpacityKey {
                    target_id,
                    tick,
                    value: 1.0,
                })
            }
            Key::AddOpacityKey => Vec::new(),
            Key::ZoomIn | Key::ZoomOut => self.zoom(key == Key::ZoomIn, tick, model),
            Key::PageLeft | Key::PageRight => self.page(key == Key::PageRight, model),
        }
    }

    pub fn selected_cel(&self, model: &TimelineModel) -> Option<(String, u32)> {
        let (track, item) = self
            .selected_item
            .or_else(|| model.selected_item().map(|(ti, ii, _, _)| (ti, ii)))?;
        let track = model.tracks.get(track)?;
        if !matches!(track.kind, super::NativeTrackKind::Drawing) {
            return None;
        }
        Some((track.id.clone(), track.items.get(item)?.tick))
    }

    fn selected_edit(&self, model: &TimelineModel) -> Option<NativeTimelineAction> {
        let (track_index, item_index) = self
            .selected_item
            .or_else(|| model.selected_item().map(|(ti, ii, _, _)| (ti, ii)))?;
        let track = model.tracks.get(track_index)?;
        let item = track.items.get(item_index)?;
        if item.tick == 0 {
            return None;
        }
        Some(if matches!(track.kind, super::NativeTrackKind::Property) {
            NativeTimelineAction::RemovePropertyKey {
                track_id: track.id.clone(),
                tick: item.tick,
            }
        } else {
            NativeTimelineAction::RemoveCel {
                track_id: track.id.clone(),
                tick: item.tick,
            }
        })
    }

    fn target_action(
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

    fn select_adjacent(
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

    fn zoom(
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

    fn page(&mut self, forward: bool, model: &TimelineModel) -> Vec<NativeTimelineAction> {
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

    fn set_centered_view(
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

    fn set_view(&mut self, viewport: Viewport, model: &TimelineModel) -> Vec<NativeTimelineAction> {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_timeline::layout::TimelineLayout;
    use crate::native_timeline::{
        NativeTimelineItem, NativeTimelineSnapshot, NativeTimelineTrack, NativeTrackKind,
    };

    fn model() -> TimelineModel {
        TimelineModel::from_snapshot(&NativeTimelineSnapshot {
            ticks_per_second: 24,
            duration_ticks: 100,
            current_tick: 20,
            tracks: vec![NativeTimelineTrack {
                id: "body".into(),
                name: "Body".into(),
                target: "character".into(),
                kind: NativeTrackKind::Drawing,
                visible: true,
                selected: true,
                items: vec![NativeTimelineItem {
                    tick: 10,
                    label: "Pose A".into(),
                    selected: true,
                    ease: None,
                }],
            }],
            selected_target: None,
            playing: false,
            loop_enabled: true,
            onion_before: 1,
            onion_after: 1,
            onion_opacity: 0.25,
            can_add_drawing_track: true,
            can_import_asset: true,
            can_edit_selected_item: true,
            status: None,
        })
    }

    #[test]
    fn escape_discards_drag_preview_without_emitting_an_edit() {
        let mut state = InteractionState::default();
        state.drag_preview = Some(DragPreview {
            track_index: 0,
            from_tick: 10,
            to_tick: 15,
        });
        let actions = state.key(Key::Escape, &model());
        assert!(actions.is_empty());
        assert!(state.drag_preview.is_none());
    }

    #[test]
    fn keyboard_transport_emits_intents_only() {
        let mut state = InteractionState::default();
        assert_eq!(
            state.key(Key::Space, &model()),
            vec![NativeTimelineAction::TogglePlayback]
        );
        assert_eq!(
            state.key(Key::Right, &model()),
            vec![NativeTimelineAction::Seek { tick: 21 }]
        );
    }

    #[test]
    fn first_track_can_target_the_canvas_selection_without_existing_rows() {
        let mut model = model();
        model.tracks.clear();
        model.selected_target = Some("character".into());
        let actions = InteractionState::default().key(Key::AddDrawingTrack, &model);
        assert_eq!(
            actions,
            vec![NativeTimelineAction::RequestAddDrawingTrack {
                target_id: "character".into(),
            }]
        );
    }

    #[test]
    fn cel_drag_keeps_preview_transient_and_emits_one_edit_on_release() {
        let model = model();
        let layout = TimelineLayout::new(
            vexel_ui::Rect {
                x: 0,
                y: 0,
                width: 600,
                height: 120,
            },
            &model,
            Viewport::full(&model),
        );
        let start_x = layout.lane_x(10).round() as i32;
        let end_x = layout.lane_x(30).round() as i32;
        let y = (layout.rows.y + 12) as i32;
        let mut state = InteractionState::default();
        let down = state.pointer(Phase::Down, (start_x, y), &layout, &model);
        assert_eq!(down.len(), 2);
        let moving = state.pointer(Phase::Move, (end_x, y), &layout, &model);
        assert!(moving.is_empty());
        assert_eq!(state.drag_preview.map(|preview| preview.to_tick), Some(30));
        let release = state.pointer(Phase::Up, (end_x, y), &layout, &model);
        assert_eq!(
            release,
            vec![NativeTimelineAction::MoveCel {
                track_id: "body".into(),
                from_tick: 10,
                to_tick: 30,
            }]
        );
        assert!(state.drag_preview.is_none());
    }

    #[test]
    fn property_key_drag_selection_and_zero_protection_emit_one_move() {
        let mut model = model();
        model.tracks[0].kind = super::super::NativeTrackKind::Property;
        model.tracks[0].items.push(super::super::model::Item {
            tick: 20,
            label: "0.5".into(),
            selected: false,
            ease: None,
        });
        let layout = TimelineLayout::new(
            vexel_ui::Rect {
                x: 0,
                y: 0,
                width: 600,
                height: 120,
            },
            &model,
            Viewport::full(&model),
        );
        let y = (layout.rows.y + 12) as i32;
        let x = layout.lane_x(20).round() as i32;
        let mut state = InteractionState::default();
        let selected = state.pointer(Phase::Down, (x, y), &layout, &model);
        assert!(
            selected
                .iter()
                .any(|action| matches!(action, NativeTimelineAction::SelectItem { tick: 20, .. }))
        );
        let zero_x = layout.lane_x(0).round() as i32;
        state.pointer(Phase::Move, (zero_x, y), &layout, &model);
        assert_eq!(state.drag_preview.map(|preview| preview.to_tick), Some(1));
        assert!(
            state
                .pointer(Phase::Up, (zero_x, y), &layout, &model)
                .iter()
                .any(|action| matches!(
                    action,
                    NativeTimelineAction::MovePropertyKey { to_tick: 1, .. }
                ))
        );
        let mut zero_selected = model.clone();
        zero_selected.tracks[0].items = vec![super::super::model::Item {
            tick: 0,
            label: "1.0".into(),
            selected: true,
            ease: None,
        }];
        state.selected_item = Some((0, 0));
        assert!(
            state.key(Key::Delete, &zero_selected).is_empty(),
            "tick zero is immutable"
        );
    }

    #[test]
    fn property_key_delete_and_track_buttons_emit_edits() {
        let mut model = model();
        model.tracks[0].kind = super::super::NativeTrackKind::Property;
        let layout = TimelineLayout::new(
            vexel_ui::Rect {
                x: 0,
                y: 0,
                width: 600,
                height: 120,
            },
            &model,
            Viewport::full(&model),
        );
        let mut state = InteractionState::default();
        assert_eq!(
            state.key(Key::Delete, &model),
            vec![NativeTimelineAction::RemovePropertyKey {
                track_id: "body".into(),
                tick: 10
            }]
        );
        let duplicate = layout.hit_test(&model, 560, (layout.rows.y + 10) as i32);
        assert_eq!(duplicate, Hit::DuplicateTrack(0));
        let p = (560, (layout.rows.y + 10) as i32);
        state.pointer(Phase::Down, p, &layout, &model);
        assert_eq!(
            state.pointer(Phase::Up, p, &layout, &model),
            vec![NativeTimelineAction::DuplicateTrack {
                track_id: "body".into()
            }]
        );
    }
}
