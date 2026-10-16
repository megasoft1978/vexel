use super::text::{
    MAX_TEXT_BYTES, fail, normalize_text, parse_position_list, parse_text_style, parse_xml_space,
    text_attribute,
};
use vexel_base::WorkControl;
use vexel_scene::{Diagnostic, ErrorKind, LimitKind, Result, TextLayout, TextRequest, TextState};
use vexel_svg_css::CssPaint;
use vexel_xml::XmlToken;

/// Convert provider glyph outlines into the owned flattened path used by the
/// shared scene API. This follows the original 96/72 scale, baseline sign,
/// x/y overrides, accumulated dx/dy, anchor, and word-spacing rules.
pub fn text_geometry(
    layout: &TextLayout,
    text: &[u8],
    x: &[f64],
    y: &[f64],
    dx: &[f64],
    dy: &[f64],
    start: [f64; 2],
    anchor_offset: f64,
    word_spacing: f64,
    transform: vexel_geometry::Affine,
    control: &mut dyn WorkControl,
) -> Result<Vec<vexel_geometry::Edge>> {
    Ok(text_path_geometry(
        layout, text, x, y, dx, dy, start, anchor_offset, word_spacing, transform, control,
    )?
    .edges)
}

/// Preserve one closed subpath per provider contour, including contours closed
/// implicitly by a subsequent Move or the end of a glyph.
pub fn text_path_geometry(
    layout: &TextLayout,
    text: &[u8],
    x: &[f64],
    y: &[f64],
    dx: &[f64],
    dy: &[f64],
    start: [f64; 2],
    anchor_offset: f64,
    word_spacing: f64,
    transform: vexel_geometry::Affine,
    control: &mut dyn WorkControl,
) -> Result<vexel_svg_path::PathDocument> {
    let mut edges = Vec::new();
    let mut subpaths = Vec::new();
    let mut codepoint = 0;
    let mut extra_x = 0.;
    let mut extra_y = 0.;
    let mut word_offset = 0.;
    let scale = 96. / 72.;
    for (glyph_index, glyph) in layout.glyphs.iter().enumerate() {
        control.checkpoint().map_err(stopped)?;
        if glyph_index < dx.len() {
            extra_x += dx[glyph_index];
        }
        if glyph_index < dy.len() {
            extra_y += dy[glyph_index];
        }
        let x_override = glyph_index < x.len();
        let y_override = glyph_index < y.len();
        if x_override {
            extra_x = 0.;
        }
        if y_override {
            extra_y = 0.;
        }
        let gx = (if x_override {
            x[glyph_index]
        } else {
            start[0] + glyph.x * scale
        }) + extra_x
            + anchor_offset
            + word_offset;
        let gy = (if y_override {
            y[glyph_index]
        } else {
            start[1] - glyph.y * scale
        }) + extra_y;
        while codepoint < text.len() && text[codepoint] & 0xc0 == 0x80 {
            codepoint += 1;
        }
        if codepoint < text.len() && text[codepoint] == b' ' {
            word_offset += word_spacing;
        }
        while codepoint < text.len() && text[codepoint] & 0xc0 != 0x80 {
            codepoint += 1;
        }
        let range_end = glyph
            .commands
            .first
            .checked_add(glyph.commands.count)
            .filter(|&end| end <= layout.commands.len())
            .ok_or_else(|| fail(ErrorKind::Unsupported, 0))?;
        let mut current = vexel_geometry::Point { x: 0., y: 0. };
        let mut first = current;
        let mut open = false;
        let mut contour_first = edges.len();
        for command in &layout.commands[glyph.commands.first..range_end] {
            control
                .charge(vexel_base::WorkKind::Media, 1)
                .map_err(stopped)?;
            let point = |i: usize| vexel_geometry::Point {
                x: gx + command.values[i] * scale,
                y: gy - command.values[i + 1] * scale,
            };
            match command.verb {
                vexel_scene::TextVerb::Move => {
                    if open {
                        close_contour(current, first, contour_first, transform, &mut edges, &mut subpaths, control)?;
                    }
                    contour_first = edges.len();
                    current = point(0);
                    first = current;
                    open = true;
                }
                vexel_scene::TextVerb::Line => {
                    let next = point(0);
                    if !open {
                        contour_first = edges.len();
                        first = current;
                        open = true;
                    }
                    append_text_edge(current, next, transform, &mut edges, control)?;
                    current = next;
                }
                vexel_scene::TextVerb::Cubic => {
                    let c1 = point(0);
                    let c2 = point(2);
                    let next = point(4);
                    if !open {
                        contour_first = edges.len();
                        first = current;
                        open = true;
                    }
                    vexel_svg_path::flatten_cubic_controlled(
                        [current, c1, c2, next],
                        transform,
                        vexel_svg_path::ParseOptions::default(),
                        0,
                        &mut edges,
                        control,
                    )
                    .map_err(path_control_error)?;
                    current = next;
                }
                vexel_scene::TextVerb::Close => {
                    if open {
                        close_contour(current, first, contour_first, transform, &mut edges, &mut subpaths, control)?;
                    }
                    current = first;
                    open = false;
                }
            }
        }
        if open {
            close_contour(current, first, contour_first, transform, &mut edges, &mut subpaths, control)?;
        }
    }
    Ok(vexel_svg_path::PathDocument {
        edges,
        subpaths,
        segments: Vec::new(),
        numbers: Vec::new(),
    })
}

