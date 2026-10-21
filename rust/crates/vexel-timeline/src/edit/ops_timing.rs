//! Project timing edits: duration and ticks per second.
use super::{fail, EditFailure, EditResult, TimelineEdit};
use crate::json::{object_fields, Field};
use crate::schema::field;
use crate::{decode, Error, TimelineContext};
use vexel_base::Span;

fn fields(source: &[u8], span: Span) -> EditResult<Vec<Field>> {
    object_fields(source, span, None).map_err(EditFailure::from)
}

/// Sets the timeline duration and optionally ticks per second. Keys and cels
/// at or after the new duration are rejected by the commit-time decode, so a
/// shrinking edit never silently drops items. A changed ticks per second also
/// sets the frame rate to the same value (one frame per tick).
pub(super) fn set_timing(
    edit: &mut TimelineEdit,
    context: &TimelineContext,
    duration_ticks: u32,
    ticks_per_second: Option<u32>,
) -> EditResult {
    let tps = ticks_per_second.unwrap_or(0);
    if duration_ticks == 0
        || duration_ticks > 14_400_000
        || (ticks_per_second.is_some() && (tps == 0 || tps > 240))
    {
        return Err(fail(Error::InvalidValue, 0));
    }
    let source = edit.source();
    let view = decode(source, context).map_err(EditFailure::from)?;
    let new_tps = ticks_per_second.unwrap_or_else(|| view.ticks_per_second());
    if view.duration_ticks() == duration_ticks && new_tps == view.ticks_per_second() {
        return Ok(());
    }
    let mut reader = crate::json::Reader::new(source, None);
    reader.scan(0).map_err(EditFailure::from)?;
    let root = fields(
        source,
        Span {
            start: 0,
            end: reader.cursor,
        },
    )?;
    let missing = || fail(Error::InvalidSchema, 0);
    let mut patches: Vec<(Span, Vec<u8>)> = Vec::new();
    patches.push((
        field(&root, b"duration_ticks").ok_or_else(missing)?.value,
        duration_ticks.to_string().into_bytes(),
    ));
    if new_tps != view.ticks_per_second() {
        patches.push((
            field(&root, b"ticks_per_second").ok_or_else(missing)?.value,
            new_tps.to_string().into_bytes(),
        ));
        patches.push((
            field(&root, b"fps").ok_or_else(missing)?.value,
            format!("{{\"numerator\":{new_tps},\"denominator\":1}}").into_bytes(),
        ));
    }
    patches.sort_by_key(|p| p.0.start);
    let mut out = Vec::new();
    out.try_reserve_exact(source.len().saturating_add(64))
        .map_err(|_| fail(Error::Limit, 0))?;
    let mut cursor = 0;
    for (span, value) in patches {
        out.extend_from_slice(&source[cursor..span.start]);
        out.extend_from_slice(&value);
        cursor = span.end;
    }
    out.extend_from_slice(&source[cursor..]);
    edit.commit(context, out)
}

#[cfg(test)]
mod tests {
    use super::super::{initialize_empty_timeline, EmptyTimelineSettings};
    use crate::{decode, EditLimits, Error, LoopMode, TimelineContext, TimelineEdit};

    fn editor() -> (TimelineEdit, TimelineContext) {
        let context = TimelineContext {
            scene_ids: vec![b"a".to_vec()],
            assets: Vec::new(),
        };
        let bytes = initialize_empty_timeline(
            EmptyTimelineSettings {
                ticks_per_second: 24,
                duration_ticks: 48,
                fps_numerator: 24,
                fps_denominator: 1,
                loop_mode: LoopMode::Repeat,
                onion_before: 0,
                onion_after: 0,
                onion_opacity: 0.25,
            },
            &context,
        )
        .unwrap();
        (
            TimelineEdit::new(&bytes, &context, EditLimits::default()).unwrap(),
            context,
        )
    }

    #[test]
    fn extends_and_shrinks_duration_and_undoes() {
        let (mut edit, context) = editor();
        edit.set_timing(&context, 240, None).unwrap();
        let view = decode(edit.source(), &context).unwrap();
        assert_eq!((view.duration_ticks(), view.ticks_per_second()), (240, 24));
        edit.set_timing(&context, 10, Some(12)).unwrap();
        let view = decode(edit.source(), &context).unwrap();
        assert_eq!((view.duration_ticks(), view.ticks_per_second()), (10, 12));
        assert_eq!(view.fps(), (12, 1));
        edit.undo(&context).unwrap();
        let view = decode(edit.source(), &context).unwrap();
        assert_eq!(view.duration_ticks(), 240);
    }

    #[test]
    fn rejects_zero_and_oversized_values() {
        let (mut edit, context) = editor();
        for (d, t) in [(0, None), (14_400_001, None), (10, Some(0)), (10, Some(241))] {
            assert_eq!(
                edit.set_timing(&context, d, t).unwrap_err().error,
                Error::InvalidValue
            );
        }
    }
}
