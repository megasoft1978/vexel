//! Footer presentation for choosing native keyframes or the SVG SMIL timeline.
//!
//! The selector owns only display state and input focus. Native commands are
//! still intents; the application decides which timeline drives the canvas.

use super::interaction::{InteractionState, Key, Phase};
use super::layout::{Hit, TimelineLayout};
use super::model::TimelineModel;
use super::{NativeTimelineAction, NativeTimelineSnapshot};
use crate::layout::paint::centred;
use vexel_ui::{FontWeight, Rect, WidgetCanvas};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TimelineMode {
    #[default]
    Native,
    SvgSmil,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PresentationAction {
    SelectMode(TimelineMode),
    Native(NativeTimelineAction),
}

/// Transient presentation state. This does not own a session, renderer, or
/// project object.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TimelinePresentation {
    pub mode: TimelineMode,
    pub interaction: InteractionState,
    mode_focus: bool,
    mode_hover: Option<TimelineMode>,
    mode_press: Option<TimelineMode>,
}

impl TimelinePresentation {
    /// Paints the native footer when selected, then overlays the selector.
    /// In SVG SMIL mode, the caller paints the existing SMIL panel first.
    pub fn paint(
        &self,
        canvas: &mut WidgetCanvas<'_>,
        panel: Rect,
        snapshot: &NativeTimelineSnapshot,
    ) {
        let model = TimelineModel::from_snapshot(snapshot);
        let layout = TimelineLayout::new(panel, &model, self.interaction.viewport(&model));
        if self.mode == TimelineMode::Native {
            super::paint::paint(canvas, panel, &model, &self.interaction);
        }
        paint_selector(canvas, &layout, self.mode, self.mode_focus, self.mode_hover);
    }

    /// Routes selector gestures first and otherwise delegates to the native
    /// panel's transient interaction state.
    pub fn pointer(
        &mut self,
        phase: Phase,
        point: (i32, i32),
        layout: &TimelineLayout,
        snapshot: &NativeTimelineSnapshot,
    ) -> Vec<PresentationAction> {
        let model = TimelineModel::from_snapshot(snapshot);
        let hit = layout.hit_test(&model, point.0, point.1);
        let selected = match hit {
            Hit::ModeNative => Some(TimelineMode::Native),
            Hit::ModeSmil => Some(TimelineMode::SvgSmil),
            _ => None,
        };
        if selected.is_some() || self.mode_press.is_some() {
            match phase {
                Phase::Down => {
                    self.interaction
                        .pointer(Phase::Cancel, point, layout, &model);
                    self.mode_focus = true;
                    self.mode_press = selected;
                    self.mode_hover = selected;
                }
                Phase::Move => self.mode_hover = selected,
                Phase::Up => {
                    let pressed = self.mode_press.take();
                    self.mode_hover = selected;
                    if pressed.is_some() && pressed == selected {
                        let mode = selected.expect("pressed selector mode remains selected");
                        self.mode = mode;
                        return vec![PresentationAction::SelectMode(mode)];
                    }
                }
                Phase::Cancel => {
                    self.mode_press = None;
                    self.mode_hover = None;
                }
            }
            return Vec::new();
        }
        self.mode_hover = None;
        if self.mode == TimelineMode::SvgSmil {
            return Vec::new();
        }
        self.interaction
            .pointer(phase, point, layout, &model)
            .into_iter()
            .map(PresentationAction::Native)
            .collect()
    }

    pub fn key(&mut self, key: Key, snapshot: &NativeTimelineSnapshot) -> Vec<PresentationAction> {
        if key == Key::Tab {
            self.mode_focus = true;
            self.interaction.focus = true;
            return Vec::new();
        }
        if self.mode_focus {
            let next = match key {
                Key::Escape => {
                    self.mode_focus = false;
                    return Vec::new();
                }
                Key::Left => Some(TimelineMode::Native),
                Key::Right => Some(TimelineMode::SvgSmil),
                Key::Return | Key::Space => Some(self.mode),
                _ => None,
            };
            if let Some(mode) = next {
                self.mode = mode;
                return vec![PresentationAction::SelectMode(mode)];
            }
            return Vec::new();
        }
        if self.mode == TimelineMode::SvgSmil {
            return Vec::new();
        }
        let model = TimelineModel::from_snapshot(snapshot);
        self.interaction
            .key(key, &model)
            .into_iter()
            .map(PresentationAction::Native)
            .collect()
    }

    pub fn hovered_tooltip(&self) -> Option<Hit> {
        match self.interaction.hover {
            Some(
                hit @ (Hit::Play
                | Hit::Loop
                | Hit::OnionBefore
                | Hit::AddDrawingTrack
                | Hit::ImportDrawing
                | Hit::DuplicateTrack(_)
                | Hit::DeleteTrack(_)
                | Hit::Ease(_)
                | Hit::EaseCustom),
            ) => Some(hit),
            _ => None,
        }
    }
}

