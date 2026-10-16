//! Canonical path provenance regressions run beside Scene's private validators.
use crate::*;

struct Run;
impl WorkControl for Run {
    fn charge(&mut self, _: WorkKind, _: u64) -> std::result::Result<(), Stop> {
        Ok(())
    }
    fn checkpoint(&mut self) -> std::result::Result<(), Stop> {
        Ok(())
    }
}

fn viewport() -> Viewport {
    Viewport {
        width: 100,
        height: 100,
        view_x: 0.,
        view_y: 0.,
        view_width: 100.,
        view_height: 100.,
        aspect: Default::default(),
    }
}
fn animation(source: &SourceOwner) -> OwnedAnimation {
    let (tokens, r) = vexel_xml::tokenize(source, vexel_xml::XmlLimits::default());
    assert_eq!(r.diagnostic.category, vexel_xml::XmlError::None);
    vexel_svg_animation::parse_owned(source, &tokens[..r.token_count]).unwrap()
}
fn path_shape(source: &[u8]) -> ShapeInput {
    let start = source.windows(2).position(|x| x == b"M0").unwrap();
    let end = start + b"M0 0 L1 1 2 3 Z".len();
    let path = vexel_svg_path::parse_path(
        source,
        Span { start, end },
        vexel_geometry::affine_identity(),
        Default::default(),
    )
    .unwrap();
    let edge_colors = (0..path.edges.len())
        .map(|i| [i as u8, 2, 3, 255])
        .collect();
    ShapeInput {
        shape: Shape::new(SourceMap {
            element: Span { start: 5, end: 31 },
            transform: vexel_geometry::affine_identity(),
            transform_node: None,
        }),
        path,
        edge_colors,
    }
}
fn shape(source: &[u8]) -> ShapeInput {
    path_shape(source)
}
fn draft_request(
    b: &mut SceneBuilder,
    control: &mut dyn WorkControl,
) -> (DraftToken, RequestToken) {
    let draft = b.begin_draft(b.root_id()).unwrap();
    let request = b.begin_request(&draft, control).unwrap();
    (draft, request)
}
fn finish_request(
    mut builder: SceneBuilder,
    draft: DraftToken,
    request: RequestToken,
    control: &mut dyn WorkControl,
) -> SceneGraph {
    builder.commit_request(&draft, request, control).unwrap();
    builder.commit_draft(&draft, control).unwrap();
    builder.freeze().unwrap()
}

#[test]
fn canonical_path_owner_stays_distinct_from_use_host_through_freeze() {
    let source = std::sync::Arc::new(
        br##"<svg><path id="p" d="M0 0 L1 1 2 3 Z"></path><use href="#p"/></svg>"##.to_vec(),
    );
    let mut c = Run;
    let document = LexicalDocument::parse(source.clone(), 1 << 20, &mut c).unwrap();
    let decoded = DecodeSource::new(&document, &mut c).unwrap();
    let element = document
        .tokens()
        .iter()
        .enumerate()
        .find(|(_, token)| {
            matches!(
                token.kind,
                vexel_xml::TokenKind::StartElement | vexel_xml::TokenKind::EmptyElement
            ) && token
                .local_name
                .slice(&source)
                .is_some_and(|name| name == b"path")
        })
        .map(|(index, _)| ElementId(index))
        .unwrap();
    let view = decoded
        .element(element, document.tokens().len(), &mut c)
        .unwrap();
    let geometry = decoded.canonical_path_source(view, &mut c).unwrap();
    let use_start = source.windows(4).position(|w| w == b"<use").unwrap();
    let use_end = source[use_start..]
        .windows(2)
        .position(|w| w == b"/>")
        .map(|n| use_start + n + 2)
        .unwrap();
    let mut input = shape(&source);
    input.shape.source.element = Span {
        start: use_start,
        end: use_end,
    };
    let mut b = SceneBuilder::new(
        source.clone(),
        animation(&source),
        7,
        viewport(),
        Limits::default(),
    )
    .unwrap();
    let (draft, request) = draft_request(&mut b, &mut c);
    let token = b
        .admit_shape_with_source(&draft, request, use_start, geometry, &mut c)
        .unwrap();
    let id = b
        .append_admitted_shape(&draft, &token, input, &mut c)
        .unwrap();
    let graph = finish_request(b, draft, request, &mut c);
    let view = graph.root();
    let geometry = view.shape_geometry_source(id).unwrap();
    assert_eq!(geometry.element(), element);
    assert_eq!(
        geometry.element_range().slice(&source).unwrap(),
        b"<path id=\"p\" d=\"M0 0 L1 1 2 3 Z\"></path>"
    );
    assert_eq!(
        geometry.path_data().slice(&source).unwrap(),
        b"M0 0 L1 1 2 3 Z"
    );
    assert_eq!(
        view.shape(id).unwrap().source.element,
        Span {
            start: use_start,
            end: use_end
        }
    );
    let raster = RasterInput::ready(view);
    assert_eq!(raster.shape_geometry_source(id), Some(geometry));
}

