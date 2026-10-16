use vexel_base::{Stop, WorkControl, WorkKind};
use vexel_scene::{
    ImagePixels, MediaError, MediaProvider, TableRange, TextCommand, TextGlyph, TextLayout,
    TextRequest, TextVerb,
};
use vexel_svg_media::{
    TextCache, decode_base64, decode_image, image_geometry, normalize_text, parse_data_uri,
    parse_position_list,
};
use vexel_xml::{TokenKind, XmlLimits, XmlSpan};

struct NoStop;
impl WorkControl for NoStop {
    fn charge(&mut self, _: WorkKind, _: u64) -> Result<(), Stop> {
        Ok(())
    }
    fn checkpoint(&mut self) -> Result<(), Stop> {
        Ok(())
    }
}

struct StopOnCharge;
impl WorkControl for StopOnCharge {
    fn charge(&mut self, _: WorkKind, _: u64) -> Result<(), Stop> {
        Err(Stop::Canceled)
    }
    fn checkpoint(&mut self) -> Result<(), Stop> {
        Err(Stop::Canceled)
    }
}

#[test]
fn image_data_uri_and_base64_keep_exact_grammar_and_offsets() {
    let source = b"data:image/png;base64,AQID";
    let uri = XmlSpan {
        offset: 0,
        length: source.len(),
    };
    let payload = parse_data_uri(source, uri).unwrap();
    assert_eq!(payload.mime, vexel_svg_media::ImageMime::Png);
    let mut control = NoStop;
    assert_eq!(
        decode_base64(source, payload.encoded, &mut control).unwrap(),
        [1, 2, 3]
    );
    let malformed = b"AQ!D";
    assert_eq!(
        decode_base64(
            malformed,
            XmlSpan {
                offset: 0,
                length: 4
            },
            &mut control
        )
        .unwrap_err()
        .kind,
        vexel_scene::ErrorKind::Unsupported
    );
    assert_eq!(
        parse_data_uri(
            b"https://host/x.png",
            XmlSpan {
                offset: 0,
                length: 18
            }
        )
        .unwrap_err()
        .offset,
        0
    );
    assert!(
        decode_base64(
            b"A",
            XmlSpan {
                offset: 0,
                length: 1
            },
            &mut control
        )
        .is_err()
    );
    let stopped = decode_base64(source, payload.encoded, &mut StopOnCharge).unwrap_err();
    assert_eq!(stopped.kind, vexel_scene::ErrorKind::Canceled);
    assert_eq!(stopped.offset, payload.encoded.offset);
}

