mod support;
use std::cell::RefCell;
use support::*;
use vexel_base::{Stop, WorkControl, WorkKind};
use vexel_scene::*;
use vexel_svg_media::{
    CustomGeometry, PublicationContext, PublishedStyle, StylePublisher, publish_text_request,
};

// Explicit fixture delegate: real owned scene publication, no production fallback.
#[derive(Default)]
struct Publisher {
    calls: RefCell<Vec<&'static str>>,
}
impl StylePublisher for Publisher {
    fn begin(
        &self,
        _: &PublicationContext<'_, '_>,
        _: &mut SceneBuilder,
        _: &DraftToken,
        _: &mut dyn WorkControl,
    ) -> Result<()> {
        self.calls.borrow_mut().push("begin");
        Ok(())
    }
    fn prepare_geometry(
        &self,
        _: &PublicationContext<'_, '_>,
        _: &mut SceneBuilder,
        _: &DraftToken,
        _: &mut dyn WorkControl,
    ) -> Result<()> {
        self.calls.borrow_mut().push("prepare");
        Ok(())
    }
    fn prepare_admitted_geometry(
        &self,
        _: &PublicationContext<'_, '_>,
        _: &mut SceneBuilder,
        _: &DraftToken,
        _: &mut dyn WorkControl,
    ) -> Result<()> {
        Ok(())
    }
    fn publish(
        &self,
        _c: &PublicationContext<'_, '_>,
        g: &CustomGeometry,
        _b: &mut SceneBuilder,
        _d: &DraftToken,
        _control: &mut dyn WorkControl,
    ) -> Result<PublishedStyle> {
        self.calls.borrow_mut().push("publish");
        let colors = vec![[0, 0, 255, 255]; g.path.edges.len()];
        Ok(PublishedStyle {
            shape: Shape::new(g.source_map),
            edge_colors: colors,
        })
    }
    fn finish(
        &self,
        _: &PublicationContext<'_, '_>,
        _: usize,
        _: &mut SceneBuilder,
        _: &DraftToken,
        _: &mut dyn WorkControl,
    ) -> Result<()> {
        self.calls.borrow_mut().push("finish");
        Ok(())
    }
}
#[derive(Clone, Copy)]
enum Mode {
    Valid,
    NoOutline,
    EmptyCommands,
    CommandCap,
    Malformed,
    Mismatch,
    MismatchNoOutline,
    Error(MediaError),
}
struct LayoutProvider {
    inner: Provider,
    mode: Mode,
}
impl MediaProvider for LayoutProvider {
    fn layout_text(
        &mut self,
        request: TextRequest<'_>,
        control: &mut dyn WorkControl,
    ) -> std::result::Result<TextLayout, MediaError> {
        let mut output = self.inner.layout_text(request, control)?;
        match self.mode {
            Mode::Valid => {}
            Mode::NoOutline => {
                for g in &mut output.glyphs {
                    g.commands.count = 0;
                }
            }
            Mode::EmptyCommands => {
                for g in &mut output.glyphs {
                    g.commands.count = 0;
                }
                output.commands.clear();
            }
            Mode::CommandCap => output.commands.resize(
                vexel_svg_media::MAX_PATH_COMMANDS,
                TextCommand {
                    verb: TextVerb::Close,
                    values: [0.; 6],
                },
            ),
            Mode::Malformed => output.advance_width = f64::NAN,
            Mode::Mismatch => output.glyphs.push(output.glyphs[0]),
            Mode::MismatchNoOutline => {
                output.glyphs.push(output.glyphs[0]);
                for glyph in &mut output.glyphs {
                    glyph.commands.count = 0;
                }
            }
            Mode::Error(e) => return Err(e),
        }
        Ok(output)
    }
    fn decode_image(
        &mut self,
        _: &[u8],
        _: &mut dyn WorkControl,
    ) -> std::result::Result<ImagePixels, MediaError> {
        Err(MediaError::Unsupported)
    }
}
fn provider(mode: Mode) -> LayoutProvider {
    LayoutProvider {
        inner: Provider::default(),
        mode,
    }
}
fn state_values(s: TextState) -> (usize, usize, f64, f64, bool, bool) {
    (
        s.text_bytes,
        s.glyph_count,
        s.cursor_x,
        s.cursor_y,
        s.has_position,
        s.pending_space,
    )
}

