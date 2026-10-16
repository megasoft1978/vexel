use std::sync::Arc;
use vexel_base::{Stop, WorkControl, WorkKind};
use vexel_scene::{
    ChargeLedger, CssPaint, ElementId, ImagePixels, LexicalDocument, Limits, MediaError,
    MediaProvider, SceneBuilder, SourceMap, TableRange, TextCommand, TextDecodeRequest, TextGlyph,
    TextLayout, TextRequest, TextState, TextVerb, Viewport,
};
use vexel_svg_media::{decode_image_request, render_text_request};
use vexel_xml::TokenKind;

struct Run;
impl WorkControl for Run {
    fn charge(&mut self, _: WorkKind, _: u64) -> Result<(), Stop> {
        Ok(())
    }
    fn checkpoint(&mut self) -> Result<(), Stop> {
        Ok(())
    }
}
struct Cancel;
impl WorkControl for Cancel {
    fn charge(&mut self, _: WorkKind, _: u64) -> Result<(), Stop> {
        Err(Stop::Canceled)
    }
    fn checkpoint(&mut self) -> Result<(), Stop> {
        Err(Stop::Canceled)
    }
}

#[derive(Default)]
struct FixtureProvider {
    text_calls: usize,
    image_calls: usize,
}
impl MediaProvider for FixtureProvider {
    fn layout_text(
        &mut self,
        _: TextRequest<'_>,
        _: &mut dyn WorkControl,
    ) -> std::result::Result<TextLayout, MediaError> {
        self.text_calls += 1;
        Ok(TextLayout {
            glyphs: vec![TextGlyph {
                glyph_id: Some(1),
                x: 0.,
                y: 0.,
                advance: 6.,
                commands: TableRange { first: 0, count: 5 },
            }],
            commands: vec![
                TextCommand {
                    verb: TextVerb::Move,
                    values: [0., 0., 0., 0., 0., 0.],
                },
                TextCommand {
                    verb: TextVerb::Line,
                    values: [5., 0., 0., 0., 0., 0.],
                },
                TextCommand {
                    verb: TextVerb::Line,
                    values: [5., 8., 0., 0., 0., 0.],
                },
                TextCommand {
                    verb: TextVerb::Line,
                    values: [0., 8., 0., 0., 0., 0.],
                },
                TextCommand {
                    verb: TextVerb::Close,
                    values: [0.; 6],
                },
            ],
            advance_width: 6.,
            ascent: 8.,
            descent: 2.,
            leading: 0.,
        })
    }
    fn decode_image(
        &mut self,
        _: &[u8],
        _: &mut dyn WorkControl,
    ) -> std::result::Result<ImagePixels, MediaError> {
        self.image_calls += 1;
        Ok(ImagePixels {
            rgba: vec![20, 40, 60, 255],
            byte_count: 4,
            row_stride: 4,
            width: 1,
            height: 1,
        })
    }
}

fn fixture() -> (LexicalDocument, SceneBuilder, usize, usize, SourceMap) {
    let source = Arc::new(
        br##"<svg><text x="4" y="12">A&amp;B</text><image x="2" y="3" width="4" height="5" href="data:image/png;base64,AQID"/><use href="#t"/></svg>"##.to_vec(),
    );
    let mut control = Run;
    let lex = LexicalDocument::parse(source.clone(), 1 << 20, &mut control).unwrap();
    let animation = vexel_svg_animation::parse_owned(&source, lex.tokens()).unwrap();
    let builder = SceneBuilder::new(
        source,
        animation,
        1,
        Viewport {
            width: 100,
            height: 100,
            view_x: 0.,
            view_y: 0.,
            view_width: 100.,
            view_height: 100.,
            aspect: Default::default(),
        },
        Limits::default(),
    )
    .unwrap();
    let text = lex
        .tokens()
        .iter()
        .position(|t| {
            t.kind == TokenKind::StartElement
                && t.local_name.slice(lex.source_owner()) == Some(b"text")
        })
        .unwrap();
    let image = lex
        .tokens()
        .iter()
        .position(|t| {
            t.kind == TokenKind::EmptyElement
                && t.local_name.slice(lex.source_owner()) == Some(b"image")
        })
        .unwrap();
    let host = lex
        .tokens()
        .iter()
        .find(|t| {
            t.kind == TokenKind::EmptyElement
                && t.local_name.slice(lex.source_owner()) == Some(b"use")
        })
        .unwrap();
    let source_map = SourceMap {
        element: vexel_base::Span {
            start: host.source.offset,
            end: host.source.offset + host.source.length,
        },
        transform: vexel_geometry::Affine {
            e: 9.,
            ..vexel_geometry::affine_identity()
        },
        transform_node: None,
    };
    (lex, builder, text, image, source_map)
}

