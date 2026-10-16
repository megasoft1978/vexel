mod support;
use support::*;
use vexel_scene::*;
use vexel_svg_css::Property;
use vexel_svg_media::{
    parse_families, parse_size, parse_spacing, parse_text_style, parse_weight, render_text_request,
    text_path_geometry,
};

#[test]
fn style_defaults_and_original_inherited_font_values_reach_provider() {
    // Original text_checks CSS and text/tspan input, including quoted fallback.
    let source = br#"<svg><style>.label{font-family:'Mock Sans',sans-serif;font-size:12px;font-weight:700;font-style:italic;text-anchor:middle;letter-spacing:2px;word-spacing:1px;fill:red}</style><g class="label"><text x="30" y="20">A B<tspan dx="2" fill="blue">C</tspan></text></g></svg>"#;
    let mut fixture = Fixture::new(source, Limits::default());
    let defaults = parse_text_style(source, &CssPaint::initial(), [100., 100.]).unwrap();
    assert!(defaults.families.is_empty());
    assert_eq!(
        (
            defaults.size_points,
            defaults.weight,
            defaults.style,
            defaults.anchor,
            defaults.letter_spacing_points,
            defaults.word_spacing
        ),
        (12., 400., TextStyle::Normal, TextAnchor::Start, 0., 0.)
    );
    let g = fixture.index(b"g");
    let text = fixture.index(b"text");
    let tspan = fixture.index(b"tspan");
    let mut shared = DecodeSource::new(&fixture.lex, &mut Run).unwrap();
    let parent = shared.compute_style(ElementId(g), None, &mut Run).unwrap();
    let style = shared
        .compute_style(ElementId(text), Some(&parent), &mut Run)
        .unwrap();
    let child_style = shared
        .compute_style(ElementId(tspan), Some(&style), &mut Run)
        .unwrap();
    let values = parse_text_style(source, &style, [100., 100.]).unwrap();
    assert_eq!(
        values.families,
        [b"Mock Sans".to_vec(), b"sans-serif".to_vec()]
    );
    assert_eq!(
        values
            .family_spans
            .iter()
            .map(|s| s.slice(source).unwrap())
            .collect::<Vec<_>>(),
        [b"Mock Sans".as_slice(), b"sans-serif".as_slice()]
    );
    assert_eq!(
        parse_families(source, &child_style).unwrap(),
        values.families
    );
    assert_eq!(parse_size(source, &style, 100.).unwrap(), 12.);
    assert_eq!(parse_weight(source, &style).unwrap(), 700.);
    assert_eq!(
        parse_spacing(source, &style, Property::LetterSpacing, 100.).unwrap(),
        2.
    );
    assert_eq!(values.anchor, TextAnchor::Middle);
    assert_eq!(values.word_spacing, 1.);

    let element = view(&fixture.lex, text);
    let map = fixture.map(text);
    let scene = fixture.builder.root_id();
    let draft = fixture.builder.begin_draft(scene).unwrap();
    let mut ledger = ChargeLedger::new(fixture.lex.source_owner(), 1 << 20, 8).unwrap();
    let mut state = TextState::default();
    let mut cache = TextCache::default();
    let mut provider = Provider::default();
    for _ in 0..2 {
        render_text_request(
            TextDecodeRequest {
                element,
                source_map: map,
                end: element.end,
                style: &style,
                transform: vexel_geometry::affine_identity(),
                state: &mut state,
                preserve_space: false,
                provider: &mut provider,
                cache: &mut cache,
                scene,
                ledger: &mut ledger,
            },
            &mut fixture.builder,
            &draft,
            &mut Run,
        )
        .unwrap();
    }
    assert_eq!(
        provider.seen.len(),
        1,
        "identical run reuses the accepted shared cache"
    );
    let seen = &provider.seen[0];
    assert_eq!(seen.text, b"A B");
    assert_eq!(seen.families, values.families);
    assert_eq!(
        (seen.size, seen.weight, seen.spacing, seen.style),
        (9., 700., 1.5, TextStyle::Italic)
    );
    assert_eq!(
        state.cursor_x, 55.,
        "advance plus word spacing retains original cursor rule"
    );
    let built = fixture.builder.scene(scene).unwrap();
    assert_eq!(built.shapes.len(), 2);
    assert_eq!(
        built.edges[0].a.x, 18.,
        "middle anchor shifts 24 SVG units by half"
    );
    assert_eq!(built.shapes[0].source, map);
    assert_eq!(cache.text_bytes, 3);
    assert_eq!(cache.entries[0].family_spans, values.family_spans);
    let child = view(&fixture.lex, tspan);
    let child_map = fixture.map(tspan);
    render_text_request(
        TextDecodeRequest {
            element: child,
            source_map: child_map,
            end: child.end,
            style: &child_style,
            transform: vexel_geometry::affine_identity(),
            state: &mut state,
            preserve_space: false,
            provider: &mut provider,
            cache: &mut cache,
            scene,
            ledger: &mut ledger,
        },
        &mut fixture.builder,
        &draft,
        &mut Run,
    )
    .unwrap();
    assert_eq!(provider.seen.len(), 2);
    assert_eq!(provider.seen[1].text, b"C");
    assert_eq!(
        (
            provider.seen[1].size,
            provider.seen[1].weight,
            provider.seen[1].spacing
        ),
        (9., 700., 1.5)
    );
    assert_eq!(
        state.cursor_x, 65.,
        "tspan inherits cursor and applies its authored dx"
    );
    let built = fixture.builder.scene(scene).unwrap();
    assert_eq!(built.shapes.len(), 3);
    assert_eq!(built.shapes[2].source, child_map);
    fixture.builder.rollback_draft(draft).unwrap();
}

