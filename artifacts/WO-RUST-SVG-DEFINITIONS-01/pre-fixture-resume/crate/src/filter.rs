//! Typed parsing for the supported SVG filter subset.
use crate::common::{number_list, parse_number, region_number};
use crate::{DefinitionSource, attr, matching_end, number, oom, span_bytes, unsupported};
use vexel_scene::{
    Diagnostic, ErrorKind, Filter, FilterInput, FilterKind, FilterPrimitive, LimitKind, Result,
};
use vexel_xml::{TokenKind, XmlSpan, XmlToken};

pub const FILTER_MAX_COUNT: usize = 4096;
pub const FILTER_MAX_PRIMITIVES: usize = 16;
pub const FILTER_MAX_TOTAL_PRIMITIVES: usize = 65536;
pub const FILTER_MAX_MERGE_INPUTS: usize = 8;

#[derive(Debug, Clone)]
pub struct FilterPlan {
    pub filter: Filter,
    pub primitives: Vec<FilterPrimitive>,
    pub definition: usize,
}
#[derive(Clone, Copy)]
struct ResultName {
    bytes_offset: usize,
    len: usize,
    present: bool,
}

pub fn parse_filter(
    source: &[u8],
    tokens: &[XmlToken],
    value: XmlSpan,
    element: usize,
) -> Result<FilterPlan> {
    let _view = DefinitionSource::new(source, tokens)?;
    let id = crate::parse_local_url(source, value)?;
    if !valid_id_lookup(source, tokens, id, element)
        || !eq(source, tokens[element].local_name, b"filter")
    {
        return Err(unsupported(value.offset));
    }
    parse_filter_at(source, tokens, element)
}
pub(crate) fn parse_filter_at(
    source: &[u8],
    tokens: &[XmlToken],
    element: usize,
) -> Result<FilterPlan> {
    let _view = DefinitionSource::new(source, tokens)?;
    if !tokens
        .get(element)
        .is_some_and(|t| eq(source, t.local_name, b"filter"))
    {
        return Err(unsupported(
            tokens.get(element).map_or(0, |t| t.name.offset),
        ));
    }
    let end = matching_end(tokens, element)?;
    let mut units = true;
    let mut primitive_units = false;
    if let Some(s) = attr(source, tokens, element, b"filterUnits") {
        units = if eq(source, s, b"userSpaceOnUse") {
            false
        } else if eq(source, s, b"objectBoundingBox") {
            true
        } else {
            return Err(unsupported(s.offset));
        };
    }
    if let Some(s) = attr(source, tokens, element, b"primitiveUnits") {
        primitive_units = if eq(source, s, b"objectBoundingBox") {
            true
        } else if eq(source, s, b"userSpaceOnUse") {
            false
        } else {
            return Err(unsupported(s.offset));
        };
    }
    let mut region = [-0.1, -0.1, 1.2, 1.2];
    let mut custom = false;
    for (i, n) in [b"x".as_slice(), b"y", b"width", b"height"]
        .iter()
        .enumerate()
    {
        if let Some(s) = attr(source, tokens, element, n) {
            custom = true;
            region[i] = region_number(source, s, region[i]).map_err(|_| number(s.offset))?;
            if i >= 2 && region[i] <= 0.0 {
                return Err(number(s.offset));
            }
        }
    }
    let mut primitives = Vec::new();
    primitives
        .try_reserve(8)
        .map_err(|_| oom(tokens[element].name.offset))?;
    let mut names = Vec::<ResultName>::new();
    names
        .try_reserve(8)
        .map_err(|_| oom(tokens[element].name.offset))?;
    let mut i = element + 1;
    while i < end {
        let t = &tokens[i];
        if !is_element(t.kind) || t.depth != tokens[element].depth + 1 {
            i += 1;
            continue;
        }
        if primitives.len() >= FILTER_MAX_PRIMITIVES {
            return Err(crate::limit(LimitKind::Filters, t.name.offset));
        }
        let child_end = matching_end(tokens, i)?;
        if child_end > end {
            return Err(crate::invalid(t.name.offset));
        }
        let p = parse_primitive(source, tokens, i, child_end, &names)?;
        let result = attr(source, tokens, i, b"result");
        if let Some(r) = result {
            if r.length == 0 || names.len() >= FILTER_MAX_PRIMITIVES {
                return Err(crate::limit(LimitKind::Filters, r.offset));
            }
            names.push(ResultName {
                bytes_offset: r.offset,
                len: r.length,
                present: true,
            });
        } else {
            names.push(ResultName {
                bytes_offset: 0,
                len: 0,
                present: false,
            });
        }
        primitives.push(p);
        i = child_end + 1;
    }
    if primitives.is_empty() {
        return Err(unsupported(tokens[element].name.offset));
    }
    Ok(FilterPlan {
        filter: Filter {
            primitives: vexel_scene::TableRange {
                first: 0,
                count: primitives.len(),
            },
            linear_rgb: !attr(source, tokens, element, b"color-interpolation-filters")
                .is_some_and(|s| eq(source, s, b"sRGB")),
            filter_units_object_bounding_box: units,
            primitive_units_object_bounding_box: primitive_units,
            custom_region: custom,
            region,
        },
        primitives,
        definition: element,
    })
}
fn valid_id_lookup(source: &[u8], tokens: &[XmlToken], id: XmlSpan, expect: usize) -> bool {
    let Some(needle) = span_bytes(source, id) else {
        return false;
    };
    for (i, t) in tokens.iter().enumerate() {
        if !is_element(t.kind) {
            continue;
        }
        for a in tokens
            .get(i + 1..)
            .unwrap_or(&[])
            .iter()
            .take_while(|a| a.kind == TokenKind::Attribute)
        {
            if a.prefix.length == 0
                && span_bytes(source, a.local_name) == Some(b"id")
                && span_bytes(source, a.value) == Some(needle)
            {
                return i == expect;
            }
        }
    }
    false
}
fn parse_primitive(
    source: &[u8],
    tokens: &[XmlToken],
    element: usize,
    end: usize,
    results: &[ResultName],
) -> Result<FilterPrimitive> {
    let t = &tokens[element];
    let name = span_bytes(source, t.local_name).unwrap_or_default();
    let kind = match name {
        b"feGaussianBlur" => FilterKind::GaussianBlur,
        b"feOffset" => FilterKind::Offset,
        b"feFlood" => FilterKind::Flood,
        b"feBlend" => FilterKind::Blend,
        b"feComposite" => FilterKind::Composite,
        b"feMerge" => FilterKind::Merge,
        b"feColorMatrix" => FilterKind::ColorMatrix,
        b"feDropShadow" => FilterKind::DropShadow,
        b"feTurbulence" => {
            return Err(Diagnostic {
                kind: ErrorKind::UnsupportedFeTurbulence,
                offset: t.name.offset,
            });
        }
        b"feDisplacementMap" => {
            return Err(Diagnostic {
                kind: ErrorKind::UnsupportedFeDisplacementMap,
                offset: t.name.offset,
            });
        }
        b"feConvolveMatrix" => {
            return Err(Diagnostic {
                kind: ErrorKind::UnsupportedFeConvolveMatrix,
                offset: t.name.offset,
            });
        }
        b"feImage" => {
            return Err(Diagnostic {
                kind: ErrorKind::UnsupportedFeImage,
                offset: t.name.offset,
            });
        }
        b"feDiffuseLighting" | b"feSpecularLighting" => {
            return Err(Diagnostic {
                kind: ErrorKind::UnsupportedFeLighting,
                offset: t.name.offset,
            });
        }
        b"feMorphology" => {
            return Err(Diagnostic {
                kind: ErrorKind::UnsupportedFeMorphology,
                offset: t.name.offset,
            });
        }
        _ => return Err(unsupported(t.name.offset)),
    };
    let count = results.len();
    let default = if count == 0 {
        FilterInput::Graphic
    } else {
        FilterInput::Previous
    };
    let mut p = FilterPrimitive {
        kind,
        input: resolve_input(
            source,
            attr(source, tokens, element, b"in"),
            default,
            results,
        )?,
        input2: resolve_input(
            source,
            attr(source, tokens, element, b"in2"),
            FilterInput::Graphic,
            results,
        )?,
        merge_inputs: [FilterInput::Graphic; 8],
        merge_count: 0,
        mode: 0,
        rgba: [0, 0, 0, 255],
        values: [0.; 20],
        source_offset: t.name.offset,
    };
    if matches!(kind, FilterKind::GaussianBlur | FilterKind::DropShadow) {
        let dev = attr(source, tokens, element, b"stdDeviation");
        let vals = if let Some(s) = dev {
            number_list(source, s, 2).map_err(|_| number(s.offset))?
        } else {
            vec![0.]
        };
        if vals.is_empty() || vals.len() > 2 {
            return Err(number(dev.map_or(t.name.offset, |x| x.offset)));
        }
        p.values[0] = vals[0];
        p.values[1] = *vals.get(1).unwrap_or(&vals[0]);
        if p.values[0] < 0. || p.values[1] < 0. || p.values[0] > 32. || p.values[1] > 32. {
            return Err(crate::limit(
                LimitKind::Filters,
                dev.map_or(t.name.offset, |x| x.offset),
            ));
        }
    }
    if matches!(kind, FilterKind::Offset | FilterKind::DropShadow) {
        for (idx, key) in [(2, b"dx".as_slice()), (3, b"dy")] {
            if let Some(s) = attr(source, tokens, element, key) {
                p.values[idx] = parse_number(source, s, 0.)?;
                if p.values[idx].abs() > 4096. {
                    return Err(crate::limit(LimitKind::Filters, s.offset));
                }
            }
        }
    }
    if matches!(kind, FilterKind::Flood | FilterKind::DropShadow) {
        if let Some(s) = attr(source, tokens, element, b"flood-color") {
            p.rgba = parse_filter_color(source, s).ok_or_else(|| unsupported(t.name.offset))?;
        }
        if let Some(s) = attr(source, tokens, element, b"flood-opacity") {
            let alpha = parse_number(source, s, 1.).map_err(|_| unsupported(t.name.offset))?;
            if !(0.0..=1.0).contains(&alpha) {
                return Err(unsupported(t.name.offset));
            }
            p.rgba[3] = (f64::from(p.rgba[3]) * alpha + 0.5).floor() as u8;
        }
    }
    if kind == FilterKind::Blend {
        if let Some(s) = attr(source, tokens, element, b"mode") {
            p.mode = match span_bytes(source, s).unwrap_or_default() {
                b"normal" => 0,
                b"multiply" => 1,
                b"screen" => 2,
                b"darken" => 3,
                b"lighten" => 4,
                _ => return Err(unsupported(s.offset)),
            };
        }
    }
    if kind == FilterKind::Composite {
        if let Some(s) = attr(source, tokens, element, b"operator") {
            p.mode = match span_bytes(source, s).unwrap_or_default() {
                b"over" => 0,
                b"in" => 1,
                b"out" => 2,
                b"atop" => 3,
                b"xor" => 4,
                b"lighter" => 5,
                b"arithmetic" => 6,
                _ => return Err(unsupported(s.offset)),
            };
        }
        if p.mode == 6 {
            for (i, key) in [b"k1".as_slice(), b"k2", b"k3", b"k4"].iter().enumerate() {
                if let Some(s) = attr(source, tokens, element, key) {
                    p.values[i] = parse_number(source, s, 0.).map_err(|_| number(s.offset))?;
                }
            }
        }
    }
    if kind == FilterKind::ColorMatrix {
        let ty = attr(source, tokens, element, b"type");
        p.mode = match ty.map(|s| span_bytes(source, s).unwrap_or_default()) {
            Some(b"saturate") => 1,
            Some(b"hueRotate") => 2,
            Some(b"luminanceToAlpha") => 3,
            None | Some(b"matrix") => 0,
            _ => return Err(unsupported(ty.unwrap().offset)),
        };
        if let Some(s) = attr(source, tokens, element, b"values") {
            let values = number_list(source, s, 20).map_err(|_| number(s.offset))?;
            let need = match p.mode {
                0 => 20,
                1 | 2 => 1,
                _ => 0,
            };
            if values.len() != need {
                return Err(number(s.offset));
            }
            if p.mode == 1 && !(0.0..=1.0).contains(&values[0]) {
                return Err(number(s.offset));
            }
            p.values[..values.len()].copy_from_slice(&values);
        } else if p.mode == 0 {
            p.values = [
                1., 0., 0., 0., 0., 0., 1., 0., 0., 0., 0., 0., 1., 0., 0., 0., 0., 0., 1., 0.,
            ];
        } else if p.mode == 1 {
            p.values[0] = 1.;
        }
    }
    if kind == FilterKind::Merge {
        let mut used = 0;
        for i in element + 1..end {
            let x = &tokens[i];
            if !is_element(x.kind) || x.depth != t.depth + 1 {
                continue;
            }
            if !eq(source, x.local_name, b"feMergeNode") {
                return Err(unsupported(x.name.offset));
            }
            if used == FILTER_MAX_MERGE_INPUTS {
                return Err(crate::limit(LimitKind::Filters, x.name.offset));
            }
            p.merge_inputs[used] = resolve_input(
                source,
                attr(source, tokens, i, b"in"),
                FilterInput::Graphic,
                results,
            )?;
            used += 1;
        }
        if used == 0 {
            return Err(unsupported(t.name.offset));
        }
        p.merge_count = used as u8;
    }
    Ok(p)
}
fn resolve_input(
    source: &[u8],
    span: Option<XmlSpan>,
    fallback: FilterInput,
    results: &[ResultName],
) -> Result<FilterInput> {
    let Some(s) = span else { return Ok(fallback) };
    let b = span_bytes(source, s).ok_or_else(|| crate::invalid(s.offset))?;
    if b.is_empty() {
        return Ok(fallback);
    }
    if b == b"SourceGraphic" {
        return Ok(FilterInput::Graphic);
    }
    if b == b"SourceAlpha" {
        return Ok(FilterInput::Alpha);
    }
    for (i, r) in results.iter().enumerate() {
        if r.present
            && r.len == b.len()
            && source.get(r.bytes_offset..r.bytes_offset + r.len) == Some(b)
        {
            return Ok(FilterInput::Result(i));
        }
    }
    Err(unsupported(s.offset))
}
fn parse_filter_color(source: &[u8], s: XmlSpan) -> Option<[u8; 4]> {
    crate::parse_color(
        source,
        vexel_base::Span {
            start: s.offset,
            end: s.offset.checked_add(s.length)?,
        },
    )
}
fn is_element(k: TokenKind) -> bool {
    matches!(k, TokenKind::StartElement | TokenKind::EmptyElement)
}
fn eq(source: &[u8], span: XmlSpan, bytes: &[u8]) -> bool {
    span_bytes(source, span) == Some(bytes)
}
