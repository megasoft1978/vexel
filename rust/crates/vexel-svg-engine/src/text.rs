//! `<text>`/`<tspan>` decoding for the production engine.
//!
//! Positioning is a port of the legacy C `svg_shape_text_render_element`
//! (`src/svg/shape/text.c`): start position from `x`/`y` lists or the running
//! cursor, `dx[0]`/`dy[0]` folded into the start exactly as C does, anchor from
//! the advance, per-glyph `x/y/dx/dy` lists and word spacing in
//! `text_path_geometry_at`. Layout comes from an injected
//! [`TextLayoutProvider`]; without one the engine keeps returning `Unsupported`.
//! Solid fill and solid stroke (with dashes) are published; paint servers and
//! clip paths on text stay typed `Unsupported`, as does `textPath`.
//!
//! A `<tspan>` takes at most one value per x/y/dx/dy; absolute x/y start a new
//! chunk. An element's direct text publishes in its own
//! call, before its child elements (text after a child keeps the legacy
//! placement the parity goldens encode).

use vexel_base::{WorkControl, WorkKind};
use vexel_scene::*;
use vexel_svg_media as media;
use vexel_xml::TokenKind;

/// Layout source for text. Implemented over the platform text layout by the
/// application wiring; the engine itself never depends on a platform crate.
pub trait TextLayoutProvider {
    fn layout_text(
        &mut self,
        request: TextRequest<'_>,
        control: &mut dyn WorkControl,
    ) -> std::result::Result<TextLayout, MediaError>;
}

/// Adapts the engine provider to the scene `MediaProvider` used by the shared
/// layout cache. A provider panic becomes a typed error, never an unwind.
struct Adapter<'a>(&'a mut dyn TextLayoutProvider);
impl MediaProvider for Adapter<'_> {
    fn layout_text(
        &mut self,
        request: TextRequest<'_>,
        control: &mut dyn WorkControl,
    ) -> std::result::Result<TextLayout, MediaError> {
        let provider = &mut *self.0;
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            provider.layout_text(request, control)
        }))
        .unwrap_or(Err(MediaError::Invalid))
    }
    fn decode_image(
        &mut self,
        _: &[u8],
        _: &mut dyn WorkControl,
    ) -> std::result::Result<ImagePixels, MediaError> {
        Err(MediaError::Unsupported)
    }
}

fn fail(kind: ErrorKind, offset: usize) -> Diagnostic {
    Diagnostic { kind, offset }
}
fn stopped(stop: vexel_base::Stop) -> Diagnostic {
    fail(
        match stop {
            vexel_base::Stop::Canceled => ErrorKind::Canceled,
            vexel_base::Stop::Limit => ErrorKind::Limit(LimitKind::Work),
            vexel_base::Stop::Overflow => ErrorKind::Limit(LimitKind::Arithmetic),
        },
        0,
    )
}

const SCALE: f64 = 96.0 / 72.0;
const MAX_DOCUMENT_TEXT_BYTES: usize = 64 * 1024 * 1024;
const MAX_DOCUMENT_GLYPHS: usize = 1_048_576;

