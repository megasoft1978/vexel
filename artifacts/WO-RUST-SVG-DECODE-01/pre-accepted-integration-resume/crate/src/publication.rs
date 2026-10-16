//! Common real primitive/custom geometry publication and typed paint handoff.
use crate::{Geometry, ResolvedStyle};
use vexel_geometry::affine_identity;
use vexel_scene::*;

pub fn append_geometry(
    ctx: &mut ResolveContext<'_, '_>,
    builder: &mut SceneBuilder,
    scene: SceneId,
    map: SourceMap,
    geometry: Geometry,
    style: &ResolvedStyle,
    viewport: [f64; 2],
    operations: &mut dyn DecodeOperations,
    control: &mut dyn WorkControl,
) -> Result<ShapeId> {
    ctx.validate()?;
    control
        .checkpoint()
        .map_err(|s| crate::source::stopped(s, map.element.start))?;
    if !std::sync::Arc::ptr_eq(ctx.shared.source_owner(), builder.source_owner())
        || map.element.slice(ctx.shared.source()).is_none()
        || !vexel_geometry::affine_valid(map.transform)
    {
        return Err(Diagnostic {
            kind: ErrorKind::InvalidArgument,
            offset: map.element.start,
        });
    }
    let inverse = crate::affine_inverse(map.transform);
    let mut shape = Shape::new(map);
    shape.filled = geometry.filled;
    shape.stroke_closed = geometry.stroke_closed;
    shape.evenodd = style.evenodd;
    shape.stroke = style.stroke;
    shape.stroke_inverse = inverse.unwrap_or_else(affine_identity);
    if shape.stroke.rgba[3] != 0 && inverse.is_none() {
        return Err(Diagnostic {
            kind: ErrorKind::Unsupported,
            offset: style.stroke_server.map_or(map.element.start, |v| v.offset),
        });
    }
    if style.dash_count > 16 {
        return Err(Diagnostic {
            kind: ErrorKind::InvalidArgument,
            offset: map.element.start,
        });
    }
    if style.dash_count != 0 {
        shape.stroke.dash_values =
            builder.append_dashes(scene, &style.dash_values[..style.dash_count], control)?;
    }
    let mut colors = Vec::new();
    colors
        .try_reserve_exact(geometry.path.edges.len())
        .map_err(|_| Diagnostic {
            kind: ErrorKind::OutOfMemory,
            offset: map.element.start,
        })?;
    colors.resize(geometry.path.edges.len(), style.fill);
    for (value, stroke) in [(style.fill_server, false), (style.stroke_server, true)] {
        if let Some(value) = value {
            let inverse = inverse.ok_or(Diagnostic {
                kind: ErrorKind::Unsupported,
                offset: value.offset,
            })?;
            let resolved = operations.resolve_paint(
                ctx,
                value,
                &geometry.path.edges,
                inverse,
                viewport,
                control,
            )?;
            ctx.validate_cache(control)?;
            let index = if stroke {
                builder.append_resolved_stroke(scene, &resolved, control)?
            } else {
                builder.append_resolved_fill(scene, &resolved, control)?
            };
            if stroke {
                shape.stroke_paint = Some(index)
            } else {
                shape.fill_paint = Some(index)
            }
        }
    }
    let pending = geometry_storage(&geometry)?
        .checked_add(colors.capacity().checked_mul(4).ok_or(Diagnostic {
            kind: ErrorKind::Limit(LimitKind::Arithmetic),
            offset: map.element.start,
        })?)
        .ok_or(Diagnostic {
            kind: ErrorKind::Limit(LimitKind::Arithmetic),
            offset: map.element.start,
        })?;
    let lease = ctx.ledger.admit(AllocationTag::None, pending)?;
    let result = builder.append_shape(
        scene,
        ShapeInput {
            shape,
            path: geometry.path,
            edge_colors: colors,
        },
        control,
    );
    ctx.ledger.release(lease)?;
    result
}
pub fn geometry_storage(geometry: &Geometry) -> Result<usize> {
    let p = &geometry.path;
    let mut n = 0usize;
    for (count, size) in [
        (p.edges.capacity(), std::mem::size_of::<Edge>()),
        (
            p.subpaths.capacity(),
            std::mem::size_of::<vexel_svg_path::PathSubpath>(),
        ),
        (
            p.segments.capacity(),
            std::mem::size_of::<vexel_svg_path::PathSegment>(),
        ),
        (
            p.numbers.capacity(),
            std::mem::size_of::<vexel_svg_path::PathNumber>(),
        ),
    ] {
        n = n
            .checked_add(count.checked_mul(size).ok_or(Diagnostic {
                kind: ErrorKind::Limit(LimitKind::Arithmetic),
                offset: 0,
            })?)
            .ok_or(Diagnostic {
                kind: ErrorKind::Limit(LimitKind::Arithmetic),
                offset: 0,
            })?;
    }
    Ok(n)
}
/// An actual downstream glyph/image geometry producer supplies owned geometry;
/// it never receives a mutable escape into the decoder's context or controls.
pub fn append_custom_element(
    ctx: &mut ResolveContext<'_, '_>,
    builder: &mut SceneBuilder,
    scene: SceneId,
    map: SourceMap,
    geometry: Geometry,
    paint: &CssPaint<'_>,
    viewport: [f64; 2],
    operations: &mut dyn DecodeOperations,
    control: &mut dyn WorkControl,
) -> Result<Option<ShapeId>> {
    let root = builder.scene(scene)?.viewport;
    let style = crate::parse_style(
        ctx.shared.source(),
        paint,
        viewport[0].hypot(viewport[1]) / 2.0f64.sqrt(),
        map.transform,
        root,
        control,
    )?;
    if !style.visible {
        return Ok(None);
    }
    append_geometry(
        ctx, builder, scene, map, geometry, &style, viewport, operations, control,
    )
    .map(Some)
}
