//! Required request atomicity regressions, converted from the archived defect proofs.
mod support;
use support::*;
use vexel_scene::*;
use vexel_svg_media::{decode_image_request, render_text_request};

const IMAGE_SOURCE: &[u8] = br#"<svg><path d="M0 0L1 0L1 1Z"/><image width="4" height="4" href="data:image/png;base64,AQID"/></svg>"#;

#[test]
fn accepted_request_savepoint_preserves_prefix_and_outer_draft_usability() {
    let mut fixture = Fixture::new(IMAGE_SOURCE, Limits::default());
    let scene = fixture.builder.root_id();
    let draft = fixture.builder.begin_draft(scene).unwrap();
    let input = fixture.prefix_input();
    let prefix = fixture
        .builder
        .append_shape(scene, input, &mut Run)
        .unwrap();
    let value = *fixture.builder.scene(scene).unwrap().shape(prefix).unwrap();
    let token = fixture.builder.begin_request(&draft, &mut Run).unwrap();
    let input = fixture.prefix_input();
    let discarded = fixture
        .builder
        .append_shape(scene, input, &mut Run)
        .unwrap();
    fixture.builder.rollback_request(&draft, token).unwrap();
    assert!(
        fixture
            .builder
            .scene(scene)
            .unwrap()
            .shape(discarded)
            .is_none()
    );
    assert_eq!(fixture.builder.scene(scene).unwrap().shapes.len(), 1);
    fixture
        .builder
        .finalize_shape(&draft, prefix, value, &mut Run)
        .unwrap();
    let token = fixture.builder.begin_request(&draft, &mut Run).unwrap();
    let input = fixture.prefix_input();
    let retained = fixture
        .builder
        .append_shape(scene, input, &mut Run)
        .unwrap();
    fixture
        .builder
        .commit_request(&draft, token, &mut Run)
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
    assert!(
        fixture
            .builder
            .scene(scene)
            .unwrap()
            .shape(retained)
            .is_some()
    );
    assert_eq!(fixture.builder.scene(scene).unwrap().shapes.len(), 2);
}

fn state_value(state: TextState) -> (f64, f64, usize, usize, bool, bool) {
    (
        state.cursor_x,
        state.cursor_y,
        state.text_bytes,
        state.glyph_count,
        state.pending_space,
        state.has_position,
    )
}
fn initial_state() -> TextState {
    TextState {
        cursor_x: 7.,
        cursor_y: 13.,
        text_bytes: 11,
        glyph_count: 3,
        pending_space: true,
        has_position: true,
    }
}
fn cache_request() -> TextRequest<'static> {
    TextRequest {
        text: b"prefix",
        font_families: &[],
        size_points: 12.,
        weight: 400.,
        style: TextStyle::Normal,
        letter_spacing_points: 0.,
    }
}
fn seed_cache(cache: &mut TextCache, provider: &mut Provider) {
    assert!(
        vexel_svg_media::layout_shared_cache(cache, cache_request(), &[], provider, &mut Run, 0)
            .unwrap()
            .is_cached()
    );
}
fn assert_cache_prefix(
    cache: &mut TextCache,
    provider: &mut Provider,
    pointers: (*const u8, *const TextCommand),
) {
    assert_eq!(
        (cache.entries.len(), cache.text_bytes, cache.commands),
        (1, 6, 5)
    );
    assert_eq!(cache.entries[0].text.as_ptr(), pointers.0);
    assert_eq!(cache.entries[0].layout.commands.as_ptr(), pointers.1);
    let calls = provider.seen.len();
    assert!(
        vexel_svg_media::layout_shared_cache(cache, cache_request(), &[], provider, &mut Run, 0)
            .unwrap()
            .is_cached()
    );
    assert_eq!(provider.seen.len(), calls);
}

struct StopAtEdge;
impl WorkControl for StopAtEdge {
    fn charge(&mut self, kind: WorkKind, _: u64) -> std::result::Result<(), Stop> {
        if kind == WorkKind::Edge {
            Err(Stop::Canceled)
        } else {
            Ok(())
        }
    }
    fn checkpoint(&mut self) -> std::result::Result<(), Stop> {
        Ok(())
    }
}

