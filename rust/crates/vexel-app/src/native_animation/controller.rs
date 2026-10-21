//! Stateful coordinator for native timeline intents.

use crate::native_timeline::NativeTimelineAction;
use crate::session::AppDocument;
use vexel_timeline::{EditLimits, EmptyTimelineSettings, LoopMode};

use super::{NativeActionOutcome, NativeAnimationError, NativeTimelineEffect, TimelineSurface};
use crate::project_session::{ProjectTimelineCandidate, ProjectTimelineError};

const DEFAULT_TICKS_PER_SECOND: u32 = 24;
const DEFAULT_DURATION_TICKS: u32 = 48;

pub struct NativeAnimationController {
    pub(super) candidate_revision: Option<u64>,
    pub(super) candidate: Option<ProjectTimelineCandidate>,
    pub(super) selected_track: Option<String>,
    pub(super) selected_item_tick: Option<u32>,
    pub(super) selected_target: Option<String>,
    pub(super) current_tick: u32,
    pub(super) first_tick: u32,
    pub(super) last_tick: u32,
    pub(super) playing: bool,
    pub(super) surface: TimelineSurface,
    pub(super) status: Option<String>,
    pub(super) settings: EmptyTimelineSettings,
    limits: EditLimits,
}

impl Default for NativeAnimationController {
    fn default() -> Self {
        Self::new(
            EmptyTimelineSettings {
                ticks_per_second: DEFAULT_TICKS_PER_SECOND,
                duration_ticks: DEFAULT_DURATION_TICKS,
                fps_numerator: DEFAULT_TICKS_PER_SECOND,
                fps_denominator: 1,
                loop_mode: LoopMode::Repeat,
                onion_before: 0,
                onion_after: 0,
                onion_opacity: 0.25,
            },
            EditLimits::default(),
        )
    }
}

impl NativeAnimationController {
    pub fn new(settings: EmptyTimelineSettings, limits: EditLimits) -> Self {
        Self {
            candidate_revision: None,
            candidate: None,
            selected_track: None,
            selected_item_tick: None,
            selected_target: None,
            current_tick: 0,
            first_tick: 0,
            last_tick: settings.duration_ticks.saturating_sub(1),
            playing: false,
            surface: TimelineSurface::Smil,
            status: None,
            settings,
            limits,
        }
    }

    pub fn surface(&self) -> TimelineSurface {
        self.surface
    }

    pub fn set_surface(&mut self, surface: TimelineSurface) {
        self.surface = surface;
        if surface != TimelineSurface::Native {
            self.playing = false;
        }
    }

    /// True when the document has a native timeline with at least one track,
    /// the same test native export uses to pick the native path.
    pub fn has_native_timeline(&self, document: &AppDocument) -> bool {
        let Ok(candidate) = document.native_timeline_candidate(self.settings, self.limits) else {
            return false;
        };
        let Ok(view) = candidate.view() else {
            return false;
        };
        view.track_count() > 0
    }

    pub fn set_selected_target(&mut self, target: Option<String>) {
        self.selected_target = target;
    }

    /// Clears state tied to the replaced document before the next snapshot.
    pub fn document_replaced(&mut self) {
        self.candidate = None;
        self.candidate_revision = None;
        self.selected_track = None;
        self.selected_item_tick = None;
        self.selected_target = None;
        self.current_tick = 0;
        self.first_tick = 0;
        self.last_tick = self.settings.duration_ticks.saturating_sub(1);
        self.playing = false;
        self.status = None;
    }

    pub(super) fn ensure_candidate(
        &mut self,
        document: &AppDocument,
    ) -> Result<(), NativeAnimationError> {
        let revision = document.state_revision();
        if self.candidate_revision == Some(revision) && self.candidate.is_some() {
            return Ok(());
        }
        self.candidate = Some(
            document
                .native_timeline_candidate(self.settings, self.limits)
                .map_err(NativeAnimationError::from)?,
        );
        self.candidate_revision = Some(revision);
        Ok(())
    }

