//! Definition planning against the accepted shared source, style and draft contracts.
use crate::planner_support::{
    add_expansions, bad, check_clip_style, clip_wrapper_prefix_len, definition_style, kind_matches,
    mask_region, number_at, oom, pattern_descriptor, stopped, unit_flag, unsupported,
    unsupported_attr, validate_clip_styles,
};
use crate::{attr, matching_end_controlled, span_bytes};
use vexel_base::{WorkControl, WorkKind};
use vexel_geometry::{Affine, affine_compose, affine_valid};
use vexel_scene::*;
use vexel_svg_css::{CssPaint, Property};
use vexel_xml::TokenKind;

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
        if ctx.builder.scene(ctx.scene)?.shape(owner).is_none() {
            return Err(bad(tok.name.offset));
        }
        if !affine_valid(transform)
            || bounds.iter().any(|v| !v.is_finite())
            || bounds[0] > bounds[2]
            || bounds[1] > bounds[3]
        {
            return Err(number_at(tok.name.offset));
        }
        let end = matching_end_controlled(tokens, element.0, control)?;
        for _ in element.0 + 1..end {
            control.charge(WorkKind::TokenVisit, 1).map_err(stopped)?;
        }
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
                let parsed =
                    crate::filter::parse_filter_at_controlled(source, tokens, element.0, control)?;
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
                if tok.prefix.length != 0 {
                    return Err(unsupported(tok.name.offset));
                }
                if let Some(offset) = unsupported_attr(
                    source,
                    tokens,
                    element.0,
                    &[b"id", b"class", b"clipPathUnits"],
                ) {
                    return Err(unsupported(offset));
                }
                let style = ctx.shared.compute_style(element, None, control)?;
                check_clip_style(&style, tok.name.offset)?;
                validate_clip_styles(ctx, element.0, end, &style, control)?;
                let units = attr(source, tokens, element.0, b"clipPathUnits");
                let object_bbox = match units.map(|s| span_bytes(source, s).unwrap_or_default()) {
                    None | Some(b"userSpaceOnUse") => false,
                    Some(b"objectBoundingBox") => true,
                    _ => return Err(unsupported(units.unwrap().offset)),
                };
                if object_bbox && (bounds[2] <= bounds[0] || bounds[3] <= bounds[1]) {
                    return Err(unsupported(tok.name.offset));
                }
                // Clip rules inherit, while all other computed properties on
                // clip children remain unsupported by the original contract.
                let owner_shape = ctx
                    .builder
                    .scene(ctx.scene)?
                    .shape(owner)
                    .copied()
                    .ok_or_else(|| bad(tok.name.offset))?;
                let user_inverse = crate::clip::inverse_affine(transform)
                    .ok_or_else(|| unsupported(tok.name.offset))?;
                descriptor = DefinitionDescriptor::Clip(Clip {
                    geometry: ClipGeometry {
                        edges: Vec::new(),
                        shapes: Vec::new(),
                    },
                    parent: None,
                    owner_node: owner_shape.source.transform_node,
                    owner_before_suffix: false,
                    object_bounding_box: object_bbox,
                    bounds,
                    user_inverse,
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
                let viewport = ctx.builder.scene(ctx.scene)?.viewport;
                let prefix = clip_wrapper_prefix_len(viewport, object_bbox)
                    .ok_or_else(|| crate::limit(LimitKind::References, tok.name.offset))?;
                let total = prefix
                    .checked_add(child_bytes)
                    .and_then(|n| n.checked_add(b"</svg>".len()))
                    .ok_or_else(|| crate::limit(LimitKind::References, tok.name.offset))?;
                if prefix >= crate::clip::CLIP_WRAPPER_BUFFER_BYTES
                    || total >= crate::clip::MAX_CLIP_JOB_REFERENCE_BYTES
                {
                    return Err(crate::limit(LimitKind::References, tok.name.offset));
                }
                let allocation = total
                    .checked_add(1)
                    .ok_or_else(|| crate::limit(LimitKind::References, tok.name.offset))?;
                next_clip_bytes = next_clip_bytes
                    .checked_add(allocation)
                    .filter(|n| *n <= crate::clip::MAX_CLIP_JOB_REFERENCE_BYTES)
                    .ok_or_else(|| crate::limit(LimitKind::References, tok.name.offset))?;
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
                if bounds[2] <= bounds[0] || bounds[3] <= bounds[1] {
                    return Err(unsupported(tok.name.offset));
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
                let mask_type = def_style.value(Property::MaskType);
                let luminance = match mask_type {
                    None => true,
                    Some(value) if value.bytes.eq_ignore_ascii_case(b"luminance") => true,
                    Some(value) if value.bytes.eq_ignore_ascii_case(b"alpha") => false,
                    Some(value) => return Err(unsupported(value.span.offset)),
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
                if let Some(offset) = unsupported_attr(
                    source,
                    tokens,
                    element.0,
                    &[
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
                    ],
                ) {
                    return Err(unsupported(offset));
                }
                let viewport = ctx.builder.scene(ctx.scene)?.viewport;
                let pat = pattern_descriptor(
                    source, tokens, element.0, bounds, transform, viewport, control,
                )?;
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
                let viewport = ctx.builder.scene(ctx.scene)?.viewport;
                let parsed = crate::parse_marker_attributes_for_viewport(
                    source,
                    tokens,
                    element.0,
                    [viewport.width as f64, viewport.height as f64],
                )?;
                let marker = MarkerDescriptor {
                    definition: element,
                    viewport: parsed.child_viewport,
                    transform: parsed.content,
                    error_offset: tok.name.offset,
                };
                descriptor = DefinitionDescriptor::Marker(marker);
                let marker_style = def_style.clone().without_inherited_markers();
                add_expansions(
                    ctx,
                    element.0,
                    end,
                    DecodeFlavor::Marker,
                    marker.transform,
                    &marker_style,
                    &mut expansions,
                    control,
                )?;
            }
        }
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
