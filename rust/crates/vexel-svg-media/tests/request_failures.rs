mod support;
use support::*;
use vexel_scene::*;
use vexel_svg_media::{decode_image_request, layout_shared_cache, render_text_request};

const SOURCE: &[u8] = br#"<svg><path d="M0 0L1 0L1 1Z"/><text>A<![CDATA[B]]></text><image width="4" height="4" href="data:image/png;base64,AQID"/></svg>"#;
struct Interrupt {
    remaining: Option<usize>,
    events: usize,
    stop: Stop,
}
impl Interrupt {
    fn event(&mut self) -> std::result::Result<(), Stop> {
        self.events += 1;
        if let Some(left) = &mut self.remaining {
            if *left == 0 {
                return Err(self.stop);
            }
            *left -= 1;
        }
        Ok(())
    }
}
impl WorkControl for Interrupt {
    fn charge(&mut self, _: WorkKind, _: u64) -> std::result::Result<(), Stop> {
        self.event()
    }
    fn checkpoint(&mut self) -> std::result::Result<(), Stop> {
        self.event()
    }
}
fn state_value(s: TextState) -> (f64, f64, usize, usize, bool, bool) {
    (
        s.cursor_x,
        s.cursor_y,
        s.text_bytes,
        s.glyph_count,
        s.pending_space,
        s.has_position,
    )
}
fn prefix_request() -> TextRequest<'static> {
    TextRequest {
        text: b"prefix",
        font_families: &[],
        size_points: 12.,
        weight: 400.,
        style: TextStyle::Normal,
        letter_spacing_points: 0.,
    }
}

