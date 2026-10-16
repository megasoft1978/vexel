//! Transient source/context contracts. No lower operation calls the decoder back.
use crate::*;
pub use vexel_svg_css::CssPaint;
use vexel_xml::{XmlSpan, XmlToken};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeFlavor {
    Normal,
    Clip { clip_rule: bool },
    Mask,
    Pattern,
    Marker,
}
#[derive(Debug, Clone, Copy)]
pub struct ElementView<'src> {
    pub(crate) source: &'src [u8],
    pub(crate) tokens: &'src [XmlToken],
    pub element: ElementId,
    pub end: usize,
}
impl<'src> ElementView<'src> {
    pub fn source(&self) -> &'src [u8] {
        self.source
    }
    pub fn tokens(&self) -> &'src [XmlToken] {
        self.tokens
    }

    pub fn validate(&self, control: &mut dyn WorkControl) -> Result<()> {
        control.checkpoint().map_err(crate::types::stopped)?;
        if self.element.0 >= self.end || self.end > self.tokens.len() {
            return Err(crate::types::invalid(0));
        }
        if self
            .tokens
            .len()
            .checked_mul(std::mem::size_of::<XmlToken>())
            .is_none_or(|n| n > 256 * 1024 * 1024)
        {
            return Err(crate::types::limit(LimitKind::TokenBytes));
        }
        let first = &self.tokens[self.element.0];
        if !matches!(
            first.kind,
            vexel_xml::TokenKind::StartElement | vexel_xml::TokenKind::EmptyElement
        ) {
            return Err(crate::types::invalid(first.source.offset));
        }
        for token in &self.tokens[self.element.0..self.end] {
            control
                .charge(WorkKind::TokenVisit, 1)
                .map_err(crate::types::stopped)?;
            for span in [
                token.source,
                token.name,
                token.value,
                token.prefix,
                token.local_name,
            ] {
                if span.slice(self.source).is_none() {
                    return Err(crate::types::invalid(span.offset));
                }
            }
            if token.depth > 64 {
                return Err(crate::types::limit(LimitKind::Depth));
            }
        }
        Ok(())
    }
}
#[derive(Debug)]
pub struct Expansion<'src> {
    pub range: TableRange,
    pub transform: Affine,
    pub viewport: [f64; 2],
    pub style: CssPaint<'src>,
    pub flavor: DecodeFlavor,
    pub post: PostOperation,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostOperation {
    None,
    AttachClip(ClipId),
    AttachMask(MaskId),
    AttachTargetedMask(MaskPostTarget),
    FinishPattern,
    FinishGroup(GroupId),
    FinishMarker,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefinitionKind {
    Clip,
    Mask,
    Pattern,
    Marker,
    Filter,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DefinitionIdentity {
    pub kind: DefinitionKind,
    pub element: ElementId,
}
#[derive(Debug)]
pub enum DefinitionDescriptor {
    Clip(Clip),
    Mask(MaskDescriptor),
    Pattern(PatternDescriptor),
    Marker(MarkerDescriptor),
    Filter {
        filter: Filter,
        primitives: Vec<FilterPrimitive>,
    },
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaskDescriptor {
    pub luminance: bool,
    pub region_object_bounding_box: bool,
    pub custom_region: bool,
    pub region: [f64; 4],
    pub region_transform: Affine,
    pub source_offset: usize,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PatternDescriptor {
    pub definition: ElementId,
    pub width: u32,
    pub height: u32,
    pub tile_width: f64,
    pub tile_height: f64,
    pub content_transform: Affine,
    pub error_offset: usize,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MarkerDescriptor {
    pub definition: ElementId,
    pub viewport: [f64; 2],
    pub transform: Affine,
    pub error_offset: usize,
}
#[derive(Debug)]
pub struct DefinitionPlan<'src> {
    pub descriptor: DefinitionDescriptor,
    pub expansions: Vec<Expansion<'src>>,
    pub post: PostOperation,
}
pub struct DecodeOptions<'ctx> {
    pub limits: Limits,
    pub system_language: &'ctx [u8],
    pub media: Option<&'ctx mut dyn MediaProvider>,
}
#[derive(Debug, Clone, Copy)]
pub struct StyleSpans {
    pub values: [Option<XmlSpan>; 39],
}
impl Default for StyleSpans {
    fn default() -> Self {
        Self { values: [None; 39] }
    }
}
#[derive(Debug)]
pub struct TraversalFrame {
    pub scene: SceneId,
    pub first: usize,
    pub limit: usize,
    pub current: usize,
    pub render_depth: usize,
    pub transform: Affine,
    pub viewport: [f64; 2],
    pub inherited_style: StyleSpans,
    pub flavor: DecodeFlavor,
    pub transform_node: Option<TransformId>,
    pub group: Option<GroupId>,
    pub text_state: TextState,
    pub preserve_space: bool,
}
#[derive(Debug)]
pub enum Continuation {
    Enter(TraversalFrame),
    Exit(PostOperation),
    AwaitPattern(PatternTicket),
}
#[derive(Debug)]
pub enum DecodeEvent<'a> {
    Progress { charged_units: u64 },
    NeedPattern(PatternRequest<'a>),
    Complete,
}
#[derive(Debug)]
pub struct PatternRequest<'a, V = Viewport, A = AnimationView<'a>> {
    pub ticket: PatternTicket,
    pub child: SceneView<'a, V, A>,
    pub definition: ElementId,
    pub width: u32,
    pub height: u32,
    pub tile_width: f64,
    pub tile_height: f64,
    pub content_transform: Affine,
    pub error_offset: usize,
}
#[derive(Debug, Default)]
pub struct ActiveDefinitions {
    pub uses: Vec<ElementId>,
    pub masks: Vec<ElementId>,
    pub patterns: Vec<ElementId>,
    pub markers: Vec<ElementId>,
}
#[derive(Debug, Default)]
pub struct PaintCache {
    pub entries: Vec<GradientCacheEntry>,
    pub slots: Vec<Option<usize>>,
    pub token_visits: usize,
}
/// Complete instance context for cached resolved paints. The cache binds the
/// immutable source separately; a definition alone cannot identify this result.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GradientCacheContext {
    pub viewport: [f64; 2],
    pub user_inverse: Affine,
    pub bounds: [f64; 4],
}
#[derive(Debug)]
pub struct GradientCacheEntry {
    pub definition: ElementId,
    pub context: GradientCacheContext,
    pub paint: Paint,
    pub stop_range: TableRange,
    pub stops: Vec<PaintStop>,
    pub source: SourceOwner,
}
#[derive(Debug, Default)]
pub struct TextCache {
    pub entries: Vec<TextCacheEntry>,
    pub text_bytes: usize,
    pub commands: usize,
}
#[derive(Debug)]
pub struct TextCacheEntry {
    pub text: Vec<u8>,
    pub family_spans: Vec<Span>,
    pub owned_families: Vec<Vec<u8>>,
    pub size_points: f64,
    pub weight: f64,
    pub letter_spacing_points: f64,
    pub style: TextStyle,
    pub layout: TextLayout,
}
#[derive(Debug)]
pub struct ResolvedPaint {
    /// Source-derived template witness selected by the actual resolver. Required
    /// for linear/radial paints, absent for other paint kinds; never a graph range.
    pub template: Option<GradientTemplateId>,
    pub paint: Paint,
    pub stops: Vec<PaintStop>,
}
/// Source borrows survive temporary context/control borrows. No context stores
/// references into itself, and no returned plan borrows a mutable builder.
pub struct ResolveContext<'src, 'ctx> {
    pub shared: &'ctx mut DecodeSource<'src>,
    pub cache: &'ctx mut PaintCache,
    pub ledger: &'ctx mut ChargeLedger,
}
pub struct DefinitionContext<'src, 'ctx> {
    pub shared: &'ctx mut DecodeSource<'src>,
    pub builder: &'ctx mut SceneBuilder,
    pub scene: SceneId,
    pub active: &'ctx mut ActiveDefinitions,
    pub ledger: &'ctx mut ChargeLedger,
}
pub struct TextDecodeRequest<'src, 'ctx> {
    pub element: ElementView<'src>,
    /// Instance annotation: may identify a use host distinct from the tokens.
    pub source_map: SourceMap,
    pub end: usize,
    pub style: &'ctx CssPaint<'src>,
    pub transform: Affine,
    pub state: &'ctx mut TextState,
    pub preserve_space: bool,
    pub provider: &'ctx mut dyn MediaProvider,
    pub cache: &'ctx mut TextCache,
    pub scene: SceneId,
    pub ledger: &'ctx mut ChargeLedger,
}
pub struct ImageDecodeRequest<'src, 'ctx> {
    pub element: ElementView<'src>,
    /// Instance annotation: may identify a use host distinct from the tokens.
    pub source_map: SourceMap,
    pub style: &'ctx CssPaint<'src>,
    pub transform: Affine,
    pub provider: &'ctx mut dyn MediaProvider,
    pub scene: SceneId,
    pub ledger: &'ctx mut ChargeLedger,
}
/// These are real dependency interfaces; no default or synthetic implementation.
pub trait DecodeOperations {
    fn resolve_paint(
        &mut self,
        ctx: &mut ResolveContext<'_, '_>,
        value: XmlSpan,
        edges: &[Edge],
        inverse: Affine,
        viewport: [f64; 2],
        control: &mut dyn WorkControl,
    ) -> Result<ResolvedPaint>;
    fn plan_definition<'src, 'ctx>(
        &mut self,
        ctx: &mut DefinitionContext<'src, 'ctx>,
        kind: DefinitionKind,
        element: ElementId,
        owner: ShapeId,
        transform: Affine,
        bounds: [f64; 4],
        style: &CssPaint<'src>,
        control: &mut dyn WorkControl,
    ) -> Result<DefinitionPlan<'src>>;
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
/// Complete actual decoder engines will implement this interface; no empty
/// engine is supplied to masquerade as a successful SVG decode.
pub trait DecodeSession {
    fn step<'s>(
        &'s mut self,
        work_quantum: u64,
        operations: &mut dyn DecodeOperations,
        control: &mut dyn WorkControl,
    ) -> Result<DecodeEvent<'s>>;
    fn resume_pattern(
        &mut self,
        ticket: PatternTicket,
        result: Result<vexel_raster::Tile>,
        control: &mut dyn WorkControl,
    ) -> Result<()>;
    fn finish(self, control: &mut dyn WorkControl) -> Result<SceneGraph>
    where
        Self: Sized;
}
