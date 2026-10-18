use crate::cache::{self, invalid, limit, unsupported};
use crate::control::PaintControl;
use crate::gradient_data;
use crate::handoff::{CachedPaint, CachedResolveContext};
use std::sync::Arc;
use vexel_base::{WorkControl, WorkKind};
use vexel_geometry::{Affine, Edge, affine_identity, affine_map_point};
use vexel_scene::{
    ChargeLedger, DecodeSource, ElementId, GradientTemplateId, Paint, PaintCache, PaintKind,
    ResolveContext, ResolvedPaint, Result, Spread, TableRange,
};
use vexel_xml::{TokenKind, XmlSpan, XmlToken};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaintReference {
    Gradient(ElementId),
    Pattern(ElementId),
}

/// Classify a local paint-server URL for the gradient resolver or definition
/// planner. Patterns are returned explicitly and are never treated as solids.
pub fn classify_fill(
    ctx: &mut ResolveContext<'_, '_>,
    value: XmlSpan,
    control: &mut dyn WorkControl,
) -> Result<PaintReference> {
    ctx.validate().map_err(|error| remap(error, value.offset))?;
    let ResolveContext {
        shared,
        cache: _,
        ledger,
    } = ctx;
    let mut control = PaintControl::new(ledger, control);
    let result = classify_inner(shared, value, &mut control);
    control.result_at(result, value.offset)
}

/// Resolve one local linear/radial paint template, then rebind instance inverse
/// and edge bounds. The returned template witness lets the scene builder share
/// graph stop storage across fills and strokes in that scene.
pub fn resolve_fill(
    ctx: &mut ResolveContext<'_, '_>,
    value: XmlSpan,
    edges: &[Edge],
    user_inverse: Affine,
    viewport: [f64; 2],
    work: &mut dyn WorkControl,
) -> Result<ResolvedPaint> {
    let cached = resolve_cached_fill(
        ctx.shared,
        ctx.cache,
        ctx.ledger,
        value,
        edges,
        user_inverse,
        viewport,
        work,
    )?;
    // This compatibility interface is explicitly owned, not a cache handoff.
    Ok(cached.to_owned(ctx.ledger, work)?.into_job_owned())
}

/// Resolve a template and borrow its actual cache storage. Only `cache` is
/// borrowed by the result; source/context/ledger/control borrows end on return.
/// Publish through CachedPaint::append_fill/append_stroke without stop copies.
pub fn resolve_cached_fill<'cache>(
    shared: &mut DecodeSource<'_>,
    cache: &'cache mut PaintCache,
    ledger: &mut ChargeLedger,
    value: XmlSpan,
    edges: &[Edge],
    user_inverse: Affine,
    viewport: [f64; 2],
    work: &mut dyn WorkControl,
) -> Result<CachedPaint<'cache>> {
    if !Arc::ptr_eq(shared.source_owner(), ledger.source_owner())
        || cache
            .entries
            .first()
            .is_some_and(|entry| !Arc::ptr_eq(&entry.source, shared.source_owner()))
    {
        return Err(invalid(value.offset));
    }
    let result = {
        let mut control = PaintControl::new(ledger, work);
        let result = resolve_inner(
            shared,
            cache,
            value,
            edges,
            user_inverse,
            viewport,
            &mut control,
        );
        control.result_at(result, value.offset)?
    };
    let entry = cache
        .entries
        .get(result.index)
        .ok_or_else(|| invalid(value.offset))?;
    Ok(CachedPaint::new(
        result.paint,
        result.template,
        &entry.stops,
        &entry.source,
        value.offset,
    ))
}

/// Consume a temporary borrowed-flow context while preserving only its cache
/// borrow in the result. Shared source, ledger and control remain reusable.
pub fn resolve_cached_context<'src, 'cache>(
    ctx: CachedResolveContext<'src, 'cache, '_>,
    value: XmlSpan,
    edges: &[Edge],
    user_inverse: Affine,
    viewport: [f64; 2],
    work: &mut dyn WorkControl,
) -> Result<CachedPaint<'cache>> {
    resolve_cached_fill(
        ctx.shared,
        ctx.cache,
        ctx.ledger,
        value,
        edges,
        user_inverse,
        viewport,
        work,
    )
}