#[test]
fn family_count_limit_and_relative_lengths_keep_original_style_errors_and_units() {
    let families = (0..33)
        .map(|i| format!("f{i}"))
        .collect::<Vec<_>>()
        .join(",");
    let source = format!("<svg><text font-family=\"{families}\">A</text></svg>");
    let fixture = Fixture::new(source.as_bytes(), Limits::default());
    let mut shared = DecodeSource::new(&fixture.lex, &mut Run).unwrap();
    let style = shared
        .compute_style(ElementId(fixture.index(b"text")), None, &mut Run)
        .unwrap();
    let offset = source.find("f32").unwrap();
    let error = parse_text_style(source.as_bytes(), &style, [80., 40.]).unwrap_err();
    assert_eq!((error.kind, error.offset), (ErrorKind::Unsupported, offset));
    let source =
        b"<svg><text font-size=\"50%\" letter-spacing=\"10%\" word-spacing=\"2px\">A</text></svg>";
    let fixture = Fixture::new(source, Limits::default());
    let mut shared = DecodeSource::new(&fixture.lex, &mut Run).unwrap();
    let style = shared
        .compute_style(ElementId(fixture.index(b"text")), None, &mut Run)
        .unwrap();
    let values = parse_text_style(source, &style, [80., 40.]).unwrap();
    assert_eq!(
        (
            values.size_points,
            values.letter_spacing_points,
            values.word_spacing
        ),
        (15., 6., 2.)
    );
}

