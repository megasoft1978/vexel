//! Add-key command: a key at the playhead holding the curve's current value.

use super::NativeAnimationController;
use crate::native_animation::{NativeActionOutcome, NativeAnimationError};
use crate::native_timeline::NativeTimelineAction;
use crate::session::AppDocument;
use vexel_timeline::{PropertyValue, sample_properties};

enum Resolved {
    /// A key with this JSON value is written to the track.
    Key { track_id: String, value: String },
    /// A key already sits on the tick.
    Existing { track_id: String },
    /// The shape has no property track yet.
    NoTrack,
}

impl NativeAnimationController {
    pub(super) fn add_key(
        &mut self,
        document: &mut AppDocument,
        track_id: Option<String>,
        target_id: Option<String>,
        tick: u32,
        outcome: &mut NativeActionOutcome,
    ) -> Result<(), NativeAnimationError> {
        let resolved = self.resolve_key(track_id.as_deref(), target_id.as_deref(), tick)?;
        let (track_id, value) = match resolved {
            Resolved::Key { track_id, value } => (track_id, value),
            Resolved::Existing { track_id } => {
                self.selected_track = Some(track_id);
                self.selected_item_tick = Some(tick);
                self.current_tick = tick;
                self.status = Some("A key already exists at this tick".into());
                return Ok(());
            }
            Resolved::NoTrack => {
                let Some(target_id) = target_id else {
                    return Err(NativeAnimationError::InvalidAction);
                };
                // No track yet: start an opacity track at the SVG default (1).
                let action = NativeTimelineAction::AddOpacityKey {
                    target_id,
                    tick,
                    value: 1.0,
                };
                *outcome = self.dispatch(document, action)?;
                return Ok(());
            }
        };
        outcome.changed = document
            .edit_timeline(self.settings, self.limits, |candidate| {
                candidate.set_property_key(track_id.as_bytes(), tick, value.as_bytes())
            })
            .map_err(NativeAnimationError::from)?;
        if outcome.changed {
            self.selected_track = Some(track_id);
            self.selected_item_tick = Some(tick);
            self.current_tick = tick;
        }
        Ok(())
    }

    fn resolve_key(
        &self,
        track_id: Option<&str>,
        target_id: Option<&str>,
        tick: u32,
    ) -> Result<Resolved, NativeAnimationError> {
        let candidate = self.candidate.as_ref().expect("candidate ensured");
        let view = candidate
            .view()
            .map_err(|_| NativeAnimationError::InvalidAction)?;
        let decode = crate::native_animation::snapshot::decode_timeline_string;
        let mut index = None;
        for i in 0..view.track_count() {
            let Some(track) = view.track_at(i) else {
                continue;
            };
            if track.kind != b"property" {
                continue;
            }
            let id = decode(track.id);
            let matches = match (track_id, target_id) {
                (Some(want), _) => id.as_deref() == Some(want),
                (None, Some(want)) => decode(track.target).as_deref() == Some(want),
                (None, None) => false,
            };
            if matches {
                index = Some((i, id.unwrap_or_default()));
                break;
            }
        }
        let Some((index, id)) = index else {
            // Only a shape without any property track starts a new one; a
            // missing or drawing track id is not a valid request.
            return if track_id.is_none() && target_id.is_some() {
                Ok(Resolved::NoTrack)
            } else {
                Err(NativeAnimationError::InvalidAction)
            };
        };
        let track = view
            .track_at(index)
            .ok_or(NativeAnimationError::InvalidAction)?;
        if (0..track.item_count)
            .any(|item| view.item_at(index, item).is_some_and(|i| i.tick == tick))
        {
            return Ok(Resolved::Existing { track_id: id });
        }
        let samples =
            sample_properties(&view, tick).map_err(|_| NativeAnimationError::InvalidAction)?;
        let sample = samples
            .iter()
            .find(|sample| sample.track_index == index)
            .ok_or(NativeAnimationError::InvalidAction)?;
        let value = match sample.value {
            PropertyValue::Path(blend) => {
                let kind = track
                    .property
                    .and_then(|property| vexel_timeline::pathmorph::PathKind::of_property(property))
                    .ok_or(NativeAnimationError::InvalidAction)?;
                let text = vexel_timeline::resolve_path(view.source(), kind, blend)
                    .map_err(|_| NativeAnimationError::InvalidAction)?;
                format!("\"{}\"", String::from_utf8_lossy(&text))
            }
            ref other => value_json(other),
        };
        Ok(Resolved::Key {
            track_id: id,
            value,
        })
    }
}

/// JSON of a sampled value, in the form the timeline stores it.
fn value_json(value: &PropertyValue) -> String {
    let list = |values: &[f64]| {
        let items: Vec<String> = values.iter().map(f64::to_string).collect();
        format!("[{}]", items.join(","))
    };
    match value {
        PropertyValue::Scalar(v) => v.to_string(),
        PropertyValue::Boolean(v) => v.to_string(),
        PropertyValue::Rgba(v) => list(v),
        PropertyValue::Transform2d(v) => list(v),
        PropertyValue::Path(_) => String::new(),
        PropertyValue::Trs(t) => format!(
            "{{\"tx\":{},\"ty\":{},\"rotation_deg\":{},\"sx\":{},\"sy\":{},\"pivot_x\":{},\"pivot_y\":{},\"skew_x_deg\":{}}}",
            t.tx, t.ty, t.rotation_deg, t.sx, t.sy, t.pivot_x, t.pivot_y, t.skew_x_deg
        ),
    }
}
