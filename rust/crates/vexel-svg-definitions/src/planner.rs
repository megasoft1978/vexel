//! Definition planning against the accepted shared source, style and draft contracts.
use crate::planner_support::{
    add_expansions, bad, check_clip_style, clip_wrapper_prefix_len, definition_style, kind_matches,
    mask_region, number_at, oom, pattern_channel_opacity, pattern_compatibility_opacity,
    pattern_descriptor, stopped, unit_flag, unsupported, unsupported_attr, validate_clip_styles,
};
use crate::{attr, matching_end_controlled, span_bytes};
use vexel_base::{WorkControl, WorkKind};
use vexel_geometry::{Affine, affine_compose, affine_valid};
use vexel_scene::*;
use vexel_svg_css::{CssPaint, Property};
use vexel_xml::TokenKind;

const PLAN_MAX_LEASES: usize = 64;
pub(crate) struct AllocationJournal {
    leases: [Option<AllocationLease>; PLAN_MAX_LEASES],
    len: usize,
}
impl AllocationJournal {
    fn new() -> Self {
        Self {
            leases: [None; PLAN_MAX_LEASES],
            len: 0,
        }
    }
    pub(crate) fn admit_vec<T>(
        &mut self,
        ledger: &mut ChargeLedger,
        values: &Vec<T>,
    ) -> Result<()> {
        if self.len == PLAN_MAX_LEASES {
            return Err(crate::limit(LimitKind::References, 0));
        }
        let lease = ledger.admit_vec(AllocationTag::ReferenceIndex, values)?;
        self.push(lease)
    }
    fn admit_bytes(&mut self, ledger: &mut ChargeLedger, bytes: usize) -> Result<()> {
        if self.len == PLAN_MAX_LEASES {
            return Err(crate::limit(LimitKind::References, 0));
        }
        let lease = ledger.admit(AllocationTag::ReferenceIndex, bytes)?;
        self.push(lease)
    }
    fn push(&mut self, lease: AllocationLease) -> Result<()> {
        if self.len == PLAN_MAX_LEASES {
            return Err(crate::limit(LimitKind::References, 0));
        }
        self.leases[self.len] = Some(lease);
        self.len += 1;
        Ok(())
    }
    pub(crate) fn rollback(&mut self, ledger: &mut ChargeLedger) {
        while self.len > 0 {
            self.len -= 1;
            if let Some(lease) = self.leases[self.len].take() {
                let _ = ledger.release(lease);
            }
        }
    }
    fn commit(&mut self) {
        self.leases[..self.len].fill(None);
        self.len = 0;
    }
}

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
        let mut charges = AllocationJournal::new();
        match self.plan_inner(
            ctx,
            kind,
            element,
            owner,
            transform,
            bounds,
            control,
            &mut charges,
        ) {
            Ok((plan, _)) => {
                let retained = match plan_capacity_bytes(&plan) {
                    Ok(retained) => retained,
                    Err(error) => {
                        charges.rollback(ctx.ledger);
                        return Err(error);
                    }
                };
                if let Err(error) = charges.admit_bytes(ctx.ledger, retained) {
                    charges.rollback(ctx.ledger);
                    return Err(error);
                }
                charges.commit();
                Ok(plan)
            }
            Err(error) => {
                charges.rollback(ctx.ledger);
                Err(error)
            }
        }
    }

    fn plan_inner<'src, 'ctx>(
        &mut self,
        ctx: &mut DefinitionContext<'src, 'ctx>,
        kind: DefinitionKind,
        element: ElementId,
        owner: ShapeId,
        transform: Affine,
        bounds: [f64; 4],
        control: &mut dyn WorkControl,
        charges: &mut AllocationJournal,
    ) -> Result<(DefinitionPlan<'src>, Option<crate::PatternMapping>)> {
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
        if ctx
            .builder
            .traversal_scene(ctx.scene)?
            .shape(owner)
            .is_none()
        {
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
        let mut pattern_mapping = None;
        let def_style = definition_style(ctx, element.0, control, charges)?;
        let descriptor;
        let post = PostOperation::None;
        match kind {
            DefinitionKind::Filter => {
                if next_filters >= crate::filter::FILTER_MAX_COUNT {
                    return Err(crate::limit(LimitKind::Filters, tok.name.offset));
                }
                let parsed = crate::filter::parse_filter_at_controlled_metered(
                    source, tokens, element.0, control, ctx.ledger, charges,
                )?;
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
                validate_clip_styles(ctx, element.0, end, &style, control, charges)?;
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
                    .traversal_scene(ctx.scene)?
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
                let viewport = ctx
                    .builder
                    .traversal_scene(ctx.scene)?
                    .viewport
                    .ok_or_else(|| bad(tok.name.offset))?;
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
                let viewport = ctx
                    .builder
                    .traversal_scene(ctx.scene)?
                    .viewport
                    .ok_or_else(|| bad(tok.name.offset))?;
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
                                viewport.width as f64
                            } else {
                                viewport.height as f64
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
                let viewport = ctx
                    .builder
                    .traversal_scene(ctx.scene)?
                    .viewport
                    .ok_or_else(|| bad(tok.name.offset))?;
                let mapping = crate::parse_pattern_mapping(
                    source, tokens, element.0, bounds, viewport, control,
                )?;
                let pat = pattern_descriptor(
                    source, tokens, mapping, bounds, transform, viewport, charges, ctx.ledger,
                )?;
                pattern_mapping = Some(mapping);
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
                let viewport = ctx
                    .builder
                    .traversal_scene(ctx.scene)?
                    .viewport
                    .ok_or_else(|| bad(tok.name.offset))?;
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
        Ok((
            DefinitionPlan {
                descriptor,
                expansions,
                post,
            },
            pattern_mapping,
        ))
    }
}

impl vexel_scene::DefinitionPlanner for DefinitionPlanner {
    fn plan_owned_definition<'src>(
        &mut self,
        ctx: &mut DefinitionContext<'src, '_>,
        kind: DefinitionKind,
        element: ElementId,
        owner: DefinitionOwner,
        transform: Affine,
        bounds: [f64; 4],
        style: &CssPaint<'src>,
        control: &mut dyn WorkControl,
    ) -> Result<ProductionDefinitionPlan<'src>> {
        self.plan_owned_production(
            ctx, kind, element, owner, None, transform, bounds, style, control,
        )
    }
}

