//! Canonical parser provenance kept distinct from the use-instance annotation.
use crate::{ElementId, Span};
use crate::{Result, WorkControl, WorkKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct GeometryProof {
    pub(crate) element: ElementId,
    pub(crate) element_range: Span,
    pub(crate) path_data: Span,
}

/// Immutable canonical SVG path provenance for one appended shape.
/// A token is minted only by `DecodeSource` after checking its owned XML tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShapeGeometrySource {
    pub(crate) shape_slot: u32,
    pub(crate) proof: GeometryProof,
    pub(crate) number_range: crate::TableRange,
}

impl ShapeGeometrySource {
    pub fn shape_slot(self) -> u32 {
        self.shape_slot
    }
    pub fn element(self) -> ElementId {
        self.proof.element
    }
    pub fn element_range(self) -> Span {
        self.proof.element_range
    }
    pub fn path_data(self) -> Span {
        self.proof.path_data
    }
    pub fn path_numbers(self) -> crate::TableRange {
        self.number_range
    }
}

pub(crate) fn validate_path_document(
    path: &vexel_svg_path::PathDocument,
    data: Span,
    control: &mut dyn WorkControl,
) -> Result<()> {
    let contains = |span: Span| span.start >= data.start && span.end <= data.end;
    let mut next_number = 0;
    for segment in &path.segments {
        control
            .charge(WorkKind::SourceRead, 1)
            .map_err(crate::types::stopped)?;
        if !contains(segment.source) {
            return Err(crate::types::invalid(segment.source.start));
        }
        let end = segment
            .first_number
            .checked_add(segment.number_count)
            .ok_or_else(crate::memory::arithmetic)?;
        let numbers = path
            .numbers
            .get(segment.first_number..end)
            .ok_or_else(|| crate::types::invalid(segment.source.start))?;
        if segment.first_number != next_number {
            return Err(crate::types::invalid(segment.source.start));
        }
        next_number = end;
        for number in numbers {
            control
                .charge(WorkKind::SourceRead, 1)
                .map_err(crate::types::stopped)?;
            if !contains(number.source) {
                return Err(crate::types::invalid(number.source.start));
            }
        }
    }
    if next_number != path.numbers.len() {
        return Err(crate::types::invalid(data.start));
    }
    Ok(())
}

pub(crate) fn validate_scene_sources<V: crate::ViewportRead, A: Copy>(
    view: &crate::SceneView<'_, V, A>,
    source: &[u8],
    control: &mut dyn WorkControl,
) -> Result<()> {
    let mut previous = None;
    let mut previous_number_end = 0;
    for geometry in view.geometry_sources {
        control
            .charge(WorkKind::TokenVisit, 1)
            .map_err(crate::types::stopped)?;
        let slot = geometry.shape_slot as usize;
        if view.shape_id(slot).is_none() {
            return Err(crate::types::invalid(geometry.proof.element_range.start));
        }
        if previous.is_some_and(|value| value >= geometry.shape_slot)
            || geometry.proof.element_range.slice(source).is_none()
            || geometry.proof.path_data.slice(source).is_none()
            || geometry.proof.path_data.start < geometry.proof.element_range.start
            || geometry.proof.path_data.end > geometry.proof.element_range.end
        {
            return Err(crate::types::invalid(geometry.proof.path_data.start));
        }
        previous = Some(geometry.shape_slot);
        let number_end = geometry
            .number_range
            .end()
            .ok_or_else(crate::memory::arithmetic)?;
        if geometry.number_range.first < previous_number_end {
            return Err(crate::types::invalid(geometry.proof.path_data.start));
        }
        previous_number_end = number_end;
        let segments = crate::path_topology::scene_segments(view.path_segments, slot, control)?;
        for segment in segments {
            control
                .charge(WorkKind::SourceRead, 1)
                .map_err(crate::types::stopped)?;
            let span = segment.path.source;
            if span.start < geometry.proof.path_data.start
                || span.end > geometry.proof.path_data.end
            {
                return Err(crate::types::invalid(span.start));
            }
            let range = crate::TableRange {
                first: segment.path.first_number,
                count: segment.path.number_count,
            };
            let end = range.end().ok_or_else(crate::memory::arithmetic)?;
            if range.first < geometry.number_range.first || end > number_end {
                return Err(crate::types::invalid(span.start));
            }
            range
                .slice(view.path_numbers)
                .ok_or_else(|| crate::types::invalid(span.start))?;
        }
        let numbers = geometry
            .number_range
            .slice(view.path_numbers)
            .ok_or_else(|| crate::types::invalid(geometry.proof.path_data.start))?;
        for number in numbers {
            control
                .charge(WorkKind::SourceRead, 1)
                .map_err(crate::types::stopped)?;
            if number.source.start < geometry.proof.path_data.start
                || number.source.end > geometry.proof.path_data.end
            {
                return Err(crate::types::invalid(number.source.start));
            }
        }
    }
    Ok(())
}