#[test]
fn canonical_geometry_is_reused_for_multiple_transformed_use_instances() {
    let source = std::sync::Arc::new(
        br##"<svg><path id="p" d="M0 0 L1 1 2 3 Z"/><use href="#p"/><use href="#p"/></svg>"##
            .to_vec(),
    );
    let mut c = Run;
    let document = LexicalDocument::parse(source.clone(), 1 << 20, &mut c).unwrap();
    let decoded = DecodeSource::new(&document, &mut c).unwrap();
    let path_element = document
        .tokens()
        .iter()
        .enumerate()
        .find(|(_, token)| {
            matches!(
                token.kind,
                vexel_xml::TokenKind::StartElement | vexel_xml::TokenKind::EmptyElement
            ) && token
                .local_name
                .slice(&source)
                .is_some_and(|name| name == b"path")
        })
        .map(|(index, _)| ElementId(index))
        .unwrap();
    let path_view = decoded
        .element(path_element, document.tokens().len(), &mut c)
        .unwrap();
    let geometry = decoded.canonical_path_source(path_view, &mut c).unwrap();
    let data_start = source
        .windows(4)
        .position(|bytes| bytes == b" d=\"")
        .unwrap()
        + 4;
    let path_data = Span {
        start: data_start,
        end: data_start + b"M0 0 L1 1 2 3 Z".len(),
    };
    let use_ranges: Vec<_> = source
        .windows(4)
        .enumerate()
        .filter(|(_, bytes)| *bytes == b"<use")
        .map(|(start, _)| {
            let end = source[start..]
                .windows(2)
                .position(|bytes| bytes == b"/>")
                .map(|n| start + n + 2)
                .unwrap();
            Span { start, end }
        })
        .collect();
    assert_eq!(use_ranges.len(), 2);

    let mut b = SceneBuilder::new(
        source.clone(),
        animation(&source),
        7,
        viewport(),
        Limits::default(),
    )
    .unwrap();
    let (draft, request) = draft_request(&mut b, &mut c);
    let transforms = [
        vexel_geometry::Affine {
            e: 12.0,
            f: -3.0,
            ..vexel_geometry::affine_identity()
        },
        vexel_geometry::Affine {
            a: 2.0,
            d: 2.0,
            e: -5.0,
            f: 8.0,
            ..vexel_geometry::affine_identity()
        },
    ];
    let mut ids = Vec::new();
    for (index, transform) in transforms.into_iter().enumerate() {
        let admission = b
            .admit_shape_with_source(&draft, request, use_ranges[index].start, geometry, &mut c)
            .unwrap();
        let mut input = shape(&source);
        input.shape.source.element = use_ranges[index];
        input.shape.source.transform = transform;
        input.path =
            vexel_svg_path::parse_path(&source, path_data, transform, Default::default()).unwrap();
        input.edge_colors = (0..input.path.edges.len())
            .map(|edge| [edge as u8, 2, 3, 255])
            .collect();
        ids.push(
            b.append_admitted_shape(&draft, &admission, input, &mut c)
                .unwrap(),
        );
    }
    let graph = finish_request(b, draft, request, &mut c);
    let view = graph.root();
    for (index, (id, transform)) in ids.into_iter().zip(transforms).enumerate() {
        let shape = view.shape(id).unwrap();
        let canonical = view.shape_geometry_source(id).unwrap();
        assert_eq!(shape.source.element, use_ranges[index]);
        assert_eq!(shape.source.transform, transform);
        assert_eq!(canonical.element(), path_element);
        assert_eq!(canonical.path_data(), path_data);
    }
    let frame = FrameBuilder::new(view, 1 << 20)
        .unwrap()
        .finish(&mut c)
        .unwrap();
    let framed = frame.view();
    for (index, id) in [view.shape_id(0).unwrap(), view.shape_id(1).unwrap()]
        .into_iter()
        .enumerate()
    {
        assert_eq!(framed.shape(id).unwrap().source.element, use_ranges[index]);
        assert_eq!(
            framed.shape_geometry_source(id).unwrap().path_data(),
            path_data
        );
    }
}

