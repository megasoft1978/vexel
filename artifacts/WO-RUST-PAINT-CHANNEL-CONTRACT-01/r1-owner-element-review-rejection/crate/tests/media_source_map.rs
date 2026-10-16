mod support;
use std::sync::Arc;
use support::*;
use vexel_scene::*;
use vexel_xml::TokenKind;

struct FixtureProvider(usize);
impl MediaProvider for FixtureProvider {
    fn layout_text(
        &mut self,
        _: TextRequest<'_>,
        _: &mut dyn WorkControl,
    ) -> std::result::Result<TextLayout, MediaError> {
        self.0 += 1;
        Err(MediaError::Unsupported)
    }
    fn decode_image(
        &mut self,
        _: &[u8],
        _: &mut dyn WorkControl,
    ) -> std::result::Result<ImagePixels, MediaError> {
        self.0 += 1;
        Err(MediaError::Unsupported)
    }
}
fn fixture() -> (LexicalDocument, SceneBuilder) {
    let source = Arc::new(
        br##"<svg><text id="t">Hi</text><image id="i"/><use href="#t"/><use href="#i"/></svg>"##
            .to_vec(),
    );
    let lex = LexicalDocument::parse(source.clone(), 1 << 20, &mut Run).unwrap();
    let animation = vexel_svg_animation::parse_owned(&source, lex.tokens()).unwrap();
    let builder = SceneBuilder::new(source, animation, 7, viewport(), Limits::default()).unwrap();
    (lex, builder)
}
fn element_index(lex: &LexicalDocument, name: &[u8]) -> usize {
    lex.tokens()
        .iter()
        .position(|t| {
            matches!(t.kind, TokenKind::StartElement | TokenKind::EmptyElement)
                && t.local_name.slice(lex.source_owner()) == Some(name)
        })
        .unwrap()
}
fn host(lex: &LexicalDocument) -> SourceMap {
    let token = &lex.tokens()[element_index(lex, b"use")];
    SourceMap {
        element: Span {
            start: token.source.offset,
            end: token.source.offset + token.source.length,
        },
        transform: vexel_geometry::affine_identity(),
        transform_node: None,
    }
}

#[test]
fn referenced_text_keeps_distinct_use_host_annotation_and_instance_transform() {
    let (lex, mut b) = fixture();
    let root = b.root_id();
    let node = b.append_transform(root, transform(None), &mut Run).unwrap();
    let shared = DecodeSource::new(&lex, &mut Run).unwrap();
    let element = shared
        .element(
            ElementId(element_index(&lex, b"text")),
            lex.tokens().len(),
            &mut Run,
        )
        .unwrap();
    let mut map = host(&lex);
    map.transform.e = 3.;
    map.transform_node = Some(node);
    let style = CssPaint::initial();
    let mut state = TextState::default();
    let mut cache = TextCache::default();
    let mut provider = FixtureProvider(0);
    let mut ledger = ChargeLedger::new(lex.source_owner(), 1 << 20, 8).unwrap();
    let request = TextDecodeRequest {
        element,
        source_map: map,
        end: element.end,
        style: &style,
        transform: vexel_geometry::affine_identity(),
        state: &mut state,
        preserve_space: false,
        provider: &mut provider,
        cache: &mut cache,
        scene: root,
        ledger: &mut ledger,
    };
    request.validate_source_map(&b, &mut Run).unwrap();
    assert_ne!(
        request.element.tokens()[request.element.element.0]
            .source
            .offset,
        map.element.start
    );
    assert_eq!(request.source_map.transform.e, 3.);
    assert_eq!(request.transform.e, 0.);
    assert_eq!(
        request.element.source().as_ptr(),
        lex.source_owner().as_slice().as_ptr()
    );
    drop(request);
    assert_eq!(provider.0, 0);
}

#[test]
fn image_maps_validate_before_provider_and_reject_bad_span_transform_and_cancel() {
    let (lex, b) = fixture();
    let root = b.root_id();
    let shared = DecodeSource::new(&lex, &mut Run).unwrap();
    let element = shared
        .element(
            ElementId(element_index(&lex, b"image")),
            lex.tokens().len(),
            &mut Run,
        )
        .unwrap();
    let style = CssPaint::initial();
    let mut provider = FixtureProvider(0);
    let mut ledger = ChargeLedger::new(lex.source_owner(), 1 << 20, 8).unwrap();
    let mut request = ImageDecodeRequest {
        element,
        source_map: host(&lex),
        style: &style,
        transform: vexel_geometry::affine_identity(),
        provider: &mut provider,
        scene: root,
        ledger: &mut ledger,
    };
    request.validate_source_map(&b, &mut Run).unwrap();
    let original = request.source_map;
    request.source_map.element.end = usize::MAX;
    assert_eq!(
        request.validate_source_map(&b, &mut Run).unwrap_err().kind,
        ErrorKind::InvalidArgument
    );
    request.source_map = original;
    request.source_map.transform.a = f64::NAN;
    assert!(request.validate_source_map(&b, &mut Run).is_err());
    request.source_map = original;
    request.transform.a = f64::NAN;
    assert!(request.validate_source_map(&b, &mut Run).is_err());
    request.transform = vexel_geometry::affine_identity();
    assert_eq!(
        request
            .validate_source_map(&b, &mut StopAfter(0))
            .unwrap_err()
            .kind,
        ErrorKind::Canceled
    );
    assert_eq!(
        request
            .validate_source_map(&b, &mut StopAfter(1))
            .unwrap_err()
            .kind,
        ErrorKind::Canceled
    );
    drop(request);
    assert_eq!(provider.0, 0);
}

