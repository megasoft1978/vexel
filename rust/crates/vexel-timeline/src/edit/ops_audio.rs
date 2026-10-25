//! Replaces the optional root `audio` array in one undoable commit.
use super::{fail, EditFailure, EditResult, TimelineEdit};
use crate::audio::{self, AudioTrack};
use crate::json::{object_fields, Reader};
use crate::schema::field;
use crate::{decode, Error, TimelineContext};
use vexel_base::Span;

/// Sets (or inserts) the root `audio` array. Validation happens in the
/// commit-time decode, so a bad track set changes nothing.
pub(super) fn set_audio_tracks(
    edit: &mut TimelineEdit,
    context: &TimelineContext,
    tracks: &[AudioTrack],
) -> EditResult {
    let source = edit.source();
    let view = decode(source, context).map_err(EditFailure::from)?;
    if view.is_empty() {
        return Err(fail(Error::InvalidSchema, 0));
    }
    audio::validate_tracks(tracks, view.duration_ticks()).map_err(EditFailure::from)?;
    let json = audio::to_json(tracks);
    let mut reader = Reader::new(source, None);
    reader.scan(0).map_err(EditFailure::from)?;
    let root_end = reader.cursor;
    let root = object_fields(source, Span { start: 0, end: root_end }, None).map_err(EditFailure::from)?;
    let mut out = Vec::new();
    out.try_reserve_exact(source.len().saturating_add(json.len()).saturating_add(16))
        .map_err(|_| fail(Error::Limit, 0))?;
    match field(&root, b"audio") {
        Some(existing) => {
            out.extend_from_slice(&source[..existing.value.start]);
            out.extend_from_slice(&json);
            out.extend_from_slice(&source[existing.value.end..]);
        }
        None => {
            if tracks.is_empty() {
                return Ok(());
            }
            // Insert before the root's closing brace.
            let close = source[..root_end].iter().rposition(|b| *b == b'}').ok_or_else(|| fail(Error::MalformedJson, 0))?;
            out.extend_from_slice(&source[..close]);
            out.extend_from_slice(b",\"audio\":");
            out.extend_from_slice(&json);
            out.extend_from_slice(&source[close..]);
        }
    }
    edit.commit(context, out)
}