struct TemplateResolution {
    index: usize,
    paint: Paint,
    template: GradientTemplateId,
}

fn classify_inner(
    shared: &mut DecodeSource<'_>,
    value: XmlSpan,
    control: &mut PaintControl<'_>,
) -> Result<PaintReference> {
    let source = shared.source();
    // The original fill URL parser classifies malformed source spans as an
    // unsupported paint value. Keep the caller's fill offset for this error.
    let raw = value
        .slice(source)
        .ok_or_else(|| unsupported(value.offset))?;
    if raw.len() < 7 || !raw.starts_with(b"url(#") || raw.last() != Some(&b')') {
        return Err(unsupported(value.offset));
    }
    let id = XmlSpan {
        offset: value.offset + 5,
        length: value.length - 6,
    };
    if id.length == 0 {
        return Err(unsupported(value.offset));
    }
    let target = shared
        .find_reference(id, control)
        .map_err(|error| remap(error, value.offset))?
        .ok_or_else(|| unsupported(value.offset))?;
    let token = shared
        .tokens()
        .get(target.0)
        .ok_or_else(|| invalid(value.offset))?;
    if token.prefix.length != 0 {
        return Err(unsupported(value.offset));
    }
    if eq(source, token.local_name, b"linearGradient")
        || eq(source, token.local_name, b"radialGradient")
    {
        Ok(PaintReference::Gradient(target))
    } else if eq(source, token.local_name, b"pattern") {
        Ok(PaintReference::Pattern(target))
    } else {
        Err(unsupported(value.offset))
    }
}

fn resolve_inner(
    shared: &mut DecodeSource<'_>,
    cache: &mut vexel_scene::PaintCache,
    value: XmlSpan,
    edges: &[Edge],
    user_inverse: Affine,
    viewport: [f64; 2],
    control: &mut PaintControl<'_>,
) -> Result<TemplateResolution> {
    control
        .checkpoint()
        .map_err(|_| limit(vexel_scene::LimitKind::Work, value.offset))?;
    let reference = classify_inner(shared, value, control)?;
    let PaintReference::Gradient(target) = reference else {
        return Err(unsupported(value.offset));
    };
    let target_token = shared
        .tokens()
        .get(target.0)
        .ok_or_else(|| invalid(value.offset))?;
    let linear = eq(shared.source(), target_token.local_name, b"linearGradient");
    let template_id = shared
        .gradient_template_id(target, viewport, control)
        .map_err(|error| remap(error, value.offset))?;
    let source_owner = Arc::clone(shared.source_owner());

    if let Some(index) = cache::find_template(
        cache,
        &source_owner,
        target,
        viewport,
        if linear {
            PaintKind::Linear
        } else {
            PaintKind::Radial
        },
        control,
        value.offset,
    )? {
        let paint = bind_instance(
            cache.entries[index].paint,
            edges,
            user_inverse,
            value.offset,
            control,
        )?;
        control
            .checkpoint()
            .map_err(|_| limit(vexel_scene::LimitKind::Work, value.offset))?;
        return Ok(TemplateResolution {
            index,
            paint,
            template: template_id,
        });
    }

    let mut chain = [ElementId(0); 16];
    let chain_count = collect_chain(shared, target, &mut chain, cache, control, value.offset)?;
    let kind = if linear {
        PaintKind::Linear
    } else {
        PaintKind::Radial
    };
    let mut definition = Paint {
        kind,
        spread: Spread::Pad,
        object_bounding_box: true,
        coordinates: [0.0; 6],
        bounds: [0.0; 4],
        user_inverse: affine_identity(),
        gradient_inverse: affine_identity(),
        stops: TableRange::EMPTY,
        pattern: None,
        pattern_inverse: affine_identity(),
        pattern_opacity: 1.0,
    };
    gradient_data::read_definition_attributes(
        shared.source(),
        shared.tokens(),
        &chain[..chain_count],
        &mut definition,
        value.offset,
    )?;
    gradient_data::read_coordinates(
        shared.source(),
        shared.tokens(),
        &chain[..chain_count],
        viewport,
        &mut definition,
        value.offset,
    )?;
    let (mut stops, stop_count, stop_lease) =
        gradient_data::collect_stops(shared, &chain[..chain_count], cache, control, value.offset)?;
    if stop_count < 2 {
        control.ledger.release(stop_lease)?;
        return Err(unsupported(value.offset));
    }
    stops.truncate(stop_count);
    definition.stops = TableRange {
        first: 0,
        count: stop_count,
    };
    let template_paint = definition;
    if control.checkpoint().is_err() {
        control.ledger.release(stop_lease)?;
        return Err(limit(vexel_scene::LimitKind::Work, value.offset));
    }
    let index = cache::insert_template(
        cache,
        &source_owner,
        target,
        viewport,
        template_paint,
        stops,
        stop_lease,
        control,
        value.offset,
    )?;
    let mut instance = bind_instance(template_paint, edges, user_inverse, value.offset, control)?;
    instance.stops = template_paint.stops;
    Ok(TemplateResolution {
        index,
        paint: instance,
        template: template_id,
    })
}

