//! Marker instancing (WO-RUST-CORPUS-UNSUPPORTED-01).
//!
//! The lower planner expands a marker once with its content transform. Here
//! that single expansion is repeated once per placement of the host shape
//! (start, mid and end vertices that reference this marker), each with the
//! placement transform of the legacy C `make_marker`. The decoder then walks
//! one expansion per instance, so no decoder change is needed.
use vexel_base::WorkControl;
use vexel_geometry::{Affine, Point};
use vexel_scene::*;
use vexel_svg_css::CssPaint;
use vexel_svg_definitions::{
    MarkerOrient, MarkerPosition, compose_marker_transform, element_scale, marker_path_placements,
    marker_primitive_placements, marker_reference_value, parse_marker_attributes_for_viewport,
    resolve_local_definition,
};

const POSITIONS: [MarkerPosition; 3] = [
    MarkerPosition::Start,
    MarkerPosition::Middle,
    MarkerPosition::End,
];

fn invalid(offset: usize) -> Diagnostic {
    Diagnostic {
        kind: ErrorKind::InvalidArgument,
        offset,
    }
}

/// Replaces the planned marker expansion by one expansion per placement.
pub(crate) fn instance<'src>(
    ctx: &mut DefinitionContext<'src, '_>,
    element: ElementId,
    owner: DefinitionOwner,
    transform: Affine,
    style: &CssPaint<'src>,
    planned: ProductionDefinitionPlan<'src>,
    control: &mut dyn WorkControl,
) -> Result<ProductionDefinitionPlan<'src>> {
    let ProductionDefinitionPlan::Definition(mut plan) = planned else {
        return Ok(planned);
    };
    let DefinitionTarget::Shape(shape_id) = owner.target() else {
        return Err(invalid(0));
    };
    let mut wanted = [false; 3];
    for (slot, position) in POSITIONS.into_iter().enumerate() {
        let Some(value) = marker_reference_value(style, position) else {
            continue;
        };
        if value.bytes.trim_ascii().eq_ignore_ascii_case(b"none") {
            continue;
        }
        let id = resolve_local_definition(ctx.shared, value.span, DefinitionKind::Marker, control)?;
        wanted[slot] = id == element;
    }
    let offset = ctx
        .shared
        .tokens()
        .get(element.0)
        .map_or(0, |t| t.name.offset);
    let viewport = ctx
        .builder
        .traversal_scene(ctx.scene)?
        .viewport
        .ok_or_else(|| invalid(offset))?;
    let attributes = parse_marker_attributes_for_viewport(
        ctx.shared.source(),
        ctx.shared.tokens(),
        element.0,
        [f64::from(viewport.width), f64::from(viewport.height)],
    )?;
    let host = ctx
        .shared
        .tokens()
        .get(owner.element().0)
        .and_then(|t| t.local_name.slice(ctx.shared.source()))
        .map(<[u8]>::to_vec)
        .unwrap_or_default();
    let view = ctx.builder.traversal_scene(ctx.scene)?;
    let shape = view.shape(shape_id).ok_or_else(|| invalid(offset))?;
    let edges = shape
        .edges
        .slice(view.edges)
        .ok_or_else(|| invalid(offset))?;
    let width = shape.stroke.width;
    let placements = match host.as_slice() {
        b"path" => {
            let all = view.path_segments;
            let first = all.partition_point(|s| s.shape.index() < shape_id.index());
            let end = all.partition_point(|s| s.shape.index() <= shape_id.index());
            let segments: Vec<_> = all[first..end].iter().map(|s| s.path).collect();
            marker_path_placements(view.edges, &segments, attributes, transform, width, control)?
        }
        b"line" | b"polyline" | b"polygon" => marker_primitive_placements(
            edges,
            host.as_slice() == b"polyline",
            attributes,
            transform,
            width,
            control,
        )?,
        _ => Vec::new(),
    };
    // The lower placement math always bisects mid-vertex angles; the legacy C
    // only does so for `orient="auto"` and otherwise uses the fixed angle.
    let fixed = match attributes.orient {
        MarkerOrient::Angle(degrees) => Some(degrees.to_radians()),
        _ => None,
    };
    let scale = if attributes.stroke_width_units {
        let scaled = width * element_scale(transform);
        if scaled.is_finite() && scaled > 0.0 {
            scaled
        } else {
            1.0
        }
    } else {
        1.0
    };
    let reference = Point {
        x: attributes.ref_x,
        y: attributes.ref_y,
    };
    let base = std::mem::take(&mut plan.expansions);
    for placement in placements {
        let slot = POSITIONS.iter().position(|p| *p == placement.position);
        if !slot.is_some_and(|slot| wanted[slot]) {
            continue;
        }
        let mut placed = placement.transform;
        if let (MarkerPosition::Middle, Some(angle)) = (placement.position, fixed) {
            placed = compose_marker_transform(
                placement.point,
                angle,
                scale,
                attributes.content,
                reference,
            )
            .ok_or_else(|| invalid(offset))?;
        }
        plan.expansions
            .try_reserve(base.len())
            .map_err(|_| Diagnostic {
                kind: ErrorKind::OutOfMemory,
                offset,
            })?;
        for expansion in &base {
            plan.expansions.push(Expansion {
                range: expansion.range,
                transform: placed,
                viewport: expansion.viewport,
                style: expansion.style,
                flavor: expansion.flavor,
                post: expansion.post,
            });
        }
    }
    Ok(ProductionDefinitionPlan::Definition(plan))
}