impl vexel_scene::PaintChannelDefinitionPlanner for DefinitionPlanner {
    fn plan_owned_pattern_definition<'src>(
        &mut self,
        ctx: &mut DefinitionContext<'src, '_>,
        element: ElementId,
        owner: DefinitionOwner,
        channel: PaintChannel,
        transform: Affine,
        bounds: [f64; 4],
        style: &CssPaint<'src>,
        control: &mut dyn WorkControl,
    ) -> Result<ProductionDefinitionPlan<'src>> {
        self.plan_owned_production(
            ctx,
            DefinitionKind::Pattern,
            element,
            owner,
            Some(channel),
            transform,
            bounds,
            style,
            control,
        )
    }
}

impl DefinitionPlanner {
    fn plan_owned_production<'src>(
        &mut self,
        ctx: &mut DefinitionContext<'src, '_>,
        kind: DefinitionKind,
        element: ElementId,
        owner: DefinitionOwner,
        channel: Option<PaintChannel>,
        transform: Affine,
        bounds: [f64; 4],
        style: &CssPaint<'src>,
        control: &mut dyn WorkControl,
    ) -> Result<ProductionDefinitionPlan<'src>> {
        owner.validate(ctx)?;
        if channel.is_some() && kind != DefinitionKind::Pattern {
            return Err(bad(0));
        }
        let counters = (
            self.mask_instances,
            self.pattern_instances,
            self.marker_instances,
            self.filter_count,
            self.filter_primitives,
            self.clip_reference_bytes,
        );
        let mut charges = AllocationJournal::new();
        let result = (|| -> Result<ProductionDefinitionPlan<'src>> {
            Ok(match owner.target() {
                DefinitionTarget::Shape(_) if kind == DefinitionKind::Filter => {
                    return Err(unsupported(0));
                }
                DefinitionTarget::Shape(shape) => {
                    let (plan, mapping) = self.plan_inner(
                        ctx,
                        kind,
                        element,
                        shape,
                        transform,
                        bounds,
                        control,
                        &mut charges,
                    )?;
                    if kind == DefinitionKind::Pattern {
                        let source = ctx.builder.source_owner();
                        let shape_source = ctx
                            .builder
                            .traversal_scene(ctx.scene)?
                            .shape(shape)
                            .copied()
                            .ok_or_else(|| bad(0))?
                            .source;
                        let local = mapping.ok_or_else(|| bad(0))?;
                        let inverse =
                            crate::clip::inverse_affine(transform).ok_or_else(|| unsupported(0))?;
                        let opacity = match channel {
                            Some(channel) => pattern_channel_opacity(source, style, channel)?,
                            None => pattern_compatibility_opacity(source, style)?,
                        };
                        let viewport = ctx
                            .builder
                            .traversal_scene(ctx.scene)?
                            .viewport
                            .ok_or_else(|| bad(0))?;
                        ProductionDefinitionPlan::Pattern {
                            plan,
                            mapping: vexel_scene::PatternMapping {
                                tile_origin: local.tile_origin,
                                user_to_base: local.user_to_base,
                                pattern_transform: local.pattern_transform,
                                user_inverse: inverse,
                                viewport: [viewport.width as f64, viewport.height as f64],
                                bounds,
                                source_map: shape_source,
                                opacity,
                            },
                        }
                    } else {
                        ProductionDefinitionPlan::Definition(plan)
                    }
                }
                DefinitionTarget::Group(group) if kind == DefinitionKind::Filter => {
                    let source = ctx.shared.source();
                    let tokens = ctx.shared.tokens();
                    let token = tokens.get(element.0).ok_or_else(|| bad(0))?;
                    if !matches!(
                        token.kind,
                        TokenKind::StartElement | TokenKind::EmptyElement
                    ) || !kind_matches(source, token.local_name, kind)
                    {
                        return Err(unsupported(token.name.offset));
                    }
                    let _end = matching_end_controlled(tokens, element.0, control)?;
                    if self.filter_count >= crate::filter::FILTER_MAX_COUNT {
                        return Err(crate::limit(LimitKind::Filters, token.name.offset));
                    }
                    let _computed = definition_style(ctx, element.0, control, &mut charges)?;
                    let parsed = crate::filter::parse_filter_at_controlled_metered(
                        source,
                        tokens,
                        element.0,
                        control,
                        ctx.ledger,
                        &mut charges,
                    )?;
                    let next_primitives = self
                        .filter_primitives
                        .checked_add(parsed.primitives.len())
                        .ok_or_else(|| crate::number(token.name.offset))?;
                    if next_primitives > crate::filter::FILTER_MAX_TOTAL_PRIMITIVES {
                        return Err(crate::limit(LimitKind::Filters, token.name.offset));
                    }
                    self.filter_count += 1;
                    self.filter_primitives = next_primitives;
                    let plan = DefinitionPlan {
                        descriptor: DefinitionDescriptor::Filter {
                            filter: parsed.filter,
                            primitives: parsed.primitives,
                        },
                        expansions: Vec::new(),
                        post: PostOperation::FinishGroup(group),
                    };
                    ProductionDefinitionPlan::Definition(plan)
                }
                DefinitionTarget::Group(_) => return Err(unsupported(0)),
            })
        })();
        if result.is_err() {
            self.restore_counters(counters);
            charges.rollback(ctx.ledger);
        }
        let result = result?;
        let DefinitionPlanCarrier { plan, mapping } = match result {
            ProductionDefinitionPlan::Definition(plan) => DefinitionPlanCarrier {
                plan,
                mapping: None,
            },
            ProductionDefinitionPlan::Pattern { plan, mapping } => DefinitionPlanCarrier {
                plan,
                mapping: Some(mapping),
            },
        };
        let retained = match plan_capacity_bytes(&plan) {
            Ok(retained) => retained,
            Err(error) => {
                self.restore_counters(counters);
                charges.rollback(ctx.ledger);
                return Err(error);
            }
        };
        if let Err(error) = charges.admit_bytes(ctx.ledger, retained) {
            self.restore_counters(counters);
            charges.rollback(ctx.ledger);
            return Err(error);
        }
        charges.commit();
        Ok(match mapping {
            Some(mapping) => ProductionDefinitionPlan::Pattern { plan, mapping },
            None => ProductionDefinitionPlan::Definition(plan),
        })
    }
}

