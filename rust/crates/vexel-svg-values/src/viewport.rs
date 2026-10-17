use crate::{
    AttributeList, ValueError, ValueLimits, parse_length_with_limits, parse_number_list_with_limits,
};
use vexel_base::Span;
use vexel_geometry::{Affine, Point, affine_compose, affine_map_point, affine_valid};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aspect {
    pub align_x: f64,
    pub align_y: f64,
    pub none: bool,
    pub slice: bool,
    pub set: bool,
}

impl Default for Aspect {
    fn default() -> Self {
        Self {
            align_x: 0.5,
            align_y: 0.5,
            none: false,
            slice: false,
            set: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MappingErrorKind {
    InvalidArgument,
    Number,
    Viewport,
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MappingError {
    pub kind: MappingErrorKind,
    pub offset: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RootViewport {
    pub width: u32,
    pub height: u32,
    pub view_x: f64,
    pub view_y: f64,
    pub view_width: f64,
    pub view_height: f64,
    pub preserve_aspect: Aspect,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NestedViewport {
    pub child_transform: Affine,
    pub viewport_transform: Affine,
    pub clip_points: [Point; 4],
    pub child_width: f64,
    pub child_height: f64,
    pub empty: bool,
}

const SVG_NAMESPACE: &[u8] = b"http://www.w3.org/2000/svg";

pub fn parse_preserve_aspect(source: &[u8], span: Span) -> Result<Aspect, MappingError> {
    parse_preserve_aspect_with_limits(source, span, ValueLimits::default())
}

pub fn parse_preserve_aspect_with_limits(
    source: &[u8],
    span: Span,
    limits: ValueLimits,
) -> Result<Aspect, MappingError> {
    if source.len() > limits.effective().source_bytes {
        return Err(err(MappingErrorKind::InvalidArgument, 0));
    }
    let bytes = span
        .slice(source)
        .ok_or(err(MappingErrorKind::InvalidArgument, span.start))?;
    let mut first = 0;
    let mut end = bytes.len();
    while first < end && space(bytes[first]) {
        first += 1;
    }
    while end > first && space(bytes[end - 1]) {
        end -= 1;
    }
    if first == end {
        return Err(err(MappingErrorKind::Unsupported, span.start + first));
    }
    let mut split = first;
    while split < end && !space(bytes[split]) {
        split += 1;
    }
    if &bytes[first..split] == b"defer" {
        first = split;
        while first < end && space(bytes[first]) {
            first += 1;
        }
        split = first;
        while split < end && !space(bytes[split]) {
            split += 1;
        }
    }
    let token = &bytes[first..split];
    let mut parsed = Aspect {
        set: true,
        ..Aspect::default()
    };
    if token == b"none" {
        skip_space(bytes, &mut split, end);
        if split != end {
            return Err(err(MappingErrorKind::Unsupported, span.start + split));
        }
        parsed.none = true;
        return Ok(parsed);
    }
    const ALIGN: [&[u8]; 9] = [
        b"xMinYMin",
        b"xMidYMin",
        b"xMaxYMin",
        b"xMinYMid",
        b"xMidYMid",
        b"xMaxYMid",
        b"xMinYMax",
        b"xMidYMax",
        b"xMaxYMax",
    ];
    let alignment = ALIGN
        .iter()
        .position(|candidate| *candidate == token)
        .ok_or(err(MappingErrorKind::Unsupported, span.start + first))?;
    parsed.align_x = (alignment % 3) as f64 * 0.5;
    parsed.align_y = (alignment / 3) as f64 * 0.5;
    first = split;
    skip_space(bytes, &mut first, end);
    if first < end {
        split = first;
        while split < end && !space(bytes[split]) {
            split += 1;
        }
        parsed.slice = &bytes[first..split] == b"slice";
        if !parsed.slice && &bytes[first..split] != b"meet" {
            return Err(err(MappingErrorKind::Unsupported, span.start + first));
        }
        first = split;
        skip_space(bytes, &mut first, end);
        if first != end {
            return Err(err(MappingErrorKind::Unsupported, span.start + first));
        }
    }
    Ok(parsed)
}

pub fn viewbox_transform(
    view_x: f64,
    view_y: f64,
    view_width: f64,
    view_height: f64,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    mut aspect: Aspect,
) -> Result<Affine, MappingError> {
    if ![view_x, view_y, view_width, view_height, x, y, width, height]
        .into_iter()
        .all(f64::is_finite)
        || view_width <= 0.0
        || view_height <= 0.0
        || width <= 0.0
        || height <= 0.0
    {
        return Err(err(MappingErrorKind::Viewport, 0));
    }
    if !aspect.set {
        aspect.align_x = 0.5;
        aspect.align_y = 0.5;
    }
    let mut scale_x = width / view_width;
    let mut scale_y = height / view_height;
    let mut free_x = 0.0;
    let mut free_y = 0.0;
    if !aspect.none {
        let scale = if aspect.slice {
            scale_x.max(scale_y)
        } else {
            scale_x.min(scale_y)
        };
        scale_x = scale;
        scale_y = scale;
        free_x = width - view_width * scale;
        free_y = height - view_height * scale;
    }
    let result = Affine {
        a: scale_x,
        b: 0.0,
        c: 0.0,
        d: scale_y,
        e: x + aspect.align_x * free_x - view_x * scale_x,
        f: y + aspect.align_y * free_y - view_y * scale_y,
    };
    if affine_valid(result) {
        Ok(result)
    } else {
        Err(err(MappingErrorKind::Number, 0))
    }
}

pub fn parse_root_viewport(attributes: AttributeList<'_>) -> Result<RootViewport, MappingError> {
    if let Some(namespace) = attributes.get(b"xmlns") {
        if attributes.value(namespace) != Some(SVG_NAMESPACE) {
            return Err(err(MappingErrorKind::Unsupported, namespace.start));
        }
    }
    let width_span = attributes.get(b"width").ok_or(err(
        MappingErrorKind::Viewport,
        attributes.element_name().start,
    ))?;
    let height_span = attributes.get(b"height").ok_or(err(
        MappingErrorKind::Viewport,
        attributes.element_name().start,
    ))?;
    let width = parse_length_with_limits(attributes.source(), width_span, 0.0, attributes.limits())
        .map_err(value_error)?;
    let height =
        parse_length_with_limits(attributes.source(), height_span, 0.0, attributes.limits())
            .map_err(value_error)?;
    if !(1.0..=4096.0).contains(&width) || !(1.0..=4096.0).contains(&height) {
        return Err(err(
            MappingErrorKind::Viewport,
            if !(1.0..=4096.0).contains(&width) {
                width_span.start
            } else {
                height_span.start
            },
        ));
    }
    let mut result = RootViewport {
        width: width.ceil() as u32,
        height: height.ceil() as u32,
        view_x: 0.0,
        view_y: 0.0,
        view_width: width,
        view_height: height,
        preserve_aspect: Aspect {
            set: true,
            ..Aspect::default()
        },
    };
    if let Some(span) = attributes.get(b"viewBox") {
        let view = parse_viewbox(attributes.source(), span, attributes.limits())?;
        result.view_x = view[0];
        result.view_y = view[1];
        result.view_width = view[2];
        result.view_height = view[3];
    }
    if let Some(span) = attributes.get(b"preserveAspectRatio") {
        result.preserve_aspect =
            parse_preserve_aspect_with_limits(attributes.source(), span, attributes.limits())?;
    }
    Ok(result)
}

pub fn nested_viewport(
    attributes: AttributeList<'_>,
    parent_width: f64,
    parent_height: f64,
    element_transform: Affine,
) -> Result<NestedViewport, MappingError> {
    let source = attributes.source();
    let x = optional_length(attributes, b"x", parent_width, 0.0)?;
    let y = optional_length(attributes, b"y", parent_height, 0.0)?;
    let width = optional_length(attributes, b"width", parent_width, parent_width)?;
    let height = optional_length(attributes, b"height", parent_height, parent_height)?;
    let element_offset = attributes.element_name().start;
    if width < 0.0
        || height < 0.0
        || x.abs() > 1.0e6
        || y.abs() > 1.0e6
        || width > 1.0e6
        || height > 1.0e6
    {
        return Err(err(MappingErrorKind::Number, element_offset));
    }
    let default = NestedViewport {
        child_transform: element_transform,
        viewport_transform: Affine {
            e: x,
            f: y,
            ..identity()
        },
        clip_points: [Point { x: 0.0, y: 0.0 }; 4],
        child_width: width,
        child_height: height,
        empty: false,
    };
    if width == 0.0 || height == 0.0 {
        return Ok(NestedViewport {
            empty: true,
            ..default
        });
    }
    let local = [
        Point { x, y },
        Point { x: x + width, y },
        Point {
            x: x + width,
            y: y + height,
        },
        Point { x, y: y + height },
    ];
    let mut clip = [Point { x: 0.0, y: 0.0 }; 4];
    for (index, point) in local.into_iter().enumerate() {
        clip[index] = affine_map_point(element_transform, point)
            .ok_or(err(MappingErrorKind::Number, element_offset))?;
    }
    let (viewport, child_width, child_height) = if let Some(span) = attributes.get(b"viewBox") {
        let view = parse_viewbox(source, span, attributes.limits())?;
        let aspect = match attributes.get(b"preserveAspectRatio") {
            Some(value) => parse_preserve_aspect_with_limits(source, value, attributes.limits())?,
            None => Aspect::default(),
        };
        (
            viewbox_transform(
                view[0], view[1], view[2], view[3], x, y, width, height, aspect,
            )
            .map_err(|error| err(error.kind, span.start))?,
            view[2],
            view[3],
        )
    } else {
        (
            Affine {
                e: x,
                f: y,
                ..identity()
            },
            width,
            height,
        )
    };
    let child = affine_compose(element_transform, viewport)
        .ok_or(err(MappingErrorKind::Number, element_offset))?;
    Ok(NestedViewport {
        child_transform: child,
        viewport_transform: viewport,
        clip_points: clip,
        child_width,
        child_height,
        empty: false,
    })
}

pub fn parse_symbol_mapping(
    target: AttributeList<'_>,
    use_element: AttributeList<'_>,
    viewport_width: f64,
    viewport_height: f64,
) -> Result<Affine, MappingError> {
    if target.source().as_ptr() != use_element.source().as_ptr()
        || target.source().len() != use_element.source().len()
    {
        return Err(err(MappingErrorKind::InvalidArgument, 0));
    }
    let source = target.source();
    // Without a viewBox the symbol maps by identity; without use width/height the size is 100%.
    let view = match target.get(b"viewBox") {
        Some(view_span) => Some(parse_viewbox(source, view_span, target.limits())?),
        None => None,
    };
    let use_source = use_element.source();
    let width_span = use_element.get(b"width");
    let height_span = use_element.get(b"height");
    let width = match width_span {
        Some(span) => {
            parse_length_with_limits(use_source, span, viewport_width, use_element.limits())
                .map_err(value_error)?
        }
        None => viewport_width,
    };
    let height = match height_span {
        Some(span) => {
            parse_length_with_limits(use_source, span, viewport_height, use_element.limits())
                .map_err(value_error)?
        }
        None => viewport_height,
    };
    if width <= 0.0 || height <= 0.0 {
        let at = width_span.or(height_span).map_or(0, |span| span.start);
        return Err(err(MappingErrorKind::Viewport, at));
    }
    let x = optional_length(use_element, b"x", viewport_width, 0.0)?;
    let y = optional_length(use_element, b"y", viewport_height, 0.0)?;
    let aspect_span = use_element
        .get(b"preserveAspectRatio")
        .or_else(|| target.get(b"preserveAspectRatio"));
    let aspect = match aspect_span {
        Some(span) if span.start != span.end => {
            let policy = if use_element.get(b"preserveAspectRatio").is_some() {
                use_element.limits()
            } else {
                target.limits()
            };
            parse_preserve_aspect_with_limits(use_source, span, policy)?
        }
        _ => Aspect::default(),
    };
    match view {
        Some(view) => viewbox_transform(
            view[0], view[1], view[2], view[3], x, y, width, height, aspect,
        ),
        None => Ok(Affine {
            e: x,
            f: y,
            ..identity()
        }),
    }
}

/// Return the first unsupported attribute's name offset, matching symbol fail-closed behavior.
pub fn symbol_attributes_supported(attributes: AttributeList<'_>) -> Result<(), MappingError> {
    const ALLOWED: [&[u8]; 23] = [
        b"id",
        b"class",
        b"style",
        b"viewBox",
        b"preserveAspectRatio",
        b"width",
        b"height",
        b"fill",
        b"fill-rule",
        b"fill-opacity",
        b"stroke",
        b"stroke-width",
        b"stroke-linecap",
        b"stroke-linejoin",
        b"stroke-miterlimit",
        b"stroke-dasharray",
        b"stroke-dashoffset",
        b"stroke-opacity",
        b"opacity",
        b"visibility",
        b"display",
        b"color",
        b"overflow",
    ];
    // Clipping is not applied to symbols; overflow values are validated only.
    const OVERFLOW: [&[u8]; 4] = [b"visible", b"hidden", b"scroll", b"auto"];
    for attribute in attributes.attributes() {
        let name = attribute
            .name
            .slice(attributes.source())
            .unwrap_or_default();
        if attribute.prefix.is_some() || !ALLOWED.contains(&name) {
            return Err(err(MappingErrorKind::Unsupported, attribute.name.start));
        }
        if name == b"overflow" {
            let value = attribute
                .value
                .slice(attributes.source())
                .unwrap_or_default();
            if !OVERFLOW.contains(&value) {
                return Err(err(MappingErrorKind::Unsupported, attribute.name.start));
            }
        }
    }
    Ok(())
}

fn parse_viewbox(source: &[u8], span: Span, limits: ValueLimits) -> Result<[f64; 4], MappingError> {
    let values = parse_number_list_with_limits(source, span, 4, limits)
        .map_err(|_| err(MappingErrorKind::Viewport, span.start))?;
    if values[2] <= 0.0
        || values[3] <= 0.0
        || (values[0] + values[2]).abs() > 1.0e6
        || (values[1] + values[3]).abs() > 1.0e6
    {
        return Err(err(MappingErrorKind::Viewport, span.start));
    }
    Ok([values[0], values[1], values[2], values[3]])
}

fn optional_length(
    attributes: AttributeList<'_>,
    name: &[u8],
    reference: f64,
    default: f64,
) -> Result<f64, MappingError> {
    match attributes.get(name) {
        Some(span) => {
            parse_length_with_limits(attributes.source(), span, reference, attributes.limits())
                .map_err(value_error)
        }
        None => Ok(default),
    }
}

fn value_error(error: ValueError) -> MappingError {
    match error {
        ValueError::InvalidSpan | ValueError::SourceLimit | ValueError::OutOfMemory => {
            err(MappingErrorKind::InvalidArgument, 0)
        }
        ValueError::Number(error) => err(MappingErrorKind::Number, error.offset),
        ValueError::UnsupportedUnit { offset } => err(MappingErrorKind::Number, offset),
    }
}

fn identity() -> Affine {
    Affine {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: 1.0,
        e: 0.0,
        f: 0.0,
    }
}
fn err(kind: MappingErrorKind, offset: usize) -> MappingError {
    MappingError { kind, offset }
}
fn space(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\r' | b'\n')
}
fn skip_space(bytes: &[u8], index: &mut usize, end: usize) {
    while *index < end && space(bytes[*index]) {
        *index += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Attribute, AttributeList};

    fn list(source: &[u8], element: &[u8], pairs: &[(&[u8], &[u8])]) -> (Vec<Attribute>, Span) {
        let element_start = source
            .windows(element.len())
            .position(|window| window == element)
            .unwrap();
        let mut attributes = Vec::new();
        for (name, value) in pairs {
            let start = source
                .windows(name.len())
                .position(|window| window == *name)
                .unwrap();
            let value_start = source
                .windows(value.len())
                .position(|window| window == *value)
                .unwrap();
            attributes.push(Attribute {
                name: Span {
                    start,
                    end: start + name.len(),
                },
                value: Span {
                    start: value_start,
                    end: value_start + value.len(),
                },
                prefix: None,
            });
        }
        let span = Span {
            start: element_start,
            end: element_start + element.len(),
        };
        (attributes, span)
    }

    #[test]
    fn aspect_meet_slice_none_and_diagnostic_byte_offsets() {
        let source = b"defer xMaxYMid slice";
        let aspect = parse_preserve_aspect(
            source,
            Span {
                start: 0,
                end: source.len(),
            },
        )
        .unwrap();
        assert_eq!(
            (aspect.align_x, aspect.align_y, aspect.slice),
            (1.0, 0.5, true)
        );
        let none = parse_preserve_aspect(b"none", Span { start: 0, end: 4 }).unwrap();
        assert!(none.none);
        let all_alignments = [
            b"xMinYMin",
            b"xMidYMin",
            b"xMaxYMin",
            b"xMinYMid",
            b"xMidYMid",
            b"xMaxYMid",
            b"xMinYMax",
            b"xMidYMax",
            b"xMaxYMax",
        ];
        for (index, alignment) in all_alignments.iter().enumerate() {
            let parsed = parse_preserve_aspect(
                *alignment,
                Span {
                    start: 0,
                    end: alignment.len(),
                },
            )
            .unwrap();
            assert_eq!(
                (parsed.align_x, parsed.align_y),
                ((index % 3) as f64 * 0.5, (index / 3) as f64 * 0.5)
            );
        }
        let source = b" xxx";
        let error = parse_preserve_aspect(source, Span { start: 1, end: 4 }).unwrap_err();
        assert_eq!(error.offset, 1);
        let form_feed = b"xMidYMid\x0cmeet";
        assert!(
            parse_preserve_aspect(
                form_feed,
                Span {
                    start: 0,
                    end: form_feed.len()
                }
            )
            .is_err()
        );
        let oversized = vec![b' '; crate::MAX_SOURCE_BYTES + 1];
        assert_eq!(
            parse_preserve_aspect(&oversized, Span { start: 0, end: 0 }).unwrap_err(),
            err(MappingErrorKind::InvalidArgument, 0)
        );
    }

    #[test]
    fn viewbox_meet_slice_none_and_overflow_limits() {
        let meet = viewbox_transform(0.0, 0.0, 10.0, 5.0, 0.0, 0.0, 20.0, 20.0, Aspect::default())
            .unwrap();
        assert_eq!((meet.a, meet.d, meet.e, meet.f), (2.0, 2.0, 0.0, 5.0));
        let slice = viewbox_transform(
            0.0,
            0.0,
            10.0,
            5.0,
            0.0,
            0.0,
            20.0,
            20.0,
            Aspect {
                slice: true,
                set: true,
                ..Aspect::default()
            },
        )
        .unwrap();
        assert_eq!((slice.a, slice.e), (4.0, -10.0));
        let none = viewbox_transform(
            0.0,
            0.0,
            10.0,
            5.0,
            0.0,
            0.0,
            20.0,
            20.0,
            Aspect {
                none: true,
                set: true,
                ..Aspect::default()
            },
        )
        .unwrap();
        assert_eq!((none.a, none.d), (2.0, 4.0));
        assert!(
            viewbox_transform(
                0.0,
                0.0,
                1e-300,
                1.0,
                0.0,
                0.0,
                1.0,
                1.0,
                Aspect {
                    none: true,
                    set: true,
                    ..Aspect::default()
                }
            )
            .is_err()
        );
        assert!(
            viewbox_transform(
                f64::INFINITY,
                0.0,
                10.0,
                10.0,
                0.0,
                0.0,
                10.0,
                10.0,
                Aspect::default()
            )
            .is_err()
        );
    }

    #[test]
    fn root_nested_and_symbol_mappings_keep_defaults_percentages_and_offsets() {
        let root_source =
            b"svg width 47.2 height 24 viewBox 10 20 12 6 preserveAspectRatio xMidYMid meet";
        let (root_attrs, root_span) = list(
            root_source,
            b"svg",
            &[
                (b"width", b"47.2"),
                (b"height", b"24"),
                (b"viewBox", b"10 20 12 6"),
                (b"preserveAspectRatio", b"xMidYMid meet"),
            ],
        );
        let root =
            parse_root_viewport(AttributeList::new(root_source, root_span, &root_attrs).unwrap())
                .unwrap();
        assert_eq!((root.width, root.height), (48, 24));
        assert_eq!(
            (root.view_x, root.view_y, root.view_width, root.view_height),
            (10.0, 20.0, 12.0, 6.0)
        );

        let default_source = b"svg width 12.5 height 8";
        let (default_attrs, default_span) = list(
            default_source,
            b"svg",
            &[(b"width", b"12.5"), (b"height", b"8")],
        );
        let default_root = parse_root_viewport(
            AttributeList::new(default_source, default_span, &default_attrs).unwrap(),
        )
        .unwrap();
        assert_eq!(
            (
                default_root.width,
                default_root.height,
                default_root.view_x,
                default_root.view_width
            ),
            (13, 8, 0.0, 12.5)
        );

        let nested_source = b"svg x 10% y 2 width 20 height 10 viewBox 0 0 10 10";
        let (nested_values, nested_span) = list(
            nested_source,
            b"svg",
            &[
                (b"x", b"10%"),
                (b"y", b"2"),
                (b"width", b"20"),
                (b"height", b"10"),
                (b"viewBox", b"0 0 10 10"),
            ],
        );
        let nested_attrs = AttributeList::new(nested_source, nested_span, &nested_values).unwrap();
        let nested = nested_viewport(nested_attrs, 100.0, 50.0, identity()).unwrap();
        assert_eq!(
            (
                nested.viewport_transform.a,
                nested.viewport_transform.e,
                nested.viewport_transform.f
            ),
            (1.0, 15.0, 2.0)
        );
        assert_eq!((nested.child_width, nested.child_height), (10.0, 10.0));
        assert_eq!(nested.clip_points[2], Point { x: 30.0, y: 12.0 });

        let symbol_source = b"symbol viewBox 0 0 10 5 preserveAspectRatio xMinYMax slice use x 25% y 4 width 20 height 20";
        let (target_values, target_span) = list(
            symbol_source,
            b"symbol",
            &[
                (b"viewBox", b"0 0 10 5"),
                (b"preserveAspectRatio", b"xMinYMax slice"),
            ],
        );
        let target = AttributeList::new(symbol_source, target_span, &target_values).unwrap();
        let (use_values, use_span) = list(
            symbol_source,
            b"use",
            &[
                (b"x", b"25%"),
                (b"y", b"4"),
                (b"width", b"20"),
                (b"height", b"20"),
            ],
        );
        let use_attrs = AttributeList::new(symbol_source, use_span, &use_values).unwrap();
        let mapping = parse_symbol_mapping(target, use_attrs, 100.0, 80.0).unwrap();
        assert_eq!((mapping.a, mapping.e, mapping.f), (4.0, 25.0, 4.0));
    }

    #[test]
    fn zero_nested_viewport_is_empty_and_symbol_allowlist_is_fail_closed() {
        let source = b"svg width 0 bad x";
        let (values, span) = list(source, b"svg", &[(b"width", b"0"), (b"bad", b"x")]);
        let attrs = AttributeList::new(source, span, &values).unwrap();
        assert!(
            nested_viewport(attrs, 10.0, 10.0, identity())
                .unwrap()
                .empty
        );
        assert_eq!(symbol_attributes_supported(attrs).unwrap_err().offset, 12);
    }
}