fn close_contour(
    current: vexel_geometry::Point,
    first: vexel_geometry::Point,
    first_edge: usize,
    transform: vexel_geometry::Affine,
    edges: &mut Vec<vexel_geometry::Edge>,
    subpaths: &mut Vec<vexel_svg_path::PathSubpath>,
    control: &mut dyn WorkControl,
) -> Result<()> {
    append_text_edge(current, first, transform, edges, control)?;
    subpaths.try_reserve(1).map_err(|_| fail(ErrorKind::OutOfMemory, 0))?;
    subpaths.push(vexel_svg_path::PathSubpath {
        first_edge,
        edge_count: edges.len() - first_edge,
        closed: true,
    });
    Ok(())
}

/// Geometry-only compatibility fixture helper. Production callers must use
/// `publish_text_request` with an explicit common-style publisher.
/// Decode direct text/CDATA children of a text or tspan element. Each authored
/// run creates one owned scene shape, with source identity fixed at append time.
/// A failed request or commit restores the scene suffix, document command
/// budget, external text state and newly admitted cache entries. Earlier scene
/// handles and cache layouts retain their identity.
pub fn render_text_request(
    request: vexel_scene::TextDecodeRequest<'_, '_>,
    builder: &mut vexel_scene::SceneBuilder,
    draft: &vexel_scene::DraftToken,
    control: &mut dyn WorkControl,
) -> Result<bool> {
    render_text_request_impl(request, builder, draft, control, None)
}

pub(crate) fn render_text_request_impl(
    request: vexel_scene::TextDecodeRequest<'_, '_>,
    builder: &mut vexel_scene::SceneBuilder,
    draft: &vexel_scene::DraftToken,
    control: &mut dyn WorkControl,
    publisher: Option<&dyn super::publication::StylePublisher>,
) -> Result<bool> {
    request.validate_source_map(builder, control)?;
    let context = super::publication::PublicationContext {
        element: request.element, end: request.end, style: request.style, source_map: request.source_map,
        transform: request.transform, scene: request.scene,
        kind: super::publication::MediaKind::Text,
    };
    let state_before = *request.state;
    let cache_before = (request.cache.entries.len(), request.cache.text_bytes, request.cache.commands);
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
    super::request::atomic_request(builder, draft, control, |builder, control, token| {
        let first = builder.scene(request.scene)?.shapes.len();
        if let Some(publisher) = publisher {
            if super::publication::display_none(&context)? {
                return Ok(request.preserve_space);
            }
            publisher.begin(&context, builder, draft, control)?;
        }
        let preserve = render_text_element(
            request.element, request.end, request.scene, request.source_map,
            request.style, request.transform, &mut *request.state,
            request.preserve_space, request.provider, &mut *request.cache,
            builder, draft, token, control, publisher.map(|p| (&context, p)),
        )?;
        if let Some(publisher) = publisher {
            publisher.finish(&context, first, builder, draft, control)?;
        }
        Ok(preserve)
    })
    }));
    if !matches!(&outcome, Ok(Ok(_))) {
        *request.state = state_before;
        request.cache.entries.truncate(cache_before.0);
        request.cache.text_bytes = cache_before.1;
        request.cache.commands = cache_before.2;
    }
    match outcome {
        Ok(result) => result,
        Err(panic) => std::panic::resume_unwind(panic),
    }
}