#[test]
fn malformed_style_is_unsupported_at_exact_property_offset_before_provider() {
    for (declaration, property, relative) in [
        ("font-size:bad", Property::FontSize, 0),
        ("font-size:0", Property::FontSize, 0),
        ("font-size:-1", Property::FontSize, 0),
        ("font-weight:bad", Property::FontWeight, 0),
        ("font-weight:450", Property::FontWeight, 0),
        ("font-weight:1000", Property::FontWeight, 0),
        ("letter-spacing:bad", Property::LetterSpacing, 0),
        ("word-spacing:bad", Property::WordSpacing, 0),
        ("font-style:sideways", Property::FontStyle, 0),
        ("text-anchor:outside", Property::TextAnchor, 0),
        ("font-family:,serif", Property::FontFamily, 0),
        // Spacing is checked before font-style, as in original parse_text_style.
        (
            "font-style:sideways;letter-spacing:bad",
            Property::LetterSpacing,
            0,
        ),
    ] {
        let source = format!("<svg><text style=\"{declaration}\">A</text></svg>");
        let mut fixture = Fixture::new(source.as_bytes(), Limits::default());
        let index = fixture.index(b"text");
        let mut shared = DecodeSource::new(&fixture.lex, &mut Run).unwrap();
        let style = shared
            .compute_style(ElementId(index), None, &mut Run)
            .unwrap();
        let offset = style.value(property).unwrap().span.offset + relative;
        let error = parse_text_style(source.as_bytes(), &style, [100., 100.]).unwrap_err();
        assert_eq!(
            (error.kind, error.offset),
            (ErrorKind::Unsupported, offset),
            "{declaration}"
        );
        let element = view(&fixture.lex, index);
        let map = fixture.map(index);
        let scene = fixture.builder.root_id();
        let draft = fixture.builder.begin_draft(scene).unwrap();
        let mut ledger = ChargeLedger::new(fixture.lex.source_owner(), 1 << 20, 8).unwrap();
        let mut state = TextState::default();
        let mut cache = TextCache::default();
        let mut provider = Provider::default();
        let error = render_text_request(
            TextDecodeRequest {
                element,
                source_map: map,
                end: element.end,
                style: &style,
                transform: vexel_geometry::affine_identity(),
                state: &mut state,
                preserve_space: false,
                provider: &mut provider,
                cache: &mut cache,
                scene,
                ledger: &mut ledger,
            },
            &mut fixture.builder,
            &draft,
            &mut Run,
        )
        .unwrap_err();
        assert_eq!(
            (error.kind, error.offset),
            (ErrorKind::Unsupported, offset),
            "{declaration}"
        );
        assert!(provider.seen.is_empty());
        assert!(fixture.builder.scene(scene).unwrap().shapes.is_empty());
        assert_eq!(state.text_bytes, 0);
        assert!(cache.entries.is_empty());
        fixture.builder.rollback_draft(draft).unwrap();
    }
}

#[test]
fn anchor_modes_and_oblique_normal_spacing_use_original_units() {
    for (anchor, expected_x) in [("start", 30.), ("middle", 26.), ("end", 22.)] {
        let source = format!(
            "<svg><text x=\"30\" style=\"font-family: serif; font-style:oblique;font-weight:bold;letter-spacing:normal;word-spacing:normal;text-anchor:{anchor}\">A</text></svg>"
        );
        let mut fixture = Fixture::new(source.as_bytes(), Limits::default());
        let index = fixture.index(b"text");
        let mut shared = DecodeSource::new(&fixture.lex, &mut Run).unwrap();
        let style = shared
            .compute_style(ElementId(index), None, &mut Run)
            .unwrap();
        let values = parse_text_style(source.as_bytes(), &style, [100., 100.]).unwrap();
        assert_eq!(
            (
                values.style,
                values.weight,
                values.letter_spacing_points,
                values.word_spacing
            ),
            (TextStyle::Oblique, 700., 0., 0.)
        );
        let element = view(&fixture.lex, index);
        let map = fixture.map(index);
        let scene = fixture.builder.root_id();
        let draft = fixture.builder.begin_draft(scene).unwrap();
        let mut ledger = ChargeLedger::new(fixture.lex.source_owner(), 1 << 20, 8).unwrap();
        let mut state = TextState::default();
        let mut cache = TextCache::default();
        let mut provider = Provider::default();
        render_text_request(
            TextDecodeRequest {
                element,
                source_map: map,
                end: element.end,
                style: &style,
                transform: vexel_geometry::affine_identity(),
                state: &mut state,
                preserve_space: false,
                provider: &mut provider,
                cache: &mut cache,
                scene,
                ledger: &mut ledger,
            },
            &mut fixture.builder,
            &draft,
            &mut Run,
        )
        .unwrap();
        assert_eq!(
            fixture.builder.scene(scene).unwrap().edges[0].a.x,
            expected_x
        );
        assert_eq!(state.cursor_x, 38.);
        fixture.builder.rollback_draft(draft).unwrap();
    }
}

