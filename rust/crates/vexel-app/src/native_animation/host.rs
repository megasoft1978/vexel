//! Shell integration for native timeline presentation, commands and playback.

use crate::animation::native_driver::NativeDriver;
use crate::animation::native_stage::{NativeStage, OnionSettings, StageUpdate};
use crate::animation::playback::LoopClock;
use crate::compose::{Frame, svg_viewport};
use crate::file_ui::FileUi;
use crate::native_timeline::interaction::{Key, Phase};
use crate::native_timeline::layout::{Hit, TimelineLayout};
use crate::native_timeline::model::TimelineModel;
use crate::native_timeline::presentation::{
    PresentationAction, TimelineMode, TimelinePresentation,
};
use crate::native_timeline::{NativeTimelineAction, NativeTimelineSnapshot};
use crate::preview::Control;
use crate::session::AppDocument;
use std::sync::Arc;
use vexel_platform_apple::{PanelBackend, RgbaFrame};
use vexel_timeline::{EditLimits, EmptyTimelineSettings};

pub fn key_for_code(code: u16) -> Option<Key> {
    Some(match code {
        48 => Key::Tab,
        36 | 76 => Key::Return,
        49 => Key::Space,
        123 => Key::Left,
        124 => Key::Right,
        126 => Key::Up,
        125 => Key::Down,
        115 => Key::Home,
        119 => Key::End,
        51 | 117 => Key::Delete,
        53 => Key::Escape,
        116 => Key::PageLeft,
        121 => Key::PageRight,
        24 => Key::ZoomIn,
        27 => Key::ZoomOut,
        _ => return None,
    })
}

pub fn pointer_accepts_mode(button: Option<u8>) -> bool {
    button == Some(0)
}

use super::{
    NativeAnimationController, NativeAnimationError, NativeFrameTag, NativeRenderRequest,
    NativeTimelineEffect, TimelineSurface,
};

/// Owns the transient native panel state and its independently clocked frame stage.
pub struct NativeAnimationHost<C: LoopClock> {
    controller: NativeAnimationController,
    driver: NativeDriver<C>,
    stage: NativeStage,
    presentation: TimelinePresentation,
    state_revision: Option<u64>,
    pointer_capture: bool,
    last_tick: Option<u32>,
}

impl<C: LoopClock> NativeAnimationHost<C> {
    pub fn new(clock: C) -> Self {
        let mut controller = NativeAnimationController::default();
        controller.set_surface(TimelineSurface::Native);
        let mut presentation = TimelinePresentation::default();
        presentation.mode = TimelineMode::Native;
        Self {
            controller,
            driver: NativeDriver::new(clock),
            stage: NativeStage::new(),
            presentation,
            state_revision: None,
            pointer_capture: false,
            last_tick: None,
        }
    }

    /// Discards selection, gestures, cached frames and playback tied to a replaced document.
    pub fn document_replaced(&mut self) {
        self.controller.document_replaced();
        self.driver.invalidate();
        self.driver.pause();
        self.stage = NativeStage::new();
        self.presentation = TimelinePresentation::default();
        self.presentation.mode = TimelineMode::Native;
        self.controller.set_surface(TimelineSurface::Native);
        self.state_revision = None;
        self.pointer_capture = false;
        self.last_tick = None;
    }

    pub fn surface(&self) -> TimelineSurface {
        self.controller.surface()
    }

    /// True when the document has a native timeline with at least one track.
    pub fn has_native_timeline(&self, document: &AppDocument) -> bool {
        self.controller.has_native_timeline(document)
    }