#[test]
fn hidden_and_collapse_text_keep_layout_cached_use_admissions_and_cursor_without_geometry() {
    for visibility in ["hidden", "collapse"] {
        let source = format!(
            "<svg><text style=\"visibility:{visibility};word-spacing:1px\">A B</text></svg>"
        );
        let mut f = Fixture::new(source.as_bytes(), Limits::default());
        let i = f.index(b"text");
        let mut shared = DecodeSource::new(&f.lex, &mut Run).unwrap();
        let style = shared.compute_style(ElementId(i), None, &mut Run).unwrap();
        let element = view(&f.lex, i);
        let map = f.map(i);
        let scene = f.builder.root_id();
        let draft = f.builder.begin_draft(scene).unwrap();
        let mut state = TextState::default();
        state.cursor_x = 11.;
        state.cursor_y = 7.;
        let mut cache = TextCache::default();
        let mut p = provider(Mode::Valid);
        let publisher = Publisher::default();
        let mut ledger = ChargeLedger::new(f.lex.source_owner(), 1 << 20, 8).unwrap();
        for expected_cursor in [36., 61.] {
            let preserve = publish_text_request(
                TextDecodeRequest {
                    element,
                    source_map: map,
                    end: element.end,
                    style: &style,
                    transform: map.transform,
                    state: &mut state,
                    preserve_space: false,
                    provider: &mut p,
                    cache: &mut cache,
                    scene,
                    ledger: &mut ledger,
                },
                &publisher,
                &mut f.builder,
                &draft,
                &mut Run,
            )
            .unwrap();
            assert!(!preserve);
            assert_eq!((state.cursor_x, state.cursor_y), (expected_cursor, 7.));
            assert!(state.has_position);
            assert!(f.builder.scene(scene).unwrap().shapes.is_empty());
        }
        assert_eq!(p.inner.seen.len(), 1);
        assert_eq!(p.inner.seen[0].text, b"A B");
        assert_eq!((state.text_bytes, state.glyph_count), (6, 6));
        assert_eq!(f.builder.document_text_commands(), 10);
        assert_eq!(
            (cache.entries.len(), cache.text_bytes, cache.commands),
            (1, 3, 5)
        );
        assert_eq!(
            &*publisher.calls.borrow(),
            &["begin", "finish", "begin", "finish"]
        );
        f.builder.commit_draft(&draft, &mut Run).unwrap();
    }
}

