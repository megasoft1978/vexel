//! Additive production delegates. Legacy DecodeOperations remains an explicit
//! owned-fixture interface; no blanket conversion or default backend exists.
use crate::{
    types::{invalid, stopped},
    *,
};
use vexel_xml::XmlSpan;
#[derive(Debug, Clone, Copy)]
pub enum DefinitionTarget {
    Shape(ShapeId),
    Group(GroupId),
}
/// Same active scene/draft/source-token owner, not a naked slot or fake shape.
#[derive(Debug, Clone, Copy)]
pub struct DefinitionOwner {
    scene: SceneId,
    draft: u64,
    generation: u64,
    target: DefinitionTarget,
    element: ElementId,
}
impl DefinitionOwner {
    pub(crate) fn matches_draft(self, draft: &DraftToken, generation: u64) -> bool {
        self.scene == draft.mark.scene && self.draft == draft.nonce && self.generation == generation
    }
    pub fn target(self) -> DefinitionTarget {
        self.target
    }
    pub fn element(self) -> ElementId {
        self.element
    }
    pub fn validate(self, ctx: &DefinitionContext<'_, '_>) -> Result<()> {
        ctx.validate()?;
        let active = ctx.builder.active_draft().ok_or_else(|| invalid(0))?;
        if ctx.scene != self.scene
            || active.scene != self.scene
            || active.nonce != self.draft
            || ctx.builder.generation != self.generation
        {
            return Err(invalid(0));
        }
        let view = ctx.builder.scene(self.scene)?;
        match self.target {
            DefinitionTarget::Shape(id) if view.shape(id).is_some() => Ok(()),
            DefinitionTarget::Group(id) if view.group(id).is_some() => Ok(()),
            _ => Err(invalid(0)),
        }
    }
}
impl SceneBuilder {
    pub fn definition_owner(
        &self,
        draft: &DraftToken,
        source: &DecodeSource<'_>,
        element: ElementId,
        target: DefinitionTarget,
        control: &mut dyn WorkControl,
    ) -> Result<DefinitionOwner> {
        self.draft_index(draft)?;
        if !std::sync::Arc::ptr_eq(self.source_owner(), source.source_owner()) {
            return Err(invalid(0));
        }
        source.element(element, crate::memory::add(element.0, 1)?, control)?;
        let view = self.scene(draft.mark.scene)?;
        match target {
            DefinitionTarget::Shape(id) if view.shape(id).is_some() => {}
            DefinitionTarget::Group(id) if view.group(id).is_some() => {}
            _ => return Err(invalid(0)),
        }
        Ok(DefinitionOwner {
            scene: draft.mark.scene,
            draft: draft.nonce,
            generation: self.generation,
            target,
            element,
        })
    }
    /// Atomic owning filter publication on a real (possibly empty) group. The
    /// child range is finalized later using the same guarded draft mutator.
    pub fn publish_group_filter(
        &mut self,
        draft: &DraftToken,
        owner: DefinitionOwner,
        filter: Filter,
        primitives: &[FilterPrimitive],
        control: &mut dyn WorkControl,
    ) -> Result<FilterId> {
        let i = self.draft_index(draft)?;
        if owner.scene != draft.mark.scene
            || owner.draft != draft.nonce
            || owner.generation != self.generation
        {
            return Err(invalid(0));
        }
        let DefinitionTarget::Group(group) = owner.target else {
            return Err(invalid(0));
        };
        self.draft_slot(draft, 9, group.index(), self.scenes[i].groups.len())?;
        let mut value = *self
            .scene(owner.scene)?
            .group(group)
            .ok_or_else(|| invalid(0))?;
        let mut mark = self.checkpoint_internal(owner.scene)?;
        let result = (|| {
            let id = self.append_filter(owner.scene, filter, primitives, control)?;
            value.filter = Some(id);
            self.finalize_group(draft, group, value, control)?;
            Ok(id)
        })();
        if result.is_err() {
            mark.epoch = self.scenes[i].epoch;
            self.rollback_internal(mark)?;
        }
        result
    }
}
/// Stops borrow the resolver cache, independently of temporary source/ledger
/// borrows. The resolver owns/meters its cache; this result owns no stop Vec.
#[derive(Debug)]
pub enum CachedPaint<'cache> {
    Plain(Paint),
    Gradient {
        template: &'cache GradientTemplateId,
        paint: Paint,
        stops: &'cache [PaintStop],
    },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaintChannel {
    Fill,
    Stroke,
}
impl SceneBuilder {
    pub fn append_cached_paint(
        &mut self,
        scene: SceneId,
        cached: CachedPaint<'_>,
        channel: PaintChannel,
        control: &mut dyn WorkControl,
    ) -> Result<usize> {
        match cached {
            CachedPaint::Gradient {
                template,
                paint,
                stops,
            } => match channel {
                PaintChannel::Fill => {
                    self.append_template_fill(scene, template, paint, stops, control)
                }
                PaintChannel::Stroke => {
                    self.append_template_stroke(scene, template, paint, stops, control)
                }
            },
            CachedPaint::Plain(paint) => {
                if matches!(paint.kind, PaintKind::Linear | PaintKind::Radial)
                    || paint.stops != TableRange::EMPTY
                {
                    return Err(invalid(0));
                }
                match channel {
                    PaintChannel::Fill => self.append_fill_paint(scene, paint, control),
                    PaintChannel::Stroke => self.append_stroke_paint(scene, paint, control),
                }
            }
        }
    }
}
pub trait CachedPaintResolver {
    fn resolve_cached<'cache>(
        &'cache mut self,
        source: &mut DecodeSource<'_>,
        ledger: &mut ChargeLedger,
        value: XmlSpan,
        edges: &[Edge],
        inverse: Affine,
        viewport: [f64; 2],
        control: &mut dyn WorkControl,
    ) -> Result<CachedPaint<'cache>>;
}
/// Actual shared resolve+publication operation; resolver output is used directly.
pub fn resolve_publish_cached(
    resolver: &mut dyn CachedPaintResolver,
    source: &mut DecodeSource<'_>,
    ledger: &mut ChargeLedger,
    builder: &mut SceneBuilder,
    scene: SceneId,
    value: XmlSpan,
    edges: &[Edge],
    inverse: Affine,
    viewport: [f64; 2],
    channel: PaintChannel,
    control: &mut dyn WorkControl,
) -> Result<usize> {
    ledger.require_owner(ledger.owner_identity(), source.source_owner())?;
    if !std::sync::Arc::ptr_eq(builder.source_owner(), source.source_owner()) {
        return Err(invalid(0));
    }
    builder.writable(scene)?;
    if value.slice(source.source()).is_none() {
        return Err(invalid(value.offset));
    }
    control.checkpoint().map_err(stopped)?;
    let cached =
        resolver.resolve_cached(source, ledger, value, edges, inverse, viewport, control)?;
    builder.append_cached_paint(scene, cached, channel, control)
}
#[derive(Debug)]
pub enum ProductionDefinitionPlan<'src> {
    Definition(DefinitionPlan<'src>),
    Pattern {
        plan: DefinitionPlan<'src>,
        mapping: PatternMapping,
    },
}
/// Every operation is required. Implementations may share lower functions with
/// fixtures explicitly, but cannot silently inherit their owned paint path.
pub trait DefinitionPlanner {
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
    ) -> Result<ProductionDefinitionPlan<'src>>;
}
/// Required production pattern planning entry point with an explicit resolved
/// paint channel. This is separate from the legacy compatibility trait so its
/// old implementors and literals remain source compatible.
pub trait PaintChannelDefinitionPlanner {
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
    ) -> Result<ProductionDefinitionPlan<'src>>;
}
pub trait MediaDecodeOperations {
    fn decode_text(
        &mut self,
        request: TextDecodeRequest<'_, '_>,
        builder: &mut SceneBuilder,
        control: &mut dyn WorkControl,
    ) -> Result<bool>;
    fn decode_image(
        &mut self,
        request: ImageDecodeRequest<'_, '_>,
        builder: &mut SceneBuilder,
        control: &mut dyn WorkControl,
    ) -> Result<ShapeId>;
}
pub trait ProductionDecodeOperations:
    CachedPaintResolver + DefinitionPlanner + MediaDecodeOperations
{
}
/// A production operation set whose pattern route cannot omit Fill or Stroke.
pub trait ChannelAwareProductionDecodeOperations:
    CachedPaintResolver + PaintChannelDefinitionPlanner + MediaDecodeOperations
{
}
pub trait ProductionDecodeSession {
    fn step<'s>(
        &'s mut self,
        quantum: u64,
        operations: &mut dyn ProductionDecodeOperations,
        control: &mut dyn WorkControl,
    ) -> Result<DecodeEvent<'s>>;
    fn resume_pattern(
        &mut self,
        ticket: PatternTicket,
        job: &mut RasterJob,
        result: Result<JobRenderOutput>,
        control: &mut dyn WorkControl,
    ) -> Result<()>;
    fn finish(self, control: &mut dyn WorkControl) -> Result<SceneGraph>
    where
        Self: Sized;
}
/// Additive session contract for decoders that route patterns through the
/// required explicit-channel planner surface.
pub trait ChannelAwareProductionDecodeSession {
    fn step<'s>(
        &'s mut self,
        quantum: u64,
        operations: &mut dyn ChannelAwareProductionDecodeOperations,
        control: &mut dyn WorkControl,
    ) -> Result<DecodeEvent<'s>>;
    fn resume_pattern(
        &mut self,
        ticket: PatternTicket,
        job: &mut RasterJob,
        result: Result<JobRenderOutput>,
        control: &mut dyn WorkControl,
    ) -> Result<()>;
    fn finish(self, control: &mut dyn WorkControl) -> Result<SceneGraph>
    where
        Self: Sized;
}
/// Validate typed owner before entering the lower planner and require its entire
/// computed pattern mapping at return. This performs no legacy owned conversion.
/// The lower adapter owns/admit/releases its plan/cache allocations via ctx.ledger.
pub fn plan_production_definition<'src>(
    planner: &mut dyn DefinitionPlanner,
    ctx: &mut DefinitionContext<'src, '_>,
    kind: DefinitionKind,
    element: ElementId,
    owner: DefinitionOwner,
    transform: Affine,
    bounds: [f64; 4],
    style: &CssPaint<'src>,
    control: &mut dyn WorkControl,
) -> Result<ProductionDefinitionPlan<'src>> {
    validate_production_plan(ctx, kind, element, owner, transform, bounds, style, control,
        |ctx, control| planner.plan_owned_definition(ctx, kind, element, owner, transform, bounds, style, control))
}