    pub fn dispatch(
        &mut self,
        document: &mut AppDocument,
        action: NativeTimelineAction,
    ) -> Result<NativeActionOutcome, NativeAnimationError> {
        self.ensure_candidate(document)?;
        self.status = None;
        let mut outcome = NativeActionOutcome::default();
        match action {
            NativeTimelineAction::SelectTrack { track_id } => {
                self.selected_track = Some(track_id);
                self.selected_item_tick = None;
            }
            NativeTimelineAction::SelectItem { track_id, tick } => {
                self.selected_track = Some(track_id);
                self.selected_item_tick = Some(tick);
            }
            NativeTimelineAction::Seek { tick } => self.current_tick = tick,
            NativeTimelineAction::SetViewport {
                first_tick,
                last_tick,
            } => {
                self.first_tick = first_tick.min(last_tick);
                self.last_tick = first_tick.max(last_tick);
            }
            NativeTimelineAction::TogglePlayback => {
                if self.surface == TimelineSurface::Native {
                    self.playing = !self.playing;
                } else {
                    self.status = Some("Select Native timeline to start playback".into());
                }
            }
            NativeTimelineAction::RequestAddDrawingTrack { target_id } => {
                outcome.effect = Some(NativeTimelineEffect::ImportSvg {
                    target_id,
                    tick: self.current_tick,
                });
            }
            NativeTimelineAction::RequestImportSvg { target_id, tick } => {
                outcome.effect = Some(NativeTimelineEffect::ImportSvg { target_id, tick });
            }
            NativeTimelineAction::SetTrackVisibility { track_id, visible } => {
                outcome.changed = document
                    .edit_timeline(self.settings, self.limits, |candidate| {
                        candidate.set_track_visibility(track_id.as_bytes(), visible)
                    })
                    .map_err(NativeAnimationError::from)?;
            }
            NativeTimelineAction::SetCel {
                track_id,
                tick,
                asset_name,
            } => {
                outcome.changed = document
                    .edit_timeline(self.settings, self.limits, |candidate| {
                        candidate.set_cel(
                            track_id.as_bytes(),
                            tick,
                            asset_name.as_bytes(),
                            asset_name.as_bytes(),
                        )
                    })
                    .map_err(NativeAnimationError::from)?;
            }
            NativeTimelineAction::RemoveCel { track_id, tick } => {
                outcome.changed = document
                    .edit_timeline(self.settings, self.limits, |candidate| {
                        candidate.remove_cel(track_id.as_bytes(), tick)
                    })
                    .map_err(NativeAnimationError::from)?;
            }
            NativeTimelineAction::MoveCel {
                track_id,
                from_tick,
                to_tick,
            } => {
                outcome.changed = document
                    .edit_timeline(self.settings, self.limits, |candidate| {
                        candidate.move_item(track_id.as_bytes(), from_tick, to_tick)
                    })
                    .map_err(NativeAnimationError::from)?;
                if outcome.changed {
                    self.selected_item_tick = Some(to_tick);
                    self.current_tick = to_tick;
                }
            }
            NativeTimelineAction::MovePropertyKey {
                track_id,
                from_tick,
                to_tick,
            } => {
                if from_tick != 0 && to_tick != 0 {
                    outcome.changed = document
                        .edit_timeline(self.settings, self.limits, |candidate| {
                            candidate.move_property_key(track_id.as_bytes(), from_tick, to_tick)
                        })
                        .map_err(NativeAnimationError::from)?;
                    if outcome.changed {
                        self.selected_track = Some(track_id);
                        self.selected_item_tick = Some(to_tick);
                        self.current_tick = to_tick;
                        self.candidate = None;
                        self.candidate_revision = None;
                    }
                }
            }
            NativeTimelineAction::RemovePropertyKey { track_id, tick } => {
                if tick != 0 {
                    outcome.changed = document
                        .edit_timeline(self.settings, self.limits, |candidate| {
                            candidate.remove_property_key(track_id.as_bytes(), tick)
                        })
                        .map_err(NativeAnimationError::from)?;
                    if outcome.changed {
                        self.selected_item_tick = None;
                        self.candidate = None;
                        self.candidate_revision = None;
                    }
                }
            }
            NativeTimelineAction::SetPropertyKeyEase {
                track_id,
                tick,
                value,
                ease,
            } => {
                if tick != 0 {
                    let ease_json = ease_json(&ease);
                    outcome.changed = document
                        .edit_timeline(self.settings, self.limits, |candidate| {
                            candidate.set_property_key_with_ease(
                                track_id.as_bytes(),
                                tick,
                                value.as_bytes(),
                                Some(ease_json.as_bytes()),
                            )
                        })
                        .map_err(NativeAnimationError::from)?;
                    if outcome.changed {
                        self.selected_track = Some(track_id.clone());
                        self.selected_item_tick = Some(tick);
                        self.candidate = None;
                        self.candidate_revision = None;
                    }
                }
            }
            NativeTimelineAction::DuplicateTrack { track_id } => {
                let candidate = self.candidate.as_ref().expect("candidate ensured");
                let new_id = available_duplicate_track_id(candidate, &track_id)
                    .map_err(|_| NativeAnimationError::InvalidAction)?;
                outcome.changed = document
                    .edit_timeline(self.settings, self.limits, |candidate| {
                        candidate.duplicate_track(track_id.as_bytes(), new_id.as_bytes())
                    })
                    .map_err(NativeAnimationError::from)?;
                if outcome.changed {
                    self.selected_track = Some(new_id);
                    self.selected_item_tick = None;
                    self.candidate = None;
                    self.candidate_revision = None;
                }
            }
            NativeTimelineAction::RemoveTrack { track_id } => {
                outcome.changed = document
                    .edit_timeline(self.settings, self.limits, |candidate| {
                        candidate.remove_track(track_id.as_bytes())
                    })
                    .map_err(NativeAnimationError::from)?;
                if outcome.changed {
                    self.selected_track = None;
                    self.selected_item_tick = None;
                    self.candidate = None;
                    self.candidate_revision = None;
                }
            }
            NativeTimelineAction::SetLoop { enabled } => {
                outcome.changed = document
                    .edit_timeline(self.settings, self.limits, |candidate| {
                        candidate.set_loop(if enabled {
                            LoopMode::Repeat
                        } else {
                            LoopMode::Once
                        })
                    })
                    .map_err(NativeAnimationError::from)?;
            }
            NativeTimelineAction::SetOnion {
                before,
                after,
                opacity,
            } => {
                outcome.changed = document
                    .edit_timeline(self.settings, self.limits, |candidate| {
                        candidate.set_onion(before, after, opacity)
                    })
                    .map_err(NativeAnimationError::from)?;
            }
            NativeTimelineAction::AddOpacityKey {
                target_id,
                tick,
                value,
            } => {
                if !value.is_finite() || !(0.0..=1.0).contains(&value) {
                    self.status = Some("Opacity must be between 0 and 1".into());
                    return Err(NativeAnimationError::InvalidAction);
                }
                let value_json = value.to_string();
                let initial_json = if tick == 0 {
                    value_json.clone()
                } else {
                    "1.0".to_owned()
                };
                let mut created_track_id = None;
                outcome.changed = document
                    .edit_timeline::<ProjectTimelineError>(
                        self.settings,
                        self.limits,
                        |candidate| -> Result<ProjectTimelineCandidate, ProjectTimelineError> {
                            let track_id = available_opacity_track_id(candidate)?;
                            let with_track = candidate.add_property_track(
                                track_id.as_bytes(),
                                target_id.as_bytes(),
                                b"opacity",
                                b"scalar",
                                b"linear",
                                initial_json.as_bytes(),
                                b"Opacity",
                            )?;
                            let edited = if tick == 0 {
                                with_track
                            } else {
                                with_track.set_property_key(
                                    track_id.as_bytes(),
                                    tick,
                                    value_json.as_bytes(),
                                )?
                            };
                            created_track_id = Some(track_id);
                            Ok(edited)
                        },
                    )
                    .map_err(NativeAnimationError::from)?;
                if outcome.changed {
                    self.selected_track = created_track_id;
                    self.selected_item_tick = Some(tick);
                    self.selected_target = Some(target_id);
                    self.current_tick = tick;
                }
            }
        }
        if outcome.changed {
            self.candidate = None;
            self.candidate_revision = None;
        }
        if outcome.effect.is_some() {
            self.selected_target = outcome.effect.as_ref().map(|effect| match effect {
                NativeTimelineEffect::ImportSvg { target_id, .. } => target_id.clone(),
            });
        }
        Ok(outcome)
    }
}

