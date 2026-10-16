//! Exact parser record events, used only when legacy post-close records overlap.
//! No source reparse, geometry rewrite or weakening of ordinary ordered ranges.
use crate::{
    types::{invalid, stopped},
    *,
};
use vexel_svg_path::{PathSegment, PathSubpath};
pub(crate) fn canonical(
    subpaths: &[PathSubpath],
    edges: TableRange,
    geometry: &[Edge],
    source_range: Span,
    segments: impl Iterator<Item = PathSegment>,
    control: &mut dyn WorkControl,
) -> Result<()> {
    let mut segments = segments.peekable();
    let (mut first, mut end, mut open, mut record) = (edges.first, edges.first, false, 0);
    let mut first_command = true;
    let mut source_end = source_range.start;
    let bound = edges.end().ok_or_else(crate::memory::arithmetic)?;
    while let Some(segment) = segments.next() {
        control.charge(WorkKind::TokenVisit, 1).map_err(stopped)?;
        if (first_command && !segment.command.eq_ignore_ascii_case(&b'M'))
            || segment.source.start < source_end
        {
            return Err(invalid(0));
        }
        first_command = false;
        source_end = segment.source.end;
        let next = segment
            .first_edge
            .checked_add(segment.edge_count)
            .ok_or_else(crate::memory::arithmetic)?;
        if segment.first_edge < edges.first
            || next > bound
            || segment.source.start < source_range.start
            || segment.source.end > source_range.end
        {
            return Err(invalid(0));
        }
        match segment.command.to_ascii_uppercase() {
            b'M' => {
                if segment.command_offset.is_none() {
                    return Err(invalid(0));
                }
                if segment.first_edge != end || segment.edge_count != 0 {
                    return Err(invalid(0));
                }
                // M's implicit closure is outside its zero-edge segment metadata.
                let closed_end = segments.peek().map_or(bound, |s| s.first_edge);
                if closed_end < end || closed_end - end > usize::from(open) {
                    return Err(invalid(0));
                }
                if open {
                    expect(subpaths, &mut record, first, closed_end, false)?;
                } else if closed_end != end {
                    return Err(invalid(0));
                }
                first = closed_end;
                end = closed_end;
                open = true;
            }
            b'Z' => {
                if segment.command_offset.is_none() {
                    return Err(invalid(0));
                }
                if segment.first_edge != end || segment.edge_count > usize::from(open) {
                    return Err(invalid(0));
                }
                end = next;
                if open {
                    if first < end
                        && geometry.get(end - 1).map(|e| e.b) != geometry.get(first).map(|e| e.a)
                    {
                        return Err(invalid(0));
                    }
                    expect(subpaths, &mut record, first, end, true)?;
                }
                open = false;
            }
            _ => {
                if segment.first_edge != end {
                    return Err(invalid(0));
                }
                end = next;
                open = true;
            }
        }
    }
    if open {
        if bound < end || bound - end > 1 {
            return Err(invalid(0));
        }
        expect(subpaths, &mut record, first, bound, false)?;
        end = bound;
    }
    if end != bound || record != subpaths.len() {
        return Err(invalid(0));
    }
    Ok(())
}
fn expect(
    paths: &[PathSubpath],
    record: &mut usize,
    first: usize,
    end: usize,
    closed: bool,
) -> Result<()> {
    let count = end
        .checked_sub(first)
        .ok_or_else(crate::memory::arithmetic)?;
    if count == 0 {
        return Ok(());
    }
    let value = paths.get(*record).ok_or_else(|| invalid(0))?;
    if value.first_edge != first || value.edge_count != count || value.closed != closed {
        return Err(invalid(0));
    }
    *record = record
        .checked_add(1)
        .ok_or_else(crate::memory::arithmetic)?;
    Ok(())
}

/// Builder publication groups segments by monotonic shape insertion slot. The
/// immutable table is searched with controlled probes, not rescanned per shape.
pub(crate) fn scene_segments<'a>(
    segments: &'a [ScenePathSegment],
    shape: usize,
    control: &mut dyn WorkControl,
) -> Result<&'a [ScenePathSegment]> {
    fn bound(
        segments: &[ScenePathSegment],
        shape: usize,
        after: bool,
        control: &mut dyn WorkControl,
    ) -> Result<usize> {
        let (mut lo, mut hi) = (0, segments.len());
        while lo < hi {
            control.charge(WorkKind::TokenVisit, 1).map_err(stopped)?;
            let mid = lo + (hi - lo) / 2;
            let index = segments[mid].shape.index();
            if index < shape || (after && index == shape) {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        Ok(lo)
    }
    let first = bound(segments, shape, false, control)?;
    let end = bound(segments, shape, true, control)?;
    segments.get(first..end).ok_or_else(|| invalid(0))
}