#[test]
fn hidden_text_font_provider_position_and_wordspacing_errors_precede_visibility_geometry_skip() {
    let cases = [
        (
            "style=\"visibility:hidden;font-size:bad\"",
            Mode::Valid,
            ErrorKind::Unsupported,
            "bad",
            0,
        ),
        (
            "style=\"visibility:collapse;font-weight:bad\"",
            Mode::Valid,
            ErrorKind::Unsupported,
            "bad",
            0,
        ),
        (
            "style=\"visibility:hidden\"",
            Mode::Error(MediaError::Unsupported),
            ErrorKind::Unsupported,
            ">A",
            1,
        ),
        (
            "style=\"visibility:hidden\"",
            Mode::Error(MediaError::OutOfMemory),
            ErrorKind::OutOfMemory,
            ">A",
            1,
        ),
        (
            "style=\"visibility:hidden\"",
            Mode::Malformed,
            ErrorKind::Limit(LimitKind::TextCommands),
            ">A",
            1,
        ),
        (
            "x=\"bad\" style=\"visibility:hidden\"",
            Mode::Valid,
            ErrorKind::Number,
            "bad",
            1,
        ),
        (
            "dy=\"bad\" style=\"visibility:collapse\"",
            Mode::NoOutline,
            ErrorKind::Number,
            "bad",
            1,
        ),
        (
            "x=\"bad\" style=\"visibility:bogus\"",
            Mode::NoOutline,
            ErrorKind::Number,
            "bad",
            1,
        ),
        (
            "style=\"visibility:hidden;word-spacing:1px\"",
            Mode::Mismatch,
            ErrorKind::Unsupported,
            ">A",
            1,
        ),
        (
            "style=\"visibility:bogus;word-spacing:1px\"",
            Mode::MismatchNoOutline,
            ErrorKind::Unsupported,
            ">A",
            1,
        ),
    ];
    for (attrs, mode, kind, needle, calls) in cases {
        let source = format!("<svg><path d=\"M0 0L1 0L1 1Z\"/><text {attrs}>A</text></svg>");
        let mut f = Fixture::new(source.as_bytes(), Limits::default());
        let i = f.index(b"text");
        let mut shared = DecodeSource::new(&f.lex, &mut Run).unwrap();
        let style = shared.compute_style(ElementId(i), None, &mut Run).unwrap();
        let element = view(&f.lex, i);
        let map = f.map(i);
        let scene = f.builder.root_id();
        let draft = f.builder.begin_draft(scene).unwrap();
        let prefix = f
            .builder
            .append_shape(scene, f.prefix_input(), &mut Run)
            .unwrap();
        let before_shape = *f.builder.scene(scene).unwrap().shape(prefix).unwrap();
        let mut state = TextState::default();
        state.cursor_x = 17.;
        let before = state;
        let mut cache = TextCache::default();
        let mut p = provider(mode);
        let publisher = Publisher::default();
        let mut ledger = ChargeLedger::new(f.lex.source_owner(), 1 << 20, 8).unwrap();
        let e = publish_text_request(
            TextDecodeRequest {
                element,
                source_map: map,
                end: element.end,
                style: &style,
                transform: map.transform,
                state: &mut state,
                preserve_space: false,
                provider: &mut p,
                cache: &mut cache,
                scene,
                ledger: &mut ledger,
            },
            &publisher,
            &mut f.builder,
            &draft,
            &mut Run,
        )
        .unwrap_err();
        let offset = source.find(needle).unwrap() + usize::from(needle == ">A");
        assert_eq!((e.kind, e.offset), (kind, offset));
        assert_eq!(p.inner.seen.len(), calls);
        assert_eq!(&*publisher.calls.borrow(), &["begin"]);
        assert_eq!(state_values(state), state_values(before));
        assert_eq!(
            (cache.entries.len(), cache.text_bytes, cache.commands),
            (0, 0, 0)
        );
        assert_eq!(f.builder.document_text_commands(), 0);
        let s = f.builder.scene(scene).unwrap();
        assert_eq!(s.shapes.len(), 1);
        assert_eq!(s.shape(prefix), Some(&before_shape));
        f.builder
            .append_shape(scene, f.prefix_input(), &mut Run)
            .unwrap();
        f.builder.commit_draft(&draft, &mut Run).unwrap();
    }
}

