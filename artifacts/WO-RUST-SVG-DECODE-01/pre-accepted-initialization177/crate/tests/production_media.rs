#[path = "support/production.rs"]
mod production;
mod support;
use production::ProductionFixture;
use std::sync::Arc;
use support::Run;
use vexel_scene::*;
use vexel_svg_decode::{Decoder, GeometryBuilder};
use vexel_xml::{TokenKind, XmlSpan};
struct FixtureProvider {
    calls: usize,
}
impl MediaProvider for FixtureProvider {
    fn layout_text(
        &mut self,
        r: TextRequest<'_>,
        c: &mut dyn WorkControl,
    ) -> std::result::Result<TextLayout, MediaError> {
        r.validate(c).map_err(|_| MediaError::Invalid)?;
        if r.text != b"x" && r.text != b"y" {
            return Err(MediaError::Unsupported);
        }
        self.calls += 1;
        Ok(TextLayout {
            glyphs: vec![TextGlyph {
                glyph_id: Some(1),
                x: 0.,
                y: 0.,
                advance: 1.,
                commands: TableRange { first: 0, count: 4 },
            }],
            commands: vec![
                TextCommand {
                    verb: TextVerb::Move,
                    values: [0.; 6],
                },
                TextCommand {
                    verb: TextVerb::Line,
                    values: [1., 0., 0., 0., 0., 0.],
                },
                TextCommand {
                    verb: TextVerb::Line,
                    values: [0., 1., 0., 0., 0., 0.],
                },
                TextCommand {
                    verb: TextVerb::Close,
                    values: [0.; 6],
                },
            ],
            advance_width: 1.,
            ascent: 1.,
            descent: 0.,
            leading: 0.,
        })
    }
    fn decode_image(
        &mut self,
        encoded: &[u8],
        c: &mut dyn WorkControl,
    ) -> std::result::Result<ImagePixels, MediaError> {
        c.checkpoint().map_err(|_| MediaError::Invalid)?;
        if encoded != b"fixture-image" {
            return Err(MediaError::Unsupported);
        }
        self.calls += 1;
        Ok(ImagePixels {
            rgba: vec![16, 32, 48, 255],
            byte_count: 4,
            row_stride: 4,
            width: 1,
            height: 1,
        })
    }
}
struct FixtureMedia {
    lower: ProductionFixture,
    seen: Vec<SourceMap>,
}
impl CachedPaintResolver for FixtureMedia {
    fn resolve_cached<'a>(
        &'a mut self,
        s: &mut DecodeSource<'_>,
        l: &mut ChargeLedger,
        v: XmlSpan,
        e: &[Edge],
        t: Affine,
        p: [f64; 2],
        c: &mut dyn WorkControl,
    ) -> Result<CachedPaint<'a>> {
        self.lower.resolve_cached(s, l, v, e, t, p, c)
    }
}
impl DefinitionPlanner for FixtureMedia {
    fn plan_owned_definition<'s>(
        &mut self,
        c: &mut DefinitionContext<'s, '_>,
        k: DefinitionKind,
        e: ElementId,
        o: DefinitionOwner,
        t: Affine,
        b: [f64; 4],
        s: &CssPaint<'s>,
        w: &mut dyn WorkControl,
    ) -> Result<ProductionDefinitionPlan<'s>> {
        self.lower.plan_owned_definition(c, k, e, o, t, b, s, w)
    }
}
impl MediaDecodeOperations for FixtureMedia {
    fn decode_text(
        &mut self,
        r: TextDecodeRequest<'_, '_>,
        b: &mut SceneBuilder,
        c: &mut dyn WorkControl,
    ) -> Result<bool> {
        r.validate_source_map(b, c)?;
        self.seen.push(r.source_map);
        let own = r.element.tokens()[r.element.element.0];
        for token in &r.element.tokens()[r.element.element.0 + 1..r.end] {
            if token.kind != TokenKind::Text || token.depth != own.depth + 1 {
                continue;
            }
            let bytes = token.value.slice(r.element.source()).unwrap();
            if bytes.is_empty() {
                continue;
            }
            let layout = r
                .provider
                .layout_text(
                    TextRequest {
                        text: bytes,
                        font_families: &[],
                        size_points: 12.,
                        weight: 400.,
                        style: TextStyle::Normal,
                        letter_spacing_points: 0.,
                    },
                    c,
                )
                .map_err(|_| Diagnostic {
                    kind: ErrorKind::Unsupported,
                    offset: token.value.offset,
                })?;
            layout.validate(c)?;
            // This fixture proves source/provider routing, not the later native command-use admission gate.
            let mut geometry = GeometryBuilder::new(r.transform, own.name.offset)?;
            let points = [
                Point {
                    x: r.state.cursor_x,
                    y: 0.,
                },
                Point {
                    x: r.state.cursor_x + 1.,
                    y: 0.,
                },
                Point {
                    x: r.state.cursor_x,
                    y: 1.,
                },
            ];
            for i in 0..3 {
                geometry.add_edge(points[i], points[(i + 1) % 3], c)?;
            }
            let g = geometry.finish(true, true, 0);
            let root = b.scene(r.scene)?.viewport;
            let style = vexel_svg_decode::parse_style(
                r.element.source(),
                r.style,
                8.,
                r.transform,
                root,
                c,
            )?;
            let mut shape = Shape::new(r.source_map);
            shape.filled = true;
            b.append_shape(
                r.scene,
                ShapeInput {
                    shape,
                    path: g.path,
                    edge_colors: vec![style.fill; 3],
                },
                c,
            )?;
            r.state.cursor_x += layout.advance_width;
            r.state.glyph_count += layout.glyphs.len();
        }
        Ok(r.preserve_space)
    }
    fn decode_image(
        &mut self,
        r: ImageDecodeRequest<'_, '_>,
        b: &mut SceneBuilder,
        c: &mut dyn WorkControl,
    ) -> Result<ShapeId> {
        r.validate_source_map(b, c)?;
        self.seen.push(r.source_map);
        let href = vexel_svg_decode::source::attribute(
            r.element.source(),
            r.element.tokens(),
            r.element.element.0,
            b"href",
            None,
        )
        .unwrap();
        let pixels = r
            .provider
            .decode_image(href.slice(r.element.source()).unwrap(), c)
            .map_err(|_| Diagnostic {
                kind: ErrorKind::Unsupported,
                offset: href.offset,
            })?;
        pixels.validate(href.offset)?;
        let lease = r.ledger.admit_vec(AllocationTag::None, &pixels.rgba)?;
        let image = b.append_image(
            r.scene,
            Image {
                pixels,
                x: 0.,
                y: 0.,
                width: 1.,
                height: 1.,
                draw_x: 0.,
                draw_y: 0.,
                draw_width: 1.,
                draw_height: 1.,
            },
            c,
        );
        r.ledger.release(lease)?;
        let mut geometry = GeometryBuilder::new(r.transform, href.offset)?;
        let p = [
            Point { x: 0., y: 0. },
            Point { x: 1., y: 0. },
            Point { x: 1., y: 1. },
            Point { x: 0., y: 1. },
        ];
        for i in 0..4 {
            geometry.add_edge(p[i], p[(i + 1) % 4], c)?;
        }
        let g = geometry.finish(true, true, 0);
        let mut shape = Shape::new(r.source_map);
        shape.image = Some(image?);
        shape.filled = true;
        b.append_shape(
            r.scene,
            ShapeInput {
                shape,
                path: g.path,
                edge_colors: vec![[255; 4]; 4],
            },
            c,
        )
    }
}
impl ProductionDecodeOperations for FixtureMedia {}
#[test]
fn nested_use_media_calls_actual_fixture_provider_and_preserves_host_maps() {
    let input="<svg width=\"8\" height=\"8\"><defs><g id=\"target\"><text fill=\"red\">x<tspan>y</tspan></text><image href=\"fixture-image\" width=\"1\" height=\"1\"/></g></defs><g transform=\"translate(2 3)\"><use href=\"#target\"/></g></svg>";
    let mut c = Run::default();
    let owner = Arc::new(input.as_bytes().to_vec());
    let doc = LexicalDocument::parse(owner.clone(), 1 << 20, &mut c).unwrap();
    let mut shared = DecodeSource::new(&doc, &mut c).unwrap();
    let mut provider = FixtureProvider { calls: 0 };
    let mut job = RasterJob::new(ChargeLedger::new(&owner, 1 << 24, 65536).unwrap()).unwrap();
    let mut d = Decoder::from_source(
        &doc,
        &mut shared,
        1,
        DecodeOptions {
            limits: Limits::default(),
            system_language: b"en",
            media: Some(&mut provider),
        },
        &mut c,
    )
    .unwrap();
    let mut ops = FixtureMedia {
        lower: Default::default(),
        seen: vec![],
    };
    loop {
        match d.step_production(1, &mut ops, &job, &mut c).unwrap() {
            DecodeEvent::Complete => break,
            DecodeEvent::Progress { charged_units } => assert!(charged_units > 0),
            _ => panic!("unexpected pattern"),
        }
    }
    let g = d.finish(&mut c).unwrap();
    assert_eq!(provider.calls, 3);
    assert_eq!(g.root().shapes.len(), 3);
    assert_eq!(g.root().images.len(), 1);
    let host = input.find("<use").unwrap();
    assert!(ops.seen.iter().all(|m| m.element.start == host));
    assert!(g
        .root()
        .shapes
        .iter()
        .all(|s| s.source.element.start == host));
    assert_eq!(g.root().edges[0].a, Point { x: 2., y: 3. });
    job.release_all().unwrap();
}