fn available_opacity_track_id(
    candidate: &ProjectTimelineCandidate,
) -> Result<String, ProjectTimelineError> {
    let view = candidate.view()?;
    for suffix in 0..=vexel_timeline::MAX_TRACKS {
        let id = format!("opacity_{suffix}");
        let exists = (0..view.track_count()).any(|index| {
            view.track_at(index)
                .and_then(|track| super::snapshot::decode_timeline_string(track.id))
                .is_some_and(|track_id| track_id == id)
        });
        if !exists {
            return Ok(id);
        }
    }
    Err(ProjectTimelineError::Limit)
}

fn ease_json(ease: &str) -> String {
    match ease {
        "step" => "\"step\"".into(),
        "ease_in" => "\"ease_in\"".into(),
        "ease_out" => "\"ease_out\"".into(),
        "ease_in_out" => "\"ease_in_out\"".into(),
        _ => "\"linear\"".into(),
    }
}

fn available_duplicate_track_id(
    candidate: &ProjectTimelineCandidate,
    original: &str,
) -> Result<String, ProjectTimelineError> {
    let view = candidate.view()?;
    for suffix in 1..=vexel_timeline::MAX_TRACKS {
        let id = format!("{original}_copy{suffix}");
        let exists = (0..view.track_count()).any(|index| {
            view.track_at(index)
                .and_then(|track| super::snapshot::decode_timeline_string(track.id))
                .is_some_and(|existing| existing == id)
        });
        if !exists {
            return Ok(id);
        }
    }
    Err(ProjectTimelineError::Limit)
}
