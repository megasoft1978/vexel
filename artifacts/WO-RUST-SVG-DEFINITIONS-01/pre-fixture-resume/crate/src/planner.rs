//! Definition planning against the accepted shared source, style and draft contracts.
use crate::common::{number_list, parse_length, parse_number};
use crate::{attr, matching_end, span_bytes};
use vexel_base::{WorkControl, WorkKind};
use vexel_geometry::{Affine, affine_compose};
use vexel_scene::*;
use vexel_svg_css::{CssPaint, Property};
use vexel_xml::{TokenKind, XmlSpan, XmlToken};

pub struct DefinitionPlanner {
    mask_instances: usize,
    pattern_instances: usize,
    marker_instances: usize,
    filter_count: usize,
    filter_primitives: usize,
    clip_reference_bytes: usize,
}
impl Default for DefinitionPlanner {
    fn default() -> Self {
        Self::new()
    }
}
impl DefinitionPlanner {
    pub fn new() -> Self {
        Self {
            mask_instances: 0,
            pattern_instances: 0,
            marker_instances: 0,
            filter_count: 0,
            filter_primitives: 0,
            clip_reference_bytes: 0,
        }
    }
    pub fn plan<'src, 'ctx>(
        &mut self,
        ctx: &mut DefinitionContext<'src, 'ctx>,
        kind: DefinitionKind,
        element: ElementId,
        owner: ShapeId,
        transform: Affine,
        bounds: [f64; 4],
        _style: &CssPaint<'src>,
        control: &mut dyn WorkControl,
    ) -> Result<DefinitionPlan<'src>> {
        ctx.validate()?;
        let source = ctx.shared.source();
        let tokens = ctx.shared.tokens();
        let tok = tokens.get(element.0).ok_or_else(|| bad(0))?;
        if !matches!(tok.kind, TokenKind::StartElement | TokenKind::EmptyElement) {
            return Err(bad(tok.source.offset));
        }
        if !kind_matches(source, tok.local_name, kind) {
            return Err(unsupported(tok.name.offset));
        }
        let end = matching_end(tokens, element.0)?;
        let active = match kind {
            DefinitionKind::Clip => None,
            DefinitionKind::Mask => Some(&ctx.active.masks),
            DefinitionKind::Pattern => Some(&ctx.active.patterns),
            DefinitionKind::Marker => Some(&ctx.active.markers),
            DefinitionKind::Filter => None,
        };
        if active.is_some_and(|a| a.contains(&element)) {
            return Err(unsupported(tok.name.offset));
        }
        let mut expansions = Vec::new();
        expansions
            .try_reserve(8)
            .map_err(|_| oom(tok.name.offset))?;
        let mut next_mask = self.mask_instances;
        let mut next_pattern = self.pattern_instances;
        let mut next_marker = self.marker_instances;
        let mut next_filters = self.filter_count;
        let mut next_filter_primitives = self.filter_primitives;
        let mut next_clip_bytes = self.clip_reference_bytes;
        let def_style = definition_style(ctx, element.0, control)?;
        let descriptor;
        let post = PostOperation::None;
        match kind {
            DefinitionKind::Filter => {
                if next_filters >= crate::filter::FILTER_MAX_COUNT {
                    return Err(crate::limit(LimitKind::Filters, tok.name.offset));
                }
                let parsed = crate::filter::parse_filter_at(source, tokens, element.0)?;
                next_filter_primitives = next_filter_primitives
                    .checked_add(parsed.primitives.len())
                    .ok_or_else(|| number_at(tok.name.offset))?;
                if next_filter_primitives > crate::filter::FILTER_MAX_TOTAL_PRIMITIVES {
                    return Err(crate::limit(LimitKind::Filters, tok.name.offset));
                }
                next_filters += 1;
                descriptor = DefinitionDescriptor::Filter {
                    filter: parsed.filter,
                    primitives: parsed.primitives,
                };
            }
            DefinitionKind::Clip => {
                if !supported_clip_attrs(source, tokens, element.0) {
                    return Err(unsupported(tok.name.offset));
                }
                let units = attr(source, tokens, element.0, b"clipPathUnits");
                let object_bbox = match units.map(|s| span_bytes(source, s).unwrap_or_default()) {
                    None | Some(b"userSpaceOnUse") => false,
                    Some(b"objectBoundingBox") => true,
                    _ => return Err(unsupported(units.unwrap().offset)),
                };
                // Clip rules inherit, while all other computed properties on
                // clip children remain unsupported by the original contract.
                let style = ctx.shared.compute_style(element, None, control)?;
                check_clip_style(&style, tok.name.offset)?;
                descriptor = DefinitionDescriptor::Clip(Clip {
                    geometry: ClipGeometry {
                        edges: Vec::new(),
                        shapes: Vec::new(),
                    },
                    parent: None,
                    owner_node: None,
                    owner_before_suffix: false,
                    object_bounding_box: object_bbox,
                    bounds,
                    user_inverse: transform,
                });
                let child_start = tok
                    .source
                    .offset
                    .checked_add(tok.source.length)
                    .ok_or_else(|| number_at(tok.name.offset))?;
                let child_end = if end == element.0 {
                    child_start
                } else {
                    tokens[end].source.offset
                };
                let child_bytes = child_end
                    .checked_sub(child_start)
                    .ok_or_else(|| bad(tok.name.offset))?;
                if child_bytes > crate::clip::MAX_CLIP_REFERENCE_BYTES {
                    return Err(crate::limit(LimitKind::References, tok.name.offset));
                }
                let child_count = tokens
                    .get(element.0 + 1..end)
                    .unwrap_or_default()
                    .iter()
                    .filter(|x| {
                        matches!(x.kind, TokenKind::StartElement | TokenKind::EmptyElement)
                            && x.depth == tok.depth + 1
                    })
                    .count();
                if child_count == 0 {
                    return Err(unsupported(tok.name.offset));
                }
                if child_count > crate::clip::MAX_CLIP_CHILDREN {
                    return Err(crate::limit(LimitKind::Nodes, tok.name.offset));
                }
                next_clip_bytes = next_clip_bytes
                    .checked_add(child_bytes)
                    .ok_or_else(|| number_at(tok.name.offset))?;
                let job = next_clip_bytes
                    .checked_add(crate::clip::CLIP_WRAPPER_BUFFER_BYTES + 6)
                    .ok_or_else(|| number_at(tok.name.offset))?;
                if crate::clip::CLIP_WRAPPER_BUFFER_BYTES > crate::clip::CLIP_WRAPPER_LIMIT_BYTES
                    || job >= crate::clip::MAX_CLIP_JOB_REFERENCE_BYTES
                {
                    return Err(crate::limit(LimitKind::References, tok.name.offset));
                }
                add_expansions(
                    ctx,
                    element.0,
                    end,
                    DecodeFlavor::Clip { clip_rule: true },
                    transform,
                    &style,
                    &mut expansions,
                    control,
                )?;
            }
            DefinitionKind::Mask => {
                if next_mask >= 1024 {
                    return Err(crate::limit(LimitKind::Masks, tok.name.offset));
                }
                next_mask += 1;
                let units = unit_flag(source, tokens, element.0, b"maskUnits", true)?;
                let content_units =
                    unit_flag(source, tokens, element.0, b"maskContentUnits", false)?;
                let mut region = [-0.1, -0.1, 1.2, 1.2];
                let mut custom = false;
                for (i, key) in [b"x".as_slice(), b"y", b"width", b"height"]
                    .iter()
                    .enumerate()
                {
                    if let Some(s) = attr(source, tokens, element.0, key) {
                        custom = true;
                        region[i] = mask_region(
                            source,
                            s,
                            units,
                            if i % 2 == 0 {
                                ctx.builder.scene(ctx.scene)?.viewport.width as f64
                            } else {
                                ctx.builder.scene(ctx.scene)?.viewport.height as f64
                            },
                        )?;
                    }
                    if i >= 2 && region[i] <= 0. {
                        return Err(number_at(
                            attr(source, tokens, element.0, key)
                                .map_or(tok.name.offset, |x| x.offset),
                        ));
                    }
                }
                let region_transform = if units {
                    let box_map = Affine {
                        a: bounds[2] - bounds[0],
                        b: 0.,
                        c: 0.,
                        d: bounds[3] - bounds[1],
                        e: bounds[0],
                        f: bounds[1],
                    };
                    affine_compose(transform, box_map).ok_or_else(|| number_at(tok.name.offset))?
                } else {
                    transform
                };
                let style = def_style;
                let mask_type = style.value(Property::MaskType);
                let luminance = match mask_type.map(|v| v.bytes) {
                    None | Some(b"luminance") => true,
                    Some(b"alpha") => false,
                    _ => return Err(unsupported(mask_type.unwrap().span.offset)),
                };
                descriptor = DefinitionDescriptor::Mask(MaskDescriptor {
                    luminance,
                    region_object_bounding_box: units,
                    custom_region: custom,
                    region,
                    region_transform,
                    source_offset: tok.name.offset,
                });
                add_expansions(
                    ctx,
                    element.0,
                    end,
                    DecodeFlavor::Mask,
                    if content_units {
                        affine_compose(
                            transform,
                            Affine {
                                a: bounds[2] - bounds[0],
                                b: 0.,
                                c: 0.,
                                d: bounds[3] - bounds[1],
                                e: bounds[0],
                                f: bounds[1],
                            },
                        )
                        .ok_or_else(|| number_at(tok.name.offset))?
                    } else {
                        transform
                    },
                    &def_style,
                    &mut expansions,
                    control,
                )?;
            }
            DefinitionKind::Pattern => {
                if next_pattern >= 1024 {
                    return Err(crate::limit(LimitKind::Patterns, tok.name.offset));
                }
                next_pattern += 1;
                if !supported_pattern_attrs(source, tokens, element.0) {
                    return Err(unsupported(tok.name.offset));
                }
                let pat = pattern_descriptor(source, tokens, element.0, bounds, transform)?;
                descriptor = DefinitionDescriptor::Pattern(pat);
                add_expansions(
                    ctx,
                    element.0,
                    end,
                    DecodeFlavor::Pattern,
                    pat.content_transform,
                    &def_style,
                    &mut expansions,
                    control,
                )?;
            }
            DefinitionKind::Marker => {
                if next_marker >= 1024 {
                    return Err(crate::limit(LimitKind::Shapes, tok.name.offset));
                }
                next_marker += 1;
                let parsed = crate::parse_marker_attributes(source, tokens, element.0)?;
                let marker = MarkerDescriptor {
                    definition: element,
                    viewport: parsed.child_viewport,
                    transform: parsed.content,
                    error_offset: tok.name.offset,
                };
                descriptor = DefinitionDescriptor::Marker(marker);
                if def_style
                    .value(Property::ClipPath)
                    .is_some_and(|v| !v.bytes.eq_ignore_ascii_case(b"none"))
                {
                    return Err(unsupported(
                        def_style.value(Property::ClipPath).unwrap().span.offset,
                    ));
                }
                add_expansions(
                    ctx,
                    element.0,
                    end,
                    DecodeFlavor::Marker,
                    marker.transform,
                    &def_style,
                    &mut expansions,
                    control,
                )?;
            }
        }
        let _ = owner;
        self.mask_instances = next_mask;
        self.pattern_instances = next_pattern;
        self.marker_instances = next_marker;
        self.filter_count = next_filters;
        self.filter_primitives = next_filter_primitives;
        self.clip_reference_bytes = next_clip_bytes;
        Ok(DefinitionPlan {
            descriptor,
            expansions,
            post,
        })
    }
}
fn definition_style<'src>(
    ctx: &mut DefinitionContext<'src, '_>,
    element: usize,
    control: &mut dyn WorkControl,
) -> Result<CssPaint<'src>> {
    let tokens = ctx.shared.tokens().to_vec();
    let depth = tokens.get(element).ok_or_else(|| bad(0))?.depth;
    let mut ancestors = Vec::new();
    ancestors
        .try_reserve(16)
        .map_err(|_| oom(tokens[element].name.offset))?;
    for (i, t) in tokens.iter().enumerate().take(element) {
        if matches!(t.kind, TokenKind::StartElement | TokenKind::EmptyElement) && t.depth < depth {
            if matching_end(&tokens, i).is_ok_and(|e| e >= element) {
                ancestors.push((t.depth, i));
            }
        }
    }
    ancestors.sort_by_key(|v| v.0);
    let mut parent = None;
    for (_, i) in ancestors {
        parent = Some(
            ctx.shared
                .compute_style(ElementId(i), parent.as_ref(), control)?,
        );
    }
    let target = ctx
        .shared
        .compute_style(ElementId(element), parent.as_ref(), control)?;
    Ok(target)
}
fn add_expansions<'src>(
    ctx: &mut DefinitionContext<'src, '_>,
    root: usize,
    end: usize,
    flavor: DecodeFlavor,
    base_transform: Affine,
    parent: &CssPaint<'src>,
    out: &mut Vec<Expansion<'src>>,
    control: &mut dyn WorkControl,
) -> Result<()> {
    let tokens = ctx.shared.tokens().to_vec();
    let mut i = root + 1;
    while i < end {
        let t = &tokens[i];
        if !matches!(t.kind, TokenKind::StartElement | TokenKind::EmptyElement)
            || t.depth != tokens[root].depth + 1
        {
            i += 1;
            continue;
        }
        control.charge(WorkKind::TokenVisit, 1).map_err(stopped)?;
        let child_end = matching_end(&tokens, i)?;
        let style = ctx
            .shared
            .compute_style(ElementId(i), Some(parent), control)?;
        if matches!(flavor, DecodeFlavor::Clip { .. }) {
            check_clip_style(&style, t.name.offset)?;
        }
        out.try_reserve(1).map_err(|_| oom(t.name.offset))?;
        out.push(Expansion {
            range: TableRange {
                first: i,
                count: child_end - i + 1,
            },
            transform: base_transform,
            viewport: [
                ctx.builder.scene(ctx.scene)?.viewport.width as f64,
                ctx.builder.scene(ctx.scene)?.viewport.height as f64,
            ],
            style,
            flavor,
            post: PostOperation::None,
        });
        i = child_end + 1;
    }
    Ok(())
}
fn check_clip_style(style: &CssPaint<'_>, _offset: usize) -> Result<()> {
    for p in Property::ALL {
        if p != Property::ClipRule && style.value(p).is_some() {
            return Err(unsupported(style.value(p).unwrap().span.offset));
        }
    }
    if let Some(v) = style.value(Property::ClipRule) {
        if !v.bytes.eq_ignore_ascii_case(b"nonzero") && !v.bytes.eq_ignore_ascii_case(b"evenodd") {
            return Err(unsupported(v.span.offset));
        }
    }
    Ok(())
}
fn kind_matches(src: &[u8], span: XmlSpan, kind: DefinitionKind) -> bool {
    let v = span_bytes(src, span).unwrap_or_default();
    let expected: &[u8] = match kind {
        DefinitionKind::Clip => b"clipPath",
        DefinitionKind::Mask => b"mask",
        DefinitionKind::Pattern => b"pattern",
        DefinitionKind::Marker => b"marker",
        DefinitionKind::Filter => b"filter",
    };
    v == expected
}
fn unit_flag(
    src: &[u8],
    tokens: &[XmlToken],
    element: usize,
    name: &[u8],
    default: bool,
) -> Result<bool> {
    let Some(s) = attr(src, tokens, element, name) else {
        return Ok(default);
    };
    match span_bytes(src, s).unwrap_or_default() {
        b"objectBoundingBox" => Ok(true),
        b"userSpaceOnUse" => Ok(false),
        _ => Err(unsupported(s.offset)),
    }
}
fn mask_region(src: &[u8], span: XmlSpan, object_bbox: bool, reference: f64) -> Result<f64> {
    if object_bbox {
        let b = span_bytes(src, span).ok_or_else(|| bad(span.offset))?;
        let pct = b.last() == Some(&b'%');
        let s = if pct {
            XmlSpan {
                offset: span.offset,
                length: span.length - 1,
            }
        } else {
            span
        };
        let n = parse_number(src, s, 0.)?;
        Ok(if pct { n / 100. } else { n })
    } else {
        let b = span_bytes(src, span).ok_or_else(|| bad(span.offset))?;
        if b.last() == Some(&b'%') {
            return Ok(parse_number(
                src,
                XmlSpan {
                    offset: span.offset,
                    length: span.length - 1,
                },
                0.,
            )? * reference
                / 100.);
        }
        parse_length(src, span, reference)
    }
}
fn pattern_descriptor(
    src: &[u8],
    tokens: &[XmlToken],
    element: usize,
    bounds: [f64; 4],
    transform: Affine,
) -> Result<PatternDescriptor> {
    let object = unit_flag(src, tokens, element, b"patternUnits", true)?;
    let content_object = unit_flag(src, tokens, element, b"patternContentUnits", false)?;
    let viewport = [bounds[2].abs().max(1.), bounds[3].abs().max(1.)];
    let mut v = [0.; 4];
    for (i, key) in [b"x".as_slice(), b"y", b"width", b"height"]
        .iter()
        .enumerate()
    {
        if let Some(s) = attr(src, tokens, element, key) {
            v[i] = if object {
                mask_region(src, s, true, 1.)?
            } else {
                let reference = if i % 2 == 0 { viewport[0] } else { viewport[1] };
                parse_length(src, s, reference)?
            };
        }
    }
    if v[2] <= 0. || v[3] <= 0. {
        return Err(number_at(
            attr(src, tokens, element, b"width")
                .or_else(|| attr(src, tokens, element, b"height"))
                .map_or(tokens[element].name.offset, |s| s.offset),
        ));
    }
    let tile_width = if object { v[2] * viewport[0] } else { v[2] };
    let tile_height = if object { v[3] * viewport[1] } else { v[3] };
    let width = tile_width.ceil();
    let height = tile_height.ceil();
    if !width.is_finite()
        || !height.is_finite()
        || width < 1.
        || height < 1.
        || width > 4096.
        || height > 4096.
        || width * height > 1_048_576.
    {
        return Err(crate::limit(
            LimitKind::Patterns,
            tokens[element].name.offset,
        ));
    }
    let mut content = transform;
    if let Some(s) = attr(src, tokens, element, b"patternTransform") {
        content =
            affine_compose(content, parse_transform(src, s)?).ok_or_else(|| number_at(s.offset))?;
    }
    if let Some(s) = attr(src, tokens, element, b"viewBox") {
        let vb = number_list(src, s, 4).map_err(|_| Diagnostic {
            kind: ErrorKind::Viewport,
            offset: s.offset,
        })?;
        if vb.len() != 4 || vb[2] <= 0. || vb[3] <= 0. {
            return Err(Diagnostic {
                kind: ErrorKind::Viewport,
                offset: s.offset,
            });
        }
        let aspect = if let Some(a) = attr(src, tokens, element, b"preserveAspectRatio") {
            vexel_svg_values::parse_preserve_aspect(
                src,
                vexel_base::Span {
                    start: a.offset,
                    end: a.offset + a.length,
                },
            )
            .map_err(|e| unsupported(e.offset))?
        } else {
            vexel_svg_values::Aspect {
                align_x: 0.5,
                align_y: 0.5,
                none: false,
                slice: false,
                set: true,
            }
        };
        let map = vexel_svg_values::viewbox_transform(
            vb[0],
            vb[1],
            vb[2],
            vb[3],
            0.,
            0.,
            tile_width,
            tile_height,
            aspect,
        )
        .map_err(|_| Diagnostic {
            kind: ErrorKind::Viewport,
            offset: s.offset,
        })?;
        content = affine_compose(content, map).ok_or_else(|| number_at(s.offset))?;
    } else if content_object {
        let map = Affine {
            a: bounds[2] - bounds[0],
            b: 0.,
            c: 0.,
            d: bounds[3] - bounds[1],
            e: bounds[0],
            f: bounds[1],
        };
        content =
            affine_compose(content, map).ok_or_else(|| number_at(tokens[element].name.offset))?;
    }
    let shift = Affine {
        a: 1.,
        b: 0.,
        c: 0.,
        d: 1.,
        e: -v[0],
        f: -v[1],
    };
    content =
        affine_compose(content, shift).ok_or_else(|| number_at(tokens[element].name.offset))?;
    Ok(PatternDescriptor {
        definition: ElementId(element),
        width: width as u32,
        height: height as u32,
        tile_width,
        tile_height,
        content_transform: content,
        error_offset: tokens[element].name.offset,
    })
}
fn parse_transform(src: &[u8], s: XmlSpan) -> Result<Affine> {
    let end = s
        .offset
        .checked_add(s.length)
        .ok_or_else(|| bad(s.offset))?;
    let (m, _) = vexel_svg_values::parse_transform(
        src,
        vexel_base::Span {
            start: s.offset,
            end,
        },
    )
    .map_err(|e| unsupported(e.offset))?;
    Ok(m)
}
fn supported_pattern_attrs(src: &[u8], tokens: &[XmlToken], element: usize) -> bool {
    let allowed: &[&[u8]] = &[
        b"id",
        b"class",
        b"style",
        b"x",
        b"y",
        b"width",
        b"height",
        b"patternUnits",
        b"patternContentUnits",
        b"patternTransform",
        b"viewBox",
        b"preserveAspectRatio",
    ];
    tokens
        .get(element + 1..)
        .unwrap_or(&[])
        .iter()
        .take_while(|t| t.kind == TokenKind::Attribute)
        .all(|a| {
            a.prefix.length == 0
                && allowed.contains(&span_bytes(src, a.local_name).unwrap_or_default())
        })
}
fn supported_clip_attrs(src: &[u8], tokens: &[XmlToken], element: usize) -> bool {
    let allowed: [&[u8]; 4] = [b"id", b"class", b"clipPathUnits", b"style"];
    tokens
        .get(element + 1..)
        .unwrap_or(&[])
        .iter()
        .take_while(|t| t.kind == TokenKind::Attribute)
        .all(|a| {
            a.prefix.length == 0
                && allowed.contains(&span_bytes(src, a.local_name).unwrap_or_default())
        })
}
fn bad(o: usize) -> Diagnostic {
    Diagnostic {
        kind: ErrorKind::InvalidArgument,
        offset: o,
    }
}
fn unsupported(o: usize) -> Diagnostic {
    Diagnostic {
        kind: ErrorKind::Unsupported,
        offset: o,
    }
}
fn number_at(o: usize) -> Diagnostic {
    Diagnostic {
        kind: ErrorKind::Number,
        offset: o,
    }
}
fn oom(o: usize) -> Diagnostic {
    Diagnostic {
        kind: ErrorKind::OutOfMemory,
        offset: o,
    }
}
fn stopped(s: vexel_base::Stop) -> Diagnostic {
    Diagnostic {
        kind: match s {
            vexel_base::Stop::Canceled => ErrorKind::Canceled,
            vexel_base::Stop::Limit => ErrorKind::Limit(LimitKind::Work),
            vexel_base::Stop::Overflow => ErrorKind::Limit(LimitKind::Arithmetic),
        },
        offset: 0,
    }
}
