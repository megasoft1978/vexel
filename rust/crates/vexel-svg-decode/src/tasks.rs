//! Source-only traversal continuations. Scene IDs retain complete owner/serial identity.
use vexel_scene::*;
#[derive(Debug, Clone, Copy)]
pub(crate) struct Context<'src> {
    pub scene: SceneId,
    pub style: CssPaint<'src>,
    pub transform: Affine,
    pub viewport: [f64; 2],
    pub depth: usize,
    pub node: Option<TransformId>,
    pub group: Option<GroupId>,
    pub host: Option<ElementId>,
    pub text: Option<usize>,
    pub preserve: bool,
    pub flavor: DecodeFlavor,
}
#[derive(Debug, Clone)]
pub(crate) struct Finish<'src> {
    pub context: Context<'src>,
    pub element: usize,
    pub first_shape: usize,
    pub group: Option<GroupId>,
    pub style: CssPaint<'src>,
    pub viewport_clip: Option<([Point; 4], Affine)>,
    pub reference: Option<ElementId>,
    pub text_root: bool,
    pub clip_done: bool,
    pub mask_done: bool,
}
#[derive(Debug)]
pub(crate) enum Task<'src> {
    Walk {
        cursor: usize,
        limit: usize,
        context: Context<'src>,
    },
    Render {
        element: usize,
        context: Context<'src>,
        symbol: bool,
    },
    Finish(Finish<'src>),
    Markers {
        node: Finish<'src>,
        values: [Option<vexel_xml::XmlSpan>; 3],
        cursor: usize,
        seen: [Option<ElementId>; 3],
    },
    PatternPlan {
        node: Finish<'src>,
        value: vexel_xml::XmlSpan,
        channel: PaintChannel,
    },
    DefinitionDone(crate::definition_work::DefinitionWork<'src>),
    MarkerDone {
        definition: ElementId,
    },
    ExpansionPost {
        scene: SceneId,
        post: PostOperation,
        first_shape: usize,
    },
    Text {
        span: vexel_xml::XmlSpan,
        cursor: usize,
    },
    CustomGeometry {
        scene: SceneId,
        request: RequestToken,
        host: ElementId,
        map: SourceMap,
        geometry: crate::Geometry,
        paint: CssPaint<'src>,
        viewport: [f64; 2],
    },
    CustomDone {
        request: RequestToken,
        owns_request: bool,
    },
}
