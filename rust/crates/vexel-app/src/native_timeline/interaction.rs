//! Transient input state for the native timeline. Every returned value is an
//! application intent; this module never mutates a project or playback engine.

use super::NativeTimelineAction;
use super::bezier::{self, BezierDraft};
use super::layout::{Hit, TimelineLayout, Viewport};
use super::model::TimelineModel;

#[path = "interaction_keys.rs"]
mod keys;

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
    AddDrawingTrack,
    ImportDrawing,
    AddKey,
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
    pub bezier: BezierDraft,
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
            bezier: BezierDraft::default(),
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
        let editor_live = self.bezier.open
            && model.selected_item().is_some_and(|(_, _, track, _)| {
                matches!(track.kind, super::NativeTrackKind::Property)
            });
        if self.bezier.open && !editor_live {
            self.bezier.open = false;
        }
        let hit = editor_live
            .then(|| layout.bezier_hit(point.0, point.1))
            .flatten()
            .unwrap_or_else(|| layout.hit_test(model, point.0, point.1));
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
                    | Hit::AddKey
                    | Hit::AddDrawingTrack
                    | Hit::ImportDrawing => {
                        self.press = Some(Press::Button(hit));
                    }
                    Hit::Ease(_)
                    | Hit::EaseCustom
                    | Hit::BezierStep { .. }
                    | Hit::BezierApply
                    | Hit::BezierCancel => self.press = Some(Press::Button(hit)),
                    Hit::BezierPanel => {}
                    Hit::Ruler | Hit::Lane(_) => {
                        self.press = Some(Press::Seek);
                    }
                    Hit::Track(index) => {
                        self.bezier.open = false;
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
                            self.bezier.open = false;
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
                        Hit::EaseCustom => {
                            if self.bezier.open {
                                self.bezier.open = false;
                            } else if let Some((_, _, _, item)) = model.selected_item() {
                                let current = item.ease.as_deref().and_then(bezier::parse);
                                self.bezier.set(current.unwrap_or(bezier::DEFAULT));
                                self.bezier.open = true;
                            }
                        }
                        Hit::BezierStep { field, up } => self.bezier.step(field, up),
                        Hit::BezierCancel => self.bezier.open = false,
                        Hit::BezierApply => {
                            if let Some((_, _, track, item)) = model.selected_item() {
                                actions.push(NativeTimelineAction::SetPropertyKeyEase {
                                    track_id: track.id.clone(),
                                    tick: item.tick,
                                    value: item.label.clone(),
                                    ease: bezier::encode(self.bezier.values),
                                });
                            }
                            self.bezier.open = false;
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
                        Hit::AddKey => actions.extend(self.add_key_action(model)),
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
                property: String::new(),
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

    #[test]
    fn add_key_targets_the_selected_track_else_the_shape_and_is_empty_without_either() {
        use super::super::layout::{TimelineLayout, Viewport};
        let mut state = InteractionState::default();
        let expected = vec![NativeTimelineAction::AddKey {
            track_id: None,
            target_id: Some("character".into()),
            tick: 20,
        }];
        assert_eq!(state.key(Key::AddKey, &model()), expected);
        let mut bare = model();
        bare.tracks.clear();
        assert!(state.key(Key::AddKey, &bare).is_empty());
        bare.selected_target = Some("shape".into());
        assert_eq!(
            state.key(Key::AddKey, &bare),
            vec![NativeTimelineAction::AddKey {
                track_id: None,
                target_id: Some("shape".into()),
                tick: 20,
            }]
        );
        // The transport button emits the same intent on a click.
        let model = model();
        let panel = vexel_ui::Rect {
            x: 0,
            y: 0,
            width: 720,
            height: 180,
        };
        let layout = TimelineLayout::new(panel, &model, Viewport::full(&model));
        let (_, r) = layout
            .control_rects()
            .into_iter()
            .find(|(hit, _)| *hit == Hit::AddKey)
            .unwrap();
        let at = ((r.x + r.width / 2) as i32, (r.y + r.height / 2) as i32);
        assert!(state.pointer(Phase::Down, at, &layout, &model).is_empty());
        assert_eq!(state.pointer(Phase::Up, at, &layout, &model), expected);
    }
}