#[test]
fn invalid_visibility_is_checked_only_for_nonzero_referenced_outline_commands() {
    for mode in [Mode::NoOutline, Mode::EmptyCommands, Mode::Valid] {
        let source =
            br#"<svg><text x="20" y="30" dx="2" dy="3" style="visibility:bogus">A</text></svg>"#;
        let mut f = Fixture::new(source, Limits::default());
        let i = f.index(b"text");
        let mut shared = DecodeSource::new(&f.lex, &mut Run).unwrap();
        let style = shared.compute_style(ElementId(i), None, &mut Run).unwrap();
        let element = view(&f.lex, i);
        let map = f.map(i);
        let scene = f.builder.root_id();
        let draft = f.builder.begin_draft(scene).unwrap();
        let mut state = TextState::default();
        let mut cache = TextCache::default();
        let mut p = provider(mode);
        let publisher = Publisher::default();
        let mut ledger = ChargeLedger::new(f.lex.source_owner(), 1 << 20, 8).unwrap();
        let result = publish_text_request(
            TextDecodeRequest {
                element,
                source_map: map,
                end: element.end,
                style: &style,
                transform: map.transform,
                state: &mut state,
                preserve_space: false,
                provider: &mut p,
                cache: &mut cache,
                scene,
                ledger: &mut ledger,
            },
            &publisher,
            &mut f.builder,
            &draft,
            &mut Run,
        );
        assert_eq!(p.inner.seen.len(), 1);
        assert!(f.builder.scene(scene).unwrap().shapes.is_empty());
        match mode {
            Mode::Valid => {
                let e = result.unwrap_err();
                let offset = source.windows(5).position(|b| b == b"bogus").unwrap();
                assert_eq!((e.kind, e.offset), (ErrorKind::Unsupported, offset));
                assert_eq!(state_values(state), state_values(TextState::default()));
                assert_eq!(f.builder.document_text_commands(), 0);
                assert!(cache.entries.is_empty());
                assert_eq!(&*publisher.calls.borrow(), &["begin"]);
            }
            _ => {
                result.unwrap();
                assert_eq!((state.cursor_x, state.cursor_y), (30., 33.));
                assert_eq!(
                    (state.text_bytes, state.glyph_count, state.has_position),
                    (1, 1, true)
                );
                let commands = if matches!(mode, Mode::NoOutline) {
                    5
                } else {
                    0
                };
                assert_eq!(f.builder.document_text_commands(), commands);
                assert_eq!(
                    (cache.entries.len(), cache.text_bytes, cache.commands),
                    (1, 1, commands)
                );
                assert_eq!(&*publisher.calls.borrow(), &["begin", "finish"]);
            }
        }
        f.builder.commit_draft(&draft, &mut Run).unwrap();
    }
}

#[test]
fn hidden_text_still_checks_document_glyph_limit_and_restores_prior_state() {
    let source = br#"<svg><text style="visibility:hidden">A</text></svg>"#;
    let mut f = Fixture::new(source, Limits::default());
    let i = f.index(b"text");
    let mut shared = DecodeSource::new(&f.lex, &mut Run).unwrap();
    let style = shared.compute_style(ElementId(i), None, &mut Run).unwrap();
    let element = view(&f.lex, i);
    let map = f.map(i);
    let scene = f.builder.root_id();
    let draft = f.builder.begin_draft(scene).unwrap();
    let mut state = TextState::default();
    state.glyph_count = 1_048_576;
    state.cursor_x = 15.;
    let before = state;
    let mut cache = TextCache::default();
    let mut p = provider(Mode::NoOutline);
    let publisher = Publisher::default();
    let mut ledger = ChargeLedger::new(f.lex.source_owner(), 1 << 20, 8).unwrap();
    let e = publish_text_request(
        TextDecodeRequest {
            element,
            source_map: map,
            end: element.end,
            style: &style,
            transform: map.transform,
            state: &mut state,
            preserve_space: false,
            provider: &mut p,
            cache: &mut cache,
            scene,
            ledger: &mut ledger,
        },
        &publisher,
        &mut f.builder,
        &draft,
        &mut Run,
    )
    .unwrap_err();
    let offset = source.iter().position(|&v| v == b'A').unwrap();
    assert_eq!(
        (e.kind, e.offset),
        (ErrorKind::Limit(LimitKind::Glyphs), offset)
    );
    assert_eq!(state_values(state), state_values(before));
    assert_eq!(p.inner.seen.len(), 1);
    assert!(cache.entries.is_empty());
    assert_eq!(f.builder.document_text_commands(), 0);
    f.builder.commit_draft(&draft, &mut Run).unwrap();
}