/// Required channel-aware production route for patterns. All source, owner,
/// style, bounds, returned descriptor/mapping, and post validation is shared
/// with the compatibility production entry point above.
pub fn plan_production_pattern<'src>(
    planner: &mut dyn PaintChannelDefinitionPlanner,
    ctx: &mut DefinitionContext<'src, '_>,
    element: ElementId,
    owner: DefinitionOwner,
    channel: PaintChannel,
    transform: Affine,
    bounds: [f64; 4],
    style: &CssPaint<'src>,
    control: &mut dyn WorkControl,
) -> Result<ProductionDefinitionPlan<'src>> {
    validate_production_plan(ctx, DefinitionKind::Pattern, element, owner, transform, bounds, style, control,
        |ctx, control| planner.plan_owned_pattern_definition(ctx, element, owner, channel, transform, bounds, style, control))
}

fn validate_production_plan<'src>(
    ctx: &mut DefinitionContext<'src, '_>,
    kind: DefinitionKind,
    element: ElementId,
    owner: DefinitionOwner,
    transform: Affine,
    bounds: [f64; 4],
    style: &CssPaint<'src>,
    control: &mut dyn WorkControl,
    plan: impl FnOnce(&mut DefinitionContext<'src, '_>, &mut dyn WorkControl) -> Result<ProductionDefinitionPlan<'src>>,
) -> Result<ProductionDefinitionPlan<'src>> {
    owner.validate(ctx)?;
    validate_style(ctx.shared.source(), style, control)?;
    ctx.shared
        .element(element, crate::memory::add(element.0, 1)?, control)?;
    if !vexel_geometry::affine_valid(transform)
        || !bounds.into_iter().all(f64::is_finite)
        || bounds[2] < 0.
        || bounds[3] < 0.
    {
        return Err(invalid(0));
    }
    let plan = plan(ctx, control)?;
    let (definition, mapping) = match &plan {
        ProductionDefinitionPlan::Definition(plan) => (plan, None),
        ProductionDefinitionPlan::Pattern { plan, mapping } => (plan, Some(*mapping)),
    };
    let actual = match &definition.descriptor {
        DefinitionDescriptor::Clip(_) => DefinitionKind::Clip,
        DefinitionDescriptor::Mask(_) => DefinitionKind::Mask,
        DefinitionDescriptor::Pattern(descriptor) => {
            if descriptor.definition != element {
                return Err(invalid(0));
            }
            let map = mapping.ok_or_else(|| invalid(0))?;
            map.pixel_inverse(*descriptor)?;
            let DefinitionTarget::Shape(shape) = owner.target else {
                return Err(invalid(0));
            };
            if ctx
                .builder
                .scene(ctx.scene)?
                .shape(shape)
                .ok_or_else(|| invalid(0))?
                .source
                != map.source_map
                || map.bounds != bounds
            {
                return Err(invalid(0));
            }
            DefinitionKind::Pattern
        }
        DefinitionDescriptor::Marker(_) => DefinitionKind::Marker,
        DefinitionDescriptor::Filter { filter, primitives } => {
            let DefinitionTarget::Group(_) = owner.target else {
                return Err(invalid(0));
            };
            crate::validate::filter(*filter, primitives, control)?;
            for p in primitives {
                control.charge(WorkKind::FilterPixel, 1).map_err(stopped)?;
                if p.source_offset > ctx.shared.source().len() {
                    return Err(invalid(p.source_offset));
                }
            }
            DefinitionKind::Filter
        }
    };
    if actual != kind || (kind != DefinitionKind::Pattern && mapping.is_some()) {
        return Err(invalid(0));
    }
    crate::mask_post::validate_production_post(ctx, kind, None, definition.post, control)?;
    if definition.expansions.len() > 65536 {
        return Err(crate::types::limit(LimitKind::References));
    }
    for expansion in &definition.expansions {
        crate::mask_post::validate_production_post(
            ctx,
            kind,
            Some(expansion.flavor),
            expansion.post,
            control,
        )?;
        control
            .charge(WorkKind::ReferenceProbe, 1)
            .map_err(stopped)?;
        expansion.range.require(ctx.shared.tokens().len(), 0)?;
        validate_style(ctx.shared.source(), &expansion.style, control)?;
        if !vexel_geometry::affine_valid(expansion.transform)
            || !expansion
                .viewport
                .into_iter()
                .all(|v| v.is_finite() && v > 0.)
        {
            return Err(invalid(0));
        }
    }
    control.checkpoint().map_err(stopped)?;
    Ok(plan)
}

fn validate_style(
    source: &[u8],
    style: &CssPaint<'_>,
    control: &mut dyn WorkControl,
) -> Result<()> {
    for property in vexel_svg_css::Property::ALL {
        control.charge(WorkKind::PaintVisit, 1).map_err(stopped)?;
        if let Some(value) = style.value(property) {
            let bytes = value
                .span
                .slice(source)
                .ok_or_else(|| invalid(value.span.offset))?;
            if bytes.as_ptr() != value.bytes.as_ptr() || bytes.len() != value.bytes.len() {
                return Err(invalid(value.span.offset));
            }
        }
    }
    Ok(())
}