fn bind_instance(
    mut paint: Paint,
    edges: &[Edge],
    inverse: Affine,
    offset: usize,
    control: &mut PaintControl<'_>,
) -> Result<Paint> {
    if edges.is_empty() {
        return Err(unsupported(offset));
    }
    let first = affine_map_point(inverse, edges[0].a).ok_or_else(|| unsupported(offset))?;
    let (mut min_x, mut min_y, mut max_x, mut max_y) = (first.x, first.y, first.x, first.y);
    for edge in edges {
        for point in [edge.a, edge.b] {
            control
                .charge(WorkKind::PaintVisit, 1)
                .map_err(|_| limit(vexel_scene::LimitKind::Work, offset))?;
            let p = affine_map_point(inverse, point).ok_or_else(|| unsupported(offset))?;
            min_x = min_x.min(p.x);
            min_y = min_y.min(p.y);
            max_x = max_x.max(p.x);
            max_y = max_y.max(p.y);
        }
    }
    paint.bounds = [min_x, min_y, max_x, max_y];
    paint.user_inverse = inverse;
    if paint.object_bounding_box && (max_x <= min_x || max_y <= min_y) {
        return Err(unsupported(offset));
    }
    Ok(paint)
}

fn collect_chain(
    shared: &mut DecodeSource<'_>,
    target: ElementId,
    chain: &mut [ElementId; 16],
    cache: &mut vexel_scene::PaintCache,
    control: &mut PaintControl<'_>,
    offset: usize,
) -> Result<usize> {
    let source = shared.source();
    let tokens = shared.tokens();
    let mut count = 0;
    let mut current = target;
    loop {
        if count == chain.len() {
            return Err(limit(vexel_scene::LimitKind::Depth, offset));
        }
        let token = tokens.get(current.0).ok_or_else(|| invalid(offset))?;
        let is_linear = eq(source, token.local_name, b"linearGradient");
        let is_radial = eq(source, token.local_name, b"radialGradient");
        if (!is_linear && !is_radial) || token.prefix.length != 0 {
            return Err(unsupported(offset));
        }
        // Each element is checked against its own kind. read_coordinates reads only
        // the target kind's geometry names, so cross-kind geometry is ignored.
        attributes_supported(source, tokens, current.0, is_linear, cache, control, offset)?;
        chain[count] = current;
        count += 1;
        let Some(href) = gradient_href(source, tokens, current.0, offset)? else {
            return Ok(count);
        };
        let id = XmlSpan {
            offset: href.offset + 1,
            length: href.length - 1,
        };
        let parent = shared
            .find_reference(id, control)
            .map_err(|error| remap(error, offset))?
            .ok_or_else(|| unsupported(offset))?;
        if chain[..count].contains(&parent) {
            return Err(unsupported(offset));
        }
        current = parent;
    }
}