pub(crate) fn decode(
    provider: &mut dyn TextLayoutProvider,
    request: TextDecodeRequest<'_, '_>,
    builder: &mut SceneBuilder,
    draft: &DraftToken,
    control: &mut dyn WorkControl,
) -> Result<bool> {
    let view = request.element;
    let (source, tokens, element, end) =
        (view.source(), view.tokens(), view.element.0, request.end);
    let own = *tokens
        .get(element)
        .ok_or_else(|| fail(ErrorKind::InvalidArgument, 0))?;
    let preserve = media::parse_xml_space(source, tokens, element, end, request.preserve_space)?;
    let viewport = builder
        .traversal_scene(request.scene)?
        .viewport
        .ok_or_else(|| fail(ErrorKind::InvalidArgument, own.name.offset))?;
    let view_size = [viewport.view_width, viewport.view_height];
    let values = media::parse_text_style(source, request.style, view_size)?;
    let style = vexel_svg_decode::parse_style(
        source,
        request.style,
        view_size[0].hypot(view_size[1]) / 2.0f64.sqrt(),
        request.transform,
        viewport,
        control,
    )?;
    // Paint servers and clip paths fail closed.
    if let Some(span) = style.fill_server.or(style.stroke_server).or(style.clip) {
        return Err(fail(ErrorKind::Unsupported, span.offset));
    }
    if style.dash_count > 16 {
        return Err(fail(ErrorKind::InvalidArgument, own.name.offset));
    }
    let families: Vec<&[u8]> = values.families.iter().map(Vec::as_slice).collect();
    let is_tspan = own.local_name.slice(source) == Some(b"tspan");
    let list = |name: &[u8], reference: f64| -> Result<Vec<f64>> {
        match media::text_attribute(source, tokens, element, end, name, None) {
            Some(span) => media::parse_position_list(source, span, reference),
            None => Ok(Vec::new()),
        }
    };
    let is_text_node = |token: &vexel_xml::XmlToken| {
        matches!(token.kind, TokenKind::Text | TokenKind::Cdata) && token.depth == own.depth + 1
    };
    let mut nodes = Vec::new();
    {
        // Scan for textPath, validate tspan lists and collect the direct text.
        let inner = tokens
            .get(element + 1..end)
            .ok_or_else(|| fail(ErrorKind::InvalidArgument, 0))?;
        for (offset, token) in inner.iter().enumerate() {
            control.charge(WorkKind::TokenVisit, 1).map_err(stopped)?;
            let child = matches!(
                token.kind,
                TokenKind::StartElement | TokenKind::EmptyElement
            );
            if token.depth > own.depth
                && child
                && token.local_name.slice(source) == Some(b"textPath")
            {
                return Err(fail(ErrorKind::Unsupported, token.name.offset));
            }
            if is_text_node(token) {
                nodes.push(element + 1 + offset);
            }
        }
        if is_tspan {
            reject_lists(
                source,
                tokens,
                element,
                end,
                &[
                    list(b"x", view_size[0])?,
                    list(b"y", view_size[1])?,
                    list(b"dx", view_size[0])?,
                    list(b"dy", view_size[1])?,
                ],
            )?;
        }
    }
    let mut adapter = Adapter(provider);
    for (n, &node) in nodes.iter().enumerate() {
        let token = &tokens[node];
        control.charge(WorkKind::TokenVisit, 1).map_err(stopped)?;
        // Text keeps its lists for every run (legacy); a tspan's first run takes them.
        let first_run = !is_tspan || n == 0;
        let offset = token.value.offset;
        if token.value.length > media::MAX_TEXT_BYTES
            || token.value.length > MAX_DOCUMENT_TEXT_BYTES.saturating_sub(request.state.text_bytes)
        {
            return Err(fail(ErrorKind::Limit(LimitKind::TextBytes), offset));
        }
        request.state.text_bytes += token.value.length;
        if token.kind == TokenKind::Cdata {
            // Literal CDATA needs a non-decoding normaliser that the media crate
            // does not export; fail closed rather than mis-decode entities.
            return Err(fail(ErrorKind::Unsupported, offset));
        }
        let normalized = media::normalize_text(source, token.value, preserve, request.state)?;
        if normalized.is_empty() {
            continue;
        }
        let text_request = TextRequest {
            text: &normalized,
            font_families: &families,
            size_points: values.size_points,
            weight: values.weight,
            style: values.style,
            letter_spacing_points: values.letter_spacing_points,
        };
        let output = media::layout_shared_cache(
            request.cache,
            text_request,
            &values.family_spans,
            &mut adapter,
            control,
            offset,
        )?;
        let layout = output.layout();
        if layout.glyphs.len() > MAX_DOCUMENT_GLYPHS.saturating_sub(request.state.glyph_count) {
            return Err(fail(ErrorKind::Limit(LimitKind::Glyphs), offset));
        }
        builder.admit_text_commands(draft, layout.commands.len(), offset, control)?;
        request.state.glyph_count += layout.glyphs.len();
        let (x, y, dx, dy) = if first_run {
            (
                list(b"x", view_size[0])?,
                list(b"y", view_size[1])?,
                list(b"dx", view_size[0])?,
                list(b"dy", view_size[1])?,
            )
        } else {
            Default::default()
        };
        let mut start_x = x.first().copied().unwrap_or(request.state.cursor_x);
        let mut start_y = y.first().copied().unwrap_or(request.state.cursor_y);
        if is_tspan {
            reject_lists(
                source,
                tokens,
                element,
                end,
                &[x.clone(), y.clone(), dx.clone(), dy.clone()],
            )?;
        }
        // C folds dx[0]/dy[0] into the start (and again per glyph); keep that,
        // tspans included (the parity goldens encode it).
        start_x += dx.first().copied().unwrap_or(0.0);
        start_y += dy.first().copied().unwrap_or(0.0);
        let advance = layout.advance_width * SCALE;
        let anchor = match values.anchor {
            TextAnchor::Start => 0.0,
            TextAnchor::Middle => -advance * 0.5,
            TextAnchor::End => -advance,
        };
        let codepoints = normalized.iter().filter(|b| **b & 0xc0 != 0x80).count();
        let spaces = normalized.iter().filter(|b| **b == b' ').count();
        if values.word_spacing != 0.0 && codepoints != layout.glyphs.len() {
            return Err(fail(ErrorKind::Unsupported, offset));
        }
        // C adds dx[i] then lets an x[i] override reset it; the media helper resets
        // first. Zeroing the shadowed deltas makes the helper match C exactly.
        let shadow = |deltas: &[f64], overrides: usize| -> Vec<f64> {
            deltas
                .iter()
                .enumerate()
                .map(|(i, d)| if i < overrides { 0.0 } else { *d })
                .collect()
        };
        let (dx_eff, dy_eff) = (shadow(&dx, x.len()), shadow(&dy, y.len()));
        let geometry_word_spacing =
            c_word_spacing(&normalized, layout.glyphs.len(), values.word_spacing)
                .ok_or_else(|| fail(ErrorKind::Unsupported, offset))?;
        let visible = style.visible && !style.display_none;
        if visible && layout.glyphs.iter().any(|g| g.commands.count != 0) {
            let path = media::text_path_geometry_at(
                layout,
                &normalized,
                &x,
                &y,
                &dx_eff,
                &dy_eff,
                [start_x, start_y],
                anchor,
                geometry_word_spacing,
                request.transform,
                offset,
                control,
            )?;
            // Neither fill nor stroke paints anything: publish no shape.
            let paints = style.fill[3] != 0 || style.stroke.rgba[3] != 0;
            if !path.edges.is_empty() && paints {
                let inverse = vexel_svg_decode::affine_inverse(request.transform);
                if inverse.is_none() && style.stroke.rgba[3] != 0 {
                    return Err(fail(ErrorKind::Unsupported, own.name.offset));
                }
                let mut shape = Shape::new(request.source_map);
                shape.filled = style.fill[3] != 0;
                shape.evenodd = style.evenodd;
                // Glyph contours are closed, as for the image outline.
                shape.stroke_closed = true;
                shape.stroke = style.stroke;
                if let Some(inverse) = inverse {
                    shape.stroke_inverse = inverse;
                }
                if style.dash_count != 0 {
                    shape.stroke.dash_values = builder.append_dashes(
                        request.scene,
                        &style.dash_values[..style.dash_count],
                        control,
                    )?;
                }
                let mut colors = Vec::new();
                colors
                    .try_reserve_exact(path.edges.len())
                    .map_err(|_| fail(ErrorKind::OutOfMemory, offset))?;
                colors.resize(path.edges.len(), style.fill);
                builder.append_shape(
                    request.scene,
                    ShapeInput {
                        shape,
                        path,
                        edge_colors: colors,
                    },
                    control,
                )?;
            }
        }
        let next_x = start_x + advance + spaces as f64 * values.word_spacing;
        if !next_x.is_finite() || !start_y.is_finite() {
            return Err(fail(ErrorKind::Number, offset));
        }
        request.state.cursor_x = next_x;
        request.state.cursor_y = start_y;
        request.state.has_position = true;
    }
    Ok(preserve)
}

