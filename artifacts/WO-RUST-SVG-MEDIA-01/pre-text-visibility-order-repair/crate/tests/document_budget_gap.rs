//! Cumulative document command and request rollback regressions.
mod support;
use support::*;
use vexel_scene::*;
use vexel_svg_media::{MAX_PATH_COMMANDS, render_text_request};

struct BudgetFixtureProvider { calls: usize }
impl MediaProvider for BudgetFixtureProvider {
    fn layout_text(&mut self, _: TextRequest<'_>, _: &mut dyn WorkControl)
        -> std::result::Result<TextLayout, MediaError>
    {
        self.calls += 1;
        // A real rectangle contour plus valid extra provider commands. Original
        // budgeting counts the entire layout, not only referenced outline verbs.
        let mut commands = vec![TextCommand { verb: TextVerb::Close, values: [0.; 6] }; MAX_PATH_COMMANDS];
        commands[..5].copy_from_slice(&rectangle());
        Ok(TextLayout {
            glyphs: vec![TextGlyph { glyph_id: None, x: 0., y: 0., advance: 1.,
                commands: TableRange { first: 0, count: 5 } }],
            commands,
            advance_width: 1., ascent: 0., descent: 0., leading: 0.,
        })
    }
    fn decode_image(&mut self, _: &[u8], _: &mut dyn WorkControl)
        -> std::result::Result<ImagePixels, MediaError> { Err(MediaError::Unsupported) }
}

struct CancelAtEdge;
impl WorkControl for CancelAtEdge {
    fn charge(&mut self, kind: WorkKind, _: u64) -> std::result::Result<(), Stop> {
        if kind == WorkKind::Edge { Err(Stop::Canceled) } else { Ok(()) }
    }
    fn checkpoint(&mut self) -> std::result::Result<(), Stop> { Ok(()) }
}
fn state_value(s: TextState) -> (f64, f64, usize, usize, bool, bool) {
    (s.cursor_x, s.cursor_y, s.text_bytes, s.glyph_count, s.pending_space, s.has_position)
}

#[test]
fn seventeenth_cached_layout_use_hits_document_limit_and_restores_counters() {
    let mut fixture = Fixture::new(br#"<svg><path d="M0 0L1 0L1 1Z"/><text>A</text></svg>"#, Limits::default());
    let index = fixture.index(b"text");
    let element = view(&fixture.lex, index);
    let map = fixture.map(index);
    let offset = fixture.lex.tokens().iter().find(|t| t.value.slice(fixture.lex.source()) == Some(b"A"))
        .unwrap().value.offset;
    let scene = fixture.builder.root_id();
    let draft = fixture.builder.begin_draft(scene).unwrap();
    let input = fixture.prefix_input();
    let prefix = fixture.builder.append_shape(scene, input, &mut Run).unwrap();
    let mut ledger = ChargeLedger::new(fixture.lex.source_owner(), 1 << 20, 8).unwrap();
    let mut provider = BudgetFixtureProvider { calls: 0 };
    let mut state = TextState::default();
    let mut cache = TextCache::default();
    for count in 1..=16 {
        render_text_request(TextDecodeRequest {
            element, source_map: map, end: element.end, style: &CssPaint::initial(),
            transform: vexel_geometry::affine_identity(), state: &mut state,
            preserve_space: false, provider: &mut provider, cache: &mut cache,
            scene, ledger: &mut ledger,
        }, &mut fixture.builder, &draft, &mut Run).unwrap();
        assert_eq!(fixture.builder.document_text_commands(), count * MAX_PATH_COMMANDS);
        assert_eq!(cache.commands, MAX_PATH_COMMANDS, "cache count does not count document uses");
        if count == 8 {
            let before = state_value(state);
            let shapes_before = fixture.builder.scene(scene).unwrap().shapes.len();
            let error = render_text_request(TextDecodeRequest {
                element, source_map: map, end: element.end, style: &CssPaint::initial(),
                transform: vexel_geometry::affine_identity(), state: &mut state,
                preserve_space: false, provider: &mut provider, cache: &mut cache,
                scene, ledger: &mut ledger,
            }, &mut fixture.builder, &draft, &mut CancelAtEdge).unwrap_err();
            assert_eq!(error.kind, ErrorKind::Canceled);
            assert_eq!(state_value(state), before);
            assert_eq!(fixture.builder.document_text_commands(), 8 * MAX_PATH_COMMANDS,
                "late failure rolls back this use while retaining earlier document admissions");
            assert_eq!(fixture.builder.scene(scene).unwrap().shapes.len(), shapes_before);
        }
    }
    assert_eq!(fixture.builder.document_text_commands(), DOCUMENT_TEXT_COMMAND_LIMIT);
    assert_eq!(fixture.builder.scene(scene).unwrap().shapes.len(), 17);
    let before = state_value(state);
    let command_pointer = cache.entries[0].layout.commands.as_ptr();
    let edges_before = fixture.builder.scene(scene).unwrap().edges.to_vec();
    for _ in 0..2 {
        let error = render_text_request(TextDecodeRequest {
            element, source_map: map, end: element.end, style: &CssPaint::initial(),
            transform: vexel_geometry::affine_identity(), state: &mut state,
            preserve_space: false, provider: &mut provider, cache: &mut cache,
            scene, ledger: &mut ledger,
        }, &mut fixture.builder, &draft, &mut Run).unwrap_err();
        assert_eq!((error.kind, error.offset), (ErrorKind::Limit(LimitKind::TextCommands), offset));
        assert_eq!(state_value(state), before);
        assert_eq!(fixture.builder.document_text_commands(), DOCUMENT_TEXT_COMMAND_LIMIT);
        assert_eq!((cache.entries.len(), cache.text_bytes, cache.commands), (1, 1, MAX_PATH_COMMANDS));
        assert_eq!(cache.entries[0].layout.commands.as_ptr(), command_pointer);
        assert_eq!(fixture.builder.scene(scene).unwrap().edges, edges_before);
    }
    assert_eq!(provider.calls, 1, "all uses and rejected retries borrow the original cached layout");
    assert_eq!(state.glyph_count, 16);
    let value = *fixture.builder.scene(scene).unwrap().shape(prefix).unwrap();
    fixture.builder.finalize_shape(&draft, prefix, value, &mut Run).unwrap();
    fixture.builder.commit_draft(&draft, &mut Run).unwrap();
    assert!(fixture.builder.scene(scene).unwrap().shape(prefix).is_some());
}