    /// Applies an MCP play, pause or seek to the native timeline and returns
    /// the reply with the state the clock now reports. Play needs the Animate
    /// panel (`animating`) on the Native surface, because frames are sampled
    /// only there; otherwise the window would not show the playback.
    pub fn mcp_control(
        &mut self,
        document: &AppDocument,
        control: Control,
        animating: bool,
    ) -> Result<String, &'static str> {
        self.observe_revision(document.state_revision());
        self.controller
            .ensure_candidate(document)
            .map_err(|_| "unsupported native timeline")?;
        let (duration, ticks_per_second, loop_mode) = {
            let candidate = self
                .controller
                .candidate
                .as_ref()
                .ok_or("unsupported native timeline")?;
            let view = candidate
                .view()
                .map_err(|_| "unsupported native timeline")?;
            (
                view.duration_ticks(),
                view.ticks_per_second(),
                view.loop_mode(),
            )
        };
        if duration == 0 || ticks_per_second == 0 {
            return Err("unsupported native timeline");
        }
        self.driver.sync_timing(
            document.state_revision(),
            duration,
            ticks_per_second,
            loop_mode,
        );
        match control {
            Control::Play => {
                if !animating || self.surface() != TimelineSurface::Native {
                    return Err("native timeline plays in Animate mode on the Native timeline");
                }
                if !self.driver.play() {
                    return Err("no preview animation to play");
                }
            }
            Control::Pause => self.driver.pause(),
            Control::Seek(seconds) => {
                if !seconds.is_finite() || !(0.0..=10.0).contains(&seconds) {
                    return Err("invalid preview animation time");
                }
                self.driver.seek(seconds);
                self.driver.invalidate();
            }
        }
        self.controller.playing = self.driver.is_playing();
        let seconds = self.driver.time();
        let tick = self.driver.sample_tick(seconds);
        self.controller.current_tick = tick;
        self.last_tick = (!self.controller.playing).then_some(tick);
        Ok(format!(
            "{{\"ok\":true,\"playing\":{},\"tick\":{tick},\"timeSeconds\":{seconds:.3}}}",
            self.controller.playing
        ))
    }

    pub fn hovered_tooltip(&self) -> Option<Hit> {
        if self.surface() == TimelineSurface::Native {
            self.presentation.hovered_tooltip()
        } else {
            None
        }
    }

    pub fn is_playing(&self) -> bool {
        self.driver.is_playing()
    }

    pub fn deactivate(&mut self) {
        if self.controller.playing || self.driver.is_playing() {
            self.controller.playing = false;
            self.driver.pause();
            self.driver.invalidate();
        }
        if self.pointer_capture {
            self.reset_presentation_interaction();
        }
    }

    pub fn snapshot(
        &mut self,
        document: &AppDocument,
        selected_target: Option<String>,
    ) -> Result<NativeTimelineSnapshot, NativeAnimationError> {
        self.observe_revision(document.state_revision());
        self.controller.set_selected_target(selected_target);
        self.controller.snapshot(document)
    }

    pub fn prepare_document(
        &mut self,
        document: &AppDocument,
        view: &mut crate::compose::DocumentView,
    ) -> bool {
        let changed = self.observe_revision(document.state_revision());
        if changed {
            view.restore_static();
        }
        changed
    }

    /// Routes one presentation event and dispatches each emitted native intent once.
    pub fn pointer<B: PanelBackend>(
        &mut self,
        phase: Phase,
        point: (i32, i32),
        panel: vexel_ui::Rect,
        files: &mut FileUi<B>,
        selected_target: Option<String>,
    ) -> Result<bool, NativeAnimationError> {
        let inside = point.0 >= panel.x as i32
            && point.1 >= panel.y as i32
            && point.0 < panel.x.saturating_add(panel.width) as i32
            && point.1 < panel.y.saturating_add(panel.height) as i32;
        if (phase == Phase::Down && !inside) || (!inside && !self.pointer_capture) {
            return Ok(false);
        }
        let snapshot = self.snapshot(files.document(), selected_target)?;
        let model = TimelineModel::from_snapshot(&snapshot);
        let viewport = self.presentation.interaction.viewport(&model);
        let layout = TimelineLayout::new(panel, &model, viewport);
        if self.presentation.mode == TimelineMode::SvgSmil
            && !self.pointer_capture
            && !matches!(
                layout.hit_test(&model, point.0, point.1),
                Hit::ModeNative | Hit::ModeSmil
            )
        {
            return Ok(false);
        }
        let actions = self.presentation.pointer(phase, point, &layout, &snapshot);
        self.dispatch_presentation(actions, files)?;
        if phase == Phase::Down {
            self.pointer_capture = true;
        } else if matches!(phase, Phase::Up | Phase::Cancel) {
            self.pointer_capture = false;
        }
        Ok(true)
    }

    pub fn key<B: PanelBackend>(
        &mut self,
        key: Key,
        files: &mut FileUi<B>,
        selected_target: Option<String>,
    ) -> Result<bool, NativeAnimationError> {
        let snapshot = self.snapshot(files.document(), selected_target)?;
        let actions = self.presentation.key(key, &snapshot);
        let handled = !actions.is_empty() || key == Key::Tab;
        self.dispatch_presentation(actions, files)?;
        Ok(handled)
    }

    pub fn paint_frame(
        &self,
        frame: RgbaFrame,
        footer: vexel_ui::Rect,
        snapshot: &NativeTimelineSnapshot,
        dark: bool,
    ) -> RgbaFrame {
        crate::overlay::paint_over(frame, dark, |canvas| {
            self.presentation.paint(canvas, footer, snapshot);
        })
    }

    /// Samples and retains the latest complete native frame for the requested output size.
    pub fn tick(&mut self, document: &AppDocument, size: (u32, u32)) -> Option<Frame> {
        self.observe_revision(document.state_revision());
        let snapshot = self.controller.snapshot(document).ok()?;
        let Some(candidate) = self.controller.candidate.as_ref().cloned() else {
            self.stop_invalid_preview();
            return None;
        };
        let view = match candidate.view() {
            Ok(view) => view,
            Err(_) => {
                self.stop_invalid_preview();
                return None;
            }
        };
        let ticks_per_second = view.ticks_per_second().max(1);
        let prep = crate::profile::phase("native.prep");
        let source = Arc::new(document.source_bytes().ok()?);
        let viewport = svg_viewport(source.clone(), &|| false).ok()?;
        drop(prep);
        let tick = snapshot
            .current_tick
            .min(snapshot.duration_ticks.saturating_sub(1));
        let request = NativeRenderRequest {
            state_revision: document.state_revision(),
            tick,
            output_width: size.0,
            output_height: size.1,
            source_svg: source,
            timeline: candidate,
            viewport,
        };
        let expected_tick = if self.controller.playing {
            self.driver.play();
            // NativeDriver chooses the deterministic integer sample from its clock.
            tick
        } else {
            self.driver.pause();
            if self.last_tick != Some(tick) {
                let seconds = f64::from(tick) / f64::from(ticks_per_second);
                self.driver.seek(seconds);
                self.last_tick = Some(tick);
            }
            tick
        };
        let tag = NativeFrameTag {
            state_revision: request.state_revision,
            tick: expected_tick,
            width: request.output_width,
            height: request.output_height,
        };
        let onion = OnionSettings {
            before: snapshot.onion_before,
            after: snapshot.onion_after,
            opacity: snapshot.onion_opacity,
        };
        self.stage.expect(tag, onion);
        let mut published = None;
        if let Some(result) = self.driver.tick(request) {
            self.stage.expect(result.tag, onion);
            match self.stage.accept(result) {
                StageUpdate::Published => {
                    self.controller.current_tick = result_tick(self.stage.expected(), tag.tick);
                    self.controller.status = None;
                    published = self.stage.frame().cloned();
                }
                StageUpdate::Failed(error) => {
                    self.controller.status = Some(format!("Native frame unavailable: {error:?}"));
                }
                StageUpdate::Stale => {}
            }
        }
        self.controller.playing = self.driver.is_playing();
        published
    }

    fn stop_invalid_preview(&mut self) {
        self.controller.playing = false;
        self.driver.pause();
        self.driver.invalidate();
        self.stage = NativeStage::new();
        self.last_tick = None;
    }

    pub fn invalidate(&mut self) {
        self.driver.invalidate();
        self.stage = NativeStage::new();
        self.last_tick = None;
    }

    fn observe_revision(&mut self, revision: u64) -> bool {
        if self.state_revision != Some(revision) {
            if self.state_revision.is_some() {
                self.driver.invalidate();
                self.stage = NativeStage::new();
                // MCP edits can change the native timeline without changing
                // source bytes. Drop selections and the playhead that refer
                // to the prior state before the next snapshot is built.
                self.controller.document_replaced();
            }
            self.state_revision = Some(revision);
            self.last_tick = None;
            self.reset_presentation_interaction();
            true
        } else {
            false
        }
    }

    fn reset_presentation_interaction(&mut self) {
        let mode = self.presentation.mode;
        self.presentation = TimelinePresentation::default();
        self.presentation.mode = mode;
        self.pointer_capture = false;
    }

    fn dispatch_presentation<B: PanelBackend>(
        &mut self,
        actions: Vec<PresentationAction>,
        files: &mut FileUi<B>,
    ) -> Result<(), NativeAnimationError> {
        for action in actions {
            match action {
                PresentationAction::SelectMode(mode) => {
                    let surface = match mode {
                        TimelineMode::Native => TimelineSurface::Native,
                        TimelineMode::SvgSmil => TimelineSurface::Smil,
                    };
                    self.controller.set_surface(surface);
                    self.driver.pause();
                    self.driver.invalidate();
                    self.stage = NativeStage::new();
                    self.last_tick = None;
                }
                PresentationAction::Native(action) => {
                    if let Err(error) = self.dispatch_action(action, files) {
                        self.controller.status =
                            Some(format!("Native timeline action failed: {error:?}"));
                    }
                }
            }
        }
        Ok(())
    }

    fn dispatch_action<B: PanelBackend>(
        &mut self,
        action: NativeTimelineAction,
        files: &mut FileUi<B>,
    ) -> Result<(), NativeAnimationError> {
        let seek_tick = match &action {
            NativeTimelineAction::Seek { tick } => Some(*tick),
            _ => None,
        };
        let toggle = matches!(action, NativeTimelineAction::TogglePlayback);
        let outcome = self
            .controller
            .handle_action(files.document_mut(), action)?;
        if outcome.changed {
            self.driver.invalidate();
            self.stage = NativeStage::new();
            self.state_revision = Some(files.document().state_revision());
            self.last_tick = None;
        }
        if let Some(tick) = seek_tick {
            let snapshot = self.controller.snapshot(files.document())?;
            let seconds = f64::from(tick.min(snapshot.duration_ticks.saturating_sub(1)))
                / f64::from(snapshot.ticks_per_second.max(1));
            self.driver.seek(seconds);
            self.last_tick = Some(tick.min(snapshot.duration_ticks.saturating_sub(1)));
            self.driver.invalidate();
        }
        if toggle {
            if self.controller.playing {
                self.driver.play();
            } else {
                self.driver.pause();
            }
        }
        if let Some(effect) = outcome.effect {
            match &effect {
                NativeTimelineEffect::ImportSvg { .. } => {
                    let settings = EmptyTimelineSettings {
                        ticks_per_second: 24,
                        duration_ticks: 240,
                        fps_numerator: 24,
                        fps_denominator: 1,
                        loop_mode: vexel_timeline::LoopMode::Repeat,
                        onion_before: 0,
                        onion_after: 0,
                        onion_opacity: 0.25,
                    };
                    let result =
                        super::import::handle(files, &effect, settings, EditLimits::default());
                    if result == super::import::ImportResult::Imported {
                        self.driver.invalidate();
                        self.stage = NativeStage::new();
                        self.state_revision = Some(files.document().state_revision());
                        self.last_tick = None;
                    } else if result == super::import::ImportResult::Failed {
                        self.controller.status = Some("SVG import failed".into());
                    }
                }
            }
        }
        Ok(())
    }
}