fn compound() -> TextLayout {
    use TextVerb::*;
    let commands = vec![
        command(Move, 0., 0.),
        command(Line, 8., 0.),
        command(Line, 8., 8.),
        command(Line, 0., 8.),
        command(Move, 2., 2.),
        command(Line, 2., 6.),
        command(Line, 6., 6.),
        command(Line, 6., 2.),
        command(Close, 0., 0.),
        command(Move, 10., 0.),
        command(Line, 12., 0.),
        command(Line, 12., 2.),
    ];
    TextLayout {
        glyphs: vec![TextGlyph {
            glyph_id: None,
            x: 0.,
            y: 0.,
            advance: 13.,
            commands: TableRange {
                first: 0,
                count: commands.len(),
            },
        }],
        commands,
        advance_width: 13.,
        ascent: 8.,
        descent: 0.,
        leading: 0.,
    }
}
struct CompoundProvider;
impl MediaProvider for CompoundProvider {
    fn layout_text(
        &mut self,
        _: TextRequest<'_>,
        _: &mut dyn WorkControl,
    ) -> std::result::Result<TextLayout, MediaError> {
        Ok(compound())
    }
    fn decode_image(
        &mut self,
        _: &[u8],
        _: &mut dyn WorkControl,
    ) -> std::result::Result<ImagePixels, MediaError> {
        Err(MediaError::Unsupported)
    }
}

#[test]
fn compound_contours_keep_winding_closure_and_scene_subpath_boundaries() {
    let path = text_path_geometry(
        &compound(),
        b"A",
        &[],
        &[],
        &[],
        &[],
        [0., 0.],
        0.,
        0.,
        vexel_geometry::affine_identity(),
        &mut Run,
    )
    .unwrap();
    assert_eq!(path.edges.len(), 11);
    assert_eq!(
        path.subpaths
            .iter()
            .map(|s| (s.first_edge, s.edge_count, s.closed))
            .collect::<Vec<_>>(),
        [(0, 4, true), (4, 4, true), (8, 3, true)]
    );
    let area = |first: usize, count: usize| {
        path.edges[first..first + count]
            .iter()
            .map(|e| e.a.x * e.b.y - e.b.x * e.a.y)
            .sum::<f64>()
    };
    assert!(
        area(0, 4) < 0. && area(4, 4) > 0.,
        "opposite contour winding retains the hole"
    );
    for subpath in &path.subpaths {
        let edges = &path.edges[subpath.first_edge..subpath.first_edge + subpath.edge_count];
        assert_eq!(edges.last().unwrap().b, edges[0].a);
        for pair in edges.windows(2) {
            assert_eq!(pair[0].b, pair[1].a);
        }
    }
    let mut fixture = Fixture::new(b"<svg><text>A</text></svg>", Limits::default());
    let index = fixture.index(b"text");
    let element = view(&fixture.lex, index);
    let map = fixture.map(index);
    let scene = fixture.builder.root_id();
    let draft = fixture.builder.begin_draft(scene).unwrap();
    let mut ledger = ChargeLedger::new(fixture.lex.source_owner(), 1 << 20, 8).unwrap();
    render_text_request(
        TextDecodeRequest {
            element,
            source_map: map,
            end: element.end,
            style: &CssPaint::initial(),
            transform: vexel_geometry::affine_identity(),
            state: &mut TextState::default(),
            preserve_space: false,
            provider: &mut CompoundProvider,
            cache: &mut TextCache::default(),
            scene,
            ledger: &mut ledger,
        },
        &mut fixture.builder,
        &draft,
        &mut Run,
    )
    .unwrap();
    let built = fixture.builder.scene(scene).unwrap();
    assert_eq!(built.shapes[0].subpaths.count, 3);
    assert_eq!(built.path_subpaths.len(), 3);
    assert_eq!(built.edges, path.edges);
    fixture.builder.commit_draft(&draft, &mut Run).unwrap();
}