fn image_failure(limits: Limits, control: &mut dyn WorkControl) -> Diagnostic {
    let mut fixture = Fixture::new(IMAGE_SOURCE, limits);
    let index = fixture.index(b"image");
    let map = fixture.map(index);
    let element = view(&fixture.lex, index);
    let scene = fixture.builder.root_id();
    let draft = fixture.builder.begin_draft(scene).unwrap();
    let input = fixture.prefix_input();
    let prefix = fixture
        .builder
        .append_shape(scene, input, &mut Run)
        .unwrap();
    let prefix_value = *fixture.builder.scene(scene).unwrap().shape(prefix).unwrap();
    let prefix_edges = fixture.builder.scene(scene).unwrap().edges.to_vec();
    let mut ledger = ChargeLedger::new(fixture.lex.source_owner(), 1 << 20, 8).unwrap();
    let mut provider = Provider::default();
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
        control,
    )
    .unwrap_err();
    let residual = fixture.builder.scene(scene).unwrap();
    assert_eq!(
        provider.image_calls, 1,
        "failure is after real fixture pixels are decoded"
    );
    assert!(
        residual.images.is_empty(),
        "failed request must release its decoded image"
    );
    assert_eq!(residual.shapes.len(), 1);
    assert_eq!(residual.shape(prefix).unwrap().source, prefix_value.source);
    assert_eq!(residual.edges, prefix_edges);
    fixture
        .builder
        .finalize_shape(&draft, prefix, prefix_value, &mut Run)
        .unwrap();
    assert_eq!(fixture.builder.document_text_commands(), 0);
    let token = fixture.builder.begin_request(&draft, &mut Run).unwrap();
    fixture
        .builder
        .commit_request(&draft, token, &mut Run)
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
    assert_eq!(fixture.builder.scene(scene).unwrap().shapes.len(), 1);
    error
}

#[test]
fn image_late_shape_limit_restores_prefix_without_residual_pixels() {
    let error = image_failure(
        Limits {
            shapes: 1,
            ..Default::default()
        },
        &mut Run,
    );
    assert_eq!(error.kind, ErrorKind::Limit(LimitKind::Shapes));
}

#[test]
fn image_late_cancellation_restores_prefix_without_residual_pixels() {
    let error = image_failure(Limits::default(), &mut StopAtEdge);
    assert_eq!(error.kind, ErrorKind::Canceled);
}

#[test]
fn image_late_capacity_failure_restores_prefix_without_residual_pixels() {
    let mut probe = Fixture::new(IMAGE_SOURCE, Limits::default());
    let scene = probe.builder.root_id();
    let draft = probe.builder.begin_draft(scene).unwrap();
    let input = probe.prefix_input();
    probe.builder.append_shape(scene, input, &mut Run).unwrap();
    // Measure the accepted builder's actual admitted Vec capacities, rather
    // than assuming an allocator growth size or using logical pixel lengths.
    let pixels = Provider::default()
        .decode_image(b"fixture", &mut Run)
        .unwrap();
    probe
        .builder
        .append_image(
            scene,
            Image {
                pixels,
                x: 0.,
                y: 0.,
                width: 4.,
                height: 4.,
                draw_x: 0.,
                draw_y: 0.,
                draw_width: 4.,
                draw_height: 4.,
            },
            &mut Run,
        )
        .unwrap();
    let budget = probe.builder.memory_stats().live_bytes;
    probe.builder.rollback_draft(draft).unwrap();
    let error = image_failure(
        Limits {
            reserved_bytes: budget,
            ..Default::default()
        },
        &mut Run,
    );
    assert_eq!(error.kind, ErrorKind::Limit(LimitKind::ReservedBytes));
}

#[test]
fn text_late_capacity_failure_restores_layout_state_cache_and_prefix() {
    let source = format!(
        "<svg><path d=\"M0 0L1 0L1 1Z\"/><text>{}</text></svg>",
        "A".repeat(128)
    );
    let mut probe = Fixture::new(source.as_bytes(), Limits::default());
    let scene = probe.builder.root_id();
    let draft = probe.builder.begin_draft(scene).unwrap();
    let input = probe.prefix_input();
    probe.builder.append_shape(scene, input, &mut Run).unwrap();
    let budget = probe.builder.memory_stats().peak_live_bytes;
    probe.builder.rollback_draft(draft).unwrap();

    let mut fixture = Fixture::new(
        source.as_bytes(),
        Limits {
            reserved_bytes: budget,
            ..Default::default()
        },
    );
    let index = fixture.index(b"text");
    let element = view(&fixture.lex, index);
    let map = fixture.map(index);
    let scene = fixture.builder.root_id();
    let draft = fixture.builder.begin_draft(scene).unwrap();
    let input = fixture.prefix_input();
    let prefix = fixture
        .builder
        .append_shape(scene, input, &mut Run)
        .unwrap();
    let mut ledger = ChargeLedger::new(fixture.lex.source_owner(), 1 << 20, 8).unwrap();
    let mut provider = Provider::default();
    let mut state = initial_state();
    let state_before = state;
    let mut cache = TextCache::default();
    seed_cache(&mut cache, &mut provider);
    let cache_pointers = (
        cache.entries[0].text.as_ptr(),
        cache.entries[0].layout.commands.as_ptr(),
    );
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
    assert_eq!(error.kind, ErrorKind::Limit(LimitKind::ReservedBytes));
    assert_eq!(provider.seen.len(), 2);
    assert_eq!(state_value(state), state_value(state_before));
    assert_cache_prefix(&mut cache, &mut provider, cache_pointers);
    assert_eq!(fixture.builder.document_text_commands(), 0);
    assert_eq!(fixture.builder.scene(scene).unwrap().shapes.len(), 1);
    let value = *fixture.builder.scene(scene).unwrap().shape(prefix).unwrap();
    fixture
        .builder
        .finalize_shape(&draft, prefix, value, &mut Run)
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
}