fn result_tick(expected: Option<NativeFrameTag>, fallback: u32) -> u32 {
    expected.map_or(fallback, |tag| tag.tick)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::playback::ManualClock;
    use crate::session::AppDocument;
    use vexel_timeline::{EditLimits, EmptyTimelineSettings, LoopMode};

    #[test]
    fn replacing_a_document_resets_surface_and_stops_native_playback() {
        let mut host = NativeAnimationHost::new(ManualClock::default());
        host.controller.playing = true;
        host.document_replaced();
        assert_eq!(host.surface(), TimelineSurface::Native);
        assert!(!host.controller.playing);
        assert!(!host.is_playing());
        assert_eq!(host.state_revision, None);
    }

    #[test]
    fn mcp_edit_refreshes_native_state() {
        let mut document = AppDocument::new(32, 24).unwrap();
        let mut host = NativeAnimationHost::new(ManualClock::default());
        let mut view = crate::compose::DocumentView::new();
        assert!(host.prepare_document(&document, &mut view));
        host.controller.selected_track = Some("old-track".into());
        host.controller.selected_item_tick = Some(12);
        host.controller.current_tick = 12;
        host.controller.playing = true;

        let settings = EmptyTimelineSettings {
            ticks_per_second: 24,
            duration_ticks: 240,
            fps_numerator: 24,
            fps_denominator: 1,
            loop_mode: LoopMode::Once,
            onion_before: 0,
            onion_after: 0,
            onion_opacity: 0.5,
        };
        assert!(
            document
                .edit_timeline(settings, EditLimits::default(), |candidate| {
                    candidate.set_loop(LoopMode::Repeat)
                })
                .unwrap()
        );

        assert!(host.prepare_document(&document, &mut view));
        assert_eq!(host.controller.selected_track, None);
        assert_eq!(host.controller.selected_item_tick, None);
        assert_eq!(host.controller.current_tick, 0);
        assert!(!host.controller.playing);
        assert_eq!(host.state_revision, Some(document.state_revision()));
    }
}