#[test]
fn foreign_source_ledger_scene_stale_and_wrong_scene_transform_are_rejected() {
    let (lex, mut b) = fixture();
    let root = b.root_id();
    let shared = DecodeSource::new(&lex, &mut Run).unwrap();
    let element = shared
        .element(
            ElementId(element_index(&lex, b"image")),
            lex.tokens().len(),
            &mut Run,
        )
        .unwrap();
    let marker = b.mark(root).unwrap();
    let stale = b.append_transform(root, transform(None), &mut Run).unwrap();
    b.rollback(marker).unwrap();
    let fresh = b.append_transform(root, transform(None), &mut Run).unwrap();
    assert_ne!(stale, fresh);
    let child = b.new_scene(viewport()).unwrap();
    let wrong_scene = b
        .append_transform(child, transform(None), &mut Run)
        .unwrap();
    let style = CssPaint::initial();
    let mut provider = FixtureProvider(0);
    let mut ledger = ChargeLedger::new(lex.source_owner(), 1 << 20, 8).unwrap();
    let mut request = ImageDecodeRequest {
        element,
        source_map: host(&lex),
        style: &style,
        transform: vexel_geometry::affine_identity(),
        provider: &mut provider,
        scene: root,
        ledger: &mut ledger,
    };
    for id in [stale, wrong_scene] {
        request.source_map.transform_node = Some(id);
        assert!(request.validate_source_map(&b, &mut Run).is_err());
    }
    request.source_map.transform_node = Some(fresh);
    request.validate_source_map(&b, &mut Run).unwrap();
    let other = builder();
    request.scene = other.root_id();
    assert!(request.validate_source_map(&b, &mut Run).is_err());
    request.scene = root;
    let (same_bytes_foreign, _) = fixture();
    let mut foreign_ledger =
        ChargeLedger::new(same_bytes_foreign.source_owner(), 1 << 20, 8).unwrap();
    request.ledger = &mut foreign_ledger;
    assert!(request.validate_source_map(&b, &mut Run).is_err());
    drop(request);
    let mut request = ImageDecodeRequest {
        element,
        source_map: host(&lex),
        style: &style,
        transform: vexel_geometry::affine_identity(),
        provider: &mut provider,
        scene: root,
        ledger: &mut ledger,
    };
    let foreign_context = DecodeSource::new(&same_bytes_foreign, &mut Run).unwrap();
    request.element = foreign_context
        .element(
            ElementId(element_index(&same_bytes_foreign, b"image")),
            same_bytes_foreign.tokens().len(),
            &mut Run,
        )
        .unwrap();
    assert!(request.validate_source_map(&b, &mut Run).is_err());
    drop(request);
    assert_eq!(provider.0, 0);
}

#[test]
fn standalone_annotations_and_paused_or_sealed_targets_are_checked() {
    let (lex, mut b) = fixture();
    let root = b.root_id();
    let shared = DecodeSource::new(&lex, &mut Run).unwrap();
    let element = shared
        .element(
            ElementId(element_index(&lex, b"image")),
            lex.tokens().len(),
            &mut Run,
        )
        .unwrap();
    let token = element.tokens()[element.element.0];
    let map = SourceMap {
        element: Span {
            start: token.source.offset,
            end: token.source.offset + token.source.length,
        },
        transform: vexel_geometry::affine_identity(),
        transform_node: None,
    };
    let style = CssPaint::initial();
    let mut provider = FixtureProvider(0);
    let mut ledger = ChargeLedger::new(lex.source_owner(), 1 << 20, 8).unwrap();
    let mut request = ImageDecodeRequest {
        element,
        source_map: map,
        style: &style,
        transform: vexel_geometry::affine_identity(),
        provider: &mut provider,
        scene: root,
        ledger: &mut ledger,
    };
    request.validate_source_map(&b, &mut Run).unwrap();
    let parent = b.begin_draft(root).unwrap();
    let child = b.new_scene(viewport()).unwrap();
    let nested = b.begin_draft(child).unwrap();
    assert!(request.validate_source_map(&b, &mut Run).is_err());
    request.scene = child;
    request.validate_source_map(&b, &mut Run).unwrap();
    b.commit_draft(&nested, &mut Run).unwrap();
    b.seal(child, &mut Run).unwrap();
    assert!(request.validate_source_map(&b, &mut Run).is_err());
    request.scene = root;
    request.validate_source_map(&b, &mut Run).unwrap();
    b.rollback_draft(parent).unwrap();
    drop(request);
    assert_eq!(provider.0, 0);
}