#[test]
fn text_second_run_limit_restores_first_run_state_cache_and_prefix() {
    let source = br#"<svg><path d="M0 0L1 0L1 1Z"/><text>A<![CDATA[B]]></text></svg>"#;
    let mut fixture = Fixture::new(
        source,
        Limits {
            shapes: 2,
            ..Default::default()
        },
    );
    let index = fixture.index(b"text");
    let map = fixture.map(index);
    let element = view(&fixture.lex, index);
    let scene = fixture.builder.root_id();
    let draft = fixture.builder.begin_draft(scene).unwrap();
    let input = fixture.prefix_input();
    let prefix = fixture
        .builder
        .append_shape(scene, input, &mut Run)
        .unwrap();
    let mut ledger = ChargeLedger::new(fixture.lex.source_owner(), 1 << 20, 8).unwrap();
    let mut provider = Provider::default();
    let mut state = initial_state();
    let state_before = state;
    let mut cache = TextCache::default();
    seed_cache(&mut cache, &mut provider);
    let cache_pointers = (
        cache.entries[0].text.as_ptr(),
        cache.entries[0].layout.commands.as_ptr(),
    );
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
    assert_eq!(error.kind, ErrorKind::Limit(LimitKind::Shapes));
    assert_eq!(provider.seen.len(), 3);
    assert_eq!(fixture.builder.scene(scene).unwrap().shapes.len(), 1);
    assert!(
        fixture
            .builder
            .scene(scene)
            .unwrap()
            .shape(prefix)
            .is_some()
    );
    assert_eq!(state_value(state), state_value(state_before));
    assert_cache_prefix(&mut cache, &mut provider, cache_pointers);
    assert_eq!(fixture.builder.document_text_commands(), 0);
    let input = fixture.prefix_input();
    fixture
        .builder
        .append_shape(scene, input, &mut Run)
        .unwrap();
    fixture.builder.commit_draft(&draft, &mut Run).unwrap();
    assert_eq!(fixture.builder.scene(scene).unwrap().shapes.len(), 2);
}

struct StopOnTextFinalize {
    edges: usize,
    final_checkpoints: usize,
}
impl WorkControl for StopOnTextFinalize {
    fn charge(&mut self, kind: WorkKind, units: u64) -> std::result::Result<(), Stop> {
        if kind == WorkKind::Edge {
            self.edges += units as usize;
        }
        Ok(())
    }
    fn checkpoint(&mut self) -> std::result::Result<(), Stop> {
        // Four geometry edges, four path-validation edges, four publish edges.
        // The publish checkpoint succeeds; finalization then stops.
        if self.edges == 12 {
            self.final_checkpoints += 1;
            if self.final_checkpoints == 2 {
                return Err(Stop::Canceled);
            }
        }
        Ok(())
    }
}

#[test]
fn text_finalization_cancel_restores_shape_state_cache_and_allows_retry() {
    let source = br#"<svg><path d="M0 0L1 0L1 1Z"/><text>A</text></svg>"#;
    let mut fixture = Fixture::new(source, Limits::default());
    let index = fixture.index(b"text");
    let map = fixture.map(index);
    let element = view(&fixture.lex, index);
    let scene = fixture.builder.root_id();
    let draft = fixture.builder.begin_draft(scene).unwrap();
    let input = fixture.prefix_input();
    let prefix = fixture
        .builder
        .append_shape(scene, input, &mut Run)
        .unwrap();
    let mut ledger = ChargeLedger::new(fixture.lex.source_owner(), 1 << 20, 8).unwrap();
    let mut provider = Provider::default();
    let mut state = initial_state();
    let state_before = state;
    let mut cache = TextCache::default();
    seed_cache(&mut cache, &mut provider);
    let cache_pointers = (
        cache.entries[0].text.as_ptr(),
        cache.entries[0].layout.commands.as_ptr(),
    );
    let mut control = StopOnTextFinalize {
        edges: 0,
        final_checkpoints: 0,
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
        &mut control,
    )
    .unwrap_err();
    assert_eq!(error.kind, ErrorKind::Canceled);
    assert_eq!(control.final_checkpoints, 2);
    assert_eq!(provider.seen.len(), 2);
    assert_eq!(fixture.builder.scene(scene).unwrap().shapes.len(), 1);
    assert_eq!(state_value(state), state_value(state_before));
    assert_cache_prefix(&mut cache, &mut provider, cache_pointers);
    assert_eq!(fixture.builder.document_text_commands(), 0);
    let value = *fixture.builder.scene(scene).unwrap().shape(prefix).unwrap();
    fixture
        .builder
        .finalize_shape(&draft, prefix, value, &mut Run)
        .unwrap();
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
        &mut Run,
    )
    .unwrap();
    assert_eq!(
        provider.seen.len(),
        3,
        "failed cache admission cannot serve the retry"
    );
    assert_eq!(fixture.builder.document_text_commands(), 5);
    fixture.builder.commit_draft(&draft, &mut Run).unwrap();
    assert_eq!(fixture.builder.scene(scene).unwrap().shapes.len(), 2);
}
