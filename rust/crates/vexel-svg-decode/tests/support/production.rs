#![allow(dead_code)]
//! Explicit lower fixtures for the production interfaces. Only the listed
//! gradients/definitions are supported; the actual decoder and scene run fully.
use crate::support::*;
use std::sync::Arc;
use vexel_geometry::affine_identity as identity;
use vexel_scene::*;
use vexel_svg_decode::Decoder;
use vexel_svg_decode::TraversalDecodeEvent as DecodeEvent;
use vexel_xml::TokenKind;
use vexel_xml::XmlSpan;
pub struct ProductionFixture {
    template: Option<GradientTemplateId>,
    pub resolves: usize,
    pub paint_error: Option<Diagnostic>,
    pub paint_edge_counts: Vec<usize>,
    pub plans: usize,
    pub invalid_flavor: bool,
    pub fail_filter: bool,
    pub stops: [PaintStop; 2],
    pub pattern_children: Vec<SceneId>,
}
impl CachedPaintResolver for ProductionFixture {
    fn resolve_cached<'a>(
        &'a mut self,
        source: &mut DecodeSource<'_>,
        _: &mut ChargeLedger,
        value: XmlSpan,
        edges: &[Edge],
        inverse: Affine,
        viewport: [f64; 2],
        control: &mut dyn WorkControl,
    ) -> Result<CachedPaint<'a>> {
        let raw = value.slice(source.source()).unwrap();
        assert!(raw.starts_with(b"url(#") && raw.ends_with(b")"));
        let id = source
            .find_reference(
                XmlSpan {
                    offset: value.offset + 5,
                    length: value.length - 6,
                },
                control,
            )?
            .unwrap();
        self.template = Some(source.gradient_template_id(id, viewport, control)?);
        self.resolves += 1;
        self.paint_edge_counts.push(edges.len());
        if let Some(error) = self.paint_error {
            return Err(error);
        }
        // Exact fixed red/blue fixture stops remain owned by this resolver.
        self.stops = [
            PaintStop {
                offset: 0.0,
                rgba: [255, 0, 0, 255],
            },
            PaintStop {
                offset: 1.0,
                rgba: [0, 0, 255, 255],
            },
        ];
        Ok(CachedPaint::Gradient {
            template: self.template.as_ref().unwrap(),
            stops: &self.stops,
            paint: Paint {
                kind: PaintKind::Linear,
                spread: Spread::Pad,
                object_bounding_box: true,
                coordinates: [0., 0., 1., 0., 0., 0.],
                bounds: [0., 0., 1., 1.],
                user_inverse: inverse,
                gradient_inverse: identity(),
                stops: TableRange { first: 0, count: 2 },
                pattern: None,
                pattern_inverse: identity(),
                pattern_opacity: 1.0,
            },
        })
    }
}
impl DefinitionPlanner for ProductionFixture {
    fn plan_owned_definition<'s>(
        &mut self,
        c: &mut DefinitionContext<'s, '_>,
        kind: DefinitionKind,
        id: ElementId,
        owner: DefinitionOwner,
        transform: Affine,
        bounds: [f64; 4],
        _: &CssPaint<'s>,
        control: &mut dyn WorkControl,
    ) -> Result<ProductionDefinitionPlan<'s>> {
        owner.validate(c)?;
        self.plans += 1;
        let token = c.shared.tokens()[id.0];
        if kind == DefinitionKind::Filter {
            assert!(matches!(owner.target(), DefinitionTarget::Group(_)));
            if self.fail_filter {
                return Err(Diagnostic {
                    kind: ErrorKind::Unsupported,
                    offset: token.name.offset,
                });
            }
            return Ok(ProductionDefinitionPlan::Definition(DefinitionPlan {
                descriptor: DefinitionDescriptor::Filter {
                    filter: Filter {
                        primitives: TableRange::EMPTY,
                        linear_rgb: false,
                        filter_units_object_bounding_box: true,
                        primitive_units_object_bounding_box: false,
                        custom_region: false,
                        region: [0., 0., 1., 1.],
                    },
                    primitives: vec![],
                },
                expansions: vec![],
                post: PostOperation::None,
            }));
        }
        let end = c
            .shared
            .tokens()
            .iter()
            .enumerate()
            .skip(id.0 + 1)
            .find(|(_, v)| v.kind == TokenKind::EndElement && v.depth == token.depth)
            .unwrap()
            .0;
        let first = c
            .shared
            .tokens()
            .iter()
            .enumerate()
            .take(end)
            .skip(id.0 + 1)
            .find(|(_, v)| matches!(v.kind, TokenKind::StartElement | TokenKind::EmptyElement))
            .unwrap()
            .0;
        let style = c.shared.compute_style(id, None, control)?;
        let flavor = match kind {
            DefinitionKind::Clip => DecodeFlavor::Clip { clip_rule: false },
            DefinitionKind::Mask => DecodeFlavor::Mask,
            DefinitionKind::Pattern => DecodeFlavor::Pattern,
            DefinitionKind::Marker => DecodeFlavor::Marker,
            _ => unreachable!(),
        };
        let descriptor = match kind {
            DefinitionKind::Clip => DefinitionDescriptor::Clip(Clip {
                geometry: ClipGeometry {
                    edges: vec![],
                    shapes: vec![],
                },
                parent: None,
                owner_node: None,
                owner_before_suffix: false,
                object_bounding_box: false,
                bounds,
                user_inverse: identity(),
            }),
            DefinitionKind::Mask => DefinitionDescriptor::Mask(MaskDescriptor {
                luminance: true,
                region_object_bounding_box: true,
                custom_region: false,
                region: [0., 0., 1., 1.],
                region_transform: identity(),
                source_offset: token.name.offset,
            }),
            DefinitionKind::Pattern => DefinitionDescriptor::Pattern(PatternDescriptor {
                definition: id,
                width: 1,
                height: 1,
                tile_width: 2.,
                tile_height: 2.,
                content_transform: transform,
                error_offset: token.name.offset,
            }),
            DefinitionKind::Marker => DefinitionDescriptor::Marker(MarkerDescriptor {
                definition: id,
                viewport: [8., 8.],
                transform,
                error_offset: token.name.offset,
            }),
            _ => unreachable!(),
        };
        let plan = DefinitionPlan {
            descriptor,
            expansions: vec![Expansion {
                range: TableRange {
                    first,
                    count: end - first,
                },
                transform,
                viewport: [8., 8.],
                style,
                flavor: if self.invalid_flavor {
                    DecodeFlavor::Normal
                } else {
                    flavor
                },
                post: PostOperation::None,
            }],
            post: PostOperation::None,
        };
        if kind == DefinitionKind::Pattern {
            let DefinitionTarget::Shape(shape) = owner.target() else {
                panic!("shape owner required")
            };
            let map = c
                .builder
                .traversal_scene(c.scene)?
                .shape(shape)
                .unwrap()
                .source;
            Ok(ProductionDefinitionPlan::Pattern {
                plan,
                mapping: PatternMapping {
                    tile_origin: [0., 0.],
                    user_to_base: identity(),
                    pattern_transform: identity(),
                    user_inverse: vexel_svg_decode::affine_inverse(transform).unwrap(),
                    viewport: [8., 8.],
                    bounds,
                    source_map: map,
                    opacity: 1.0,
                },
            })
        } else {
            Ok(ProductionDefinitionPlan::Definition(plan))
        }
    }
}
impl PaintChannelDefinitionPlanner for ProductionFixture {
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
        let mut result = self.plan_owned_definition(
            c,
            DefinitionKind::Pattern,
            element,
            owner,
            transform,
            bounds,
            style,
            control,
        )?;
        let property = match channel {
            PaintChannel::Fill => vexel_svg_css::Property::FillOpacity,
            PaintChannel::Stroke => vexel_svg_css::Property::StrokeOpacity,
        };
        let opacity = match style.value(property) {
            Some(v) => vexel_svg_decode::parse_opacity_value_controlled(
                c.shared.source(),
                v.span,
                control,
            )?,
            None => 1.0,
        };
        if let ProductionDefinitionPlan::Pattern { mapping, .. } = &mut result {
            mapping.opacity = opacity;
        }
        Ok(result)
    }
}
impl ChannelAwareProductionDecodeOperations for ProductionFixture {}
impl vexel_svg_decode::DecoderProductionOperations for ProductionFixture {
    fn decode_text_scoped(
        &mut self,
        r: TextDecodeRequest<'_, '_>,
        b: &mut SceneBuilder,
        _: &DraftToken,
        c: &mut dyn WorkControl,
    ) -> Result<bool> {
        MediaDecodeOperations::decode_text(self, r, b, c)
    }
    fn decode_image_scoped(
        &mut self,
        r: ImageDecodeRequest<'_, '_>,
        b: &mut SceneBuilder,
        _: &DraftToken,
        c: &mut dyn WorkControl,
    ) -> Result<ShapeId> {
        MediaDecodeOperations::decode_image(self, r, b, c)
    }
}
impl MediaDecodeOperations for ProductionFixture {
    fn decode_text(
        &mut self,
        r: TextDecodeRequest<'_, '_>,
        b: &mut SceneBuilder,
        w: &mut dyn WorkControl,
    ) -> Result<bool> {
        RejectOperations.decode_text(r, b, w)
    }
    fn decode_image(
        &mut self,
        r: ImageDecodeRequest<'_, '_>,
        b: &mut SceneBuilder,
        w: &mut dyn WorkControl,
    ) -> Result<ShapeId> {
        RejectOperations.decode_image(r, b, w)
    }
}
impl ProductionDecodeOperations for ProductionFixture {}
impl ScopedMediaDecodeOperations for ProductionFixture {
    fn decode_text(
        &mut self,
        request: TextDecodeRequest<'_, '_>,
        builder: &mut SceneBuilder,
        draft: &DraftToken,
        control: &mut dyn WorkControl,
    ) -> Result<bool> {
        let _ = (builder, draft, control);
        Err(Diagnostic {
            kind: ErrorKind::Unsupported,
            offset: request.source_map.element.start,
        })
    }
    fn decode_image(
        &mut self,
        request: ImageDecodeRequest<'_, '_>,
        builder: &mut SceneBuilder,
        draft: &DraftToken,
        control: &mut dyn WorkControl,
    ) -> Result<ShapeId> {
        let _ = (builder, draft, control);
        Err(Diagnostic {
            kind: ErrorKind::Unsupported,
            offset: request.source_map.element.start,
        })
    }
}
impl ScopedProductionDecodeOperations for ProductionFixture {}
pub fn output(
    job: &mut RasterJob,
    generation: u64,
    control: &mut dyn WorkControl,
) -> JobRenderOutput {
    let pixels = job
        .buffer(RasterPool::Output, 16 * 1024 * 1024, control)
        .unwrap();
    job.reserve(pixels, 4, control).unwrap();
    job.extend(pixels, &[255, 0, 0, 255], control).unwrap();
    JobRenderOutput {
        generation,
        pixels,
        width: 1,
        height: 1,
        stride: 4,
    }
}
pub fn run_production(input: &str, fixture: &mut ProductionFixture) -> Result<(SceneGraph, usize)> {
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
    let mut patterns = 0;
    loop {
        match d.step_production(64, fixture, &job, &mut run)? {
            DecodeEvent::NeedPattern(request) => {
                fixture.pattern_children.push(request.child.id);
                let ticket = request.ticket;
                assert_eq!((request.width, request.height), (1, 1));
                let pixels = output(&mut job, 17, &mut run);
                d.resume_pattern_production(ticket, &mut job, Ok(pixels), &mut run)?;
                patterns += 1;
            }
            DecodeEvent::Progress { charged_units } => assert!(charged_units > 0),
            DecodeEvent::Complete => break,
        }
    }
    let graph = d.finish(&mut run)?;
    assert!(Arc::ptr_eq(graph.source_owner(), &source));
    assert_eq!(job.stats().output_bytes, 0);
    Ok((graph, patterns))
}

impl Default for ProductionFixture {
    fn default() -> Self {
        Self {
            template: None,
            resolves: 0,
            paint_error: None,
            paint_edge_counts: Vec::new(),
            plans: 0,
            invalid_flavor: false,
            fail_filter: false,
            pattern_children: Vec::new(),
            stops: [PaintStop {
                offset: 0.,
                rgba: [0; 4],
            }; 2],
        }
    }
}
