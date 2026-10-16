//! Marker geometry math and strict definition attribute parsing.
use crate::common::{number_list, parse_length};
use crate::{attr, span_bytes};
use vexel_geometry::{Affine, Edge, Point, affine_compose, affine_identity};
use vexel_scene::{Diagnostic, ErrorKind, LimitKind, Result};
use vexel_svg_css::{CssPaint, CssValue, Property};
use vexel_xml::{TokenKind, XmlSpan, XmlToken};
const PI: f64 = 3.14159265358979323846;
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MarkerOrient {
    Angle(f64),
    Auto,
    AutoStartReverse,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MarkerAttributes {
    pub width: f64,
    pub height: f64,
    pub ref_x: f64,
    pub ref_y: f64,
    pub stroke_width_units: bool,
    pub orient: MarkerOrient,
    pub content: Affine,
    pub child_viewport: [f64; 2],
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkerPosition {
    Start,
    Middle,
    End,
}

/// Selects the authored position-specific marker, falling back to the raw
/// legacy `marker` shorthand value. CSS keeps the shorthand as its own value;
/// this lookup performs the SVG marker mapping at the definition boundary.
pub fn reference_value<'src>(
    style: &CssPaint<'src>,
    position: MarkerPosition,
) -> Option<CssValue<'src>> {
    let specific = match position {
        MarkerPosition::Start => Property::MarkerStart,
        MarkerPosition::Middle => Property::MarkerMid,
        MarkerPosition::End => Property::MarkerEnd,
    };
    style
        .value(specific)
        .or_else(|| style.value(Property::Marker))
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MarkerPlacement {
    pub position: MarkerPosition,
    pub point: Point,
    pub angle: f64,
    pub transform: Affine,
}

pub fn parse_attributes(
    source: &[u8],
    tokens: &[XmlToken],
    element: usize,
) -> Result<MarkerAttributes> {
    parse_attributes_for_viewport(source, tokens, element, [1.0, 1.0])
}

/// Parses marker lengths using the active viewport, as the definition walker
/// does for percentage marker dimensions.
pub fn parse_attributes_for_viewport(
    source: &[u8],
    tokens: &[XmlToken],
    element: usize,
    viewport: [f64; 2],
) -> Result<MarkerAttributes> {
    if viewport.iter().any(|v| !v.is_finite() || *v <= 0.0) {
        return Err(number(tokens.get(element).map_or(0, |t| t.name.offset)));
    }
    const ALLOWED: &[&[u8]] = &[
        b"id",
        b"class",
        b"style",
        b"markerWidth",
        b"markerHeight",
        b"refX",
        b"refY",
        b"markerUnits",
        b"orient",
        b"viewBox",
        b"preserveAspectRatio",
        b"overflow",
    ];
    for a in tokens
        .get(element + 1..)
        .ok_or_else(|| bad(0))?
        .iter()
        .take_while(|t| t.kind == TokenKind::Attribute)
    {
        if a.prefix.length != 0
            || !ALLOWED.contains(&span_bytes(source, a.local_name).unwrap_or_default())
        {
            return Err(unsupported(a.name.offset));
        }
        if span_bytes(source, a.local_name) == Some(b"overflow") {
            let b = span_bytes(source, a.value).unwrap_or_default();
            if !b.eq_ignore_ascii_case(b"visible") && !b.eq_ignore_ascii_case(b"hidden") {
                return Err(unsupported(a.value.offset));
            }
        }
    }
    let width = attr(source, tokens, element, b"markerWidth")
        .map(|s| parse_length(source, s, viewport[0]))
        .transpose()?
        .unwrap_or(3.);
    let height = attr(source, tokens, element, b"markerHeight")
        .map(|s| parse_length(source, s, viewport[1]))
        .transpose()?
        .unwrap_or(3.);
    if width <= 0. || height <= 0. {
        return Err(limit(tokens[element].name.offset));
    }
    if width > 4096. || height > 4096. {
        return Err(limit(tokens[element].name.offset));
    }
    let ref_x = attr(source, tokens, element, b"refX")
        .map(|s| parse_length(source, s, width))
        .transpose()?
        .unwrap_or(0.);
    let ref_y = attr(source, tokens, element, b"refY")
        .map(|s| parse_length(source, s, height))
        .transpose()?
        .unwrap_or(0.);
    let stroke_width_units = match attr(source, tokens, element, b"markerUnits")
        .map(|s| span_bytes(source, s).unwrap_or_default())
    {
        None | Some(b"strokeWidth") => true,
        Some(b"userSpaceOnUse") => false,
        _ => {
            return Err(unsupported(
                attr(source, tokens, element, b"markerUnits")
                    .unwrap()
                    .offset,
            ));
        }
    };
    let orient = if let Some(s) = attr(source, tokens, element, b"orient") {
        parse_orient(source, s)?
    } else {
        MarkerOrient::Angle(0.)
    };
    let mut content = affine_identity();
    let child_viewport = [width, height];
    if let Some(s) = attr(source, tokens, element, b"viewBox") {
        let v = number_list(source, s, 4)?;
        if v.len() != 4 || v[2] <= 0. || v[3] <= 0. {
            return Err(Diagnostic {
                kind: ErrorKind::Viewport,
                offset: s.offset,
            });
        }
        let aspect = if let Some(a) = attr(source, tokens, element, b"preserveAspectRatio") {
            let end = a.offset.checked_add(a.length).ok_or_else(|| Diagnostic {
                kind: ErrorKind::InvalidArgument,
                offset: a.offset,
            })?;
            vexel_svg_values::parse_preserve_aspect(
                source,
                vexel_base::Span {
                    start: a.offset,
                    end,
                },
            )
            .map_err(|e| Diagnostic {
                kind: ErrorKind::Unsupported,
                offset: e.offset,
            })?
        } else {
            vexel_svg_values::Aspect {
                align_x: 0.5,
                align_y: 0.5,
                none: false,
                slice: false,
                set: true,
            }
        };
        content = vexel_svg_values::viewbox_transform(
            v[0], v[1], v[2], v[3], 0., 0., width, height, aspect,
        )
        .map_err(|_| Diagnostic {
            kind: ErrorKind::Viewport,
            offset: s.offset,
        })?;
        let Some(reference) =
            vexel_geometry::affine_map_point(content, Point { x: ref_x, y: ref_y })
        else {
            return Err(Diagnostic {
                kind: ErrorKind::Viewport,
                offset: s.offset,
            });
        };
        return Ok(MarkerAttributes {
            width,
            height,
            ref_x: reference.x,
            ref_y: reference.y,
            stroke_width_units,
            orient,
            content,
            child_viewport: [v[2], v[3]],
        });
    }
    Ok(MarkerAttributes {
        width,
        height,
        ref_x,
        ref_y,
        stroke_width_units,
        orient,
        content,
        child_viewport,
    })
}
pub fn direction_angle(edge: Edge) -> f64 {
    (edge.b.y - edge.a.y).atan2(edge.b.x - edge.a.x)
}
pub fn middle_angle(incoming: Edge, outgoing: Edge) -> f64 {
    let first = direction_angle(incoming);
    let mut delta = (direction_angle(outgoing) - first) % (2.0 * PI);
    if delta > PI {
        delta -= 2.0 * PI;
    } else if delta < -PI {
        delta += 2.0 * PI;
    }
    first + delta * 0.5
}
pub fn element_scale(m: Affine) -> f64 {
    let xx = m.a * m.a + m.b * m.b;
    let yy = m.c * m.c + m.d * m.d;
    let xy = m.a * m.c + m.b * m.d;
    ((xx + yy + ((xx - yy) * (xx - yy) + 4. * xy * xy).sqrt()) * 0.5)
        .max(0.)
        .sqrt()
}
pub fn compose_marker_transform(
    position: Point,
    angle: f64,
    scale: f64,
    content: Affine,
    reference: Point,
) -> Option<Affine> {
    let tr = Affine {
        a: 1.,
        b: 0.,
        c: 0.,
        d: 1.,
        e: position.x,
        f: position.y,
    };
    let rot = Affine {
        a: angle.cos(),
        b: angle.sin(),
        c: -angle.sin(),
        d: angle.cos(),
        e: 0.,
        f: 0.,
    };
    let sc = Affine {
        a: scale,
        b: 0.,
        c: 0.,
        d: scale,
        e: 0.,
        f: 0.,
    };
    let sh = Affine {
        a: 1.,
        b: 0.,
        c: 0.,
        d: 1.,
        e: -reference.x,
        f: -reference.y,
    };
    let x = affine_compose(tr, rot)?;
    let x = affine_compose(x, sc)?;
    let x = affine_compose(x, sh)?;
    affine_compose(x, content)
}
fn parse_orient(source: &[u8], span: XmlSpan) -> Result<MarkerOrient> {
    let b = span_bytes(source, span).ok_or_else(|| bad(span.offset))?;
    if b.eq_ignore_ascii_case(b"auto") {
        return Ok(MarkerOrient::Auto);
    }
    if b.eq_ignore_ascii_case(b"auto-start-reverse") {
        return Ok(MarkerOrient::AutoStartReverse);
    }
    let mut first = 0usize;
    let mut last = b.len();
    while first < last && crate::ascii_space(b[first]) {
        first += 1;
    }
    while last > first && crate::ascii_space(b[last - 1]) {
        last -= 1;
    }
    if last - first >= 3 && &b[last - 3..last] == b"deg" {
        last -= 3;
        while last > first && crate::ascii_space(b[last - 1]) {
            last -= 1;
        }
    }
    if first == last {
        return Err(number(span.offset));
    }
    let s = XmlSpan {
        offset: span
            .offset
            .checked_add(first)
            .ok_or_else(|| bad(span.offset))?,
        length: last - first,
    };
    Ok(MarkerOrient::Angle(crate::common::parse_number(
        source, s, 0.,
    )?))
}
fn bad(offset: usize) -> Diagnostic {
    Diagnostic {
        kind: ErrorKind::InvalidArgument,
        offset,
    }
}
fn number(offset: usize) -> Diagnostic {
    Diagnostic {
        kind: ErrorKind::Number,
        offset,
    }
}
fn unsupported(offset: usize) -> Diagnostic {
    Diagnostic {
        kind: ErrorKind::Unsupported,
        offset,
    }
}
fn limit(offset: usize) -> Diagnostic {
    Diagnostic {
        kind: ErrorKind::Limit(LimitKind::Shapes),
        offset,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn angle_wrap_uses_short_turn() {
        let e = |a: f64| Edge {
            a: Point { x: 0., y: 0. },
            b: Point {
                x: a.cos(),
                y: a.sin(),
            },
        };
        let m = middle_angle(e(179f64.to_radians()), e((-179f64).to_radians()));
        assert!((m.abs() - std::f64::consts::PI).abs() < 1e-12);
    }
    #[test]
    fn element_scale_is_largest_singular_value() {
        let m = Affine {
            a: 3.,
            b: 0.,
            c: 0.,
            d: 2.,
            e: 0.,
            f: 0.,
        };
        assert_eq!(element_scale(m), 3.);
    }
    #[test]
    fn mid_markers_follow_authored_segments_not_flattened_edges() {
        let edges = [
            Edge {
                a: Point { x: 0., y: 0. },
                b: Point { x: 1., y: 0. },
            },
            Edge {
                a: Point { x: 1., y: 0. },
                b: Point { x: 2., y: 0. },
            },
            Edge {
                a: Point { x: 2., y: 0. },
                b: Point { x: 2., y: 1. },
            },
            Edge {
                a: Point { x: 2., y: 1. },
                b: Point { x: 0., y: 0. },
            },
        ];
        let seg = |command, first_edge, edge_count| vexel_svg_path::PathSegment {
            command,
            source: vexel_base::Span { start: 0, end: 1 },
            command_offset: None,
            first_number: 0,
            number_count: 0,
            first_edge,
            edge_count,
        };
        let attrs = MarkerAttributes {
            width: 3.,
            height: 3.,
            ref_x: 0.,
            ref_y: 0.,
            stroke_width_units: false,
            orient: MarkerOrient::Auto,
            content: affine_identity(),
            child_viewport: [3., 3.],
        };
        let cancel = std::sync::atomic::AtomicBool::new(false);
        let mut control = vexel_base::BudgetControl::new(&cancel, u64::MAX);
        let placed = path_placements(
            &edges,
            &[
                seg(b'M', 0, 0),
                seg(b'C', 0, 2),
                seg(b'L', 2, 1),
                seg(b'Z', 3, 1),
            ],
            attrs,
            affine_identity(),
            1.,
            &mut control,
        )
        .unwrap();
        assert_eq!(
            placed.iter().map(|p| p.position).collect::<Vec<_>>(),
            vec![
                MarkerPosition::Start,
                MarkerPosition::Middle,
                MarkerPosition::Middle,
                MarkerPosition::End
            ]
        );
    }
    #[test]
    fn transform_keeps_position_reference_order() {
        let m = compose_marker_transform(
            Point { x: 10., y: 20. },
            0.,
            2.,
            affine_identity(),
            Point { x: 1., y: 2. },
        )
        .unwrap();
        let p = vexel_geometry::affine_map_point(m, Point { x: 1., y: 2. }).unwrap();
        assert_eq!(p, Point { x: 10., y: 20. });
    }
    #[test]
    fn path_marker_loop_charges_empty_segments_and_stops_without_partial_output() {
        let segment = vexel_svg_path::PathSegment {
            command: b'L',
            source: vexel_base::Span { start: 0, end: 1 },
            command_offset: None,
            first_number: 0,
            number_count: 0,
            first_edge: 0,
            edge_count: 0,
        };
        let attributes = MarkerAttributes {
            width: 1.,
            height: 1.,
            ref_x: 0.,
            ref_y: 0.,
            stroke_width_units: false,
            orient: MarkerOrient::Auto,
            content: affine_identity(),
            child_viewport: [1., 1.],
        };
        let cancel = std::sync::atomic::AtomicBool::new(false);
        let mut control = vexel_base::BudgetControl::new(&cancel, 2);
        assert!(
            path_placements(
                &[],
                &[segment; 3],
                attributes,
                affine_identity(),
                1.,
                &mut control
            )
            .is_err()
        );
        assert_eq!(control.stats().units(vexel_base::WorkKind::Expansion), 2);
        let cancel = std::sync::atomic::AtomicBool::new(true);
        let mut control = vexel_base::BudgetControl::new(&cancel, u64::MAX);
        assert_eq!(
            path_placements(
                &[],
                &[segment],
                attributes,
                affine_identity(),
                1.,
                &mut control
            )
            .unwrap_err()
            .kind,
            ErrorKind::Canceled
        );
    }
    #[test]
    fn primitive_marker_pair_loop_charges_before_each_pair() {
        let edge = Edge {
            a: Point { x: 0., y: 0. },
            b: Point { x: 1., y: 0. },
        };
        let attributes = MarkerAttributes {
            width: 1.,
            height: 1.,
            ref_x: 0.,
            ref_y: 0.,
            stroke_width_units: false,
            orient: MarkerOrient::Auto,
            content: affine_identity(),
            child_viewport: [1., 1.],
        };
        let cancel = std::sync::atomic::AtomicBool::new(false);
        let mut control = vexel_base::BudgetControl::new(&cancel, 1);
        let error = primitive_placements(
            &[edge; 4],
            false,
            attributes,
            affine_identity(),
            1.,
            &mut control,
        )
        .unwrap_err();
        assert_eq!(error.kind, ErrorKind::Limit(crate::LimitKind::Work));
        assert_eq!(control.stats().units(vexel_base::WorkKind::Expansion), 1);
    }
    #[test]
    fn marker_aspect_span_overflow_is_a_diagnostic() {
        let xml = b"<marker viewBox='0 0 1 1' preserveAspectRatio='none'/>";
        let (mut tokens, result) = vexel_xml::tokenize(xml, vexel_xml::XmlLimits::default());
        assert_eq!(result.diagnostic.category, vexel_xml::XmlError::None);
        let element = tokens
            .iter()
            .position(|t| t.kind == TokenKind::EmptyElement)
            .unwrap();
        let aspect = tokens
            .iter_mut()
            .find(|t| {
                t.kind == TokenKind::Attribute
                    && span_bytes(xml, t.local_name) == Some(b"preserveAspectRatio")
            })
            .unwrap();
        aspect.value.offset = usize::MAX;
        aspect.value.length = 1;
        let error = parse_attributes(xml, &tokens, element).unwrap_err();
        assert_eq!(error.kind, ErrorKind::InvalidArgument);
        assert_eq!(error.offset, usize::MAX);
    }
}
/// Marker placements over original parser path segments; only authored segment
/// boundaries receive mid markers, even when a curve flattened to many edges.
pub fn path_placements(
    edges: &[Edge],
    segments: &[vexel_svg_path::PathSegment],
    attributes: MarkerAttributes,
    transform: Affine,
    stroke_width: f64,
    control: &mut dyn vexel_base::WorkControl,
) -> Result<Vec<MarkerPlacement>> {
    let scale = marker_scale(attributes, transform, stroke_width);
    let mut out = Vec::new();
    out.try_reserve(edges.len().min(MAX_MARKER_INSTANCES))
        .map_err(|_| oom(0))?;
    let mut first = None;
    let mut last = None;
    for segment in segments {
        control
            .charge(vexel_base::WorkKind::Expansion, 1)
            .map_err(crate::planner_support::stopped)?;
        let command = segment.command.to_ascii_uppercase();
        if command == b'M' {
            flush(&mut out, &mut first, &mut last, attributes, scale)?;
        }
        if segment.edge_count == 0 {
            continue;
        }
        let end = segment
            .first_edge
            .checked_add(segment.edge_count)
            .ok_or_else(|| bad(segment.source.start))?;
        let path = edges
            .get(segment.first_edge..end)
            .ok_or_else(|| bad(segment.source.start))?;
        let a = path[0];
        let b = *path.last().ok_or_else(|| bad(segment.source.start))?;
        if let Some(incoming) = last {
            push(
                &mut out,
                MarkerPosition::Middle,
                incoming.b,
                middle_angle(incoming, a),
                attributes,
                scale,
            )?;
        } else {
            first = Some(a);
            push(
                &mut out,
                MarkerPosition::Start,
                a.a,
                oriented(a, MarkerPosition::Start, attributes.orient),
                attributes,
                scale,
            )?;
        }
        last = Some(b);
        if command == b'Z' {
            flush(&mut out, &mut first, &mut last, attributes, scale)?;
        }
    }
    flush(&mut out, &mut first, &mut last, attributes, scale)?;
    Ok(out)
}
/// Primitive marker placement follows the owned decoder's line/polyline/polygon edge ranges.
pub fn primitive_placements(
    edges: &[Edge],
    polyline: bool,
    attributes: MarkerAttributes,
    transform: Affine,
    stroke_width: f64,
    control: &mut dyn vexel_base::WorkControl,
) -> Result<Vec<MarkerPlacement>> {
    let count = edges.len().saturating_sub(usize::from(polyline));
    if count == 0 {
        return Ok(Vec::new());
    }
    if count > MAX_MARKER_INSTANCES.saturating_sub(1) {
        return Err(limit(0));
    }
    let used = &edges[..count];
    let scale = marker_scale(attributes, transform, stroke_width);
    let mut out = Vec::new();
    out.try_reserve_exact(count.saturating_add(1))
        .map_err(|_| oom(0))?;
    let first = used[0];
    push(
        &mut out,
        MarkerPosition::Start,
        first.a,
        oriented(first, MarkerPosition::Start, attributes.orient),
        attributes,
        scale,
    )?;
    for pair in used.windows(2) {
        control
            .charge(vexel_base::WorkKind::Expansion, 1)
            .map_err(crate::planner_support::stopped)?;
        push(
            &mut out,
            MarkerPosition::Middle,
            pair[0].b,
            middle_angle(pair[0], pair[1]),
            attributes,
            scale,
        )?;
    }
    let last = *used.last().unwrap();
    push(
        &mut out,
        MarkerPosition::End,
        last.b,
        oriented(last, MarkerPosition::End, attributes.orient),
        attributes,
        scale,
    )?;
    Ok(out)
}
fn marker_scale(a: MarkerAttributes, transform: Affine, width: f64) -> f64 {
    if a.stroke_width_units {
        let v = width * element_scale(transform);
        if v.is_finite() && v > 0. { v } else { 1. }
    } else {
        1.
    }
}
fn oriented(edge: Edge, position: MarkerPosition, orient: MarkerOrient) -> f64 {
    match orient {
        MarkerOrient::Angle(d) => d * PI / 180.,
        MarkerOrient::Auto => direction_angle(edge),
        MarkerOrient::AutoStartReverse => {
            direction_angle(edge)
                + if position == MarkerPosition::Start {
                    PI
                } else {
                    0.
                }
        }
    }
}
fn push(
    out: &mut Vec<MarkerPlacement>,
    position: MarkerPosition,
    point: Point,
    angle: f64,
    a: MarkerAttributes,
    scale: f64,
) -> Result<()> {
    if out.len() >= MAX_MARKER_INSTANCES {
        return Err(limit(0));
    }
    let transform = compose_marker_transform(
        point,
        angle,
        scale,
        a.content,
        Point {
            x: a.ref_x,
            y: a.ref_y,
        },
    )
    .ok_or_else(|| number(0))?;
    out.try_reserve(1).map_err(|_| Diagnostic {
        kind: ErrorKind::OutOfMemory,
        offset: 0,
    })?;
    out.push(MarkerPlacement {
        position,
        point,
        angle,
        transform,
    });
    Ok(())
}
const MAX_MARKER_INSTANCES: usize = 1024;
fn oom(offset: usize) -> Diagnostic {
    Diagnostic {
        kind: ErrorKind::OutOfMemory,
        offset,
    }
}
fn flush(
    out: &mut Vec<MarkerPlacement>,
    first: &mut Option<Edge>,
    last: &mut Option<Edge>,
    a: MarkerAttributes,
    scale: f64,
) -> Result<()> {
    if first.is_some() {
        if let Some(end) = *last {
            push(
                out,
                MarkerPosition::End,
                end.b,
                oriented(end, MarkerPosition::End, a.orient),
                a,
                scale,
            )?;
        }
    }
    *first = None;
    *last = None;
    Ok(())
}