fn render_text_element(
    view: vexel_scene::ElementView<'_>,
    end: usize,
    scene: vexel_scene::SceneId,
    source_map: vexel_scene::SourceMap,
    style: &CssPaint<'_>,
    transform: vexel_geometry::Affine,
    state: &mut TextState,
    parent_preserve_space: bool,
    provider: &mut dyn vexel_scene::MediaProvider,
    cache: &mut vexel_scene::TextCache,
    builder: &mut vexel_scene::SceneBuilder,
    draft: &vexel_scene::DraftToken,
    request_token: vexel_scene::RequestToken,
    control: &mut dyn WorkControl,
    publication: Option<(&super::publication::PublicationContext<'_, '_>, &dyn super::publication::StylePublisher)>,
) -> Result<bool> {
    view.validate(control)?;
    let source = view.source();
    let tokens = view.tokens();
    let element = view.element.0;
    if source_map.element.slice(source).is_none() {
        return Err(fail(ErrorKind::InvalidArgument, source_map.element.start));
    }
    let preserve = parse_xml_space(source, tokens, element, end, parent_preserve_space)?;
    let viewport = builder.scene(scene)?.viewport;
    let viewport_size = [viewport.view_width, viewport.view_height];
    let values = parse_text_style(source, style, viewport_size)?;
    let mut family_refs: Vec<&[u8]> = Vec::new();
    family_refs
        .try_reserve_exact(values.families.len())
        .map_err(|_| fail(ErrorKind::OutOfMemory, tokens[element].name.offset))?;
    family_refs.extend(values.families.iter().map(Vec::as_slice));
    for token in tokens
        .get(element + 1..end)
        .ok_or_else(|| fail(ErrorKind::InvalidArgument, 0))?
    {
        control
            .charge(vexel_base::WorkKind::TokenVisit, 1)
            .map_err(stopped)?;
        if token.depth > tokens[element].depth
            && matches!(
                token.kind,
                vexel_xml::TokenKind::StartElement | vexel_xml::TokenKind::EmptyElement
            )
            && token.local_name.slice(source) == Some(b"textPath")
        {
            return Err(fail(ErrorKind::Unsupported, token.name.offset));
        }
    }
    for token in tokens
        .get(element + 1..end)
        .ok_or_else(|| fail(ErrorKind::InvalidArgument, 0))?
    {
        control
            .charge(vexel_base::WorkKind::TokenVisit, 1)
            .map_err(stopped)?;
        if !matches!(
            token.kind,
            vexel_xml::TokenKind::Text | vexel_xml::TokenKind::Cdata
        ) || token.depth != tokens[element].depth + 1
        {
            continue;
        }
        if token.value.length > MAX_TEXT_BYTES
            || state.text_bytes > 64 * 1024 * 1024
            || token.value.length > (64 * 1024 * 1024usize).saturating_sub(state.text_bytes)
        {
            return Err(fail(
                ErrorKind::Limit(LimitKind::TextBytes),
                token.value.offset,
            ));
        }
        state.text_bytes = state
            .text_bytes
            .checked_add(token.value.length)
            .ok_or_else(|| fail(ErrorKind::Limit(LimitKind::Arithmetic), token.value.offset))?;
        let normalized = normalize_text(source, token.value, preserve, state)?;
        if normalized.is_empty() {
            continue;
        }
        let request = TextRequest {
            text: &normalized,
            font_families: &family_refs,
            size_points: values.size_points,
            weight: values.weight,
            style: values.style,
            letter_spacing_points: values.letter_spacing_points,
        };
        let output = super::text_cache::layout_shared(
            cache,
            request,
            &values.family_spans,
            provider,
            control,
            token.value.offset,
        )?;
        let layout = output.layout();
        if state.glyph_count > 1_048_576usize
            || layout.glyphs.len() > 1_048_576usize - state.glyph_count
        {
            return Err(fail(
                ErrorKind::Limit(LimitKind::Glyphs),
                token.value.offset,
            ));
        }
        builder.admit_text_commands(draft, layout.commands.len(), token.value.offset, control)?;
        state.glyph_count = state
            .glyph_count
            .checked_add(layout.glyphs.len())
            .ok_or_else(|| fail(ErrorKind::Limit(LimitKind::Arithmetic), token.value.offset))?;
        let x = parse_positions_if_present(source, tokens, element, end, b"x", viewport_size[0])?;
        let y = parse_positions_if_present(source, tokens, element, end, b"y", viewport_size[1])?;
        let dx = parse_positions_if_present(source, tokens, element, end, b"dx", viewport_size[0])?;
        let dy = parse_positions_if_present(source, tokens, element, end, b"dy", viewport_size[1])?;
        let start_x =
            x.first().copied().unwrap_or(state.cursor_x) + dx.first().copied().unwrap_or(0.);
        let start_y =
            y.first().copied().unwrap_or(state.cursor_y) + dy.first().copied().unwrap_or(0.);
        let codepoints = normalized.iter().filter(|b| **b & 0xc0 != 0x80).count();
        let spaces = normalized.iter().filter(|b| **b == b' ').count();
        if values.word_spacing != 0. && codepoints != layout.glyphs.len() {
            return Err(fail(ErrorKind::Unsupported, token.value.offset));
        }
        let anchor_offset = match values.anchor {
            vexel_scene::TextAnchor::Start => 0.,
            vexel_scene::TextAnchor::Middle => -layout.advance_width * (96. / 72.) * 0.5,
            vexel_scene::TextAnchor::End => -layout.advance_width * (96. / 72.),
        };
        // The original text helper performs layout/admission/position checks
        // before custom geometry examines visibility. Hidden runs still advance
        // cursor and logical use counters; no-outline runs never check visibility.
        let hidden = publication.is_some_and(|(context, _)|
            super::publication::visibility_hidden(context));
        if layout.glyphs.iter().any(|g| g.commands.count != 0) && !hidden {
            let admission = if let Some((context, publisher)) = publication {
                super::publication::validate_visibility(context)?;
                super::publication::prepare_admission(publisher, context, builder, draft, request_token, control)?
            } else {
                builder.admit_shape(draft, request_token, tokens[element].name.offset, control)?
            };
            let path = text_path_geometry(
                layout,
                &normalized,
                &x,
                &y,
                &dx,
                &dy,
                [start_x, start_y],
                anchor_offset,
                values.word_spacing,
                transform,
                control,
            )?;
            if !path.edges.is_empty() {
                if let Some((context, publisher)) = publication {
                    super::publication::publish_checked(publisher, context, super::publication::CustomGeometry {
                        path, source_map, image: None, admission,
                    }, builder, draft, control)?;
                } else {
                    let mut colors = Vec::new();
                    colors
                        .try_reserve_exact(path.edges.len())
                        .map_err(|_| fail(ErrorKind::OutOfMemory, token.value.offset))?;
                    colors.resize(path.edges.len(), [0; 4]);
                    let mut shape = vexel_scene::Shape::new(source_map);
                    shape.filled = true;
                    let id = builder.append_admitted_shape(
                        draft, &admission,
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
                        .ok_or_else(|| fail(ErrorKind::InvalidArgument, source_map.element.start))?;
                    builder.finalize_shape(draft, id, published, control)?;
                }
            } else {
                builder.cancel_shape_admission(draft, &admission)?;
            }
        }
        state.cursor_x =
            start_x + layout.advance_width * (96. / 72.) + spaces as f64 * values.word_spacing;
        state.cursor_y = start_y;
        state.has_position = true;
    }
    Ok(preserve)
}

fn parse_positions_if_present(
    source: &[u8],
    tokens: &[XmlToken],
    element: usize,
    end: usize,
    name: &[u8],
    reference: f64,
) -> Result<Vec<f64>> {
    match text_attribute(source, tokens, element, end, name, None) {
        Some(span) => parse_position_list(source, span, reference),
        None => Ok(Vec::new()),
    }
}

fn append_text_edge(
    from: vexel_geometry::Point,
    to: vexel_geometry::Point,
    transform: vexel_geometry::Affine,
    edges: &mut Vec<vexel_geometry::Edge>,
    control: &mut dyn WorkControl,
) -> Result<()> {
    vexel_svg_path::append_edge(
        from,
        to,
        transform,
        vexel_svg_path::MAX_PATH_EDGES,
        0,
        edges,
    )
    .map_err(path_error)?;
    control
        .charge(vexel_base::WorkKind::Edge, 1)
        .map_err(stopped)
}
fn stopped(stop: vexel_base::Stop) -> Diagnostic {
    Diagnostic {
        kind: match stop {
            vexel_base::Stop::Canceled => ErrorKind::Canceled,
            vexel_base::Stop::Limit => ErrorKind::Limit(LimitKind::Work),
            vexel_base::Stop::Overflow => ErrorKind::Limit(LimitKind::Arithmetic),
        },
        offset: 0,
    }
}
fn path_error(error: vexel_svg_path::Diagnostic) -> Diagnostic {
    Diagnostic {
        kind: match error.kind {
            vexel_svg_path::DiagnosticKind::Limit => ErrorKind::Limit(LimitKind::PathCommands),
            vexel_svg_path::DiagnosticKind::OutOfMemory => ErrorKind::OutOfMemory,
            vexel_svg_path::DiagnosticKind::Number
            | vexel_svg_path::DiagnosticKind::InvalidTransform => ErrorKind::Number,
            _ => ErrorKind::Unsupported,
        },
        offset: error.offset,
    }
}
fn path_control_error(
    error: vexel_base::ControlledError<vexel_svg_path::Diagnostic>,
) -> Diagnostic {
    match error {
        vexel_base::ControlledError::Original(error) => path_error(error),
        vexel_base::ControlledError::Canceled => stopped(vexel_base::Stop::Canceled),
        vexel_base::ControlledError::Limit => stopped(vexel_base::Stop::Limit),
        vexel_base::ControlledError::Overflow => stopped(vexel_base::Stop::Overflow),
        vexel_base::ControlledError::OutOfMemory => Diagnostic {
            kind: ErrorKind::OutOfMemory,
            offset: 0,
        },
    }
}