#[test]
fn text_geometry_advances_one_scalar_and_applies_position_deltas_once() {
    use TextVerb::*;
    let commands = vec![
        command(Move, 0., 0.),
        command(Line, 1., 0.),
        command(Move, 0., 0.),
        command(Line, 1., 0.),
        command(Move, 0., 0.),
        command(Line, 1., 0.),
    ];
    let layout = TextLayout {
        glyphs: (0..3)
            .map(|i| TextGlyph {
                glyph_id: None,
                x: (i as f64) * 10.,
                y: 0.,
                advance: 10.,
                commands: TableRange {
                    first: i * 2,
                    count: 2,
                },
            })
            .collect(),
        commands,
        advance_width: 30.,
        ascent: 1.,
        descent: 0.,
        leading: 0.,
    };
    let path = text_path_geometry(
        &layout,
        "A B".as_bytes(),
        &[],
        &[],
        &[],
        &[],
        [0., 0.],
        0.,
        5.,
        vexel_geometry::affine_identity(),
        &mut Run,
    )
    .unwrap();
    assert_eq!(
        path.edges.iter().map(|e| e.a.x).collect::<Vec<_>>(),
        [
            0.,
            1.3333333333333333,
            13.333333333333332,
            14.666666666666666,
            31.666666666666664,
            33.
        ]
    );

    let path = text_path_geometry(
        &layout,
        b"ABC",
        &[50.],
        &[60.],
        &[2.],
        &[3.],
        [0., 0.],
        0.,
        0.,
        vexel_geometry::affine_identity(),
        &mut Run,
    )
    .unwrap();
    assert_eq!(
        path.edges.iter().map(|e| e.a.x).collect::<Vec<_>>(),
        [
            52.,
            53.333333333333336,
            15.333333333333332,
            16.666666666666664,
            28.666666666666664,
            29.999999999999996
        ]
    );
    assert_eq!(
        path.edges.iter().map(|e| e.a.y).collect::<Vec<_>>(),
        [63., 63., 3., 3., 3., 3.]
    );
}

#[path = "support_cases/geometry_offset_cases.rs"]
mod geometry_offset_cases;

#[test]
fn cdata_keeps_literal_entity_and_unmatched_ampersand() {
    let source = b"<svg><text><![CDATA[&amp; &]]></text></svg>";
    let mut fixture = Fixture::new(source, Limits::default());
    let index = fixture.index(b"text");
    let element = view(&fixture.lex, index);
    let map = fixture.map(index);
    let scene = fixture.builder.root_id();
    let draft = fixture.builder.begin_draft(scene).unwrap();
    let mut ledger = ChargeLedger::new(fixture.lex.source_owner(), 1 << 20, 8).unwrap();
    let mut provider = Provider::default();
    render_text_request(
        TextDecodeRequest {
            element,
            source_map: map,
            end: element.end,
            style: &CssPaint::initial(),
            transform: vexel_geometry::affine_identity(),
            state: &mut TextState::default(),
            preserve_space: false,
            provider: &mut provider,
            cache: &mut TextCache::default(),
            scene,
            ledger: &mut ledger,
        },
        &mut fixture.builder,
        &draft,
        &mut Run,
    )
    .unwrap();
    assert_eq!(provider.seen[0].text, b"&amp; &");
    fixture.builder.rollback_draft(draft).unwrap();
}

