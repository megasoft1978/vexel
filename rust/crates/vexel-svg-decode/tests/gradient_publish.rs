#[path = "support/production.rs"]
mod production;
mod support;
use production::*;
use std::sync::Arc;
use support::Run;
use vexel_scene::*;
use vexel_svg_decode::{Decoder, DecoderProductionOperations, TraversalDecodeEvent as DecodeEvent};
use vexel_xml::XmlSpan;

/// Production resolver for gradient fills. Kind comes from the referenced
/// element name; stops are fixed per kind. Every other operation delegates to
/// the explicit fixture, so decode-side publication runs unchanged.
#[derive(Default)]
struct GradientOps {
    fixture: ProductionFixture,
    template: Option<GradientTemplateId>,
    stops: Vec<PaintStop>,
    resolves: usize,
}

const LINEAR_STOPS: [([u8; 4], f64); 2] = [([255, 0, 0, 255], 0.0), ([0, 0, 255, 255], 1.0)];
const RADIAL_STOPS: [([u8; 4], f64); 3] = [
    ([0, 255, 0, 255], 0.0),
    ([255, 255, 0, 255], 0.5),
    ([0, 0, 0, 255], 1.0),
];

impl CachedPaintResolver for GradientOps {
    fn resolve_cached<'a>(
        &'a mut self,
        source: &mut DecodeSource<'_>,
        _: &mut ChargeLedger,
        value: XmlSpan,
        _edges: &[Edge],
        inverse: Affine,
        viewport: [f64; 2],
        control: &mut dyn WorkControl,
    ) -> Result<CachedPaint<'a>> {
        self.resolves += 1;
        let id = source
            .find_reference(
                XmlSpan {
                    offset: value.offset + 5,
                    length: value.length - 6,
                },
                control,
            )?
            .ok_or(Diagnostic {
                kind: ErrorKind::Unsupported,
                offset: value.offset,
            })?;
        let name = source.tokens()[id.0]
            .local_name
            .slice(source.source())
            .ok_or(Diagnostic {
                kind: ErrorKind::Unsupported,
                offset: value.offset,
            })?;
        let (kind, table): (PaintKind, &[([u8; 4], f64)]) = match name {
            b"linearGradient" => (PaintKind::Linear, &LINEAR_STOPS),
            b"radialGradient" => (PaintKind::Radial, &RADIAL_STOPS),
            _ => {
                return Err(Diagnostic {
                    kind: ErrorKind::Unsupported,
                    offset: value.offset,
                });
            }
        };
        self.template = Some(source.gradient_template_id(id, viewport, control)?);
        self.stops = table
            .iter()
            .map(|&(rgba, offset)| PaintStop { offset, rgba })
            .collect();
        Ok(CachedPaint::Gradient {
            template: self.template.as_ref().unwrap(),
            stops: &self.stops,
            paint: Paint {
                kind,
                spread: Spread::Pad,
                object_bounding_box: true,
                coordinates: [0., 0., 1., 0., 0., 0.],
                bounds: [0., 0., 1., 1.],
                user_inverse: inverse,
                gradient_inverse: vexel_geometry::affine_identity(),
                stops: TableRange {
                    first: 0,
                    count: table.len(),
                },
                pattern: None,
                pattern_inverse: vexel_geometry::affine_identity(),
                pattern_opacity: 1.0,
            },
        })
    }
}

impl DefinitionPlanner for GradientOps {
    fn plan_owned_definition<'s>(
        &mut self,
        c: &mut DefinitionContext<'s, '_>,
        kind: DefinitionKind,
        id: ElementId,
        owner: DefinitionOwner,
        transform: Affine,
        bounds: [f64; 4],
        style: &CssPaint<'s>,
        control: &mut dyn WorkControl,
    ) -> Result<ProductionDefinitionPlan<'s>> {
        self.fixture
            .plan_owned_definition(c, kind, id, owner, transform, bounds, style, control)
    }
}

impl PaintChannelDefinitionPlanner for GradientOps {
    fn plan_owned_pattern_definition<'src>(
        &mut self,
        c: &mut DefinitionContext<'src, '_>,
        element: ElementId,
        owner: DefinitionOwner,
        channel: PaintChannel,
        transform: Affine,
        bounds: [f64; 4],
        style: &CssPaint<'src>,
        control: &mut dyn WorkControl,
    ) -> Result<ProductionDefinitionPlan<'src>> {
        self.fixture.plan_owned_pattern_definition(
            c, element, owner, channel, transform, bounds, style, control,
        )
    }
}

impl MediaDecodeOperations for GradientOps {
    fn decode_text(
        &mut self,
        r: TextDecodeRequest<'_, '_>,
        b: &mut SceneBuilder,
        w: &mut dyn WorkControl,
    ) -> Result<bool> {
        MediaDecodeOperations::decode_text(&mut self.fixture, r, b, w)
    }
    fn decode_image(
        &mut self,
        r: ImageDecodeRequest<'_, '_>,
        b: &mut SceneBuilder,
        w: &mut dyn WorkControl,
    ) -> Result<ShapeId> {
        MediaDecodeOperations::decode_image(&mut self.fixture, r, b, w)
    }
}

