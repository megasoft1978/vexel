//! Owned style plans over canonical source spans. No builder/context borrow escapes.
use crate::invalid;
use vexel_base::{Span, WorkControl, WorkKind};
use vexel_geometry::Affine;
use vexel_scene::{Diagnostic, ErrorKind, Linecap, Linejoin, Result, Stroke, Viewport};
use vexel_svg_css::{CssPaint, CssValue, Property};
use vexel_xml::XmlSpan;

#[derive(Debug, Clone)]
pub struct ResolvedStyle {
    pub fill: [u8; 4],
    pub stroke: Stroke,
    pub dash_values: [f64; 16],
    pub dash_count: usize,
    pub fill_server: Option<XmlSpan>,
    pub stroke_server: Option<XmlSpan>,
    pub clip: Option<XmlSpan>,
    pub mask: Option<XmlSpan>,
    pub filter: Option<XmlSpan>,
    pub markers: [Option<XmlSpan>; 3],
    pub evenodd: bool,
    pub visible: bool,
    pub display_none: bool,
    pub opacity: f64,
    pub blend_mode: u8,
    pub isolation: bool,
}
fn error(kind: ErrorKind, span: XmlSpan) -> Diagnostic {
    Diagnostic {
        kind,
        offset: span.offset,
    }
}
fn ws(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\r' | b'\n')
}
fn value_span(v: XmlSpan) -> Result<Span> {
    Ok(Span {
        start: v.offset,
        end: v.end().ok_or_else(|| invalid(v.offset))?,
    })
}
pub fn multiply_alpha(rgba: &mut [u8; 4], opacity: f64) {
    rgba[3] = (f64::from(rgba[3]) * opacity).round() as u8;
}
pub fn paint_server_value(source: &[u8], span: XmlSpan) -> bool {
    span.slice(source).is_some_and(|v| {
        let n = v.iter().position(|b| !ws(*b)).unwrap_or(v.len());
        v[n..].starts_with(b"url(")
    })
}
pub fn parse_style_color(
    source: &[u8],
    style: &CssPaint<'_>,
    _property: Property,
    span: XmlSpan,
) -> Result<[u8; 4]> {
    let raw = span.slice(source).ok_or_else(|| invalid(span.offset))?;
    if raw.eq_ignore_ascii_case(b"none") {
        return Ok([0; 4]);
    }
    let color_span = if raw.eq_ignore_ascii_case(b"currentColor") {
        let Some(color) = style.value(Property::Color) else {
            return Ok([0, 0, 0, 255]);
        };
        if color.bytes.eq_ignore_ascii_case(b"currentColor") {
            return Err(error(ErrorKind::Unsupported, span));
        }
        color.span
    } else {
        span
    };
    vexel_svg_color::parse_color(source, value_span(color_span)?)
        .ok_or_else(|| error(ErrorKind::Unsupported, span))
}
pub fn parse_opacity_value(source: &[u8], span: XmlSpan) -> Result<f64> {
    parse_opacity_value_controlled(source, span, &mut NoStop)
}
pub fn parse_opacity_value_controlled(
    source: &[u8],
    span: XmlSpan,
    control: &mut dyn WorkControl,
) -> Result<f64> {
    let bytes = span.slice(source).ok_or_else(|| invalid(span.offset))?;
    let mut a = 0;
    let mut z = bytes.len();
    while a < z && ws(bytes[a]) {
        crate::source::charge(control, WorkKind::SourceRead, 1, span.offset + a)?;
        a += 1
    }
    while z > a && ws(bytes[z - 1]) {
        crate::source::charge(control, WorkKind::SourceRead, 1, span.offset + z - 1)?;
        z -= 1
    }
    let percent = z > a && bytes[z - 1] == b'%';
    if percent {
        z -= 1;
        while z > a && ws(bytes[z - 1]) {
            crate::source::charge(control, WorkKind::SourceRead, 1, span.offset + z - 1)?;
            z -= 1
        }
    }
    let mut n = crate::read_number_controlled(
        source,
        XmlSpan {
            offset: span.offset + a,
            length: z - a,
        },
        control,
    )
    .map_err(|e| crate::numeric_error(e, span.offset))?;
    if percent {
        n /= 100.0
    }
    if !n.is_finite() || !(0.0..=1.0).contains(&n) {
        return Err(error(ErrorKind::Number, span));
    }
    Ok(n)
}
pub(crate) fn parse_opacity(
    source: &[u8],
    v: Option<CssValue<'_>>,
    control: &mut dyn WorkControl,
) -> Result<f64> {
    v.map(|v| parse_opacity_value_controlled(source, v.span, control))
        .transpose()
        .map(|v| v.unwrap_or(1.0))
}
pub fn style_display_none(style: &CssPaint<'_>, fallback: usize) -> Result<bool> {
    let Some(v) = style.value(Property::Display) else {
        return Ok(false);
    };
    if v.bytes.eq_ignore_ascii_case(b"none") {
        return Ok(true);
    }
    if [b"inline".as_slice(), b"block", b"contents"]
        .iter()
        .any(|k| v.bytes.eq_ignore_ascii_case(k))
    {
        return Ok(false);
    }
    Err(Diagnostic {
        kind: ErrorKind::Unsupported,
        offset: if v.span.offset == 0 {
            fallback
        } else {
            v.span.offset
        },
    })
}
pub fn parse_dasharray(
    source: &[u8],
    span: XmlSpan,
    reference: f64,
    control: &mut dyn WorkControl,
) -> Result<([f64; 16], usize)> {
    let mut values = [0.0; 16];
    let mut count = 0;
    if span
        .slice(source)
        .is_some_and(|v| v.eq_ignore_ascii_case(b"none"))
    {
        return Ok((values, 0));
    }
    let mut cursor = crate::NumberReader::new(source, span)?;
    loop {
        if cursor.finished(control)? {
            break;
        }
        if count == 16 {
            return Err(error(ErrorKind::Number, span));
        }
        let value = cursor
            .length(reference, control)
            .map_err(|e| crate::numeric_error(e, span.offset))?;
        if !value.is_finite() || value <= 0.0 || value > 1_000_000.0 {
            return Err(error(ErrorKind::Number, span));
        }
        values[count] = value;
        count += 1;
        let before = cursor.index;
        if cursor.finished(control)? {
            break;
        }
        if source.get(cursor.index) == Some(&b',') {
            if !cursor.separator(control)? {
                return Err(error(ErrorKind::Number, span));
            }
        } else if before == cursor.index {
            return Err(error(ErrorKind::Number, span));
        }
    }
    if count == 0 {
        return Err(error(ErrorKind::Number, span));
    }
    Ok((values, count))
}
pub fn uniform_scale(t: Affine) -> Option<f64> {
    if ![t.a, t.b, t.c, t.d].into_iter().all(f64::is_finite) {
        return None;
    }
    let xx = t.a * t.a + t.b * t.b;
    let yy = t.c * t.c + t.d * t.d;
    let xy = t.a * t.c + t.b * t.d;
    let d = (xx - yy).hypot(2.0 * xy);
    let max = ((xx + yy + d) * 0.5).max(0.0).sqrt();
    let min = ((xx + yy - d) * 0.5).max(0.0).sqrt();
    (max.is_finite() && max > 0.0 && (max - min).abs() <= max * 1e-8).then_some(max)
}
pub fn apply_vector_effect(
    source: &[u8],
    style: &CssPaint<'_>,
    t: Affine,
    viewport: Viewport,
    stroke: &mut Stroke,
    fallback: usize,
) -> Result<()> {
    let Some(v) = style.value(Property::VectorEffect) else {
        return Ok(());
    };
    if v.bytes.eq_ignore_ascii_case(b"none") {
        return Ok(());
    }
    if !v.bytes.eq_ignore_ascii_case(b"non-scaling-stroke") {
        return Err(Diagnostic {
            kind: ErrorKind::Unsupported,
            offset: if v.span.offset == 0 {
                fallback
            } else {
                v.span.offset
            },
        });
    }
    let scale = uniform_scale(t).ok_or_else(|| error(ErrorKind::Unsupported, v.span))?;
    if viewport.view_width <= 0.0 || viewport.view_height <= 0.0 {
        return Err(error(ErrorKind::Unsupported, v.span));
    }
    let mut x = f64::from(viewport.width) / viewport.view_width;
    let mut y = f64::from(viewport.height) / viewport.view_height;
    if !viewport.aspect.none {
        let s = if viewport.aspect.slice {
            x.max(y)
        } else {
            x.min(y)
        };
        x = s;
        y = s
    }
    let max = x.max(y);
    if !max.is_finite() || max <= 0.0 || (max - x.min(y)).abs() > max * 1e-8 {
        return Err(error(ErrorKind::Unsupported, v.span));
    }
    let combined = scale * max;
    if !combined.is_finite() || combined <= 0.0 {
        return Err(error(ErrorKind::Number, v.span));
    }
    stroke.width /= combined;
    let _ = source;
    if !stroke.width.is_finite() || stroke.width < 0.0 {
        return Err(error(ErrorKind::Number, v.span));
    }
    Ok(())
}
/// All returned values are owned numbers or canonical source spans.
pub fn parse_style(
    source: &[u8],
    paint: &CssPaint<'_>,
    reference: f64,
    transform: Affine,
    viewport: Viewport,
    control: &mut dyn WorkControl,
) -> Result<ResolvedStyle> {
    for p in Property::ALL {
        if let Some(v) = paint.value(p) {
            if !v.span.slice(source).is_some_and(|bytes| {
                bytes.as_ptr() == v.bytes.as_ptr() && bytes.len() == v.bytes.len()
            }) {
                return Err(invalid(v.span.offset));
            }
            crate::source::charge_bytes(
                control,
                WorkKind::PaintVisit,
                v.bytes.len(),
                v.span.offset,
            )?;
        }
    }
    let mut out = ResolvedStyle {
        fill: [0, 0, 0, 255],
        stroke: Stroke::initial(),
        dash_values: [0.0; 16],
        dash_count: 0,
        fill_server: None,
        stroke_server: None,
        clip: None,
        mask: None,
        filter: None,
        markers: [None; 3],
        evenodd: false,
        visible: true,
        display_none: style_display_none(paint, 0)?,
        opacity: parse_opacity(source, paint.value(Property::Opacity), control)?,
        blend_mode: 0,
        isolation: false,
    };
    if let Some(v) = paint.value(Property::MixBlendMode) {
        out.blend_mode = vexel_scene::BLEND_MODE_NAMES
            .iter()
            .position(|n| *n == v.bytes)
            .ok_or_else(|| error(ErrorKind::Unsupported, v.span))? as u8;
    }
    if let Some(v) = paint.value(Property::Isolation) {
        if v.bytes.eq_ignore_ascii_case(b"isolate") {
            out.isolation = true
        } else if !v.bytes.eq_ignore_ascii_case(b"auto") {
            return Err(error(ErrorKind::Unsupported, v.span));
        }
    }
    let declaration = |p| {
        paint
            .value(p)
            .filter(|v| !v.bytes.eq_ignore_ascii_case(b"none"))
            .map(|v| v.span)
    };
    out.clip = declaration(Property::ClipPath);
    out.mask = declaration(Property::Mask);
    out.filter = declaration(Property::Filter);
    for (i, p) in [
        Property::MarkerStart,
        Property::MarkerMid,
        Property::MarkerEnd,
    ]
    .into_iter()
    .enumerate()
    {
        out.markers[i] = paint
            .value(p)
            .or_else(|| paint.value(Property::Marker))
            .filter(|v| !v.bytes.eq_ignore_ascii_case(b"none"))
            .map(|v| v.span)
    }
    if let Some(v) = paint.value(Property::Visibility) {
        if v.bytes.eq_ignore_ascii_case(b"hidden") || v.bytes.eq_ignore_ascii_case(b"collapse") {
            out.visible = false;
            return Ok(out);
        } else if !v.bytes.eq_ignore_ascii_case(b"visible") {
            return Err(error(ErrorKind::Unsupported, v.span));
        }
    }
    if let Some(v) = paint.value(Property::Fill) {
        if v.bytes.starts_with(b"url(#") {
            out.fill_server = Some(v.span)
        } else {
            out.fill = parse_style_color(source, paint, Property::Fill, v.span)?
        }
    }
    if let Some(v) = paint.value(Property::FillRule) {
        if v.bytes.eq_ignore_ascii_case(b"evenodd") {
            out.evenodd = true
        } else if !v.bytes.eq_ignore_ascii_case(b"nonzero") {
            return Err(error(ErrorKind::Unsupported, v.span));
        }
    }
    multiply_alpha(
        &mut out.fill,
        parse_opacity(source, paint.value(Property::FillOpacity), control)?,
    );
    if let Some(v) = paint.value(Property::Stroke) {
        if paint_server_value(source, v.span) {
            out.stroke_server = Some(v.span);
            out.stroke.paint_server = true;
            out.stroke.rgba = [0, 0, 0, 255]
        } else {
            out.stroke.rgba = parse_style_color(source, paint, Property::Stroke, v.span)?
        }
    }
    for (p, min, max, length) in [
        (Property::StrokeWidth, 0.0, 1024.0, true),
        (Property::StrokeMiterlimit, 1.0, 64.0, false),
        (Property::StrokeDashoffset, -1e6, 1e6, true),
    ] {
        if let Some(v) = paint.value(p) {
            let n = if length {
                crate::read_length_controlled(source, v.span, reference, control)
            } else {
                crate::read_number_controlled(source, v.span, control)
            }
            .map_err(|e| crate::numeric_error(e, v.span.offset))?;
            if !n.is_finite() || n < min || n > max {
                return Err(error(ErrorKind::Number, v.span));
            }
            match p {
                Property::StrokeWidth => out.stroke.width = n,
                Property::StrokeMiterlimit => out.stroke.miter_limit = n,
                _ => out.stroke.dash_offset = n,
            }
        }
    }
    if let Some(v) = paint.value(Property::StrokeLinecap) {
        out.stroke.cap = if v.bytes.eq_ignore_ascii_case(b"butt") {
            Linecap::Butt
        } else if v.bytes.eq_ignore_ascii_case(b"square") {
            Linecap::Square
        } else if v.bytes.eq_ignore_ascii_case(b"round") {
            Linecap::Round
        } else {
            return Err(error(ErrorKind::Unsupported, v.span));
        }
    }
    if let Some(v) = paint.value(Property::StrokeLinejoin) {
        out.stroke.join = if v.bytes.eq_ignore_ascii_case(b"miter")
            || v.bytes.eq_ignore_ascii_case(b"miter-clip")
        {
            Linejoin::Miter
        } else if v.bytes.eq_ignore_ascii_case(b"round") || v.bytes.eq_ignore_ascii_case(b"arcs") {
            Linejoin::Round
        } else if v.bytes.eq_ignore_ascii_case(b"bevel") {
            Linejoin::Bevel
        } else {
            return Err(error(ErrorKind::Unsupported, v.span));
        }
    }
    if let Some(v) = paint.value(Property::StrokeDasharray) {
        (out.dash_values, out.dash_count) = parse_dasharray(source, v.span, reference, control)?
    }
    multiply_alpha(
        &mut out.stroke.rgba,
        parse_opacity(source, paint.value(Property::StrokeOpacity), control)?,
    );
    if let Some(v) = paint.value(Property::PaintOrder) {
        if !v.bytes.eq_ignore_ascii_case(b"normal") {
            let mut seen = [false; 3];
            let mut count = 0;
            let mut cursor = 0;
            while cursor < v.bytes.len() {
                while cursor < v.bytes.len() && ws(v.bytes[cursor]) {
                    cursor += 1
                }
                if cursor == v.bytes.len() {
                    break;
                }
                let start = cursor;
                while cursor < v.bytes.len() && !ws(v.bytes[cursor]) {
                    cursor += 1
                }
                let key = &v.bytes[start..cursor];
                let index = [b"fill".as_slice(), b"stroke", b"markers"]
                    .iter()
                    .position(|k| key.eq_ignore_ascii_case(k))
                    .ok_or(Diagnostic {
                        kind: ErrorKind::Unsupported,
                        offset: v.span.offset + start,
                    })?;
                if count == 3 || seen[index] {
                    return Err(Diagnostic {
                        kind: ErrorKind::Unsupported,
                        offset: v.span.offset + start,
                    });
                }
                if count == 0 {
                    out.stroke.paint_order = u8::from(index == 1)
                }
                seen[index] = true;
                count += 1;
            }
            if count == 0 {
                return Err(error(ErrorKind::Unsupported, v.span));
            }
        }
    }
    if let Some(v) = paint.value(Property::ShapeRendering) {
        if v.bytes.eq_ignore_ascii_case(b"crispEdges") {
            out.stroke.crisp_edges = true
        } else if ![b"auto".as_slice(), b"optimizeSpeed", b"geometricPrecision"]
            .iter()
            .any(|k| v.bytes.eq_ignore_ascii_case(k))
        {
            return Err(error(ErrorKind::Unsupported, v.span));
        }
    }
    apply_vector_effect(source, paint, transform, viewport, &mut out.stroke, 0)?;
    Ok(out)
}

struct NoStop;
impl WorkControl for NoStop {
    fn charge(&mut self, _: WorkKind, _: u64) -> std::result::Result<(), vexel_base::Stop> {
        Ok(())
    }
    fn checkpoint(&mut self) -> std::result::Result<(), vexel_base::Stop> {
        Ok(())
    }
}
