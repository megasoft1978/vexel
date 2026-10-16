mod support;

use support::*;
use vexel_base::WorkControl;
use vexel_scene::{
    ChargeLedger, CssPaint, ErrorKind, ImagePixels, Limits, MediaError, MediaProvider, TextCache,
    TextDecodeRequest, TextLayout, TextRequest, TextState,
};
use vexel_svg_media::render_text_request;

struct OverflowProvider(Provider);
impl MediaProvider for OverflowProvider {
    fn layout_text(
        &mut self,
        request: TextRequest<'_>,
        control: &mut dyn WorkControl,
    ) -> Result<TextLayout, MediaError> {
        let mut layout = self.0.layout_text(request, control)?;
        layout.advance_width = f64::MAX;
        layout.commands.clear();
        for glyph in &mut layout.glyphs {
            glyph.commands.count = 0;
        }
        Ok(layout)
    }

    fn decode_image(
        &mut self,
        encoded: &[u8],
        control: &mut dyn WorkControl,
    ) -> Result<ImagePixels, MediaError> {
        self.0.decode_image(encoded, control)
    }
}

#[test]
fn finite_advance_overflow_rolls_back_cache_budget_and_scene_prefix() {
    let source = b"<svg><path d=\"M0 0L1 0L1 1Z\"/><text>A</text></svg>";
    let mut fixture = Fixture::new(source, Limits::default());
    let index = fixture.index(b"text");
    let text = view(&fixture.lex, index);
    let source_map = fixture.map(index);
    let scene = fixture.builder.root_id();
    let draft = fixture.builder.begin_draft(scene).unwrap();
    let prefix = fixture
        .builder
        .append_shape(scene, fixture.prefix_input(), &mut Run)
        .unwrap();
    let prefix_shape = *fixture.builder.scene(scene).unwrap().shape(prefix).unwrap();
    let mut state = TextState::default();
    state.cursor_x = 11.;
    state.cursor_y = 7.;
    state.has_position = true;
    let state_before = (
        state.text_bytes,
        state.glyph_count,
        state.cursor_x,
        state.cursor_y,
        state.has_position,
        state.pending_space,
    );
    let mut cache = TextCache::default();
    let mut ledger = ChargeLedger::new(fixture.lex.source_owner(), 1 << 20, 8).unwrap();
    let mut provider = OverflowProvider(Provider::default());
    let budget_before = fixture.builder.document_text_commands();
    let scene_before = fixture.builder.scene(scene).unwrap();
    let shape_count_before = scene_before.shapes.len();
    let edge_count_before = scene_before.edges.len();
    let image_count_before = scene_before.images.len();

    let error = render_text_request(
        TextDecodeRequest {
            element: text,
            source_map,
            end: text.end,
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

    let authored_offset = source.windows(3).position(|bytes| bytes == b">A<").unwrap() + 1;
    assert_eq!(
        (error.kind, error.offset),
        (ErrorKind::Number, authored_offset)
    );
    assert_eq!(
        (
            state.text_bytes,
            state.glyph_count,
            state.cursor_x,
            state.cursor_y,
            state.has_position,
            state.pending_space,
        ),
        state_before
    );
    assert!(cache.entries.is_empty());
    assert_eq!(cache.text_bytes, 0);
    assert_eq!(cache.commands, 0);
    assert_eq!(fixture.builder.document_text_commands(), budget_before);
    let scene_after = fixture.builder.scene(scene).unwrap();
    assert_eq!(scene_after.shapes.len(), shape_count_before);
    assert_eq!(scene_after.edges.len(), edge_count_before);
    assert_eq!(scene_after.images.len(), image_count_before);
    assert_eq!(scene_after.shape(prefix), Some(&prefix_shape));
    assert_eq!(provider.0.seen.len(), 1);
    assert_eq!(provider.0.seen[0].text, b"A");
    fixture
        .builder
        .append_shape(scene, fixture.prefix_input(), &mut Run)
        .unwrap();
    fixture.builder.commit_draft(&draft, &mut Run).unwrap();
}