fn interrupt_probe(text: bool, at: Option<usize>, stop: Stop) -> usize {
    let mut fixture = Fixture::new(SOURCE, Limits::default());
    let scene = fixture.builder.root_id();
    let index = fixture.index(if text { b"text" } else { b"image" });
    let map = fixture.map(index);
    let element = view(&fixture.lex, index);
    let draft = fixture.builder.begin_draft(scene).unwrap();
    let input = fixture.prefix_input();
    let prefix = fixture
        .builder
        .append_shape(scene, input, &mut Run)
        .unwrap();
    let prefix_value = *fixture.builder.scene(scene).unwrap().shape(prefix).unwrap();
    let prefix_edges = fixture.builder.scene(scene).unwrap().edges.to_vec();
    let mut ledger = ChargeLedger::new(fixture.lex.source_owner(), 1 << 20, 8).unwrap();
    let ledger_before = ledger.stats().live_bytes;
    let mut state = TextState {
        cursor_x: 7.,
        cursor_y: 9.,
        text_bytes: 13,
        glyph_count: 2,
        has_position: true,
        pending_space: true,
    };
    let state_before = state_value(state);
    let mut provider = Provider::default();
    let mut cache = TextCache::default();
    assert!(
        layout_shared_cache(
            &mut cache,
            prefix_request(),
            &[],
            &mut provider,
            &mut Run,
            0
        )
        .unwrap()
        .is_cached()
    );
    let key_pointer = cache.entries[0].text.as_ptr();
    let commands_pointer = cache.entries[0].layout.commands.as_ptr();
    let mut control = Interrupt {
        remaining: at,
        events: 0,
        stop,
    };
    let result = if text {
        render_text_request(
            TextDecodeRequest {
                element,
                source_map: map,
                end: element.end,
                style: &CssPaint::initial(),
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
            &mut control,
        )
        .map(|_| ())
    } else {
        decode_image_request(
            ImageDecodeRequest {
                element,
                source_map: map,
                style: &CssPaint::initial(),
                transform: vexel_geometry::affine_identity(),
                provider: &mut provider,
                scene,
                ledger: &mut ledger,
            },
            &mut fixture.builder,
            &draft,
            &mut control,
        )
        .map(|_| ())
    };
    if let Some(at) = at {
        let expected = match stop {
            Stop::Canceled => ErrorKind::Canceled,
            Stop::Limit => ErrorKind::Limit(LimitKind::Work),
            Stop::Overflow => ErrorKind::Limit(LimitKind::Arithmetic),
        };
        assert_eq!(
            result.unwrap_err().kind,
            expected,
            "text={text}, interruption={at}"
        );
        assert_eq!(
            control.events,
            at + 1,
            "rollback must not poll stopped control"
        );
        let restored = fixture.builder.scene(scene).unwrap();
        assert_eq!(restored.shapes.len(), 1);
        assert!(restored.images.is_empty());
        assert_eq!(restored.edges, prefix_edges);
        assert_eq!(restored.path_subpaths.len(), 1);
        assert_eq!(restored.shape(prefix).unwrap().source, prefix_value.source);
        assert_eq!(state_value(state), state_before);
        assert_eq!(
            (cache.entries.len(), cache.text_bytes, cache.commands),
            (1, 6, 5)
        );
        assert_eq!(cache.entries[0].text.as_ptr(), key_pointer);
        assert_eq!(cache.entries[0].layout.commands.as_ptr(), commands_pointer);
        assert_eq!(fixture.builder.document_text_commands(), 0);
        let calls = provider.seen.len();
        assert!(
            layout_shared_cache(
                &mut cache,
                prefix_request(),
                &[],
                &mut provider,
                &mut Run,
                0
            )
            .unwrap()
            .is_cached()
        );
        assert_eq!(provider.seen.len(), calls);
    } else {
        result.unwrap();
        assert_eq!(
            fixture.builder.scene(scene).unwrap().shapes.len(),
            if text { 3 } else { 2 }
        );
        assert_eq!(
            fixture.builder.scene(scene).unwrap().images.len(),
            usize::from(!text)
        );
        assert_eq!(
            fixture.builder.document_text_commands(),
            if text { 10 } else { 0 }
        );
    }
    assert_eq!(ledger.stats().live_bytes, ledger_before);
    fixture
        .builder
        .finalize_shape(&draft, prefix, prefix_value, &mut Run)
        .unwrap();
    // A second real append and outer commit prove usable authorization, rather
    // than relying on an outer rollback to erase the failed request.
    let input = fixture.prefix_input();
    fixture
        .builder
        .append_shape(scene, input, &mut Run)
        .unwrap();
    fixture.builder.commit_draft(&draft, &mut Run).unwrap();
    assert!(
        fixture
            .builder
            .scene(scene)
            .unwrap()
            .shape(prefix)
            .is_some()
    );
    control.events
}

#[test]
fn image_every_control_boundary_restores_prefix_through_request_commit() {
    let events = interrupt_probe(false, None, Stop::Canceled);
    assert!(events > 20);
    for stop in [Stop::Canceled, Stop::Limit, Stop::Overflow] {
        for at in 0..events {
            interrupt_probe(false, Some(at), stop);
        }
    }
}

#[test]
fn text_every_control_boundary_restores_state_cache_and_prefix_through_request_commit() {
    let events = interrupt_probe(true, None, Stop::Canceled);
    assert!(events > 50);
    for stop in [Stop::Canceled, Stop::Limit, Stop::Overflow] {
        for at in 0..events {
            interrupt_probe(true, Some(at), stop);
        }
    }
}

fn controlled_text_scan_probe(source: &[u8], at: Option<usize>) -> usize {
    let mut fixture = Fixture::new(source, Limits::default());
    let scene = fixture.builder.root_id();
    let index = fixture.index(b"text");
    let map = fixture.map(index);
    let element = view(&fixture.lex, index);
    let mut shared = DecodeSource::new(&fixture.lex, &mut Run).unwrap();
    let style = shared
        .compute_style(ElementId(index), None, &mut Run)
        .unwrap();
    let draft = fixture.builder.begin_draft(scene).unwrap();
    let prefix = fixture
        .builder
        .append_shape(scene, fixture.prefix_input(), &mut Run)
        .unwrap();
    let prefix_shape = *fixture.builder.scene(scene).unwrap().shape(prefix).unwrap();
    let prefix_edges = fixture.builder.scene(scene).unwrap().edges.to_vec();
    let mut state = TextState {
        cursor_x: 7.,
        cursor_y: 9.,
        text_bytes: 13,
        glyph_count: 2,
        has_position: true,
        pending_space: true,
    };
    let before = state_value(state);
    let mut cache = TextCache::default();
    let mut provider = Provider::default();
    let mut ledger = ChargeLedger::new(fixture.lex.source_owner(), 1 << 20, 8).unwrap();
    let mut control = Interrupt {
        remaining: at,
        events: 0,
        stop: Stop::Canceled,
    };
    let result = render_text_request(
        TextDecodeRequest {
            element,
            source_map: map,
            end: element.end,
            style: &style,
            transform: map.transform,
            state: &mut state,
            preserve_space: false,
            provider: &mut provider,
            cache: &mut cache,
            scene,
            ledger: &mut ledger,
        },
        &mut fixture.builder,
        &draft,
        &mut control,
    );
    if let Some(at) = at {
        let error = result.unwrap_err();
        assert_eq!(error.kind, ErrorKind::Canceled, "interruption={at}");
        assert_eq!(control.events, at + 1);
        let scene_data = fixture.builder.scene(scene).unwrap();
        assert_eq!(scene_data.shapes.len(), 1);
        assert_eq!(scene_data.edges, prefix_edges);
        assert_eq!(
            scene_data.shape(prefix).unwrap().source,
            prefix_shape.source
        );
        assert_eq!(state_value(state), before);
        assert!(cache.entries.is_empty());
        assert_eq!(fixture.builder.document_text_commands(), 0);
        // Same draft and text state/cache are authorized for a clean retry.
        render_text_request(
            TextDecodeRequest {
                element,
                source_map: map,
                end: element.end,
                style: &style,
                transform: map.transform,
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
        assert!(fixture.builder.scene(scene).unwrap().shapes.len() > 1);
    } else {
        result.unwrap();
    }
    fixture.builder.commit_draft(&draft, &mut Run).unwrap();
    control.events
}

#[test]
fn text_controlled_scans_stop_inside_each_input_loop_and_rollback_request_state() {
    let cases: [&[u8]; 4] = [
        b"<svg><path d=\"M0 0L1 0L1 1Z\"/><text style=\"font-family:AlphaBeta\">A</text></svg>",
        b"<svg><path d=\"M0 0L1 0L1 1Z\"/><text x=\"1,2,3\">A</text></svg>",
        b"<svg><path d=\"M0 0L1 0L1 1Z\"/><text>A&amp;B</text></svg>",
        b"<svg><path d=\"M0 0L1 0L1 1Z\"/><text>A  B</text></svg>",
    ];
    for source in cases {
        let events = controlled_text_scan_probe(source, None);
        assert!(events > 10);
        for at in 0..events {
            controlled_text_scan_probe(source, Some(at));
        }
    }
}

struct FailingProvider {
    inner: Provider,
    calls: usize,
    failure: Option<MediaError>,
}
impl MediaProvider for FailingProvider {
    fn layout_text(
        &mut self,
        request: TextRequest<'_>,
        control: &mut dyn WorkControl,
    ) -> std::result::Result<TextLayout, MediaError> {
        self.calls += 1;
        if self.calls == 2 {
            if let Some(error) = self.failure {
                return Err(error);
            }
            let mut malformed = self.inner.layout_text(request, control)?;
            malformed.glyphs[0].commands = TableRange {
                first: usize::MAX,
                count: 2,
            };
            return Ok(malformed);
        }
        self.inner.layout_text(request, control)
    }
    fn decode_image(
        &mut self,
        _: &[u8],
        _: &mut dyn WorkControl,
    ) -> std::result::Result<ImagePixels, MediaError> {
        self.calls += 1;
        Err(self.failure.unwrap_or(MediaError::Corrupt))
    }
}
fn errors() -> [MediaError; 8] {
    [
        MediaError::OutOfMemory,
        MediaError::Limit,
        MediaError::Unsupported,
        MediaError::Invalid,
        MediaError::InvalidArgument,
        MediaError::InvalidUtf8,
        MediaError::Animated,
        MediaError::Corrupt,
    ]
}
fn expected(error: MediaError, text: bool) -> ErrorKind {
    match error {
        MediaError::OutOfMemory => ErrorKind::OutOfMemory,
        MediaError::Limit => ErrorKind::Limit(if text {
            LimitKind::TextCommands
        } else {
            LimitKind::Images
        }),
        _ => ErrorKind::Unsupported,
    }
}

#[test]
fn second_text_provider_failure_or_malformed_layout_restores_entire_request() {
    for failure in errors().into_iter().map(Some).chain(std::iter::once(None)) {
        let mut fixture = Fixture::new(SOURCE, Limits::default());
        let scene = fixture.builder.root_id();
        let index = fixture.index(b"text");
        let map = fixture.map(index);
        let element = view(&fixture.lex, index);
        let offset = fixture
            .lex
            .tokens()
            .iter()
            .find(|t| t.kind == vexel_xml::TokenKind::Cdata)
            .unwrap()
            .value
            .offset;
        let draft = fixture.builder.begin_draft(scene).unwrap();
        let input = fixture.prefix_input();
        let prefix = fixture
            .builder
            .append_shape(scene, input, &mut Run)
            .unwrap();
        let mut state = TextState::default();
        let mut cache = TextCache::default();
        let mut ledger = ChargeLedger::new(fixture.lex.source_owner(), 1 << 20, 8).unwrap();
        let mut provider = FailingProvider {
            inner: Provider::default(),
            calls: 0,
            failure,
        };
        let error = render_text_request(
            TextDecodeRequest {
                element,
                source_map: map,
                end: element.end,
                style: &CssPaint::initial(),
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
            (
                failure.map_or(ErrorKind::Unsupported, |e| expected(e, true)),
                offset
            )
        );
        assert_eq!(provider.calls, 2);
        assert_eq!(state_value(state), state_value(TextState::default()));
        assert_eq!(
            (cache.entries.len(), cache.text_bytes, cache.commands),
            (0, 0, 0)
        );
        assert_eq!(fixture.builder.document_text_commands(), 0);
        assert_eq!(fixture.builder.scene(scene).unwrap().shapes.len(), 1);
        assert!(
            fixture
                .builder
                .scene(scene)
                .unwrap()
                .shape(prefix)
                .is_some()
        );
        fixture.builder.commit_draft(&draft, &mut Run).unwrap();
    }
}

#[test]
fn image_provider_failures_preserve_original_classes_offsets_and_outer_prefix() {
    for failure in errors() {
        let mut fixture = Fixture::new(SOURCE, Limits::default());
        let scene = fixture.builder.root_id();
        let index = fixture.index(b"image");
        let map = fixture.map(index);
        let element = view(&fixture.lex, index);
        let href = fixture
            .lex
            .tokens()
            .iter()
            .find(|t| t.local_name.slice(SOURCE) == Some(b"href"))
            .unwrap()
            .value;
        let offset = vexel_svg_media::parse_data_uri(SOURCE, href)
            .unwrap()
            .encoded
            .offset;
        let draft = fixture.builder.begin_draft(scene).unwrap();
        let input = fixture.prefix_input();
        let prefix = fixture
            .builder
            .append_shape(scene, input, &mut Run)
            .unwrap();
        let mut ledger = ChargeLedger::new(fixture.lex.source_owner(), 1 << 20, 8).unwrap();
        let mut provider = FailingProvider {
            inner: Provider::default(),
            calls: 0,
            failure: Some(failure),
        };
        let error = decode_image_request(
            ImageDecodeRequest {
                element,
                source_map: map,
                style: &CssPaint::initial(),
                transform: vexel_geometry::affine_identity(),
                provider: &mut provider,
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
            (expected(failure, false), offset)
        );
        assert_eq!(provider.calls, 1);
        assert!(fixture.builder.scene(scene).unwrap().images.is_empty());
        assert_eq!(fixture.builder.scene(scene).unwrap().shapes.len(), 1);
        assert!(
            fixture
                .builder
                .scene(scene)
                .unwrap()
                .shape(prefix)
                .is_some()
        );
        fixture.builder.commit_draft(&draft, &mut Run).unwrap();
    }
}
