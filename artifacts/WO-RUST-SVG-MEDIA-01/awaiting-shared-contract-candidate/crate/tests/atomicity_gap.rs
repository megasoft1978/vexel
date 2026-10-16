//! Defect evidence for the missing accepted shared request savepoint contract.
//! Passing these tests documents the blocker; it does not prove R2 atomicity.
mod support;
use support::*;
use vexel_scene::*;
use vexel_svg_media::{decode_image_request, render_text_request};

const IMAGE_SOURCE: &[u8] = br#"<svg><path d="M0 0L1 0L1 1Z"/><image width="4" height="4" href="data:image/png;base64,AQID"/></svg>"#;

#[test]
fn accepted_public_api_rejects_request_savepoints_inside_outer_draft() {
    let mut fixture = Fixture::new(IMAGE_SOURCE, Limits::default());
    let scene = fixture.builder.root_id();
    let before_outer = fixture.builder.mark(scene).unwrap();
    let draft = fixture.builder.begin_draft(scene).unwrap();
    let input = fixture.prefix_input();
    let prefix = fixture.builder.append_shape(scene, input, &mut Run).unwrap();
    assert_eq!(fixture.builder.mark(scene).unwrap_err().kind, ErrorKind::InvalidArgument);
    assert_eq!(fixture.builder.rollback(before_outer).unwrap_err().kind, ErrorKind::InvalidArgument);
    assert_eq!(fixture.builder.begin_draft(scene).unwrap_err().kind, ErrorKind::InvalidArgument);
    let value = *fixture.builder.scene(scene).unwrap().shape(prefix).unwrap();
    fixture.builder.finalize_shape(&draft, prefix, value, &mut Run).unwrap();
    let input = fixture.prefix_input();
    fixture.builder.append_shape(scene, input, &mut Run).unwrap();
    fixture.builder.commit_draft(&draft, &mut Run).unwrap();
    assert_eq!(fixture.builder.scene(scene).unwrap().shapes.len(), 2);
}

struct StopAtEdge;
impl WorkControl for StopAtEdge {
    fn charge(&mut self, kind: WorkKind, _: u64) -> std::result::Result<(), Stop> {
        if kind == WorkKind::Edge { Err(Stop::Canceled) } else { Ok(()) }
    }
    fn checkpoint(&mut self) -> std::result::Result<(), Stop> { Ok(()) }
}

fn image_failure(limits: Limits, control: &mut dyn WorkControl) -> Diagnostic {
    let mut fixture = Fixture::new(IMAGE_SOURCE, limits);
    let index = fixture.index(b"image");
    let map = fixture.map(index);
    let element = view(&fixture.lex, index);
    let scene = fixture.builder.root_id();
    let draft = fixture.builder.begin_draft(scene).unwrap();
    let input = fixture.prefix_input();
    let prefix = fixture.builder.append_shape(scene, input, &mut Run).unwrap();
    let prefix_value = *fixture.builder.scene(scene).unwrap().shape(prefix).unwrap();
    let prefix_edges = fixture.builder.scene(scene).unwrap().edges.to_vec();
    let mut ledger = ChargeLedger::new(fixture.lex.source_owner(), 1 << 20, 8).unwrap();
    let mut provider = Provider::default();
    let error = decode_image_request(ImageDecodeRequest { element, source_map: map,
        style: &CssPaint::initial(), transform: vexel_geometry::affine_identity(),
        provider: &mut provider, scene, ledger: &mut ledger },
        &mut fixture.builder, &draft, control).unwrap_err();
    let residual = fixture.builder.scene(scene).unwrap();
    assert_eq!(provider.image_calls, 1, "failure is after real fixture pixels are decoded");
    assert_eq!(residual.images.len(), 1, "KNOWN DEFECT: failed request leaves decoded image");
    assert_eq!(residual.shapes.len(), 1);
    assert_eq!(residual.shape(prefix).unwrap().source, prefix_value.source);
    assert_eq!(residual.edges, prefix_edges);
    fixture.builder.finalize_shape(&draft, prefix, prefix_value, &mut Run).unwrap();
    fixture.builder.rollback_draft(draft).unwrap();
    assert!(fixture.builder.scene(scene).unwrap().images.is_empty());
    assert!(fixture.builder.scene(scene).unwrap().shapes.is_empty(),
        "outer rollback also erases the prefix: it cannot implement request rollback");
    error
}