#[test]
fn admitted_geometry_rejects_forged_segment_and_number_spans_outside_path_data() {
    let source = std::sync::Arc::new(br#"<svg><path id="p" d="M0 0 L1 1 2 3 Z"/></svg>"#.to_vec());
    let mut c = Run;
    let document = LexicalDocument::parse(source.clone(), 1 << 20, &mut c).unwrap();
    let decoded = DecodeSource::new(&document, &mut c).unwrap();
    let view = decoded
        .element(ElementId(1), document.tokens().len(), &mut c)
        .unwrap();
    let geometry = decoded.canonical_path_source(view, &mut c).unwrap();
    let path_start = source
        .windows(5)
        .position(|bytes| bytes == b"<path")
        .unwrap();
    let element_end = source[path_start..]
        .windows(2)
        .position(|bytes| bytes == b"/>")
        .map(|offset| path_start + offset + 2)
        .unwrap();
    let path_element = Span {
        start: path_start,
        end: element_end,
    };
    let mut b = SceneBuilder::new(
        source.clone(),
        animation(&source),
        7,
        viewport(),
        Limits::default(),
    )
    .unwrap();
    let (draft, request) = draft_request(&mut b, &mut c);
    let admission = b
        .admit_shape_with_source(&draft, request, 15, geometry, &mut c)
        .unwrap();
    let mut input = shape(&source);
    input.path.segments[0].source = path_element;
    let error = b
        .append_admitted_shape(&draft, &admission, input, &mut c)
        .unwrap_err();
    assert_eq!(error.kind, ErrorKind::InvalidArgument);
    assert_eq!(error.offset, path_element.start);
    b.rollback_request(&draft, request).unwrap();
    b.commit_draft(&draft, &mut c).unwrap();
    assert!(b.freeze().unwrap().root().shapes.is_empty());

    let mut b = SceneBuilder::new(
        source.clone(),
        animation(&source),
        7,
        viewport(),
        Limits::default(),
    )
    .unwrap();
    let (draft, request) = draft_request(&mut b, &mut c);
    let admission = b
        .admit_shape_with_source(&draft, request, 15, geometry, &mut c)
        .unwrap();
    let mut input = shape(&source);
    input.path.numbers[0].source = path_element;
    let error = b
        .append_admitted_shape(&draft, &admission, input, &mut c)
        .unwrap_err();
    assert_eq!(error.kind, ErrorKind::InvalidArgument);
    assert_eq!(error.offset, path_element.start);
    b.rollback_request(&draft, request).unwrap();
    b.commit_draft(&draft, &mut c).unwrap();
    assert!(b.freeze().unwrap().root().shapes.is_empty());
}