struct Interrupt {
    at: usize,
    calls: usize,
    stop: Stop,
}
impl WorkControl for Interrupt {
    fn checkpoint(&mut self) -> std::result::Result<(), Stop> {
        self.calls += 1;
        if self.calls == self.at {
            Err(self.stop)
        } else {
            Ok(())
        }
    }
    fn charge(&mut self, _: WorkKind, _: u64) -> std::result::Result<(), Stop> {
        self.checkpoint()
    }
}
fn probe_hidden(at: usize, stop: Stop, cached: bool) -> usize {
    let source=br#"<svg><path d="M0 0L1 0L1 1Z"/><text style="visibility:hidden">A<![CDATA[B]]></text></svg>"#;
    let mut f = Fixture::new(source, Limits::default());
    let i = f.index(b"text");
    let mut shared = DecodeSource::new(&f.lex, &mut Run).unwrap();
    let style = shared.compute_style(ElementId(i), None, &mut Run).unwrap();
    let element = view(&f.lex, i);
    let map = f.map(i);
    let scene = f.builder.root_id();
    let draft = f.builder.begin_draft(scene).unwrap();
    let prefix = f
        .builder
        .append_shape(scene, f.prefix_input(), &mut Run)
        .unwrap();
    let prefix_value = *f.builder.scene(scene).unwrap().shape(prefix).unwrap();
    let mut state = TextState {
        cursor_x: 11.,
        cursor_y: 9.,
        text_bytes: 13,
        glyph_count: 2,
        pending_space: true,
        has_position: true,
    };
    let mut cache = TextCache::default();
    let mut p = provider(Mode::Valid);
    let publisher = Publisher::default();
    let mut ledger = ChargeLedger::new(f.lex.source_owner(), 1 << 20, 8).unwrap();
    if cached {
        publish_text_request(
            TextDecodeRequest {
                element,
                source_map: map,
                end: element.end,
                style: &style,
                transform: map.transform,
                state: &mut state,
                preserve_space: false,
                provider: &mut p,
                cache: &mut cache,
                scene,
                ledger: &mut ledger,
            },
            &publisher,
            &mut f.builder,
            &draft,
            &mut Run,
        )
        .unwrap();
        assert_eq!(p.inner.seen.len(), 2);
    }
    let before = state;
    let doc_before = f.builder.document_text_commands();
    let cache_before = (cache.entries.len(), cache.text_bytes, cache.commands);
    let pointers = cache
        .entries
        .iter()
        .map(|e| (e.text.as_ptr(), e.layout.commands.as_ptr()))
        .collect::<Vec<_>>();
    let mut control = Interrupt { at, calls: 0, stop };
    let result = publish_text_request(
        TextDecodeRequest {
            element,
            source_map: map,
            end: element.end,
            style: &style,
            transform: map.transform,
            state: &mut state,
            preserve_space: false,
            provider: &mut p,
            cache: &mut cache,
            scene,
            ledger: &mut ledger,
        },
        &publisher,
        &mut f.builder,
        &draft,
        &mut control,
    );
    if at == usize::MAX {
        result.unwrap();
        assert!(state.cursor_x > before.cursor_x);
        assert_eq!(f.builder.document_text_commands(), doc_before + 10);
        assert_eq!(f.builder.scene(scene).unwrap().shapes.len(), 1);
    } else {
        let expected = match stop {
            Stop::Canceled => ErrorKind::Canceled,
            Stop::Limit => ErrorKind::Limit(LimitKind::Work),
            Stop::Overflow => ErrorKind::Limit(LimitKind::Arithmetic),
        };
        assert_eq!(result.unwrap_err().kind, expected);
        assert_eq!(state_values(state), state_values(before));
        assert_eq!(f.builder.document_text_commands(), doc_before);
        assert_eq!(
            (cache.entries.len(), cache.text_bytes, cache.commands),
            cache_before
        );
        assert_eq!(
            cache
                .entries
                .iter()
                .map(|e| (e.text.as_ptr(), e.layout.commands.as_ptr()))
                .collect::<Vec<_>>(),
            pointers
        );
        let s = f.builder.scene(scene).unwrap();
        assert_eq!(s.shapes.len(), 1);
        assert_eq!(s.shape(prefix), Some(&prefix_value));
        assert_eq!(s.edges.len(), prefix_value.edges.count);
        assert!(s.images.is_empty());
        assert!(s.clips.is_empty());
        assert!(s.groups.is_empty());
        f.builder
            .append_shape(scene, f.prefix_input(), &mut Run)
            .unwrap();
    }
    f.builder.commit_draft(&draft, &mut Run).unwrap();
    control.calls
}
#[test]
fn hidden_text_every_control_boundary_rolls_back_state_cache_budget_and_preserves_outer_prefix() {
    for cached in [false, true] {
        let events = probe_hidden(usize::MAX, Stop::Canceled, cached);
        assert!(events > 20);
        for stop in [Stop::Canceled, Stop::Limit, Stop::Overflow] {
            for at in 1..=events {
                probe_hidden(at, stop, cached);
            }
        }
    }
}