#[test]
fn image_late_shape_limit_documents_residual_pixels_and_missing_atomicity() {
    let error = image_failure(Limits { shapes: 1, ..Default::default() }, &mut Run);
    assert_eq!(error.kind, ErrorKind::Limit(LimitKind::Shapes));
}

#[test]
fn image_late_cancellation_documents_residual_pixels_and_missing_atomicity() {
    let error = image_failure(Limits::default(), &mut StopAtEdge);
    assert_eq!(error.kind, ErrorKind::Canceled);
}

#[test]
fn image_late_capacity_admission_failure_documents_residual_pixels() {
    let mut probe = Fixture::new(IMAGE_SOURCE, Limits::default());
    let scene = probe.builder.root_id();
    let draft = probe.builder.begin_draft(scene).unwrap();
    let input = probe.prefix_input();
    probe.builder.append_shape(scene, input, &mut Run).unwrap();
    // Measure the accepted builder's actual admitted Vec capacities, rather
    // than assuming an allocator growth size or using logical pixel lengths.
    let pixels = Provider::default().decode_image(b"fixture", &mut Run).unwrap();
    probe.builder.append_image(scene, Image { pixels, x: 0., y: 0., width: 4., height: 4.,
        draw_x: 0., draw_y: 0., draw_width: 4., draw_height: 4. }, &mut Run).unwrap();
    let budget = probe.builder.memory_stats().live_bytes;
    probe.builder.rollback_draft(draft).unwrap();
    let error = image_failure(Limits { reserved_bytes: budget, ..Default::default() }, &mut Run);
    assert_eq!(error.kind, ErrorKind::Limit(LimitKind::ReservedBytes));
}

#[test]
fn text_late_capacity_failure_documents_retained_layout_state_and_cache() {
    let source = format!("<svg><path d=\"M0 0L1 0L1 1Z\"/><text>{}</text></svg>", "A".repeat(128));
    let mut probe = Fixture::new(source.as_bytes(), Limits::default());
    let scene = probe.builder.root_id();
    let draft = probe.builder.begin_draft(scene).unwrap();
    let input = probe.prefix_input();
    probe.builder.append_shape(scene, input, &mut Run).unwrap();
    let budget = probe.builder.memory_stats().peak_live_bytes;
    probe.builder.rollback_draft(draft).unwrap();

    let mut fixture = Fixture::new(source.as_bytes(), Limits { reserved_bytes: budget, ..Default::default() });
    let index = fixture.index(b"text");
    let element = view(&fixture.lex, index);
    let map = fixture.map(index);
    let scene = fixture.builder.root_id();
    let draft = fixture.builder.begin_draft(scene).unwrap();
    let input = fixture.prefix_input();
    let prefix = fixture.builder.append_shape(scene, input, &mut Run).unwrap();
    let mut ledger = ChargeLedger::new(fixture.lex.source_owner(), 1 << 20, 8).unwrap();
    let mut provider = Provider::default();
    let mut state = TextState::default();
    let mut cache = TextCache::default();
    let error = render_text_request(TextDecodeRequest { element, source_map: map,
        end: element.end, style: &CssPaint::initial(), transform: vexel_geometry::affine_identity(),
        state: &mut state, preserve_space: false, provider: &mut provider,
        cache: &mut cache, scene, ledger: &mut ledger },
        &mut fixture.builder, &draft, &mut Run).unwrap_err();
    assert_eq!(error.kind, ErrorKind::Limit(LimitKind::ReservedBytes));
    assert_eq!(provider.seen.len(), 1);
    assert_eq!((state.text_bytes, state.glyph_count, cache.entries.len()), (128, 128, 1),
        "KNOWN DEFECT: failed request keeps admitted glyph layout and mutated state");
    assert_eq!(fixture.builder.scene(scene).unwrap().shapes.len(), 1);
    let value = *fixture.builder.scene(scene).unwrap().shape(prefix).unwrap();
    fixture.builder.finalize_shape(&draft, prefix, value, &mut Run).unwrap();
    fixture.builder.rollback_draft(draft).unwrap();
}

