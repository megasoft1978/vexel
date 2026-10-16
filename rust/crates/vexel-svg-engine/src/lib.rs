#![forbid(unsafe_code)]
//! Production composition adapter for the resumable SVG decoder.
//!
//! Gradients resolve through the retained paint cache and are published via the
//! scene's borrowed cached-paint handoff. Definitions use the real planner.
//! Text and images currently return source-offset `Unsupported` diagnostics.

use vexel_base::WorkControl;
use vexel_geometry::{Affine, Edge};
use vexel_scene::*;
use vexel_svg_css::CssPaint;
use vexel_svg_definitions::DefinitionPlanner as Planner;
use vexel_svg_paint::resolve_cached_fill;
use vexel_xml::XmlSpan;

/// Required production delegates for one decoder session.
pub struct SvgEngineOperations {
    paint_cache: PaintCache,
    planner: Planner,
    template: Option<GradientTemplateId>,
    clip_plan_attempted: bool,
    clip_plan_definition: Option<usize>,
    clip_plan_bounds: Option<[f64; 4]>,
    successful_clip_plan: bool,
    clip_plan_error: Option<Diagnostic>,
    clip_context_error: Option<Diagnostic>,
    clip_owner_error: Option<Diagnostic>,
}
impl Default for SvgEngineOperations {
    fn default() -> Self {
        Self::new()
    }
}
impl SvgEngineOperations {
    pub fn new() -> Self {
        Self {
            paint_cache: PaintCache::default(),
            planner: Planner::new(),
            template: None,
            clip_plan_attempted: false,
            clip_plan_definition: None,
            clip_plan_bounds: None,
            successful_clip_plan: false,
            clip_plan_error: None,
            clip_context_error: None,
            clip_owner_error: None,
        }
    }
    pub fn paint_cache(&self) -> &PaintCache {
        &self.paint_cache
    }
    /// Whether the actual DefinitionPlanner has successfully produced a clip
    /// plan during this session. This is useful for diagnostics and parity QA.
    pub fn has_successful_clip_plan(&self) -> bool {
        self.successful_clip_plan
    }
    /// Whether a clip request reached the production definition planner.
    pub fn clip_plan_was_attempted(&self) -> bool {
        self.clip_plan_attempted
    }
    /// Token index given to the last clip-planner call.
    pub fn clip_plan_definition(&self) -> Option<usize> {
        self.clip_plan_definition
    }
    /// Object bounds supplied to the last clip-planner call.
    pub fn clip_plan_bounds(&self) -> Option<[f64; 4]> {
        self.clip_plan_bounds
    }
    /// Last actual clip-planner diagnostic, if its call failed.
    pub fn clip_plan_error(&self) -> Option<Diagnostic> {
        self.clip_plan_error
    }
    /// Preflight diagnostics for the production planner context and owner.
    pub fn clip_preflight_errors(&self) -> (Option<Diagnostic>, Option<Diagnostic>) {
        (self.clip_context_error, self.clip_owner_error)
    }
}

