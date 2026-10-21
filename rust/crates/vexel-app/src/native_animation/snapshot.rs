//! Immutable native timeline presentation snapshots.

use crate::native_timeline::{
    NativeTimelineItem, NativeTimelineSnapshot, NativeTimelineTrack, NativeTrackKind,
};
use crate::session::AppDocument;

use super::{NativeAnimationController, NativeAnimationError};

pub(super) fn decode_timeline_string(encoded: &[u8]) -> Option<String> {
    let mut decoded = Vec::new();
    decoded.try_reserve_exact(encoded.len()).ok()?;
    decoded.resize(encoded.len(), 0);
    let length = vexel_timeline::string_copy(encoded, &mut decoded)?;
    decoded.truncate(length);
    String::from_utf8(decoded).ok()
}

impl NativeAnimationController {
    pub fn snapshot(
        &mut self,
        document: &AppDocument,
    ) -> Result<NativeTimelineSnapshot, NativeAnimationError> {
        if let Err(error) = self.ensure_candidate(document) {
            self.candidate = None;
            self.candidate_revision = None;
            return Ok(self.disabled_snapshot(format!(
                "Native timeline unavailable: {error:?}; authoring is disabled"
            )));
        }
        let candidate = self.candidate.as_ref().expect("candidate was just loaded");
        let view = match candidate.view() {
            Ok(view) => view,
            Err(error) => {
                return Ok(self.disabled_snapshot(format!(
                    "Native timeline unavailable: {error:?}; authoring is disabled"
                )));
            }
        };

        let (onion_before, onion_after, onion_opacity) = view.onion();
        let duration_ticks = view.duration_ticks().max(1);
        self.current_tick = self.current_tick.min(duration_ticks - 1);
        self.first_tick = self.first_tick.min(duration_ticks - 1);
        self.last_tick = self.last_tick.min(duration_ticks - 1).max(self.first_tick);
        let mut tracks = Vec::with_capacity(view.track_count());
        let mut track_ids = Vec::with_capacity(view.track_count());
        for track_index in 0..view.track_count() {
            let Some(track) = view.track_at(track_index) else {
                continue;
            };
            let id = decode_timeline_string(track.id).expect("validated timeline track ID");
            let target =
                decode_timeline_string(track.target).expect("validated timeline track target");
            let kind_name =
                decode_timeline_string(track.kind).expect("validated timeline track kind");
            let kind = if kind_name == "drawing" {
                NativeTrackKind::Drawing
            } else {
                NativeTrackKind::Property
            };
            let mut items = Vec::with_capacity(track.item_count);
            for item_index in 0..track.item_count {
                if let Some(item) = view.item_at(track_index, item_index) {
                    let label = match item.asset {
                        Some(asset) => {
                            decode_timeline_string(asset).expect("validated timeline asset name")
                        }
                        None => item
                            .value_json
                            .map(|bytes| String::from_utf8_lossy(bytes).into_owned())
                            .unwrap_or_default(),
                    };
                    items.push(NativeTimelineItem {
                        tick: item.tick,
                        label,
                        selected: self.selected_track.as_deref() == Some(id.as_str())
                            && self.selected_item_tick == Some(item.tick),
                        ease: item.ease.map(|ease| match ease {
                            vexel_timeline::sample::Ease::Step => "step".to_owned(),
                            vexel_timeline::sample::Ease::Linear => "linear".to_owned(),
                            vexel_timeline::sample::Ease::EaseIn => "ease_in".to_owned(),
                            vexel_timeline::sample::Ease::EaseOut => "ease_out".to_owned(),
                            vexel_timeline::sample::Ease::EaseInOut => "ease_in_out".to_owned(),
                            vexel_timeline::sample::Ease::Bezier(p) => {
                                crate::native_timeline::bezier::encode(p)
                            }
                            other => format!("{other:?}"),
                        }),
                    });
                }
            }
            track_ids.push(id.clone());
            tracks.push(NativeTimelineTrack {
                id: id.clone(),
                name: track
                    .name
                    .map(|name| {
                        decode_timeline_string(name).expect("validated timeline track name")
                    })
                    .unwrap_or_else(|| id.clone()),
                target,
                property: track
                    .property
                    .and_then(decode_timeline_string)
                    .unwrap_or_default(),
                kind,
                visible: track.visible,
                selected: self.selected_track.as_deref() == Some(id.as_str()),
                items,
            });
        }
        if self
            .selected_track
            .as_ref()
            .is_some_and(|selected| !track_ids.iter().any(|id| id == selected))
        {
            self.selected_track = None;
            self.selected_item_tick = None;
        }
        if let Some(selected_track) = self.selected_track.as_deref() {
            let item_exists = tracks
                .iter()
                .find(|track| track.id == selected_track)
                .is_some_and(|track| {
                    self.selected_item_tick
                        .is_some_and(|tick| track.items.iter().any(|item| item.tick == tick))
                });
            if !item_exists {
                self.selected_item_tick = None;
            }
        } else {
            self.selected_item_tick = None;
        }
        let selected_target = self
            .selected_track
            .as_ref()
            .and_then(|id| tracks.iter().find(|track| &track.id == id))
            .map(|track| track.target.clone())
            .or_else(|| self.selected_target.clone());
        let editable = candidate.is_editable();
        if self
            .status
            .as_deref()
            .is_some_and(|status| status.starts_with("Native timeline unavailable:"))
        {
            self.status = None;
        }
        Ok(NativeTimelineSnapshot {
            ticks_per_second: view.ticks_per_second(),
            duration_ticks,
            current_tick: self.current_tick,
            tracks,
            selected_target,
            playing: self.playing,
            loop_enabled: view.loop_mode() != vexel_timeline::LoopMode::Once,
            onion_before,
            onion_after,
            onion_opacity,
            can_add_drawing_track: editable && self.selected_target.is_some(),
            can_import_asset: editable && self.selected_target.is_some(),
            can_edit_selected_item: editable
                && self.selected_track.is_some()
                && self.selected_item_tick.is_some(),
            status: self.status.clone(),
        })
    }

    fn disabled_snapshot(&mut self, reason: String) -> NativeTimelineSnapshot {
        self.status = Some(reason);
        self.playing = false;
        let duration_ticks = self.settings.duration_ticks.max(1);
        NativeTimelineSnapshot {
            ticks_per_second: self.settings.ticks_per_second.max(1),
            duration_ticks,
            current_tick: self.current_tick.min(duration_ticks - 1),
            tracks: Vec::new(),
            selected_target: self.selected_target.clone(),
            playing: false,
            loop_enabled: false,
            onion_before: self.settings.onion_before,
            onion_after: self.settings.onion_after,
            onion_opacity: self.settings.onion_opacity,
            can_add_drawing_track: false,
            can_import_asset: false,
            can_edit_selected_item: false,
            status: self.status.clone(),
        }
    }
}