#[test]
fn text_second_run_limit_documents_retained_first_run_state_and_cache() {
    let source = br#"<svg><path d="M0 0L1 0L1 1Z"/><text>A<![CDATA[B]]></text></svg>"#;
    let mut fixture = Fixture::new(source, Limits { shapes: 2, ..Default::default() });
    let index = fixture.index(b"text");
    let map = fixture.map(index);
    let element = view(&fixture.lex, index);
    let scene = fixture.builder.root_id();
    let draft = fixture.builder.begin_draft(scene).unwrap();
    let input = fixture.prefix_input();
    let prefix = fixture.builder.append_shape(scene, input, &mut Run).unwrap();
    let mut ledger = ChargeLedger::new(fixture.lex.source_owner(), 1 << 20, 8).unwrap();
    let mut provider = Provider::default();
    let mut state = TextState::default();
    let mut cache = TextCache::default();
    let error = render_text_request(TextDecodeRequest { element, source_map: map,
        end: element.end, style: &CssPaint::initial(), transform: vexel_geometry::affine_identity(),
        state: &mut state, preserve_space: false, provider: &mut provider,
        cache: &mut cache, scene, ledger: &mut ledger },
        &mut fixture.builder, &draft, &mut Run).unwrap_err();
    assert_eq!(error.kind, ErrorKind::Limit(LimitKind::Shapes));
    assert_eq!(provider.seen.len(), 2);
    assert_eq!(fixture.builder.scene(scene).unwrap().shapes.len(), 2,
        "KNOWN DEFECT: failed text request leaves its first run");
    assert!(fixture.builder.scene(scene).unwrap().shape(prefix).is_some());
    assert_eq!((state.text_bytes, state.glyph_count, cache.entries.len()), (2, 2, 2));
    fixture.builder.rollback_draft(draft).unwrap();
}

struct StopOnTextFinalize { edges: usize, final_checkpoints: usize }
impl WorkControl for StopOnTextFinalize {
    fn charge(&mut self, kind: WorkKind, units: u64) -> std::result::Result<(), Stop> {
        if kind == WorkKind::Edge { self.edges += units as usize; }
        Ok(())
    }
    fn checkpoint(&mut self) -> std::result::Result<(), Stop> {
        // Four geometry edges, four path-validation edges, four publish edges.
        // The publish checkpoint succeeds; finalization then stops.
        if self.edges == 12 {
            self.final_checkpoints += 1;
            if self.final_checkpoints == 2 { return Err(Stop::Canceled); }
        }
        Ok(())
    }
}

#[test]
fn text_finalization_cancel_documents_retained_shape_after_glyph_work() {
    let source = br#"<svg><path d="M0 0L1 0L1 1Z"/><text>A</text></svg>"#;
    let mut fixture = Fixture::new(source, Limits::default());
    let index = fixture.index(b"text");
    let map = fixture.map(index);
    let element = view(&fixture.lex, index);
    let scene = fixture.builder.root_id();
    let draft = fixture.builder.begin_draft(scene).unwrap();
    let input = fixture.prefix_input();
    let prefix = fixture.builder.append_shape(scene, input, &mut Run).unwrap();
    let mut ledger = ChargeLedger::new(fixture.lex.source_owner(), 1 << 20, 8).unwrap();
    let mut provider = Provider::default();
    let mut state = TextState::default();
    let mut cache = TextCache::default();
    let mut control = StopOnTextFinalize { edges: 0, final_checkpoints: 0 };
    let error = render_text_request(TextDecodeRequest { element, source_map: map,
        end: element.end, style: &CssPaint::initial(), transform: vexel_geometry::affine_identity(),
        state: &mut state, preserve_space: false, provider: &mut provider,
        cache: &mut cache, scene, ledger: &mut ledger },
        &mut fixture.builder, &draft, &mut control).unwrap_err();
    assert_eq!(error.kind, ErrorKind::Canceled);
    assert_eq!(control.final_checkpoints, 2);
    assert_eq!(provider.seen.len(), 1);
    assert_eq!(fixture.builder.scene(scene).unwrap().shapes.len(), 2,
        "KNOWN DEFECT: failed finalization leaves the just-published text shape");
    let value = *fixture.builder.scene(scene).unwrap().shape(prefix).unwrap();
    fixture.builder.finalize_shape(&draft, prefix, value, &mut Run).unwrap();
    let input = fixture.prefix_input();
    fixture.builder.append_shape(scene, input, &mut Run).unwrap();
    fixture.builder.rollback_draft(draft).unwrap();
}