#[test]
fn typed_text_and_image_requests_append_with_host_annotation() {
    let (lex, mut builder, text_index, image_index, map) = fixture();
    let scene = builder.root_id();
    let mut control = Run;
    let shared = vexel_scene::DecodeSource::new(&lex, &mut control).unwrap();
    let text = shared
        .element(ElementId(text_index), lex.tokens().len(), &mut control)
        .unwrap();
    let image = shared
        .element(ElementId(image_index), lex.tokens().len(), &mut control)
        .unwrap();
    let mut ledger = ChargeLedger::new(lex.source_owner(), 1 << 20, 8).unwrap();
    let style = CssPaint::initial();
    let mut state = TextState::default();
    let mut cache = vexel_scene::TextCache::default();
    let mut provider = FixtureProvider::default();
    let draft = builder.begin_draft(scene).unwrap();
    let text_request = TextDecodeRequest {
        element: text,
        source_map: map,
        end: text.end,
        style: &style,
        transform: vexel_geometry::affine_identity(),
        state: &mut state,
        preserve_space: false,
        provider: &mut provider,
        cache: &mut cache,
        scene,
        ledger: &mut ledger,
    };
    assert!(!render_text_request(text_request, &mut builder, &draft, &mut control).unwrap());
    let text_id = builder.scene(scene).unwrap().shape_id(0).unwrap();
    let text_shape = builder.scene(scene).unwrap().shape(text_id).unwrap();
    assert_eq!(text_shape.source, map);
    assert_eq!(
        (cache.entries.len(), cache.text_bytes, cache.commands),
        (1, 3, 5)
    );

    let image_request = vexel_scene::ImageDecodeRequest {
        element: image,
        source_map: map,
        style: &style,
        transform: vexel_geometry::affine_identity(),
        provider: &mut provider,
        scene,
        ledger: &mut ledger,
    };
    let image_id = decode_image_request(image_request, &mut builder, &draft, &mut control).unwrap();
    assert_eq!(
        builder
            .scene(scene)
            .unwrap()
            .shape(image_id)
            .unwrap()
            .source,
        map
    );
    assert_eq!((provider.text_calls, provider.image_calls), (1, 1));
    builder.rollback_draft(draft).unwrap();
}

#[test]
fn invalid_typed_source_map_stops_before_provider_or_append() {
    let (lex, mut builder, _, image_index, mut map) = fixture();
    let scene = builder.root_id();
    map.element.end = usize::MAX;
    let mut control = Run;
    let shared = vexel_scene::DecodeSource::new(&lex, &mut control).unwrap();
    let image = shared
        .element(ElementId(image_index), lex.tokens().len(), &mut control)
        .unwrap();
    let mut ledger = ChargeLedger::new(lex.source_owner(), 1 << 20, 8).unwrap();
    let style = CssPaint::initial();
    let mut provider = FixtureProvider::default();
    let draft = builder.begin_draft(scene).unwrap();
    let request = vexel_scene::ImageDecodeRequest {
        element: image,
        source_map: map,
        style: &style,
        transform: vexel_geometry::affine_identity(),
        provider: &mut provider,
        scene,
        ledger: &mut ledger,
    };
    assert!(decode_image_request(request, &mut builder, &draft, &mut control).is_err());
    assert_eq!(provider.image_calls, 0);
    assert_eq!(builder.scene(scene).unwrap().shapes.len(), 0);
    assert_eq!(builder.scene(scene).unwrap().images.len(), 0);
    builder.rollback_draft(draft).unwrap();
}

#[test]
fn canceled_typed_request_stops_before_provider_or_append() {
    let (lex, mut builder, _, image_index, map) = fixture();
    let scene = builder.root_id();
    let mut setup = Run;
    let shared = vexel_scene::DecodeSource::new(&lex, &mut setup).unwrap();
    let image = shared
        .element(ElementId(image_index), lex.tokens().len(), &mut setup)
        .unwrap();
    let mut ledger = ChargeLedger::new(lex.source_owner(), 1 << 20, 8).unwrap();
    let style = CssPaint::initial();
    let mut provider = FixtureProvider::default();
    let draft = builder.begin_draft(scene).unwrap();
    let request = vexel_scene::ImageDecodeRequest {
        element: image,
        source_map: map,
        style: &style,
        transform: vexel_geometry::affine_identity(),
        provider: &mut provider,
        scene,
        ledger: &mut ledger,
    };
    assert_eq!(
        decode_image_request(request, &mut builder, &draft, &mut Cancel)
            .unwrap_err()
            .kind,
        vexel_scene::ErrorKind::Canceled
    );
    assert_eq!(provider.image_calls, 0);
    assert_eq!(builder.scene(scene).unwrap().shapes.len(), 0);
    assert_eq!(builder.scene(scene).unwrap().images.len(), 0);
    builder.rollback_draft(draft).unwrap();
}
