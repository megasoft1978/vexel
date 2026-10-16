//! Resumable source-backed decoding session.
use crate::style::{parse_opacity, parse_style_color};
use crate::{invalid, map_path_error, root_viewport, scan_ignored};
use vexel_base::{Span, Stop, WorkControl, WorkKind};
use vexel_geometry::{affine_identity, Affine};
use vexel_scene::*;
use vexel_svg_css::{CssPaint, Property};
use vexel_xml::{TokenKind, XmlSpan};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecoderState {
    Running,
    Complete,
    Failed,
}
#[derive(Clone, Copy)]
struct WalkFrame<'src> {
    style: CssPaint<'src>,
    transform: Affine,
    viewport: [f64; 2],
    hidden: bool,
    skip: bool,
}

/// Owned scene transaction over one externally owned canonical lexical document.
pub struct Decoder<'src, 'ctx> {
    shared: DecodeSource<'src>,
    builder: Option<SceneBuilder>,
    draft: Option<DraftToken>,
    frames: [Option<WalkFrame<'src>>; 65],
    cursor: usize,
    root_seen: bool,
    root: SceneId,
    generation: u64,
    state: DecoderState,
    language: &'ctx [u8],
    media: Option<&'ctx mut dyn MediaProvider>,
    switch_skip: Option<(usize, usize)>,
    visited_attributes: Vec<bool>,
}
impl<'src, 'ctx> Decoder<'src, 'ctx> {
    pub fn new(
        document: &'src LexicalDocument,
        generation: u64,
        options: DecodeOptions<'ctx>,
        control: &mut dyn WorkControl,
    ) -> Result<Self> {
        validate_language(options.system_language)?;
        let shared = DecodeSource::new(document, control)?;
        let source = shared.source();
        let tokens = shared.tokens();
        let root_element = tokens
            .iter()
            .position(|t| {
                matches!(t.kind, TokenKind::StartElement | TokenKind::EmptyElement)
                    && t.local_name.slice(source) == Some(b"svg")
            })
            .ok_or(Diagnostic {
                kind: ErrorKind::Viewport,
                offset: 0,
            })?;
        let (viewport, _, _) = root_viewport(source, tokens, root_element)?;
        let animation =
            vexel_svg_animation::parse_owned_controlled(shared.source_owner(), tokens, control)
                .map_err(|e| match e {
                    vexel_base::ControlledError::Original(_) => Diagnostic {
                        kind: ErrorKind::InvalidArgument,
                        offset: 0,
                    },
                    vexel_base::ControlledError::Canceled => stopped(Stop::Canceled),
                    vexel_base::ControlledError::Limit => stopped(Stop::Limit),
                    vexel_base::ControlledError::Overflow => stopped(Stop::Overflow),
                    vexel_base::ControlledError::OutOfMemory => Diagnostic {
                        kind: ErrorKind::OutOfMemory,
                        offset: 0,
                    },
                })?;
        let mut builder = SceneBuilder::new(
            std::sync::Arc::clone(shared.source_owner()),
            animation,
            generation,
            viewport,
            options.limits,
        )?;
        let root = builder.root_id();
        scan_ignored(source, tokens, &mut builder, control)?;
        let draft = builder.begin_draft(root)?;
        let mut visited_attributes = Vec::new();
        visited_attributes
            .try_reserve_exact(tokens.len())
            .map_err(|_| Diagnostic {
                kind: ErrorKind::OutOfMemory,
                offset: 0,
            })?;
        visited_attributes.resize(tokens.len(), false);
        let mut frames = [None; 65];
        frames[0] = Some(WalkFrame {
            style: CssPaint::initial(),
            transform: affine_identity(),
            viewport: [viewport.view_width, viewport.view_height],
            hidden: false,
            skip: false,
        });
        Ok(Self {
            shared,
            builder: Some(builder),
            draft: Some(draft),
            frames,
            cursor: 0,
            root_seen: false,
            root,
            generation,
            state: DecoderState::Running,
            language: options.system_language,
            media: options.media,
            switch_skip: None,
            visited_attributes,
        })
    }
    pub fn state(&self) -> DecoderState {
        self.state
    }
    pub fn generation(&self) -> u64 {
        self.generation
    }
    pub fn builder(&self) -> Option<&SceneBuilder> {
        self.builder.as_ref()
    }
    pub fn has_media_provider(&self) -> bool {
        self.media.is_some()
    }
    pub fn step(
        &mut self,
        work_quantum: u64,
        operations: &mut dyn DecodeOperations,
        control: &mut dyn WorkControl,
    ) -> Result<DecodeEvent<'_>> {
        if self.state == DecoderState::Failed {
            return Err(invalid(0));
        }
        if self.state == DecoderState::Complete {
            return Ok(DecodeEvent::Complete);
        }
        if work_quantum == 0 {
            return Err(invalid(0));
        }
        let total = self.shared.tokens().len();
        let mut charged = 0u64;
        while self.cursor < total && charged < work_quantum {
            if let Some((selected_end, switch_end)) = self.switch_skip {
                if self.cursor > selected_end {
                    self.cursor = switch_end + 1;
                    self.switch_skip = None;
                    continue;
                }
            }
            control.checkpoint().map_err(|s| self.abort(stopped(s)))?;
            control
                .charge(WorkKind::TokenVisit, 1)
                .map_err(|s| self.abort(stopped(s)))?;
            charged += 1;
            let i = self.cursor;
            let token = self.shared.tokens()[i];
            self.cursor += 1;
            match token.kind {
                TokenKind::EndElement => {
                    let slot = token.depth.min(64);
                    self.frames[slot] = None;
                    continue;
                }
                TokenKind::StartElement | TokenKind::EmptyElement => {}
                _ => continue,
            }
            let depth = token.depth;
            if depth > 64 {
                return Err(self.abort(Diagnostic {
                    kind: ErrorKind::Limit(LimitKind::Depth),
                    offset: token.name.offset,
                }));
            }
            let parent = self.frames[depth.saturating_sub(1)].unwrap_or(WalkFrame {
                style: CssPaint::initial(),
                transform: affine_identity(),
                viewport: [1.0, 1.0],
                hidden: false,
                skip: false,
            });
            let element = ElementId(i);
            let style = self
                .shared
                .compute_style(element, Some(&parent.style), control)
                .map_err(|e| self.abort(e))?;
            let transform = crate::transform_node(
                self.shared.source(),
                self.shared.tokens(),
                i,
                parent.transform,
            )
            .map_err(|e| self.abort(e))?;
            let name = token
                .local_name
                .slice(self.shared.source())
                .unwrap_or_default();
            let root_element = name == b"svg" && depth == 0;
            let attrs_supported = {
                let builder = self.builder.as_mut().ok_or_else(|| invalid(0))?;
                crate::traverse_attributes_supported(
                    self.shared.source(),
                    self.shared.tokens(),
                    i,
                    root_element,
                    &mut self.visited_attributes,
                    builder,
                )
            }
            .map_err(|e| self.abort(e))?;
            if !parent.skip && !attrs_supported {
                return Err(self.abort(Diagnostic {
                    kind: ErrorKind::Unsupported,
                    offset: token.name.offset,
                }));
            }
            if let Some(v) = style.value(Property::Display) {
                if ![b"none".as_slice(), b"inline", b"block", b"contents"]
                    .iter()
                    .any(|x| v.bytes.eq_ignore_ascii_case(x))
                {
                    return Err(self.abort(Diagnostic {
                        kind: ErrorKind::Unsupported,
                        offset: v.span.offset,
                    }));
                }
            }
            if let Some(v) = style.value(Property::Visibility) {
                if ![b"visible".as_slice(), b"hidden", b"collapse"]
                    .iter()
                    .any(|x| v.bytes.eq_ignore_ascii_case(x))
                {
                    return Err(self.abort(Diagnostic {
                        kind: ErrorKind::Unsupported,
                        offset: v.span.offset,
                    }));
                }
            }
            let display_none = style.keyword_equals(Property::Display, b"none") || parent.hidden;
            let skip = parent.skip
                || matches!(
                    name,
                    b"defs"
                        | b"symbol"
                        | b"clipPath"
                        | b"mask"
                        | b"pattern"
                        | b"marker"
                        | b"linearGradient"
                        | b"radialGradient"
                        | b"filter"
                        | b"meta"
                        | b"title"
                        | b"desc"
                        | b"metadata"
                );
            let frame = WalkFrame {
                style,
                transform,
                viewport: parent.viewport,
                hidden: display_none,
                skip,
            };
            if token.kind == TokenKind::StartElement {
                self.frames[depth] = Some(frame)
            }
            if name == b"svg" {
                if depth != 0 {
                    return Err(self.abort(Diagnostic {
                        kind: ErrorKind::Unsupported,
                        offset: token.name.offset,
                    }));
                }
                self.root_seen = true;
                continue;
            }
            if token.local_name.slice(self.shared.source()) == Some(b"switch") {
                let end = matching_end(self.shared.tokens(), i).ok_or_else(|| {
                    self.abort(Diagnostic {
                        kind: ErrorKind::Xml(vexel_xml::XmlError::MismatchedTag),
                        offset: token.name.offset,
                    })
                })?;
                let selected = crate::select_switch_child(
                    self.shared.source(),
                    self.shared.tokens(),
                    i,
                    end + 1,
                    self.language,
                )
                .map_err(|e| self.abort(e))?;
                match selected {
                    Some((first, limit)) => {
                        self.cursor = first;
                        self.switch_skip = Some((limit - 1, end));
                    }
                    None => self.cursor = end + 1,
                }
                continue;
            }
            if display_none || skip {
                continue;
            }
            if style.keyword_equals(Property::Visibility, b"hidden")
                || style.keyword_equals(Property::Visibility, b"collapse")
            {
                continue;
            }
            if name == b"path"
                || matches!(
                    name,
                    b"rect" | b"circle" | b"ellipse" | b"line" | b"polyline" | b"polygon"
                )
            {
                self.decode_element(i, token, frame, operations, control)
                    .map_err(|e| self.abort(e))?;
            } else if matches!(
                name,
                b"g" | b"a"
                    | b"defs"
                    | b"title"
                    | b"desc"
                    | b"metadata"
                    | b"style"
                    | b"symbol"
                    | b"clipPath"
                    | b"mask"
                    | b"pattern"
                    | b"marker"
                    | b"linearGradient"
                    | b"radialGradient"
                    | b"filter"
            ) {
            } else if matches!(name, b"text" | b"image" | b"use") {
                return Err(self.abort(Diagnostic {
                    kind: ErrorKind::Unsupported,
                    offset: token.name.offset,
                }));
            } else {
                return Err(self.abort(Diagnostic {
                    kind: ErrorKind::Unsupported,
                    offset: token.name.offset,
                }));
            }
        }
        if self.cursor == total {
            if !self.root_seen {
                return Err(self.abort(Diagnostic {
                    kind: ErrorKind::Viewport,
                    offset: 0,
                }));
            }
            self.state = DecoderState::Complete;
            return Ok(DecodeEvent::Complete);
        }
        Ok(DecodeEvent::Progress {
            charged_units: charged,
        })
    }
    fn decode_element(
        &mut self,
        index: usize,
        token: vexel_xml::XmlToken,
        frame: WalkFrame<'src>,
        _operations: &mut dyn DecodeOperations,
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        let geometry = if token.local_name.slice(self.shared.source()) == Some(b"path") {
            let d = attribute(self.shared.source(), self.shared.tokens(), index, b"d").ok_or(
                Diagnostic {
                    kind: ErrorKind::Path,
                    offset: token.name.offset,
                },
            )?;
            crate::Geometry {
                path: vexel_svg_path::parse_path(
                    self.shared.source(),
                    Span {
                        start: d.offset,
                        end: d.offset + d.length,
                    },
                    frame.transform,
                    vexel_svg_path::ParseOptions {
                        max_commands: 65_536,
                        max_edges: 4 * 1024 * 1024,
                        max_numbers: 28 * 1024 * 1024,
                    },
                )
                .map_err(map_path_error)?,
                filled: true,
                stroke_closed: true,
            }
        } else {
            crate::element_geometry(
                self.shared.source(),
                self.shared.tokens(),
                index,
                frame.transform,
                frame.viewport,
            )?
        };
        if geometry.path.edges.is_empty() {
            return Ok(());
        }
        let source_map = SourceMap {
            element: Span {
                start: token.source.offset,
                end: token.source.offset + token.source.length,
            },
            transform: frame.transform,
            transform_node: None,
        };
        let mut shape = Shape::new(source_map);
        shape.filled = geometry.filled;
        shape.stroke_closed = geometry.stroke_closed;
        let fill = frame.style.value(Property::Fill);
        let stroke = frame.style.value(Property::Stroke);
        let mut rgba = match fill {
            Some(_v) if frame.style.keyword_equals(Property::Fill, b"none") => [0, 0, 0, 0],
            Some(v) => {
                parse_style_color(self.shared.source(), &frame.style, Property::Fill, v.span)?
            }
            None => [0, 0, 0, 255],
        };
        if frame.style.keyword_equals(Property::Fill, b"none") {
            shape.filled = false
        }
        let opacity = parse_opacity(self.shared.source(), frame.style.value(Property::Opacity))?
            * parse_opacity(
                self.shared.source(),
                frame.style.value(Property::FillOpacity),
            )?;
        rgba[3] = ((f64::from(rgba[3]) * opacity) + 0.5).floor() as u8;
        let mut colors = Vec::new();
        colors
            .try_reserve(geometry.path.edges.len())
            .map_err(|_| Diagnostic {
                kind: ErrorKind::OutOfMemory,
                offset: token.name.offset,
            })?;
        colors.resize(geometry.path.edges.len(), rgba);
        if let Some(value) = stroke {
            if !frame.style.keyword_equals(Property::Stroke, b"none") {
                shape.stroke.rgba = parse_style_color(
                    self.shared.source(),
                    &frame.style,
                    Property::Stroke,
                    value.span,
                )?;
                let stroke_opacity =
                    parse_opacity(self.shared.source(), frame.style.value(Property::Opacity))?
                        * parse_opacity(
                            self.shared.source(),
                            frame.style.value(Property::StrokeOpacity),
                        )?;
                shape.stroke.rgba[3] =
                    ((f64::from(shape.stroke.rgba[3]) * stroke_opacity) + 0.5).floor() as u8;
                shape.stroke.width = frame
                    .style
                    .value(Property::StrokeWidth)
                    .map(|v| crate::read_length(self.shared.source(), v.span, frame.viewport[0]))
                    .transpose()?
                    .unwrap_or(1.0);
                if !(0.0..=1024.0).contains(&shape.stroke.width) {
                    return Err(Diagnostic {
                        kind: ErrorKind::Number,
                        offset: frame
                            .style
                            .value(Property::StrokeWidth)
                            .map_or(token.name.offset, |v| v.span.offset),
                    });
                }
                if let Some(v) = frame.style.value(Property::StrokeMiterlimit) {
                    shape.stroke.miter_limit = crate::read_number(self.shared.source(), v.span)?;
                    if !(1.0..=64.0).contains(&shape.stroke.miter_limit) {
                        return Err(Diagnostic {
                            kind: ErrorKind::Number,
                            offset: v.span.offset,
                        });
                    }
                }
                if let Some(v) = frame.style.value(Property::StrokeLinecap) {
                    shape.stroke.cap = if v.bytes.eq_ignore_ascii_case(b"butt") {
                        Linecap::Butt
                    } else if v.bytes.eq_ignore_ascii_case(b"square") {
                        Linecap::Square
                    } else if v.bytes.eq_ignore_ascii_case(b"round") {
                        Linecap::Round
                    } else {
                        return Err(Diagnostic {
                            kind: ErrorKind::Unsupported,
                            offset: v.span.offset,
                        });
                    }
                }
                if let Some(v) = frame.style.value(Property::StrokeLinejoin) {
                    shape.stroke.join = if v.bytes.eq_ignore_ascii_case(b"miter")
                        || v.bytes.eq_ignore_ascii_case(b"miter-clip")
                    {
                        Linejoin::Miter
                    } else if v.bytes.eq_ignore_ascii_case(b"bevel") {
                        Linejoin::Bevel
                    } else if v.bytes.eq_ignore_ascii_case(b"round")
                        || v.bytes.eq_ignore_ascii_case(b"arcs")
                    {
                        Linejoin::Round
                    } else {
                        return Err(Diagnostic {
                            kind: ErrorKind::Unsupported,
                            offset: v.span.offset,
                        });
                    }
                }
                if let Some(v) = frame.style.value(Property::StrokeDashoffset) {
                    shape.stroke.dash_offset =
                        crate::read_length(self.shared.source(), v.span, frame.viewport[0])?;
                    if shape.stroke.dash_offset.abs() > 1e6 {
                        return Err(Diagnostic {
                            kind: ErrorKind::Number,
                            offset: v.span.offset,
                        });
                    }
                }
                if let Some(v) = frame.style.value(Property::StrokeDasharray) {
                    if !v.bytes.eq_ignore_ascii_case(b"none") {
                        return Err(Diagnostic {
                            kind: ErrorKind::Unsupported,
                            offset: v.span.offset,
                        });
                    }
                }
                if let Some(v) = frame.style.value(Property::VectorEffect) {
                    if !v.bytes.eq_ignore_ascii_case(b"none") {
                        return Err(Diagnostic {
                            kind: ErrorKind::Unsupported,
                            offset: v.span.offset,
                        });
                    }
                }
                if let Some(v) = frame.style.value(Property::ShapeRendering) {
                    if v.bytes.eq_ignore_ascii_case(b"crispEdges") {
                        shape.stroke.crisp_edges = true
                    } else if ![b"auto".as_slice(), b"optimizeSpeed", b"geometricPrecision"]
                        .iter()
                        .any(|x| v.bytes.eq_ignore_ascii_case(x))
                    {
                        return Err(Diagnostic {
                            kind: ErrorKind::Unsupported,
                            offset: v.span.offset,
                        });
                    }
                }
                shape.stroke_inverse =
                    crate::transform::affine_inverse(frame.transform).ok_or(Diagnostic {
                        kind: ErrorKind::Number,
                        offset: token.name.offset,
                    })?;
            }
        }
        if let Some(v) = frame.style.value(Property::FillRule) {
            if v.bytes.eq_ignore_ascii_case(b"evenodd") {
                shape.evenodd = true
            } else if !v.bytes.eq_ignore_ascii_case(b"nonzero") {
                return Err(Diagnostic {
                    kind: ErrorKind::Unsupported,
                    offset: v.span.offset,
                });
            }
        }
        for property in [
            Property::ClipPath,
            Property::Filter,
            Property::Mask,
            Property::Marker,
            Property::MarkerStart,
            Property::MarkerMid,
            Property::MarkerEnd,
            Property::MixBlendMode,
            Property::Isolation,
            Property::PaintOrder,
        ] {
            if let Some(v) = frame.style.value(property) {
                if !v.bytes.eq_ignore_ascii_case(b"none")
                    && !(property == Property::PaintOrder
                        && v.bytes.eq_ignore_ascii_case(b"normal"))
                    && !(property == Property::MixBlendMode
                        && v.bytes.eq_ignore_ascii_case(b"normal"))
                    && !(property == Property::Isolation && v.bytes.eq_ignore_ascii_case(b"auto"))
                {
                    return Err(Diagnostic {
                        kind: ErrorKind::Unsupported,
                        offset: v.span.offset,
                    });
                }
            }
        }
        let input = ShapeInput {
            shape,
            path: geometry.path,
            edge_colors: colors,
        };
        self.builder
            .as_mut()
            .ok_or_else(|| invalid(0))?
            .append_shape(self.root, input, control)?;
        Ok(())
    }
    fn abort(&mut self, error: Diagnostic) -> Diagnostic {
        if let (Some(builder), Some(token)) = (self.builder.as_mut(), self.draft.take()) {
            let _ = builder.rollback_draft(token);
        }
        self.state = DecoderState::Failed;
        error
    }
    pub fn resume_pattern(
        &mut self,
        ticket: PatternTicket,
        result: Result<vexel_raster::Tile>,
        _control: &mut dyn WorkControl,
    ) -> Result<()> {
        let _ = ticket;
        drop(result);
        Err(self.abort(invalid(0)))
    }
    pub fn finish(mut self, control: &mut dyn WorkControl) -> Result<SceneGraph> {
        if self.state != DecoderState::Complete {
            return Err(self.abort(invalid(0)));
        }
        let builder = self.builder.as_mut().ok_or_else(|| invalid(0))?;
        if let Some(token) = self.draft.take() {
            builder.commit_draft(&token, control)?;
        }
        self.builder
            .take()
            .ok_or_else(|| invalid(0))?
            .freeze_controlled(control)
    }
}
impl DecodeSession for Decoder<'_, '_> {
    fn step<'s>(
        &'s mut self,
        work_quantum: u64,
        operations: &mut dyn DecodeOperations,
        control: &mut dyn WorkControl,
    ) -> Result<DecodeEvent<'s>> {
        Decoder::step(self, work_quantum, operations, control)
    }
    fn resume_pattern(
        &mut self,
        ticket: PatternTicket,
        result: Result<vexel_raster::Tile>,
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        Decoder::resume_pattern(self, ticket, result, control)
    }
    fn finish(self, control: &mut dyn WorkControl) -> Result<SceneGraph> {
        Decoder::finish(self, control)
    }
}
fn stopped(stop: Stop) -> Diagnostic {
    Diagnostic {
        kind: match stop {
            Stop::Canceled => ErrorKind::Canceled,
            Stop::Limit => ErrorKind::Limit(LimitKind::Work),
            Stop::Overflow => ErrorKind::Limit(LimitKind::Arithmetic),
        },
        offset: 0,
    }
}
fn validate_language(language: &[u8]) -> Result<()> {
    if language.is_empty() || language.len() > 64 {
        return Err(invalid(0));
    }
    for (i, b) in language.iter().copied().enumerate() {
        if !(b.is_ascii_alphanumeric() || b == b'-')
            || b == b'-' && (i == 0 || i + 1 == language.len() || language[i - 1] == b'-')
        {
            return Err(invalid(i));
        }
    }
    Ok(())
}
fn attribute(
    source: &[u8],
    tokens: &[vexel_xml::XmlToken],
    element: usize,
    name: &[u8],
) -> Option<XmlSpan> {
    tokens
        .get(element + 1..)?
        .iter()
        .take_while(|t| t.kind == TokenKind::Attribute)
        .find(|t| t.prefix.length == 0 && t.local_name.slice(source) == Some(name))
        .map(|t| t.value)
}
fn matching_end(tokens: &[vexel_xml::XmlToken], start: usize) -> Option<usize> {
    let t = tokens.get(start)?;
    if t.kind == TokenKind::EmptyElement {
        return Some(start);
    }
    tokens
        .get(start + 1..)?
        .iter()
        .position(|v| v.kind == TokenKind::EndElement && v.depth == t.depth)
        .map(|n| start + 1 + n)
}