impl CachedPaintResolver for SvgEngineOperations {
    fn resolve_cached<'a>(
        &'a mut self,
        source: &mut DecodeSource<'_>,
        ledger: &mut ChargeLedger,
        value: XmlSpan,
        edges: &[Edge],
        inverse: Affine,
        viewport: [f64; 2],
        control: &mut dyn WorkControl,
    ) -> Result<CachedPaint<'a>> {
        let resolved = resolve_cached_fill(
            source,
            &mut self.paint_cache,
            ledger,
            value,
            edges,
            inverse,
            viewport,
            control,
        )?;
        self.template = Some(resolved.template);
        Ok(CachedPaint::Gradient {
            template: self.template.as_ref().expect("just assigned"),
            paint: resolved.paint,
            stops: resolved.stops,
        })
    }
}
impl vexel_scene::DefinitionPlanner for SvgEngineOperations {
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
        // V1 subset policy: filters and markers are deferred and fail closed at
        // the definition element. (The planner/Decoder filter contracts disagree
        // on the post operation, and marker output is not subset-verified.)
        if matches!(kind, DefinitionKind::Filter | DefinitionKind::Marker) {
            return Err(Diagnostic {
                kind: ErrorKind::Unsupported,
                offset: definition_offset(ctx, element),
            });
        }
        if kind == DefinitionKind::Clip {
            self.clip_plan_attempted = true;
            self.clip_plan_definition = Some(element.0);
            self.clip_plan_bounds = Some(bounds);
            self.clip_context_error = ctx.validate().err();
            self.clip_owner_error = owner.validate(ctx).err();
        }
        let planned = vexel_scene::DefinitionPlanner::plan_owned_definition(
            &mut self.planner,
            ctx,
            kind,
            element,
            owner,
            transform,
            bounds,
            style,
            control,
        );
        // The planner reports some unsupported owners with offset 0; point the
        // typed diagnostic at the definition element instead.
        let planned = planned.map_err(|error| match error.kind {
            ErrorKind::Unsupported if error.offset == 0 => Diagnostic {
                kind: ErrorKind::Unsupported,
                offset: definition_offset(ctx, element),
            },
            _ => error,
        });
        if kind == DefinitionKind::Clip && planned.is_ok() {
            self.successful_clip_plan = true;
        }
        if kind == DefinitionKind::Clip {
            self.clip_plan_error = planned.as_ref().err().copied();
        }
        planned
    }
}
impl PaintChannelDefinitionPlanner for SvgEngineOperations {
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
        vexel_scene::PaintChannelDefinitionPlanner::plan_owned_pattern_definition(
            &mut self.planner,
            ctx,
            element,
            owner,
            channel,
            transform,
            bounds,
            style,
            control,
        )
    }
}
fn definition_offset(ctx: &DefinitionContext<'_, '_>, element: ElementId) -> usize {
    ctx.shared
        .tokens()
        .get(element.0)
        .map_or(0, |token| token.name.offset)
}
fn media_offset(element: &ElementView<'_>) -> usize {
    element
        .tokens()
        .get(element.element.0)
        .map_or(0, |token| token.source.offset)
}
impl MediaDecodeOperations for SvgEngineOperations {
    fn decode_text(
        &mut self,
        request: TextDecodeRequest<'_, '_>,
        _: &mut SceneBuilder,
        _: &mut dyn WorkControl,
    ) -> Result<bool> {
        Err(Diagnostic {
            kind: ErrorKind::Unsupported,
            offset: media_offset(&request.element),
        })
    }
    fn decode_image(
        &mut self,
        request: ImageDecodeRequest<'_, '_>,
        _: &mut SceneBuilder,
        _: &mut dyn WorkControl,
    ) -> Result<ShapeId> {
        Err(Diagnostic {
            kind: ErrorKind::Unsupported,
            offset: media_offset(&request.element),
        })
    }
}
impl ChannelAwareProductionDecodeOperations for SvgEngineOperations {}
impl vexel_svg_decode::DecoderProductionOperations for SvgEngineOperations {
    fn decode_text_scoped(
        &mut self,
        request: TextDecodeRequest<'_, '_>,
        builder: &mut SceneBuilder,
        _: &DraftToken,
        control: &mut dyn WorkControl,
    ) -> Result<bool> {
        MediaDecodeOperations::decode_text(self, request, builder, control)
    }
    fn decode_image_scoped(
        &mut self,
        request: ImageDecodeRequest<'_, '_>,
        builder: &mut SceneBuilder,
        _: &DraftToken,
        control: &mut dyn WorkControl,
    ) -> Result<ShapeId> {
        MediaDecodeOperations::decode_image(self, request, builder, control)
    }
}

/// Bounded incremental driver over the actual source-preserving Decoder.
/// Pattern events retain the Decoder continuation; callers must render and
/// resume the exact ticket before stepping again. Finish delegates the Decoder's
/// complete/no-pending-work guard.
pub struct SvgEngineSession<'src, 'ctx> {
    decoder: vexel_svg_decode::Decoder<'src, 'ctx>,
    operations: SvgEngineOperations,
}
impl<'src, 'ctx> SvgEngineSession<'src, 'ctx> {
    pub fn from_source(
        document: &'src LexicalDocument,
        source: &'ctx mut DecodeSource<'src>,
        generation: u64,
        options: DecodeOptions<'ctx>,
        control: &mut dyn WorkControl,
    ) -> Result<Self> {
        Ok(Self {
            decoder: vexel_svg_decode::Decoder::from_source(
                document, source, generation, options, control,
            )?,
            operations: SvgEngineOperations::new(),
        })
    }
    pub fn step<'s>(
        &'s mut self,
        quantum: u64,
        job: &RasterJob,
        control: &mut dyn WorkControl,
    ) -> Result<vexel_svg_decode::TraversalDecodeEvent<'s>> {
        self.decoder
            .step_production(quantum, &mut self.operations, job, control)
    }
    pub fn resume_pattern(
        &mut self,
        ticket: PatternTicket,
        job: &mut RasterJob,
        result: Result<JobRenderOutput>,
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        self.decoder
            .resume_pattern_production(ticket, job, result, control)
    }
    pub fn finish(self, control: &mut dyn WorkControl) -> Result<DecodedScene> {
        self.decoder.finish_decoded(control)
    }
    pub fn state(&self) -> vexel_svg_decode::DecoderState {
        self.decoder.state()
    }
    pub fn blocked_contract(&self) -> Option<vexel_svg_decode::ContractBlocker> {
        self.decoder.blocked_contract()
    }
    pub fn operations(&self) -> &SvgEngineOperations {
        &self.operations
    }
}