struct DefinitionPlanCarrier<'src> {
    plan: DefinitionPlan<'src>,
    mapping: Option<vexel_scene::PatternMapping>,
}

impl DefinitionPlanner {
    fn restore_counters(&mut self, values: (usize, usize, usize, usize, usize, usize)) {
        self.mask_instances = values.0;
        self.pattern_instances = values.1;
        self.marker_instances = values.2;
        self.filter_count = values.3;
        self.filter_primitives = values.4;
        self.clip_reference_bytes = values.5;
    }
}

fn plan_capacity_bytes(plan: &DefinitionPlan<'_>) -> Result<usize> {
    let mut bytes = plan
        .expansions
        .capacity()
        .checked_mul(std::mem::size_of::<Expansion<'_>>())
        .ok_or_else(|| crate::limit(LimitKind::Arithmetic, 0))?;
    if let DefinitionDescriptor::Filter { primitives, .. } = &plan.descriptor {
        bytes = bytes
            .checked_add(
                primitives
                    .capacity()
                    .checked_mul(std::mem::size_of::<FilterPrimitive>())
                    .ok_or_else(|| crate::limit(LimitKind::Arithmetic, 0))?,
            )
            .ok_or_else(|| crate::limit(LimitKind::Arithmetic, 0))?;
    }
    Ok(bytes)
}