/// A tspan takes one value per x/y/dx/dy: fail at the first longer list.
fn reject_lists(
    source: &[u8],
    tokens: &[vexel_xml::XmlToken],
    element: usize,
    end: usize,
    lists: &[Vec<f64>; 4],
) -> Result<()> {
    for (name, values) in [b"x".as_slice(), b"y", b"dx", b"dy"].into_iter().zip(lists) {
        if values.len() > 1 {
            let span = media::text_attribute(source, tokens, element, end, name, None);
            return Err(fail(
                ErrorKind::Unsupported,
                span.map_or(tokens[element].name.offset, |span| span.offset),
            ));
        }
    }
    Ok(())
}

/// The C `text_geometry` advances its byte cursor with `while ((b & 0xC0) !=
/// 0x80)`, which runs past a whole ASCII tail after the first glyph, so word
/// spacing only ever moves glyphs when the scan lands on a space at a glyph
/// boundary (in practice never for ASCII). The media helper applies spacing at
/// every space, so reproduce the C decision: return the spacing to hand the
/// helper (0.0 when C applies none), or `None` when the C pattern is neither
/// "no increments" nor "every space" (rare non-ASCII shapes), which fails closed.
fn c_word_spacing(text: &[u8], glyphs: usize, spacing: f64) -> Option<f64> {
    if spacing == 0.0 {
        return Some(0.0);
    }
    let (mut at, mut c_incs, mut space_incs) = (0usize, Vec::new(), Vec::new());
    let mut scalars = text
        .iter()
        .enumerate()
        .filter(|(_, b)| **b & 0xc0 != 0x80)
        .map(|(_, b)| *b);
    for _ in 0..glyphs {
        while at < text.len() && text[at] & 0xc0 == 0x80 {
            at += 1;
        }
        c_incs.push(at < text.len() && text[at] == b' ');
        while at < text.len() && text[at] & 0xc0 != 0x80 {
            at += 1;
        }
        space_incs.push(scalars.next() == Some(b' '));
    }
    if c_incs.iter().all(|i| !i) {
        Some(0.0)
    } else if c_incs == space_incs {
        Some(spacing)
    } else {
        None
    }
}