#[test]
fn unavailable_native_provider_stays_unsupported_at_text_run_offset() {
    let mut fixture = Fixture::new(b"<svg><text>A</text></svg>", Limits::default());
    let index = fixture.index(b"text");
    let element = view(&fixture.lex, index);
    let map = fixture.map(index);
    let scene = fixture.builder.root_id();
    let draft = fixture.builder.begin_draft(scene).unwrap();
    let mut ledger = ChargeLedger::new(fixture.lex.source_owner(), 1 << 20, 8).unwrap();
    let mut provider = Provider {
        fail_text: Some(MediaError::Unsupported),
        ..Default::default()
    };
    let offset = fixture
        .lex
        .source()
        .windows(1)
        .position(|w| w == b"A")
        .unwrap();
    let error = render_text_request(
        TextDecodeRequest {
            element,
            source_map: map,
            end: element.end,
            style: &CssPaint::initial(),
            transform: vexel_geometry::affine_identity(),
            state: &mut TextState::default(),
            preserve_space: false,
            provider: &mut provider,
            cache: &mut TextCache::default(),
            scene,
            ledger: &mut ledger,
        },
        &mut fixture.builder,
        &draft,
        &mut Run,
    )
    .unwrap_err();
    assert_eq!((error.kind, error.offset), (ErrorKind::Unsupported, offset));
    assert!(fixture.builder.scene(scene).unwrap().shapes.is_empty());
    fixture.builder.rollback_draft(draft).unwrap();
}

#[test]
fn explicit_text_request_end_bounds_authored_runs() {
    let mut fixture = Fixture::new(b"<svg><text>A<![CDATA[B]]></text></svg>", Limits::default());
    let index = fixture.index(b"text");
    let element = view(&fixture.lex, index);
    let map = fixture.map(index);
    let end = fixture
        .lex
        .tokens()
        .iter()
        .position(|t| t.kind == vexel_xml::TokenKind::Cdata)
        .unwrap();
    let scene = fixture.builder.root_id();
    let draft = fixture.builder.begin_draft(scene).unwrap();
    let mut ledger = ChargeLedger::new(fixture.lex.source_owner(), 1 << 20, 8).unwrap();
    let mut provider = Provider::default();
    render_text_request(
        TextDecodeRequest {
            element,
            source_map: map,
            end,
            style: &CssPaint::initial(),
            transform: vexel_geometry::affine_identity(),
            state: &mut TextState::default(),
            preserve_space: false,
            provider: &mut provider,
            cache: &mut TextCache::default(),
            scene,
            ledger: &mut ledger,
        },
        &mut fixture.builder,
        &draft,
        &mut Run,
    )
    .unwrap();
    assert_eq!(provider.seen.len(), 1);
    assert_eq!(provider.seen[0].text, b"A");
    assert_eq!(fixture.builder.scene(scene).unwrap().shapes.len(), 1);
    fixture.builder.commit_draft(&draft, &mut Run).unwrap();
}

#[test]
fn malformed_position_keeps_number_classification_and_source_offset() {
    let source = b"<svg><text x=\"bad\">A</text></svg>";
    let mut fixture = Fixture::new(source, Limits::default());
    let index = fixture.index(b"text");
    let element = view(&fixture.lex, index);
    let map = fixture.map(index);
    let scene = fixture.builder.root_id();
    let draft = fixture.builder.begin_draft(scene).unwrap();
    let mut ledger = ChargeLedger::new(fixture.lex.source_owner(), 1 << 20, 8).unwrap();
    let mut provider = Provider::default();
    let error = render_text_request(
        TextDecodeRequest {
            element,
            source_map: map,
            end: element.end,
            style: &CssPaint::initial(),
            transform: vexel_geometry::affine_identity(),
            state: &mut TextState::default(),
            preserve_space: false,
            provider: &mut provider,
            cache: &mut TextCache::default(),
            scene,
            ledger: &mut ledger,
        },
        &mut fixture.builder,
        &draft,
        &mut Run,
    )
    .unwrap_err();
    assert_eq!(
        (error.kind, error.offset),
        (
            ErrorKind::Number,
            source.windows(3).position(|w| w == b"bad").unwrap()
        )
    );
    assert_eq!(provider.seen.len(), 1);
    assert!(fixture.builder.scene(scene).unwrap().shapes.is_empty());
    fixture.builder.rollback_draft(draft).unwrap();
}
