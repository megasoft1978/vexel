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
    append_geometry_internal(
        ctx,
        builder,
        scene,
        map,
        geometry,
        style,
        viewport,
        &mut crate::operations::Operations::Fixture(operations),
        control,
    )
}
/// Production uses the resolver's borrowed stop template directly.
pub fn append_geometry_production(
    ctx: &mut ResolveContext<'_, '_>,
    builder: &mut SceneBuilder,
    scene: SceneId,
    map: SourceMap,
    geometry: Geometry,
    style: &ResolvedStyle,
    viewport: [f64; 2],
    operations: &mut dyn ProductionDecodeOperations,
    control: &mut dyn WorkControl,
) -> Result<ShapeId> {
    append_geometry_internal(
        ctx,
        builder,
        scene,
        map,
        geometry,
        style,
        viewport,
        &mut crate::operations::Operations::Production(operations),
        control,
    )
}
pub(crate) fn append_geometry_internal(
    ctx: &mut ResolveContext<'_, '_>,
    builder: &mut SceneBuilder,
    scene: SceneId,
    map: SourceMap,
    geometry: Geometry,
    style: &ResolvedStyle,
    viewport: [f64; 2],
    operations: &mut crate::operations::Operations<'_>,
    control: &mut dyn WorkControl,
) -> Result<ShapeId> {
    ctx.validate()?;
    let lease = ctx
        .ledger
        .admit(AllocationTag::None, geometry_storage(&geometry)?)?;
    let result = append_admitted_geometry(
        ctx, builder, scene, map, geometry, style, viewport, operations, control,
    );
    ctx.ledger.release(lease)?;
    result
}
fn append_admitted_geometry(
    ctx: &mut ResolveContext<'_, '_>,
    builder: &mut SceneBuilder,
    scene: SceneId,
    map: SourceMap,
    geometry: Geometry,
    style: &ResolvedStyle,
    viewport: [f64; 2],
    operations: &mut crate::operations::Operations<'_>,
    control: &mut dyn WorkControl,
) -> Result<ShapeId> {
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
    let (mut colors, color_lease) = crate::storage::allocate(
        ctx.ledger,
        geometry.path.edges.len(),
        AllocationTag::None,
        control,
    )?;
    let result = (|| {
        for _ in &geometry.path.edges {
            crate::source::charge(control, WorkKind::SourceRead, 1, map.element.start)?;
            colors.push(style.fill);
        }
        for (value, stroke) in [(style.fill_server, false), (style.stroke_server, true)] {
            if let Some(value) = value {
                let inverse = inverse.ok_or(Diagnostic {
                    kind: ErrorKind::Unsupported,
                    offset: value.offset,
                })?;
                let index = operations.publish_paint(
                    ctx,
                    builder,
                    scene,
                    value,
                    &geometry.path.edges,
                    inverse,
                    viewport,
                    if stroke {
                        PaintChannel::Stroke
                    } else {
                        PaintChannel::Fill
                    },
                    control,
                )?;
                if stroke {
                    shape.stroke_paint = Some(index)
                } else {
                    shape.fill_paint = Some(index)
                }
            }
        }
        builder.append_shape(
            scene,
            ShapeInput {
                shape,
                path: geometry.path,
                edge_colors: colors,
            },
            control,
        )
    })();
    ctx.ledger.release(color_lease)?;
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
/// The real production custom path retains the resolver's borrowed cache result.
pub fn append_custom_element_production(
    ctx: &mut ResolveContext<'_, '_>,
    builder: &mut SceneBuilder,
    scene: SceneId,
    map: SourceMap,
    geometry: Geometry,
    paint: &CssPaint<'_>,
    viewport: [f64; 2],
    operations: &mut dyn ProductionDecodeOperations,
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
    append_geometry_production(
        ctx, builder, scene, map, geometry, &style, viewport, operations, control,
    )
    .map(Some)
}