fn attributes_supported(
    source: &[u8],
    tokens: &[XmlToken],
    element: usize,
    linear: bool,
    cache: &mut vexel_scene::PaintCache,
    control: &mut PaintControl<'_>,
    offset: usize,
) -> Result<()> {
    let mut i = element
        .checked_add(1)
        .ok_or_else(|| limit(vexel_scene::LimitKind::Arithmetic, offset))?;
    while tokens
        .get(i)
        .is_some_and(|token| token.kind == TokenKind::Attribute)
    {
        cache::token_visit(cache, control, offset)?;
        let token = &tokens[i];
        let name = token.local_name;
        let href = eq(source, name, b"href")
            && (token.prefix.length == 0
                || (eq(source, token.prefix, b"xlink")
                    && vexel_svg_reference::xlink_is_bound(source, tokens)));
        let common = token.prefix.length == 0
            && [
                b"id".as_slice(),
                b"class",
                b"style",
                b"gradientUnits",
                b"spreadMethod",
                b"gradientTransform",
            ]
            .iter()
            .any(|n| eq(source, name, n));
        let coordinates: &[&[u8]] = if linear {
            &[b"x1", b"y1", b"x2", b"y2"]
        } else {
            &[b"cx", b"cy", b"r", b"fx", b"fy"]
        };
        let coordinate =
            token.prefix.length == 0 && coordinates.iter().any(|n| eq(source, name, n));
        let collect = eq(source, name, b"collect")
            && eq(source, token.prefix, b"inkscape")
            && inkscape_is_bound(source, tokens);
        if !href && !common && !coordinate && !collect {
            return Err(unsupported(offset));
        }
        i += 1;
    }
    Ok(())
}

/// Same root-declaration rule as `vexel_svg_reference::xlink_is_bound`.
fn inkscape_is_bound(source: &[u8], tokens: &[XmlToken]) -> bool {
    for (i, token) in tokens.iter().enumerate() {
        if token.kind != TokenKind::StartElement || token.depth != 0 {
            continue;
        }
        return tokens
            .get(i + 1..)
            .unwrap_or_default()
            .iter()
            .take_while(|t| t.kind == TokenKind::Attribute)
            .any(|attr| {
                eq(source, attr.name, b"xmlns:inkscape")
                    && eq(
                        source,
                        attr.value,
                        b"http://www.inkscape.org/namespaces/inkscape",
                    )
            });
    }
    false
}

fn gradient_href(
    source: &[u8],
    tokens: &[XmlToken],
    element: usize,
    offset: usize,
) -> Result<Option<XmlSpan>> {
    let mut found = None;
    let mut i = element
        .checked_add(1)
        .ok_or_else(|| limit(vexel_scene::LimitKind::Arithmetic, offset))?;
    while tokens
        .get(i)
        .is_some_and(|token| token.kind == TokenKind::Attribute)
    {
        let token = &tokens[i];
        if eq(source, token.local_name, b"href") {
            let span = token.value;
            let raw = span.slice(source).ok_or_else(|| invalid(offset))?;
            if found.is_some()
                || (token.prefix.length != 0
                    && (!eq(source, token.prefix, b"xlink")
                        || !vexel_svg_reference::xlink_is_bound(source, tokens)))
                || raw.len() < 2
                || raw[0] != b'#'
            {
                return Err(unsupported(offset));
            }
            found = Some(span);
        }
        i += 1;
    }
    Ok(found)
}

pub(crate) fn eq(source: &[u8], span: XmlSpan, text: &[u8]) -> bool {
    span.slice(source) == Some(text)
}
pub(crate) fn keyword(source: &[u8], span: XmlSpan, text: &[u8]) -> bool {
    vexel_svg_css::keyword_equals(source, span, text)
}

pub(crate) fn remap(error: vexel_scene::Diagnostic, offset: usize) -> vexel_scene::Diagnostic {
    vexel_scene::Diagnostic { offset, ..error }
}