pub fn paint_selector(
    canvas: &mut WidgetCanvas<'_>,
    layout: &TimelineLayout,
    selected: TimelineMode,
    focused: bool,
    hovered: Option<TimelineMode>,
) {
    let bounds = layout.mode_selector();
    if bounds.width < 28 || bounds.height == 0 {
        return;
    }
    let tokens = canvas.tokens;
    canvas.paint_round(bounds, 6, tokens.panel_raised, tokens.separator, 1);
    let half = bounds.width / 2;
    for mode in [TimelineMode::Native, TimelineMode::SvgSmil] {
        let index = u32::from(mode == TimelineMode::SvgSmil);
        let cell = Rect {
            x: bounds.x.saturating_add(index.saturating_mul(half)),
            y: bounds.y.saturating_add(2),
            width: if index == 0 {
                half.saturating_sub(1)
            } else {
                bounds.width.saturating_sub(index.saturating_mul(half))
            },
            height: bounds.height.saturating_sub(4),
        };
        let active = mode == selected;
        if active {
            canvas.paint_round(cell, 4, tokens.selection_fill, 0, 0);
        } else if hovered == Some(mode) {
            canvas.paint_round(cell, 4, tokens.panel_hover, 0, 0);
        }
        centred(
            canvas,
            match mode {
                TimelineMode::Native => "Native",
                TimelineMode::SvgSmil => "SVG SMIL",
            },
            cell,
            if active {
                tokens.accent
            } else {
                tokens.text_secondary
            },
            11,
            if active {
                FontWeight::Semibold
            } else {
                FontWeight::Regular
            },
        );
    }
    if focused && bounds.width >= 4 && bounds.height >= 4 {
        canvas.paint_round(bounds, 6, 0, tokens.accent, 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_timeline::{NativeTimelineSnapshot, NativeTimelineTrack, NativeTrackKind};

    fn snapshot() -> NativeTimelineSnapshot {
        NativeTimelineSnapshot {
            ticks_per_second: 24,
            duration_ticks: 20,
            current_tick: 0,
            tracks: vec![NativeTimelineTrack {
                id: "body".into(),
                name: "Body".into(),
                target: "character".into(),
                kind: NativeTrackKind::Drawing,
                visible: true,
                selected: false,
                items: Vec::new(),
            }],
            selected_target: None,
            playing: false,
            loop_enabled: false,
            onion_before: 2,
            onion_after: 2,
            onion_opacity: 0.25,
            can_add_drawing_track: false,
            can_import_asset: false,
            can_edit_selected_item: false,
            status: Some("Editing unavailable".into()),
        }
    }

    fn layout(snapshot: &NativeTimelineSnapshot) -> (TimelineModel, TimelineLayout) {
        let model = TimelineModel::from_snapshot(snapshot);
        let panel = Rect {
            x: 0,
            y: 0,
            width: 720,
            height: 180,
        };
        let layout =
            TimelineLayout::new(panel, &model, super::super::layout::Viewport::full(&model));
        (model, layout)
    }

    #[test]
    fn mode_change_is_emitted_once_on_selector_release() {
        let snapshot = snapshot();
        let (_, layout) = layout(&snapshot);
        let selector = layout.mode_selector();
        let point = (
            (selector.x + selector.width * 3 / 4) as i32,
            (selector.y + selector.height / 2) as i32,
        );
        let mut presentation = TimelinePresentation::default();
        assert!(
            presentation
                .pointer(Phase::Down, point, &layout, &snapshot)
                .is_empty()
        );
        assert!(
            presentation
                .pointer(Phase::Move, point, &layout, &snapshot)
                .is_empty()
        );
        assert_eq!(presentation.mode, TimelineMode::Native);
        assert_eq!(
            presentation.pointer(Phase::Up, point, &layout, &snapshot),
            vec![PresentationAction::SelectMode(TimelineMode::SvgSmil)]
        );
        assert_eq!(presentation.mode, TimelineMode::SvgSmil);
    }

    #[test]
    fn selector_keyboard_focus_switches_modes_without_native_intents() {
        let snapshot = snapshot();
        let mut presentation = TimelinePresentation::default();
        assert!(presentation.key(Key::Tab, &snapshot).is_empty());
        assert_eq!(
            presentation.key(Key::Right, &snapshot),
            vec![PresentationAction::SelectMode(TimelineMode::SvgSmil)]
        );
        assert_eq!(
            presentation.key(Key::Space, &snapshot),
            vec![PresentationAction::SelectMode(TimelineMode::SvgSmil)]
        );
        assert!(presentation.key(Key::Escape, &snapshot).is_empty());
        assert_eq!(
            presentation.key(Key::Space, &snapshot),
            Vec::<PresentationAction>::new()
        );
    }

    #[test]
    fn hovering_each_icon_exposes_its_tooltip_target() {
        let snapshot = snapshot();
        let (_, layout) = layout(&snapshot);
        for (hit, _) in layout.control_rects() {
            let rect = layout
                .control_rects()
                .into_iter()
                .find(|(candidate, _)| *candidate == hit)
                .unwrap()
                .1;
            let point = (
                (rect.x + rect.width / 2) as i32,
                (rect.y + rect.height / 2) as i32,
            );
            let mut presentation = TimelinePresentation::default();
            let _ = presentation.pointer(Phase::Move, point, &layout, &snapshot);
            assert_eq!(presentation.hovered_tooltip(), Some(hit));
        }
    }
}
