use vexel_base::{Span, WorkControl, WorkKind};
use vexel_scene::{
    Diagnostic, ErrorKind, ImagePixels, LimitKind, MediaError, MediaProvider, Result,
};
use vexel_xml::{TokenKind, XmlSpan, XmlToken};

pub const MAX_ENCODED_IMAGE_BYTES: usize = 32 * 1024 * 1024;
pub const MAX_IMAGE_URI_BYTES: usize = 44_739_266;
pub const MAX_IMAGE_DIMENSION: u32 = 16_384;
pub const MAX_TOTAL_IMAGE_PIXELS: usize = 64 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImagePayload {
    pub mime: ImageMime,
    pub encoded: XmlSpan,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageMime {
    Png,
    Jpeg,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImageGeometry {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub draw_x: f64,
    pub draw_y: f64,
    pub draw_width: f64,
    pub draw_height: f64,
}
#[derive(Debug)]
pub struct ImagePlan {
    pub geometry: ImageGeometry,
    pub pixels: Option<ImagePixels>,
    pub source: XmlSpan,
}

fn fail(kind: ErrorKind, offset: usize) -> Diagnostic {
    Diagnostic { kind, offset }
}
fn valid_span(source: &[u8], span: XmlSpan) -> Option<&[u8]> {
    span.slice(source)
}

/// Exact byte comparison used by image attribute and URI parsing.
pub fn span_is(source: &[u8], span: XmlSpan, literal: &[u8]) -> bool {
    valid_span(source, span).is_some_and(|value| value == literal)
}

/// Find one unprefixed or explicitly prefixed attribute on a start element.
pub fn attribute(
    source: &[u8],
    tokens: &[XmlToken],
    element: usize,
    end: usize,
    local: &[u8],
    prefix: Option<&[u8]>,
) -> Option<XmlSpan> {
    let token = tokens.get(element)?;
    if !matches!(
        token.kind,
        TokenKind::StartElement | TokenKind::EmptyElement
    ) {
        return None;
    }
    for attr in tokens
        .get(element + 1..end.min(tokens.len()))?
        .iter()
        .take_while(|t| t.kind == TokenKind::Attribute)
    {
        if !span_is(source, attr.local_name, local) {
            continue;
        }
        let has_prefix = attr.prefix.length != 0;
        if match prefix {
            None => !has_prefix,
            Some(p) => has_prefix && span_is(source, attr.prefix, p),
        } {
            return Some(attr.value);
        }
    }
    None
}

pub fn base64_value(byte: u8) -> Option<u8> {
    match byte {
        b'A'..=b'Z' => Some(byte - b'A'),
        b'a'..=b'z' => Some(byte - b'a' + 26),
        b'0'..=b'9' => Some(byte - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

/// Decode canonical padded base64. Invalid alphabet/padding is unsupported;
/// invalid spans, empty data, and frozen size caps retain a source offset.
pub fn decode_base64(
    source: &[u8],
    payload: XmlSpan,
    control: &mut dyn WorkControl,
) -> Result<Vec<u8>> {
    let bytes = valid_span(source, payload)
        .ok_or_else(|| fail(ErrorKind::InvalidArgument, payload.offset))?;
    if bytes.is_empty() || bytes.len() > MAX_IMAGE_URI_BYTES || bytes.len() % 4 != 0 {
        return Err(fail(ErrorKind::Limit(LimitKind::Images), payload.offset));
    }
    let padding = usize::from(bytes.ends_with(b"=")) + usize::from(bytes.ends_with(b"=="));
    let groups = bytes.len() / 4;
    let decoded_len = groups
        .checked_mul(3)
        .and_then(|n| n.checked_sub(padding))
        .ok_or_else(|| fail(ErrorKind::Limit(LimitKind::Arithmetic), payload.offset))?;
    if decoded_len == 0 || decoded_len > MAX_ENCODED_IMAGE_BYTES {
        return Err(fail(ErrorKind::Limit(LimitKind::Images), payload.offset));
    }
    let mut output = Vec::new();
    output
        .try_reserve_exact(decoded_len)
        .map_err(|_| fail(ErrorKind::OutOfMemory, payload.offset))?;
    for (group_index, block) in bytes.chunks_exact(4).enumerate() {
        control
            .charge(WorkKind::Media, 1)
            .map_err(|stop| Diagnostic {
                kind: match stop {
                    vexel_base::Stop::Canceled => ErrorKind::Canceled,
                    vexel_base::Stop::Limit => ErrorKind::Limit(LimitKind::Work),
                    vexel_base::Stop::Overflow => ErrorKind::Limit(LimitKind::Arithmetic),
                },
                offset: payload.offset + group_index * 4,
            })?;
        let final_group = group_index + 1 == groups;
        let a = base64_value(block[0]);
        let b = base64_value(block[1]);
        let c = if block[2] == b'=' {
            Some(0)
        } else {
            base64_value(block[2])
        };
        let d = if block[3] == b'=' {
            Some(0)
        } else {
            base64_value(block[3])
        };
        let legal = final_group
            && ((block[2] == b'=' && block[3] == b'=') || (block[2] != b'=' && block[3] == b'='));
        if a.is_none()
            || b.is_none()
            || c.is_none()
            || d.is_none()
            || ((block[2] == b'=' || block[3] == b'=') && !legal)
            || (block[2] == b'=' && (b.unwrap_or(0) & 15) != 0)
            || (block[3] == b'=' && block[2] != b'=' && (c.unwrap_or(0) & 3) != 0)
        {
            return Err(fail(
                ErrorKind::Unsupported,
                payload.offset + group_index * 4,
            ));
        }
        let bits = ((a.unwrap() as u32) << 18)
            | ((b.unwrap() as u32) << 12)
            | ((c.unwrap() as u32) << 6)
            | d.unwrap() as u32;
        output.push((bits >> 16) as u8);
        if block[2] != b'=' {
            output.push((bits >> 8) as u8);
        }
        if block[3] != b'=' {
            output.push(bits as u8);
        }
    }
    if output.len() != decoded_len {
        return Err(fail(ErrorKind::Unsupported, payload.offset));
    }
    Ok(output)
}

/// Accept only the two frozen inline URI headers, with no external lookup.
pub fn parse_data_uri(source: &[u8], uri: XmlSpan) -> Result<ImagePayload> {
    let bytes =
        valid_span(source, uri).ok_or_else(|| fail(ErrorKind::InvalidArgument, uri.offset))?;
    let comma = bytes
        .iter()
        .position(|&b| b == b',')
        .ok_or_else(|| fail(ErrorKind::Unsupported, uri.offset))?;
    let mime = match &bytes[..comma] {
        b"data:image/png;base64" => ImageMime::Png,
        b"data:image/jpeg;base64" => ImageMime::Jpeg,
        _ => return Err(fail(ErrorKind::Unsupported, uri.offset)),
    };
    if comma + 1 == bytes.len() {
        return Err(fail(ErrorKind::Unsupported, uri.offset));
    }
    Ok(ImagePayload {
        mime,
        encoded: XmlSpan {
            offset: uri.offset + comma + 1,
            length: bytes.len() - comma - 1,
        },
    })
}

/// Parse SVG image x/y/width/height and meet/slice/none placement geometry.
pub fn image_geometry(
    source: &[u8],
    tokens: &[XmlToken],
    element: usize,
    end: usize,
    viewport: [f64; 2],
    pixel_size: Option<[u32; 2]>,
    aspect: vexel_svg_values::Aspect,
) -> Result<ImageGeometry> {
    let names: [&[u8]; 4] = [b"x", b"y", b"width", b"height"];
    let mut values = [0.; 4];
    let mut seen = [false; 4];
    for i in 0..4 {
        let Some(span) = attribute(source, tokens, element, end, names[i], None) else {
            continue;
        };
        seen[i] = true;
        let reference = viewport[if i == 0 || i == 2 { 0 } else { 1 }];
        values[i] = vexel_svg_values::parse_length(
            source,
            Span::from_offset_length(span.offset, span.length)
                .ok_or_else(|| fail(ErrorKind::Number, span.offset))?,
            reference,
        )
        .map_err(|_| fail(ErrorKind::Number, span.offset))?;
    }
    if !seen[2] || !seen[3] || values[2] < 0. || values[3] < 0. {
        return Err(fail(
            ErrorKind::Number,
            tokens.get(element).map_or(0, |t| t.name.offset),
        ));
    }
    let mut g = ImageGeometry {
        x: values[0],
        y: values[1],
        width: values[2],
        height: values[3],
        draw_x: values[0],
        draw_y: values[1],
        draw_width: values[2],
        draw_height: values[3],
    };
    if let Some([pw, ph]) = pixel_size {
        if pw == 0 || ph == 0 {
            return Err(fail(ErrorKind::Unsupported, tokens[element].name.offset));
        }
        if !aspect.none {
            let sx = g.width / pw as f64;
            let sy = g.height / ph as f64;
            let scale = if aspect.slice { sx.max(sy) } else { sx.min(sy) };
            g.draw_width = pw as f64 * scale;
            g.draw_height = ph as f64 * scale;
            g.draw_x += (g.width - g.draw_width) * aspect.align_x;
            g.draw_y += (g.height - g.draw_height) * aspect.align_y;
        }
    }
    if [
        g.x,
        g.y,
        g.width,
        g.height,
        g.draw_x,
        g.draw_y,
        g.draw_width,
        g.draw_height,
    ]
    .iter()
    .any(|n| !n.is_finite())
    {
        return Err(fail(ErrorKind::Number, tokens[element].name.offset));
    }
    let right = g.x + g.width;
    let bottom = g.y + g.height;
    if [g.x, g.y, right, bottom]
        .iter()
        .any(|v| v.abs() > 1_000_000.)
    {
        return Err(fail(ErrorKind::Number, tokens[element].name.offset));
    }
    Ok(g)
}

/// Call the explicitly supplied decoder and enforce the frozen pixel bounds.
pub fn decode_image(
    provider: &mut dyn MediaProvider,
    encoded: &[u8],
    offset: usize,
    control: &mut dyn WorkControl,
) -> Result<ImagePixels> {
    let pixels = provider
        .decode_image(encoded, control)
        .map_err(|e| media_error(e, offset))?;
    pixels
        .validate(offset)
        .map_err(|_| fail(ErrorKind::Unsupported, offset))?;
    if pixels.width > MAX_IMAGE_DIMENSION || pixels.height > MAX_IMAGE_DIMENSION {
        return Err(fail(ErrorKind::Unsupported, offset));
    }
    let pixel_count = (pixels.width as usize)
        .checked_mul(pixels.height as usize)
        .ok_or_else(|| fail(ErrorKind::Unsupported, offset))?;
    if pixel_count > MAX_TOTAL_IMAGE_PIXELS {
        return Err(fail(ErrorKind::Limit(LimitKind::Images), offset));
    }
    Ok(pixels)
}
/// Rust ownership replaces the old provider release callback.
pub fn release_image(pixels: ImagePixels) {
    drop(pixels);
}

/// Parse one SVG image element and ask the explicit provider to decode inline
/// PNG/JPEG bytes. Zero-area images retain geometry and do not invoke a provider.
pub fn image_element(
    view: vexel_scene::ElementView<'_>,
    viewport: [f64; 2],
    provider: &mut dyn MediaProvider,
    control: &mut dyn WorkControl,
) -> Result<ImagePlan> {
    view.validate(control)?;
    let source = view.source();
    let tokens = view.tokens();
    let element = view.element.0;
    let end = view.end;
    let token = tokens
        .get(element)
        .ok_or_else(|| fail(ErrorKind::InvalidArgument, 0))?;
    let name = token.name;
    let base_geometry = image_geometry(
        source,
        tokens,
        element,
        end,
        viewport,
        None,
        vexel_svg_values::Aspect::default(),
    )?;
    if base_geometry.width == 0. || base_geometry.height == 0. {
        return Ok(ImagePlan {
            geometry: base_geometry,
            pixels: None,
            source: token.source,
        });
    }
    let plain = attribute(source, tokens, element, end, b"href", None);
    let xlink = attribute(source, tokens, element, end, b"href", Some(b"xlink"));
    let href = match (plain, xlink) {
        (Some(_), Some(v)) => return Err(fail(ErrorKind::Unsupported, v.offset)),
        (Some(v), None) => v,
        (None, Some(v)) if xlink_is_bound(source, tokens) => v,
        (None, Some(v)) => return Err(fail(ErrorKind::Unsupported, v.offset)),
        (None, None) => return Err(fail(ErrorKind::Unsupported, name.offset)),
    };
    let payload = parse_data_uri(source, href)?;
    let encoded = decode_base64(source, payload.encoded, control)?;
    let pixels = decode_image(provider, &encoded, payload.encoded.offset, control)?;
    let aspect =
        if let Some(span) = attribute(source, tokens, element, end, b"preserveAspectRatio", None) {
            vexel_svg_values::parse_preserve_aspect(
                source,
                Span::from_offset_length(span.offset, span.length)
                    .ok_or_else(|| fail(ErrorKind::Unsupported, span.offset))?,
            )
            .map_err(|_| fail(ErrorKind::Unsupported, span.offset))?
        } else {
            vexel_svg_values::Aspect {
                set: true,
                ..Default::default()
            }
        };
    let geometry = image_geometry(
        source,
        tokens,
        element,
        end,
        viewport,
        Some([pixels.width, pixels.height]),
        aspect,
    )?;
    Ok(ImagePlan {
        geometry,
        pixels: Some(pixels),
        source: token.source,
    })
}

/// Geometry-only compatibility fixture helper. Production callers must use
/// `publish_image_request` with an explicit common-style publisher.
/// The host annotation is validated before parsing, provider calls, or mutation,
/// then stored verbatim on the appended shape while `transform` maps geometry.
pub fn decode_image_request(
    request: vexel_scene::ImageDecodeRequest<'_, '_>,
    builder: &mut vexel_scene::SceneBuilder,
    draft: &vexel_scene::DraftToken,
    control: &mut dyn WorkControl,
) -> Result<vexel_scene::ShapeId> {
    request.validate_source_map(builder, control)?;
    super::request::atomic_request(builder, draft, control, |builder, control| {
        let viewport = builder.scene(request.scene)?.viewport;
        let plan = image_element(
            request.element,
            [viewport.view_width, viewport.view_height],
            request.provider,
            control,
        )?;
        append_image_plan_mapped(
            plan, request.source_map, request.scene, request.transform,
            builder, draft, control, None,
        )
    })
}

/// Geometry-only compatibility fixture helper; production uses
/// `publish_image_request` with a required style publisher.
/// Append an owned image and its hit-test geometry inside the caller's real
/// scene draft. Builder reserve/accounting leases own all transferred storage.
pub fn append_image_plan(
    plan: ImagePlan,
    source_assignment: XmlSpan,
    scene: vexel_scene::SceneId,
    transform: vexel_geometry::Affine,
    builder: &mut vexel_scene::SceneBuilder,
    draft: &vexel_scene::DraftToken,
    control: &mut dyn WorkControl,
) -> Result<vexel_scene::ShapeId> {
    let element = Span::from_offset_length(source_assignment.offset, source_assignment.length)
        .ok_or_else(|| fail(ErrorKind::InvalidArgument, source_assignment.offset))?;
    super::request::atomic_request(builder, draft, control, |builder, control| {
        append_image_plan_mapped(
            plan,
            vexel_scene::SourceMap { element, transform, transform_node: None },
            scene, transform, builder, draft, control, None,
        )
    })
}

pub(crate) fn append_image_plan_mapped(
    plan: ImagePlan,
    source_map: vexel_scene::SourceMap,
    scene: vexel_scene::SceneId,
    transform: vexel_geometry::Affine,
    builder: &mut vexel_scene::SceneBuilder,
    draft: &vexel_scene::DraftToken,
    control: &mut dyn WorkControl,
    publication: Option<(&super::publication::PublicationContext<'_, '_>, &dyn super::publication::StylePublisher)>,
) -> Result<vexel_scene::ShapeId> {
    let source = source_map.element;
    if source.slice(builder.source_owner().as_slice()).is_none() {
        return Err(fail(ErrorKind::InvalidArgument, source.start));
    }
    let geom = plan.geometry;
    let mut image_index = None;
    if let Some(pixels) = plan.pixels {
        let index = builder.append_image(
            scene,
            vexel_scene::Image {
                pixels,
                x: geom.x,
                y: geom.y,
                width: geom.width,
                height: geom.height,
                draw_x: geom.draw_x,
                draw_y: geom.draw_y,
                draw_width: geom.draw_width,
                draw_height: geom.draw_height,
            },
            control,
        )?;
        image_index = Some(index);
    }
    let points = [
        vexel_geometry::Point {
            x: geom.x,
            y: geom.y,
        },
        vexel_geometry::Point {
            x: geom.x + geom.width,
            y: geom.y,
        },
        vexel_geometry::Point {
            x: geom.x + geom.width,
            y: geom.y + geom.height,
        },
        vexel_geometry::Point {
            x: geom.x,
            y: geom.y + geom.height,
        },
    ];
    let mut edges = Vec::new();
    edges
        .try_reserve_exact(4)
        .map_err(|_| fail(ErrorKind::OutOfMemory, plan.source.offset))?;
    for i in 0..4 {
        let a = vexel_geometry::affine_map_point(transform, points[i])
            .ok_or_else(|| fail(ErrorKind::Number, source.start))?;
        let b = vexel_geometry::affine_map_point(transform, points[(i + 1) % 4])
            .ok_or_else(|| fail(ErrorKind::Number, source.start))?;
        edges.push(vexel_geometry::Edge { a, b });
    }
    let mut subpaths = Vec::new();
    subpaths
        .try_reserve_exact(1)
        .map_err(|_| fail(ErrorKind::OutOfMemory, plan.source.offset))?;
    subpaths.push(vexel_svg_path::PathSubpath {
        first_edge: 0,
        edge_count: 4,
        closed: true,
    });
    let path = vexel_svg_path::PathDocument {
        edges,
        subpaths,
        segments: Vec::new(),
        numbers: Vec::new(),
    };
    if let Some((context, publisher)) = publication {
        return super::publication::publish_checked(publisher, context, super::publication::CustomGeometry {
            path, source_map, image: image_index,
        }, builder, draft, control);
    }
    let mut colors = Vec::new();
    colors
        .try_reserve_exact(4)
        .map_err(|_| fail(ErrorKind::OutOfMemory, plan.source.offset))?;
    colors.resize(4, [0; 4]);
    let mut shape = vexel_scene::Shape::new(source_map);
    shape.image = image_index;
    let id = builder.append_shape(
        scene,
        vexel_scene::ShapeInput {
            shape,
            path,
            edge_colors: colors,
        },
        control,
    )?;
    let published = *builder
        .scene(scene)?
        .shape(id)
        .ok_or_else(|| fail(ErrorKind::InvalidArgument, source.start))?;
    builder.finalize_shape(draft, id, published, control)?;
    Ok(id)
}
fn xlink_is_bound(source: &[u8], tokens: &[XmlToken]) -> bool {
    for (i, token) in tokens.iter().enumerate() {
        if token.kind != TokenKind::StartElement || token.depth != 0 {
            continue;
        }
        for attr in tokens
            .get(i + 1..)
            .unwrap_or_default()
            .iter()
            .take_while(|t| t.kind == TokenKind::Attribute)
        {
            if attr.name.slice(source) == Some(b"xmlns:xlink")
                && attr.value.slice(source) == Some(b"http://www.w3.org/1999/xlink")
            {
                return true;
            }
        }
        return false;
    }
    false
}
fn media_error(error: MediaError, offset: usize) -> Diagnostic {
    let kind = match error {
        MediaError::InvalidArgument
        | MediaError::Unsupported
        | MediaError::Invalid
        | MediaError::InvalidUtf8
        | MediaError::Animated
        | MediaError::Corrupt => ErrorKind::Unsupported,
        MediaError::Limit => ErrorKind::Limit(LimitKind::Images),
        MediaError::OutOfMemory => ErrorKind::OutOfMemory,
    };
    fail(kind, offset)
}