#[test]
fn seventeenth_hidden_cached_command_cap_use_fails_and_restores_prior_budget_state_and_cache() {
    let source = br#"<svg><path d="M0 0L1 0L1 1Z"/><text style="visibility:hidden">A</text></svg>"#;
    let mut f = Fixture::new(source, Limits::default());
    let i = f.index(b"text");
    let mut shared = DecodeSource::new(&f.lex, &mut Run).unwrap();
    let style = shared.compute_style(ElementId(i), None, &mut Run).unwrap();
    let element = view(&f.lex, i);
    let map = f.map(i);
    let scene = f.builder.root_id();
    let draft = f.builder.begin_draft(scene).unwrap();
    let prefix = f
        .builder
        .append_shape(scene, f.prefix_input(), &mut Run)
        .unwrap();
    let mut state = TextState::default();
    let mut cache = TextCache::default();
    let mut p = provider(Mode::CommandCap);
    let publisher = Publisher::default();
    let mut ledger = ChargeLedger::new(f.lex.source_owner(), 1 << 20, 8).unwrap();
    for count in 1..=16 {
        publish_text_request(
            TextDecodeRequest {
                element,
                source_map: map,
                end: element.end,
                style: &style,
                transform: map.transform,
                state: &mut state,
                preserve_space: false,
                provider: &mut p,
                cache: &mut cache,
                scene,
                ledger: &mut ledger,
            },
            &publisher,
            &mut f.builder,
            &draft,
            &mut Run,
        )
        .unwrap();
        assert_eq!(
            f.builder.document_text_commands(),
            count * vexel_svg_media::MAX_PATH_COMMANDS
        );
        assert_eq!(
            (state.glyph_count, state.cursor_x),
            (count, count as f64 * 8.)
        );
    }
    let before = state;
    let commands = cache.entries[0].layout.commands.as_ptr();
    let e = publish_text_request(
        TextDecodeRequest {
            element,
            source_map: map,
            end: element.end,
            style: &style,
            transform: map.transform,
            state: &mut state,
            preserve_space: false,
            provider: &mut p,
            cache: &mut cache,
            scene,
            ledger: &mut ledger,
        },
        &publisher,
        &mut f.builder,
        &draft,
        &mut Run,
    )
    .unwrap_err();
    let offset = source.iter().position(|&v| v == b'A').unwrap();
    assert_eq!(
        (e.kind, e.offset),
        (ErrorKind::Limit(LimitKind::TextCommands), offset)
    );
    assert_eq!(state_values(state), state_values(before));
    assert_eq!(p.inner.seen.len(), 1);
    assert_eq!(f.builder.document_text_commands(), 4 * 1024 * 1024);
    assert_eq!(
        (cache.entries.len(), cache.text_bytes, cache.commands),
        (1, 1, 262144)
    );
    assert_eq!(cache.entries[0].layout.commands.as_ptr(), commands);
    assert!(
        publisher
            .calls
            .borrow()
            .iter()
            .all(|&s| s == "begin" || s == "finish")
    );
    let s = f.builder.scene(scene).unwrap();
    assert_eq!(s.shapes.len(), 1);
    assert!(s.shape(prefix).is_some());
    f.builder
        .append_shape(scene, f.prefix_input(), &mut Run)
        .unwrap();
    f.builder.commit_draft(&draft, &mut Run).unwrap();
}
