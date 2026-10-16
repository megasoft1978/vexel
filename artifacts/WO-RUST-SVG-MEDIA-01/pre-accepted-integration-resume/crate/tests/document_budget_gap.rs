//! Full-scope defect evidence, not an acceptance test for document limits.
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
        // Legacy validation admits commands not referenced by a glyph range.
        // They still consume its document-wide layout command budget.
        Ok(TextLayout {
            glyphs: vec![TextGlyph { glyph_id: None, x: 0., y: 0., advance: 1.,
                commands: TableRange::EMPTY }],
            commands: vec![TextCommand { verb: TextVerb::Close, values: [0.; 6] }; MAX_PATH_COMMANDS],
            advance_width: 1., ascent: 0., descent: 0., leading: 0.,
        })
    }
    fn decode_image(&mut self, _: &[u8], _: &mut dyn WorkControl)
        -> std::result::Result<ImagePixels, MediaError> { Err(MediaError::Unsupported) }
}

#[test]
fn document_command_limit_is_missing_on_repeated_cached_layout_uses() {
    let mut fixture = Fixture::new(b"<svg><text>A</text></svg>", Limits::default());
    let index = fixture.index(b"text");
    let element = view(&fixture.lex, index);
    let map = fixture.map(index);
    let scene = fixture.builder.root_id();
    let draft = fixture.builder.begin_draft(scene).unwrap();
    let mut ledger = ChargeLedger::new(fixture.lex.source_owner(), 1 << 20, 8).unwrap();
    let mut provider = BudgetFixtureProvider { calls: 0 };
    let mut state = TextState::default();
    let mut cache = TextCache::default();
    let mut successes = 0;
    for _ in 0..17 {
        render_text_request(TextDecodeRequest {
            element, source_map: map, end: element.end, style: &CssPaint::initial(),
            transform: vexel_geometry::affine_identity(), state: &mut state,
            preserve_space: false, provider: &mut provider, cache: &mut cache,
            scene, ledger: &mut ledger,
        }, &mut fixture.builder, &draft, &mut Run).unwrap();
        successes += 1;
    }
    assert_eq!(provider.calls, 1);
    assert_eq!(cache.commands, MAX_PATH_COMMANDS);
    assert_eq!(successes, 17, "KNOWN DEFECT: original document limit rejects use 17");
    assert_eq!(state.glyph_count, 17);
    assert!(17 * MAX_PATH_COMMANDS > 4 * 1024 * 1024);
    fixture.builder.rollback_draft(draft).unwrap();
}