impl ScopedMediaDecodeOperations for GradientOps {
    fn decode_text(
        &mut self,
        r: TextDecodeRequest<'_, '_>,
        b: &mut SceneBuilder,
        d: &DraftToken,
        w: &mut dyn WorkControl,
    ) -> Result<bool> {
        ScopedMediaDecodeOperations::decode_text(&mut self.fixture, r, b, d, w)
    }
    fn decode_image(
        &mut self,
        r: ImageDecodeRequest<'_, '_>,
        b: &mut SceneBuilder,
        d: &DraftToken,
        w: &mut dyn WorkControl,
    ) -> Result<ShapeId> {
        ScopedMediaDecodeOperations::decode_image(&mut self.fixture, r, b, d, w)
    }
}

impl DecoderProductionOperations for GradientOps {
    fn decode_text_scoped(
        &mut self,
        r: TextDecodeRequest<'_, '_>,
        b: &mut SceneBuilder,
        d: &DraftToken,
        w: &mut dyn WorkControl,
    ) -> Result<bool> {
        self.fixture.decode_text_scoped(r, b, d, w)
    }
    fn decode_image_scoped(
        &mut self,
        r: ImageDecodeRequest<'_, '_>,
        b: &mut SceneBuilder,
        d: &DraftToken,
        w: &mut dyn WorkControl,
    ) -> Result<ShapeId> {
        self.fixture.decode_image_scoped(r, b, d, w)
    }
}

impl ChannelAwareProductionDecodeOperations for GradientOps {}
impl ProductionDecodeOperations for GradientOps {}
impl ScopedProductionDecodeOperations for GradientOps {}

fn run(input: &str, ops: &mut GradientOps) -> Result<SceneGraph> {
    let source = Arc::new(input.as_bytes().to_vec());
    let mut run = Run::default();
    let doc = LexicalDocument::parse(source.clone(), 256 * 1024 * 1024, &mut run)?;
    let mut shared = DecodeSource::new(&doc, &mut run)?;
    let mut job = RasterJob::new(ChargeLedger::new(&source, 64 * 1024 * 1024, 65536)?)
        .map_err(|e| e.diagnostic)?;
    let mut d = Decoder::from_source(
        &doc,
        &mut shared,
        17,
        DecodeOptions {
            limits: Limits::default(),
            system_language: b"en",
            media: None,
        },
        &mut run,
    )?;
    loop {
        match d.step_production(64, ops, &job, &mut run)? {
            DecodeEvent::NeedPattern(request) => {
                let ticket = request.ticket;
                let pixels = output(&mut job, 17, &mut run);
                d.resume_pattern_production(ticket, &mut job, Ok(pixels), &mut run)?;
            }
            DecodeEvent::Progress { .. } => {}
            DecodeEvent::Complete => break,
        }
    }
    d.finish(&mut run)
}

fn stops_of(graph: &SceneGraph, paint: Paint) -> Vec<([u8; 4], f64)> {
    let r = graph.root();
    r.stops[paint.stops.first..paint.stops.first + paint.stops.count]
        .iter()
        .map(|s| (s.rgba, s.offset))
        .collect()
}

#[test]
fn linear_gradient_fill_publishes_linear_with_its_stops() {
    let input = "<svg width=\"8\" height=\"8\"><defs><linearGradient id=\"g\"><stop stop-color=\"red\"/><stop offset=\"1\" stop-color=\"blue\"/></linearGradient></defs><rect width=\"4\" height=\"4\" fill=\"url(#g)\"/></svg>";
    let mut ops = GradientOps::default();
    let graph = run(input, &mut ops).unwrap();
    assert_eq!(ops.resolves, 1);
    let r = graph.root();
    assert_eq!(r.shapes.len(), 1);
    let paint = r.fill_paints[r.shapes[0].fill_paint.unwrap()];
    assert_eq!(paint.kind, PaintKind::Linear);
    assert_eq!(stops_of(&graph, paint), LINEAR_STOPS.to_vec());
    assert!(r.shapes[0].stroke_paint.is_none());
}

#[test]
fn radial_gradient_fill_publishes_radial_with_its_stops() {
    let input = "<svg width=\"8\" height=\"8\"><defs><radialGradient id=\"g\"><stop stop-color=\"lime\"/><stop offset=\"0.5\" stop-color=\"yellow\"/><stop offset=\"1\" stop-color=\"black\"/></radialGradient></defs><rect width=\"4\" height=\"4\" fill=\"url(#g)\"/></svg>";
    let mut ops = GradientOps::default();
    let graph = run(input, &mut ops).unwrap();
    let r = graph.root();
    let paint = r.fill_paints[r.shapes[0].fill_paint.unwrap()];
    assert_eq!(paint.kind, PaintKind::Radial);
    assert_eq!(stops_of(&graph, paint), RADIAL_STOPS.to_vec());
}

#[test]
fn unsupported_gradient_kind_fails_closed_with_unsupported() {
    let input = "<svg width=\"8\" height=\"8\"><defs><rect id=\"g\" width=\"1\" height=\"1\"/></defs><rect width=\"4\" height=\"4\" fill=\"url(#g)\"/></svg>";
    let mut ops = GradientOps::default();
    let error = run(input, &mut ops).unwrap_err();
    assert_eq!(error.kind, ErrorKind::Unsupported);
    assert_eq!(ops.resolves, 1);
}