#[test]
fn image_geometry_preserves_meet_slice_none_and_viewport_lengths() {
    let source = b"<image x=\"10%\" y=\"2\" width=\"4\" height=\"2\"/>";
    let (tokens, _) = vexel_xml::tokenize(source, XmlLimits::default());
    let elem = tokens
        .iter()
        .position(|t| t.kind == TokenKind::EmptyElement)
        .unwrap();
    let meet = image_geometry(
        source,
        &tokens,
        elem,
        tokens.len(),
        [100., 20.],
        Some([2, 2]),
        vexel_svg_values::Aspect::default(),
    )
    .unwrap();
    assert_eq!(
        [
            meet.x,
            meet.y,
            meet.draw_x,
            meet.draw_width,
            meet.draw_height
        ],
        [10., 2., 11., 2., 2.]
    );
    let slice = image_geometry(
        source,
        &tokens,
        elem,
        tokens.len(),
        [100., 20.],
        Some([2, 2]),
        vexel_svg_values::Aspect {
            slice: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(
        [slice.draw_x, slice.draw_width, slice.draw_height],
        [10., 4., 4.]
    );
    let none = image_geometry(
        source,
        &tokens,
        elem,
        tokens.len(),
        [100., 20.],
        Some([2, 2]),
        vexel_svg_values::Aspect {
            none: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(
        [none.draw_x, none.draw_y, none.draw_width, none.draw_height],
        [10., 2., 4., 2.]
    );
    let invalid = b"<image width=\"-1\" height=\"2\"/>";
    let (bad_tokens, _) = vexel_xml::tokenize(invalid, XmlLimits::default());
    let bad_elem = bad_tokens
        .iter()
        .position(|t| t.kind == TokenKind::EmptyElement)
        .unwrap();
    assert!(
        image_geometry(
            invalid,
            &bad_tokens,
            bad_elem,
            bad_tokens.len(),
            [8., 8.],
            None,
            Default::default()
        )
        .is_err()
    );
}

#[test]
fn text_entities_whitespace_and_position_lists_match_source_contract() {
    let source = b" A &amp;  B ";
    let mut state = vexel_scene::TextState::default();
    let value = normalize_text(
        source,
        XmlSpan {
            offset: 0,
            length: source.len(),
        },
        false,
        &mut state,
    )
    .unwrap();
    assert_eq!(value, b"A & B");
    let preserved = normalize_text(
        source,
        XmlSpan {
            offset: 0,
            length: source.len(),
        },
        true,
        &mut state,
    )
    .unwrap();
    assert_eq!(preserved, b" A &  B ");
    let positions = b"1, 2 3.5%";
    assert_eq!(
        parse_position_list(
            positions,
            XmlSpan {
                offset: 0,
                length: positions.len()
            },
            200.
        )
        .unwrap(),
        [1., 2., 7.]
    );
    for bad in [b"".as_slice(), b",1", b"1,,2", b"1,"] {
        assert!(
            parse_position_list(
                bad,
                XmlSpan {
                    offset: 0,
                    length: bad.len()
                },
                20.
            )
            .is_err()
        );
    }
    let capped =
        std::iter::repeat_n("1 ", vexel_svg_media::MAX_POSITION_VALUES + 1).collect::<String>();
    let cap_error = parse_position_list(
        capped.as_bytes(),
        XmlSpan {
            offset: 0,
            length: capped.len(),
        },
        8.,
    )
    .unwrap_err();
    assert_eq!(
        cap_error.kind,
        vexel_scene::ErrorKind::Number
    );
    assert_eq!(cap_error.offset, vexel_svg_media::MAX_POSITION_VALUES * 2);
    let bad_entity = b"&#x110000;";
    assert_eq!(
        normalize_text(
            bad_entity,
            XmlSpan {
                offset: 0,
                length: bad_entity.len()
            },
            true,
            &mut vexel_scene::TextState::default()
        )
        .unwrap_err()
        .offset,
        0
    );
}

#[derive(Default)]
struct Fixture {
    layouts: usize,
}
impl MediaProvider for Fixture {
    fn layout_text(
        &mut self,
        request: TextRequest<'_>,
        _: &mut dyn WorkControl,
    ) -> Result<TextLayout, MediaError> {
        self.layouts += 1;
        let glyphs = request.text.iter().filter(|b| **b & 0xc0 != 0x80).count();
        Ok(TextLayout {
            glyphs: (0..glyphs)
                .map(|i| TextGlyph {
                    glyph_id: None,
                    x: i as f64 * 8.,
                    y: 0.,
                    advance: 8.,
                    commands: TableRange::EMPTY,
                })
                .collect(),
            commands: Vec::new(),
            advance_width: glyphs as f64 * 8.,
            ascent: 9.,
            descent: 3.,
            leading: 0.,
        })
    }
    fn decode_image(
        &mut self,
        _: &[u8],
        _: &mut dyn WorkControl,
    ) -> Result<ImagePixels, MediaError> {
        Err(MediaError::Unsupported)
    }
}

struct InvalidImageProvider;
impl MediaProvider for InvalidImageProvider {
    fn layout_text(
        &mut self,
        _: TextRequest<'_>,
        _: &mut dyn WorkControl,
    ) -> Result<TextLayout, MediaError> {
        Err(MediaError::Unsupported)
    }
    fn decode_image(
        &mut self,
        _: &[u8],
        _: &mut dyn WorkControl,
    ) -> Result<ImagePixels, MediaError> {
        Ok(ImagePixels {
            rgba: vec![0; 4],
            byte_count: 4,
            row_stride: 3,
            width: 1,
            height: 1,
        })
    }
}

#[test]
fn malformed_provider_pixels_keep_unsupported_classification() {
    let error = decode_image(&mut InvalidImageProvider, b"encoded", 23, &mut NoStop).unwrap_err();
    assert_eq!(error.kind, vexel_scene::ErrorKind::Unsupported);
    assert_eq!(error.offset, 23);
}

#[test]
fn image_sampler_matches_premultiplied_bilinear_rounding_and_bounds() {
    let pixels = ImagePixels {
        rgba: vec![
            255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
        ],
        byte_count: 16,
        row_stride: 8,
        width: 2,
        height: 2,
    };
    let geometry = vexel_svg_media::ImageGeometry {
        x: 0.,
        y: 0.,
        width: 4.,
        height: 4.,
        draw_x: 0.,
        draw_y: 0.,
        draw_width: 4.,
        draw_height: 4.,
    };
    assert_eq!(
        vexel_svg_media::sample_image(
            &pixels,
            geometry,
            vexel_geometry::affine_identity(),
            1.5,
            1.5
        ),
        Some([159, 64, 64, 255])
    );
    let inverse_translation = vexel_geometry::Affine {
        a: 1.,
        b: 0.,
        c: 0.,
        d: 1.,
        e: -1.,
        f: 0.,
    };
    assert_eq!(
        vexel_svg_media::sample_image(&pixels, geometry, inverse_translation, 2.5, 1.5),
        Some([159, 64, 64, 255])
    );
    let letterboxed = vexel_svg_media::ImageGeometry {
        draw_x: 1.,
        draw_width: 2.,
        ..geometry
    };
    assert_eq!(
        vexel_svg_media::sample_image(
            &pixels,
            letterboxed,
            vexel_geometry::affine_identity(),
            0.5,
            1.5
        ),
        None
    );

    let alpha_pixels = ImagePixels {
        rgba: vec![0, 0, 0, 0, 255, 0, 0, 255],
        byte_count: 8,
        row_stride: 8,
        width: 2,
        height: 1,
    };
    let line = vexel_svg_media::ImageGeometry {
        x: 0.,
        y: 0.,
        width: 2.,
        height: 1.,
        draw_x: 0.,
        draw_y: 0.,
        draw_width: 2.,
        draw_height: 1.,
    };
    assert_eq!(
        vexel_svg_media::sample_image(
            &alpha_pixels,
            line,
            vexel_geometry::affine_identity(),
            1.,
            0.5
        ),
        Some([255, 0, 0, 128])
    );
}

#[test]
fn text_cache_owns_keys_and_reuses_exact_layout_requests() {
    let families: [&[u8]; 1] = [b"Mock Sans"];
    let request = TextRequest {
        text: b"same",
        font_families: &families,
        size_points: 9.,
        weight: 700.,
        style: vexel_scene::TextStyle::Italic,
        letter_spacing_points: 1.5,
    };
    let mut cache = TextCache::new();
    let mut provider = Fixture::default();
    let mut control = NoStop;
    assert!(
        cache
            .layout(request, &mut provider, &mut control, 4)
            .unwrap()
            .is_cached()
    );
    assert!(
        cache
            .layout(request, &mut provider, &mut control, 4)
            .unwrap()
            .is_cached()
    );
    assert_eq!(provider.layouts, 1);
    assert_eq!(cache.stored_text_bytes(), 4);
}

#[test]
fn text_cache_entry_cap_and_image_capacity_validation_fail_closed() {
    let families: [&[u8]; 0] = [];
    let mut cache = TextCache::new();
    let mut provider = Fixture::default();
    let mut control = NoStop;
    for i in 0..vexel_svg_media::TEXT_CACHE_ENTRIES {
        let text = format!("run-{i}");
        let request = TextRequest {
            text: text.as_bytes(),
            font_families: &families,
            size_points: 12.,
            weight: 400.,
            style: vexel_scene::TextStyle::Normal,
            letter_spacing_points: 0.,
        };
        assert!(
            cache
                .layout(request, &mut provider, &mut control, i)
                .unwrap()
                .is_cached()
        );
    }
    assert_eq!(cache.len(), vexel_svg_media::TEXT_CACHE_ENTRIES);
    let extra = TextRequest {
        text: b"overflow",
        font_families: &families,
        size_points: 12.,
        weight: 400.,
        style: vexel_scene::TextStyle::Normal,
        letter_spacing_points: 0.,
    };
    assert!(
        !cache
            .layout(extra, &mut provider, &mut control, 99)
            .unwrap()
            .is_cached()
    );
    assert_eq!(cache.len(), vexel_svg_media::TEXT_CACHE_ENTRIES);

    let invalid = ImagePixels {
        rgba: vec![0; 4],
        byte_count: 4,
        row_stride: 3,
        width: 1,
        height: 1,
    };
    assert_eq!(invalid.validate(17).unwrap_err().offset, 17);
    let oversized = ImagePixels {
        rgba: Vec::new(),
        byte_count: 0,
        row_stride: 0,
        width: vexel_svg_media::MAX_IMAGE_DIMENSION + 1,
        height: 1,
    };
    assert_eq!(
        oversized.validate(18).unwrap_err().kind,
        vexel_scene::ErrorKind::InvalidArgument
    );
}

#[test]
fn glyph_outline_closure_and_command_range_validation_inputs_are_owned() {
    let layout = TextLayout {
        glyphs: vec![TextGlyph {
            glyph_id: Some(7),
            x: 0.,
            y: 0.,
            advance: 4.,
            commands: TableRange { first: 0, count: 4 },
        }],
        commands: vec![
            TextCommand {
                verb: TextVerb::Move,
                values: [0., 0., 0., 0., 0., 0.],
            },
            TextCommand {
                verb: TextVerb::Line,
                values: [4., 0., 0., 0., 0., 0.],
            },
            TextCommand {
                verb: TextVerb::Line,
                values: [4., 4., 0., 0., 0., 0.],
            },
            TextCommand {
                verb: TextVerb::Close,
                values: [0.; 6],
            },
        ],
        advance_width: 4.,
        ascent: 3.,
        descent: 1.,
        leading: 0.,
    };
    let mut control = NoStop;
    let edges = vexel_svg_media::text_geometry(
        &layout,
        b"A",
        &[],
        &[],
        &[],
        &[],
        [1., 5.],
        0.,
        0.,
        vexel_geometry::affine_identity(),
        &mut control,
    )
    .unwrap();
    assert_eq!(edges.len(), 3);
    assert_eq!(
        edges.first().unwrap().a,
        vexel_geometry::Point { x: 1., y: 5. }
    );
    assert_eq!(
        edges.last().unwrap().b,
        vexel_geometry::Point { x: 1., y: 5. }
    );
}
