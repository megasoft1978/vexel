//! Public source/provider/builder/job fixtures; no product renderer registration.
mod support;
use support::*;
use std::sync::Arc;
use vexel_scene::*;
use vexel_geometry::affine_identity as identity;

#[derive(Default)]
struct Meter { total: u64, stroke: u64 }
impl WorkControl for Meter {
    fn charge(&mut self, kind: WorkKind, n: u64) -> std::result::Result<(), Stop> {
        self.total = self.total.checked_add(n).ok_or(Stop::Overflow)?;
        if kind == WorkKind::StrokeTest { self.stroke += n; }
        Ok(())
    }
    fn checkpoint(&mut self) -> std::result::Result<(), Stop> { Ok(()) }
}
fn lexical(path: &str) -> LexicalDocument {
    LexicalDocument::parse(Arc::new(format!("<svg><g id=\"target\"><path d=\"{path}\"/></g><g/></svg>").into_bytes()), 1 << 20, &mut Run).unwrap()
}
fn build(doc: &LexicalDocument) -> SceneBuilder {
    SceneBuilder::new(doc.source_owner().clone(), animation(doc.source_owner()), 7, viewport(), Limits::default()).unwrap()
}
fn parsed(doc: &LexicalDocument, transform: Affine) -> ShapeInput {
    let source = doc.source();
    let start = source.windows(4).position(|b| b == b" d=\"").unwrap() + 4;
    let end = start + source[start..].iter().position(|b| *b == b'"').unwrap();
    let element_start = source.windows(5).position(|b| b == b"<path").unwrap();
    let element_end = end + 3;
    let path = vexel_svg_path::parse_path(source, Span { start, end }, transform, Default::default()).unwrap();
    ShapeInput { edge_colors: vec![[4, 3, 2, 255]; path.edges.len()], path,
        shape: Shape::new(SourceMap { element: Span { start: element_start, end: element_end }, transform, transform_node: None }) }
}
fn token(doc: &LexicalDocument, name: &[u8]) -> ElementId {
    ElementId(doc.tokens().iter().position(|t| t.local_name.slice(doc.source()) == Some(name)).unwrap())
}
fn publish(b: &mut SceneBuilder, doc: &LexicalDocument) -> ShapeId {
    b.append_shape(b.root_id(), parsed(doc, identity()), &mut Run).unwrap()
}

// Explicit fixture provider owns one layout cache; successful document USES are
// admitted through the shared builder and not through this cache's storage count.
struct TextProvider { commands: usize, fail: bool }
impl MediaProvider for TextProvider {
    fn layout_text(&mut self, request: TextRequest<'_>, control: &mut dyn WorkControl) -> std::result::Result<TextLayout, MediaError> {
        if self.fail { return Err(MediaError::Corrupt); }
        request.validate(control).map_err(|_| MediaError::Invalid)?;
        Ok(TextLayout { glyphs: vec![TextGlyph { glyph_id: Some(1), x: 0., y: 0., advance: 1., commands: TableRange { first: 0, count: self.commands } }],
            commands: vec![TextCommand { verb: TextVerb::Move, values: [0.; 6] }; self.commands],
            advance_width: 1., ascent: 1., descent: 0., leading: 0. })
    }
    fn decode_image(&mut self, encoded: &[u8], control: &mut dyn WorkControl) -> std::result::Result<ImagePixels, MediaError> {
        control.checkpoint().map_err(|_| MediaError::Invalid)?;
        if encoded != b"owned-fixture" { return Err(MediaError::Corrupt); }
        Ok(image().pixels)
    }
}
fn text_request() -> TextRequest<'static> {
    TextRequest { text: b"x", font_families: &[], size_points: 12., weight: 400., style: TextStyle::Normal, letter_spacing_points: 0. }
}
fn media_use(b: &mut SceneBuilder, d: &DraftToken, doc: &LexicalDocument,
    layout: &TextLayout, late_failure: bool, control: &mut dyn WorkControl) -> Result<ShapeId> {
    let request = b.begin_request(d, control)?;
    let result = (|| {
        // Full provider layout validation is provider work, not cache admission.
        layout.validate(control)?;
        b.admit_text_commands(d, layout.commands.len(), 5, control)?;
        let id = b.append_shape(b.root_id(), parsed(doc, identity()), control)?;
        if late_failure { return Err(Diagnostic { kind: ErrorKind::OutOfMemory, offset: 5 }); }
        b.commit_request(d, request, control)?;
        Ok(id)
    })();
    if result.is_err() { b.rollback_request(d, request)?; }
    result
}
#[test]
fn sixteen_cached_layout_uses_and_exact_seventeenth_failure_with_no_suffix() {
    let doc = lexical("M0 0 L1 1 Z");
    let mut b = build(&doc);
    let d = b.begin_draft(b.root_id()).unwrap();
    let mut provider = TextProvider { commands: 262144, fail: false };
    let layout = provider.layout_text(text_request(), &mut Run).unwrap();
    let cache_commands = layout.commands.len();
    for i in 1..=16 {
        media_use(&mut b, &d, &doc, &layout, false, &mut Run).unwrap();
        assert_eq!(b.document_text_commands(), i * cache_commands);
    }
    let before = b.memory_stats();
    let error = media_use(&mut b, &d, &doc, &layout, false, &mut Run).unwrap_err();
    assert_eq!(error, Diagnostic { kind: ErrorKind::Limit(LimitKind::TextCommands), offset: 5 });
    assert_eq!(b.document_text_commands(), DOCUMENT_TEXT_COMMAND_LIMIT);
    assert_eq!(b.scene(b.root_id()).unwrap().shapes.len(), 16);
    assert_eq!(b.memory_stats(), before);
    assert_eq!(layout.commands.len(), 262144); // cache cap remains separate
    b.commit_draft(&d, &mut Run).unwrap();
    assert_eq!(b.freeze().unwrap().root().shapes.len(), 16);
}
#[test]
fn failed_provider_publication_cancel_abort_and_nested_draft_restore_logical_use() {
    let doc = lexical("M0 0 L1 1 Z");
    let mut b = build(&doc);
    let root = b.root_id();
    let d = b.begin_draft(root).unwrap();
    let mut provider = TextProvider { commands: 9, fail: true };
    let request = b.begin_request(&d, &mut Run).unwrap();
    assert_eq!(provider.layout_text(text_request(), &mut Run).unwrap_err(), MediaError::Corrupt);
    b.rollback_request(&d, request).unwrap();
    assert_eq!(b.document_text_commands(), 0);
    provider.fail = false;
    let layout = provider.layout_text(text_request(), &mut Run).unwrap();
    let before = b.memory_stats();
    assert_eq!(media_use(&mut b, &d, &doc, &layout, true, &mut Run).unwrap_err().kind, ErrorKind::OutOfMemory);
    assert_eq!(b.document_text_commands(), 0);
    assert!(b.scene(root).unwrap().shapes.is_empty());
    assert!(b.memory_stats().total_allocated_bytes > before.total_allocated_bytes);
    assert!(b.memory_stats().live_bytes >= before.live_bytes);
    let request = b.begin_request(&d, &mut Run).unwrap();
    assert_eq!(b.admit_text_commands(&d, 9, 5, &mut StopAfter(0)).unwrap_err().kind, ErrorKind::Canceled);
    b.admit_text_commands(&d, 9, 5, &mut Run).unwrap();
    let child = b.new_scene(viewport()).unwrap();
    let nested = b.begin_draft(child).unwrap();
    b.admit_text_commands(&nested, 11, 5, &mut Run).unwrap();
    assert_eq!(b.document_text_commands(), 20); // same actual document ownership
    b.rollback_draft(nested).unwrap();
    assert_eq!(b.document_text_commands(), 9);
    b.abort_request(&d).unwrap();
    assert_eq!(b.document_text_commands(), 0);
    assert!(b.commit_request(&d, request, &mut Run).is_err());
    media_use(&mut b, &d, &doc, &layout, false, &mut Run).unwrap();
    assert_eq!(b.document_text_commands(), 9);
    b.rollback_draft(d).unwrap();
    assert_eq!(b.document_text_commands(), 0);
}
#[test]
fn text_priority_foreign_offset_huge_count_and_scene_mark_independence() {
    let mut b = builder();
    let root = b.root_id();
    let d = b.begin_draft(root).unwrap();
    let request = b.begin_request(&d, &mut Run).unwrap();
    let mut other = builder();
    let foreign = other.begin_draft(other.root_id()).unwrap();
    assert!(b.admit_text_commands(&foreign, 1, 5, &mut Run).is_err());
    assert!(b.admit_text_commands(&d, 1, usize::MAX, &mut Run).is_err());
    let e = b.admit_text_commands(&d, usize::MAX, 5, &mut StopAfter(0)).unwrap_err();
    assert_eq!(e.kind, ErrorKind::Limit(LimitKind::TextCommands)); // original limit priority
    assert_eq!(e.offset, 5);
    b.admit_text_commands(&d, 100, 5, &mut Run).unwrap();
    b.commit_request(&d, request, &mut Run).unwrap();
    b.commit_draft(&d, &mut Run).unwrap();
    let mark = b.mark(root).unwrap();
    let d = b.begin_draft(root).unwrap();
    let request = b.begin_request(&d, &mut Run).unwrap();
    b.admit_text_commands(&d, 200, 5, &mut Run).unwrap();
    b.commit_request(&d, request, &mut Run).unwrap();
    b.commit_draft(&d, &mut Run).unwrap();
    b.rollback(mark).unwrap();
    assert_eq!(b.document_text_commands(), 300); // scene-local marks do not rewind independent document admission
}

fn grid_job(source: &SourceOwner) -> (RasterJob, JobRasterScratch, JobBuffer<u8>, JobBuffer<GridCell>, JobBuffer<u32>) {
    let mut j = RasterJob::new(ChargeLedger::new(source, 1 << 20, 128).unwrap()).unwrap();
    let scratch = JobRasterScratch::new(&mut j, &mut Run).unwrap();
    let cells = j.buffer(RasterPool::Grid, 65536, &mut Run).unwrap();
    let refs = j.buffer(RasterPool::Grid, 1000000, &mut Run).unwrap();
    j.extend(scratch.dash.segments, &[DashSegment { edge: Edge { a: Point { x: 0., y: 0. }, b: Point { x: 1., y: 1. } }, start_cap: Linecap::Butt, end_cap: Linecap::Butt }], &mut Run).unwrap();
    j.extend(cells, &[GridCell { first: 0, count: 1 }], &mut Run).unwrap();
    j.extend(refs, &[0], &mut Run).unwrap();
    j.extend(scratch.dash.shapes, &[JobDashRange { segments: TableRange { first: 0, count: 1 }, joins: TableRange::EMPTY,
        grid_cells: cells, grid_refs: refs, grid_min_x: 0., grid_min_y: 0., grid_cell_size: 1., grid_columns: 1, grid_rows: 1 }], &mut Run).unwrap();
    j.extend(scratch.active_edges, &[0; 32], &mut Run).unwrap();
    j.extend(scratch.pixel_runs, &[PixelRun { first_x: 0, end_x: 1, center_only: false }; 32], &mut Run).unwrap();
    let output = j.buffer(RasterPool::Output, 128, &mut Run).unwrap();
    j.extend(output, &[0; 128], &mut Run).unwrap();
    (j, scratch, output, cells, refs)
}
#[test]
fn validated_once_repeated_grid_reads_actual_row_fill_and_output_writes() {
    let doc = lexical("M0 0 L1 1 Z");
    let mut b = build(&doc);
    let d = b.begin_draft(b.root_id()).unwrap();
    let id = publish(&mut b, &doc);
    b.commit_draft(&d, &mut Run).unwrap();
    let graph = b.freeze().unwrap();
    let (mut j, s, out, _, _) = grid_job(graph.source_owner());
    let before = j.stats();
    let old_work = j.work();
    let mut meter = Meter::default();
    {
        let mut scope = j.compose(graph.source_owner(), s.dash, &mut meter).unwrap();
        assert_eq!(scope.work().stroke_tests - old_work.stroke_tests, 4); // segment/range/cell/ref
        for row in 0..32 {
            scope.with_buffers(s.active_edges, s.pixel_runs, &mut meter, |cache, active, runs, c| {
                let (_, cells, refs) = cache.grid_at(0)?;
                c.charge(WorkKind::StrokeTest, 1).unwrap(); // actual provider/sample work
                assert_eq!(refs[cells[0].first], 0);
                assert_eq!(cache.segments().len(), 1);
                assert!(cache.joins().is_empty());
                assert_eq!(cache.shapes().len(), 1);
                assert!(graph.root().shape(id).is_some());
                active[row] = refs[0];
                runs[row].first_x = row as u32;
                runs[row].end_x = row as u32 + 1;
                Ok(())
            }).unwrap();
            scope.with_buffers(s.pixel_runs, out, &mut meter, |cache, runs, output, c| {
                assert_eq!(cache.grid_at(0)?.2[0], 0);
                c.charge(WorkKind::ShapeSample, 16).unwrap(); // original sample count, no revalidation
                output[row * 4..row * 4 + 4].copy_from_slice(&[runs[row].first_x as u8, 2, 3, 255]);
                Ok(())
            }).unwrap();
        }
        scope.charge_work(WorkKind::ShapeRowVisit, 32, &mut meter).unwrap();
        assert_eq!(scope.stats(), before); // no per-row metadata or cache allocations
    }
    assert_eq!(meter.stroke, 4 + 32);
    assert_eq!(j.work().total_work - old_work.total_work, 4 + 32 + 32 * 16 + 32);
    assert_eq!(&j.slice(out).unwrap()[124..], &[31, 2, 3, 255]);
    assert_eq!(j.stats(), before);
    assert_eq!(j.finish().unwrap().live_bytes, 0);
}
#[test]
fn composition_alias_pool_foreign_stale_source_and_cancel_guards() {
    let source = owner();
    let (mut j, s, out, cells, refs) = grid_job(&source);
    let (foreign, _, foreign_out, _, _) = grid_job(&source);
    let equal_bytes = Arc::new(source.as_ref().clone());
    assert!(j.compose(&equal_bytes, s.dash, &mut Run).is_err());
    let before = j.stats();
    {
        let mut scope = j.compose(&source, s.dash, &mut Run).unwrap();
        assert!(scope.with_buffers(out, out, &mut Run, |_, _, _, _| Ok(())).is_err());
        assert!(scope.with_buffers(out, foreign_out, &mut Run, |_, _, _, _| Ok(())).is_err());
        assert!(scope.with_buffers(cells, refs, &mut Run, |_, _, _, _| Ok(())).is_err());
        assert_eq!(scope.with_buffers(out, s.active_edges, &mut StopAfter(0), |_, _, _, _| Ok(())).unwrap_err().kind, ErrorKind::Canceled);
    }
    assert_eq!(j.stats(), before);
    assert_eq!(j.slice(refs).unwrap(), &[0]);
    j.release(out).unwrap();
    let replacement = j.buffer::<u8>(RasterPool::Output, 128, &mut Run).unwrap();
    {
        let mut scope = j.compose(&source, s.dash, &mut Run).unwrap();
        assert!(scope.with_buffers(out, s.active_edges, &mut Run, |_, _, _, _| Ok(())).is_err());
        scope.with_buffers(replacement, s.active_edges, &mut Run, |_, _, _, _| Ok(())).unwrap();
    }
    j.release(refs).unwrap();
    assert!(j.compose(&source, s.dash, &mut Run).is_err());
    assert!(j.compose(&source, s.dash, &mut StopAfter(0)).is_err());
    drop(foreign);
    j.release_all().unwrap();
    assert_eq!(j.finish().unwrap().live_bytes, 0);
}
#[test]
fn composition_panic_cancel_partial_work_and_ownership_are_honest() {
    let source = owner();
    let (mut j, s, out, _, refs) = grid_job(&source);
    let before = j.stats();
    let mut meter = Meter::default();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut scope = j.compose(&source, s.dash, &mut meter).unwrap();
        scope.with_buffers(out, s.active_edges, &mut meter, |cache, output, _, control| -> Result<()> {
            assert_eq!(cache.grid_at(0)?.2, &[0]);
            control.charge(WorkKind::ShapePixel, 3).unwrap();
            output[0] = 99;
            panic!("fixture engine interruption");
        }).unwrap();
    }));
    assert!(result.is_err());
    assert_eq!(j.slice(out).unwrap()[0], 99); // discarded output, no fake rollback
    assert_eq!(j.slice(refs).unwrap(), &[0]);
    assert_eq!(j.stats(), before);
    assert_eq!(j.work().shape_pixels, 3);
    {
        let mut scope = j.compose(&source, s.dash, &mut Run).unwrap();
        let e = scope.with_buffers(out, s.active_edges, &mut StopAfter(1), |_, output, _, c| {
            c.charge(WorkKind::ShapeSample, 1).map_err(|_| Diagnostic { kind: ErrorKind::Canceled, offset: 0 })?;
            output[1] = 7;
            c.charge(WorkKind::ShapeSample, 1).map_err(|_| Diagnostic { kind: ErrorKind::Canceled, offset: 0 })
        }).unwrap_err();
        assert_eq!(e.kind, ErrorKind::Canceled);
    }
    assert_eq!(j.slice(out).unwrap()[1], 7);
    assert_eq!(j.work().shape_samples, 2); // actual reported stop-site included
    j.release_all().unwrap(); // even discarded descriptors reclaimed
    assert_eq!(j.finish().unwrap().live_bytes, 0);
}

#[test]
fn original_post_close_smooth_and_explicit_geometry_metadata_freeze_and_frame() {
    let cases = [
        "M1 1 C2 2 3 3 4 4 Z S6 6 7 7",
        "M1 1 C2 2 3 3 4 4 Z C1 1 6 6 7 7",
        "M1 1 C2 2 3 3 4 4 Z Z S6 6 7 7 Z S8 8 9 9",
        "M1 1 L4 4 Z L7 7 M10 10 L11 11 Z L12 12",
        "M1 1 L4 4 Z L7 7 M10 10 L11 11",
    ];
    let mut pair = Vec::new();
    for data in cases {
        let doc = lexical(data);
        let mut b = build(&doc);
        let root = b.root_id();
        let d = b.begin_draft(root).unwrap();
        let transform = Affine { a: 2., d: 3., e: 9., f: -4., ..identity() };
        let input = parsed(&doc, transform);
        let edges = input.path.edges.clone();
        let subpaths = input.path.subpaths.clone();
        let segments = input.path.segments.clone();
        let numbers = input.path.numbers.clone();
        let first = b.append_shape(root, input, &mut Run).unwrap();
        let second = b.append_shape(root, parsed(&doc, transform), &mut Run).unwrap();
        b.commit_draft(&d, &mut Run).unwrap();
        b.seal(root, &mut Run).unwrap();
        let graph = b.freeze_controlled(&mut Run).unwrap();
        let view = graph.root();
        assert_eq!(view.shape(first).unwrap().edges.slice(view.edges).unwrap(), edges);
        for (i, original) in subpaths.iter().enumerate() {
            let published = view.path_subpaths[i + subpaths.len()];
            assert_eq!(published.first_edge, original.first_edge + edges.len());
            assert_eq!(published.edge_count, original.edge_count);
            assert_eq!(published.closed, original.closed);
        }
        for (i, original) in segments.iter().enumerate() {
            let published = view.path_segments[i + segments.len()];
            assert_eq!(published.shape, second);
            assert_eq!(published.path.source, original.source);
            assert_eq!(published.path.command_offset, original.command_offset);
            assert_eq!(published.path.first_edge, original.first_edge + edges.len());
            assert_eq!(published.path.first_number, original.first_number + numbers.len());
            assert_eq!(published.path.command, original.command);
            let mut expected = *original;
            expected.first_edge += edges.len();
            expected.first_number += numbers.len();
            assert_eq!(published.path, expected);
        }
        assert_eq!(&view.path_numbers[..numbers.len()], numbers);
        assert_eq!(&view.path_numbers[numbers.len()..], numbers);
        let mut frame = FrameBuilder::new(view, 1 << 20).unwrap();
        frame.replace_edges(edges.iter().chain(&edges).copied().collect(), &mut Run).unwrap();
        let frame = frame.finish(&mut Run).unwrap();
        assert_eq!(frame.view().path_subpaths, view.path_subpaths);
        assert_eq!(frame.view().path_segments, view.path_segments);
        if pair.len() < 2 { pair.push(edges); }
    }
    assert_eq!(pair[0], pair[1]); // exact original implicit/explicit smooth assertion
}
#[test]
fn post_close_forged_overlap_segment_ranges_foreign_arithmetic_and_cancel_reject() {
    let doc = lexical("M1 1 L4 4 Z L7 7");
    for corruption in 0..8 {
        let mut b = build(&doc);
        let root = b.root_id();
        let mut input = parsed(&doc, identity());
        assert_eq!(input.path.subpaths.len(), 2);
        match corruption {
            0 => input.path.subpaths[0].closed = false,
            1 => input.path.subpaths[1].first_edge += 1,
            2 => input.path.subpaths[1].edge_count = usize::MAX,
            3 => { input.path.segments[2].command = b'L'; input.path.segments[2].number_count = 2; },
            4 => input.path.segments[3].first_edge = 0,
            5 => input.path.segments[2].command_offset = None, // forged implicit Z
            6 => input.path.segments[2].source = Span { start: 0, end: 1 },
            _ => { let end = input.path.subpaths[0].edge_count; input.path.edges[end - 1].b.x += 1.; },
        }
        let before = b.memory_stats();
        assert!(b.append_shape(root, input, &mut Run).is_err(), "corruption {corruption}");
        assert!(b.scene(root).unwrap().shapes.is_empty());
        assert_eq!(b.memory_stats(), before);
    }
    let mut b = build(&doc);
    let root = b.root_id();
    let d = b.begin_draft(root).unwrap();
    assert_eq!(b.append_shape(root, parsed(&doc, identity()), &mut StopAfter(3)).unwrap_err().kind, ErrorKind::Canceled);
    assert!(b.scene(root).unwrap().path_subpaths.is_empty());
    publish(&mut b, &doc);
    assert_eq!(b.commit_draft(&d, &mut StopAfter(3)).unwrap_err().kind, ErrorKind::Canceled);
    b.commit_draft(&d, &mut Run).unwrap();
    assert_eq!(b.freeze().unwrap().root().shapes.len(), 1);
}

fn mask_fixture(doc: &LexicalDocument, b: &mut SceneBuilder, d: &DraftToken) -> (GroupId, MaskId, DefinitionOwner) {
    let root = b.root_id();
    let shape = publish(b, doc);
    let shapes = TableRange { first: shape.index(), count: 1 };
    let group = b.append_group(root, OpacityGroup { shapes, ..group() }, &mut Run).unwrap();
    let child = b.new_scene(viewport()).unwrap();
    let child_draft = b.begin_draft(child).unwrap();
    b.append_shape(child, parsed(doc, identity()), &mut Run).unwrap();
    b.commit_draft(&child_draft, &mut Run).unwrap();
    b.seal(child, &mut Run).unwrap();
    let mask = b.append_mask(root, mask(child), &mut Run).unwrap();
    let source = DecodeSource::new(doc, &mut Run).unwrap();
    let owner = b.definition_owner(d, &source, token(doc, b"g"), DefinitionTarget::Group(group), &mut Run).unwrap();
    (group, mask, owner)
}
#[test]
fn nonempty_mask_expansion_post_attaches_exact_real_group_and_range() {
    let doc = lexical("M0 0 L1 1 Z");
    let source = DecodeSource::new(&doc, &mut Run).unwrap();
    let mut b = build(&doc);
    let root = b.root_id();
    let d = b.begin_draft(root).unwrap();
    let (target_group, mask, owner) = mask_fixture(&doc, &mut b, &d);
    let untouched = b.append_group(root, OpacityGroup { shapes: TableRange { first: 1, count: 0 }, ..group() }, &mut Run).unwrap();
    let shapes = b.scene(root).unwrap().group(target_group).unwrap().shapes;
    let target = b.mask_post_target(&d, &source, owner, shapes, mask, &mut Run).unwrap();
    let style = CssPaint::initial();
    let expansion = Expansion { range: TableRange { first: token(&doc, b"path").0, count: 1 }, transform: identity(), viewport: [100., 100.],
        style, flavor: DecodeFlavor::Mask, post: PostOperation::AttachTargetedMask(target) };
    assert_eq!(target.group(), target_group);
    assert_eq!(target.shapes(), shapes);
    assert_eq!(target.mask(), mask);
    assert!(b.scene(root).unwrap().scene(b.scene(root).unwrap().mask(mask).unwrap().content).unwrap().shapes.len() == 1);
    b.apply_mask_post(&d, expansion.post, &mut Run).unwrap();
    assert_eq!(b.scene(root).unwrap().group(target_group).unwrap().mask, Some(mask));
    assert_eq!(b.scene(root).unwrap().group(untouched).unwrap().mask, None);
    b.commit_draft(&d, &mut Run).unwrap();
    assert!(b.apply_mask_post(&d, expansion.post, &mut Run).is_err());
    assert_eq!(b.freeze().unwrap().root().group(target_group).unwrap().mask, Some(mask));
}
#[test]
fn mask_target_wrong_source_owner_scope_range_postkind_cancel_and_stale_request() {
    let doc = lexical("M0 0 L1 1 Z");
    let source = DecodeSource::new(&doc, &mut Run).unwrap();
    let mut b = build(&doc);
    let root = b.root_id();
    let d = b.begin_draft(root).unwrap();
    let r = b.begin_request(&d, &mut Run).unwrap();
    let (group, mask, owner) = mask_fixture(&doc, &mut b, &d);
    let shapes = b.scene(root).unwrap().group(group).unwrap().shapes;
    assert!(b.mask_post_target(&d, &source, owner, TableRange::EMPTY, mask, &mut Run).is_err());
    let foreign_doc = lexical("M0 0 L1 1 Z");
    let foreign_source = DecodeSource::new(&foreign_doc, &mut Run).unwrap();
    assert!(b.mask_post_target(&d, &foreign_source, owner, shapes, mask, &mut Run).is_err());
    let target = b.mask_post_target(&d, &source, owner, shapes, mask, &mut Run).unwrap();
    assert!(b.apply_mask_post(&d, PostOperation::AttachMask(mask), &mut Run).is_err());
    assert!(b.apply_mask_post(&d, PostOperation::FinishGroup(group), &mut Run).is_err());
    assert_eq!(b.apply_mask_post(&d, PostOperation::AttachTargetedMask(target), &mut StopAfter(0)).unwrap_err().kind, ErrorKind::Canceled);
    assert_eq!(b.scene(root).unwrap().group(group).unwrap().mask, None);
    let child = b.new_scene(viewport()).unwrap();
    let nested = b.begin_draft(child).unwrap();
    assert!(b.apply_mask_post(&nested, PostOperation::AttachTargetedMask(target), &mut Run).is_err());
    b.rollback_draft(nested).unwrap();
    b.rollback_request(&d, r).unwrap();
    let (replacement, _, _) = mask_fixture(&doc, &mut b, &d);
    assert_eq!(replacement.index(), group.index());
    assert_ne!(replacement, group);
    assert!(b.apply_mask_post(&d, PostOperation::AttachTargetedMask(target), &mut Run).is_err());
    assert_eq!(b.scene(root).unwrap().group(replacement).unwrap().mask, None);
    b.rollback_draft(d).unwrap();
}
#[test]
fn genuine_shared_atomic_style_reference_append_commit_quantum_and_resume() {
    let doc = lexical("M0 0 L1 1 Z");
    let mut source = DecodeSource::new(&doc, &mut Run).unwrap();
    let mut b = build(&doc);
    let root = b.root_id();
    let d = b.begin_draft(root).unwrap();
    let mut meter = Meter::default();
    let mut q = DecodeQuantum::new(1).unwrap();
    assert!(matches!(q.compute_style(&mut source, token(&doc, b"path"), None, &mut meter).unwrap(), AtomicStep::Complete(_)));
    assert!(q.work().total_work > 1);
    assert_eq!(q.work().total_work, meter.total);
    assert_eq!(q.overshoot(), meter.total - 1);
    let before = meter.total;
    let input = parsed(&doc, identity());
    let pending = match q.append_shape(&mut b, root, input, &mut meter).unwrap() {
        AtomicStep::Boundary(input) => input,
        _ => panic!("cannot begin another operation after boundary"),
    };
    assert_eq!(meter.total, before);
    assert!(b.scene(root).unwrap().shapes.is_empty());
    let mut q = DecodeQuantum::new(1).unwrap();
    let shape = match q.append_shape(&mut b, root, pending, &mut meter).unwrap() {
        AtomicStep::Complete(id) => id, _ => panic!("fresh quantum"),
    };
    assert_eq!(q.work().total_work, meter.total - before);
    assert!(q.overshoot() > 0);
    let mut q = DecodeQuantum::new(1).unwrap();
    let offset = doc.source().windows(6).position(|s| s == b"target").unwrap();
    let before = meter.total;
    assert!(matches!(q.find_reference(&mut source, vexel_xml::XmlSpan { offset, length: 6 }, &mut meter).unwrap(), AtomicStep::Complete(Some(_))));
    assert_eq!(q.work().total_work, meter.total - before);
    let request = b.begin_request(&d, &mut Run).unwrap();
    let mut q = DecodeQuantum::new(1).unwrap();
    let before = meter.total;
    assert!(matches!(q.commit_request(&mut b, &d, request, &mut meter).unwrap(), AtomicStep::Complete(())));
    assert_eq!(q.work().total_work, meter.total - before);
    assert!(q.overshoot() > 0);
    let mut q = DecodeQuantum::new(1).unwrap();
    assert!(matches!(q.commit_draft(&mut b, &d, &mut meter).unwrap(), AtomicStep::Complete(())));
    assert!(q.overshoot() > 0);
    assert!(b.freeze().unwrap().root().shape(shape).is_some());
}
#[test]
fn atomic_error_cancel_overflow_work_visible_and_no_partial_append_commit() {
    let doc = lexical("M0 0 L1 1 Z");
    let mut b = build(&doc);
    let root = b.root_id();
    let d = b.begin_draft(root).unwrap();
    let mut q = DecodeQuantum::new(99).unwrap();
    assert_eq!(q.append_shape(&mut b, root, parsed(&doc, identity()), &mut StopAfter(3)).unwrap_err().kind, ErrorKind::Canceled);
    assert_eq!(q.work().total_work, 4); // three admitted reports plus actual stop site
    assert!(b.scene(root).unwrap().shapes.is_empty());
    publish(&mut b, &doc);
    let request = b.begin_request(&d, &mut Run).unwrap();
    assert_eq!(q.commit_request(&mut b, &d, request, &mut StopAfter(1)).unwrap_err().kind, ErrorKind::Canceled);
    assert!(q.work().total_work > 4);
    b.rollback_request(&d, request).unwrap();
    let mut q = DecodeQuantum::new(u64::MAX).unwrap();
    q.atomic(&mut Run, |c| { c.charge(WorkKind::TokenVisit, u64::MAX).unwrap(); Ok(()) }).unwrap();
    assert_eq!(q.work().total_work, u64::MAX);
    assert_eq!(q.atomic(&mut Run, |_| -> Result<()> { panic!("boundary cannot invoke") }).unwrap(), AtomicStep::Boundary(()));
    assert!(DecodeQuantum::new(0).is_err());
    b.rollback_draft(d).unwrap();
}
#[test]
fn real_native_capacity_arithmetic_and_overlap_failure_preserve_composable_cache() {
    let source = owner();
    let mut j = RasterJob::new(ChargeLedger::new(&source, usize::MAX, 128).unwrap()).unwrap();
    let s = JobRasterScratch::new(&mut j, &mut Run).unwrap();
    let bytes = j.buffer::<u8>(RasterPool::Scratch, usize::MAX, &mut Run).unwrap();
    let ints = j.buffer::<u32>(RasterPool::Scratch, usize::MAX, &mut Run).unwrap();
    j.extend(bytes, &[11], &mut Run).unwrap();
    let before = j.stats();
    assert_eq!(j.reserve(bytes, isize::MAX as usize + 1, &mut Run).unwrap_err().kind, ErrorKind::OutOfMemory);
    assert_eq!(j.stats(), before); // real std capacity rejection, no injected OOM
    assert_eq!(j.reserve(ints, usize::MAX, &mut Run).unwrap_err().kind, ErrorKind::Limit(LimitKind::Arithmetic));
    let before = j.stats();
    assert_eq!(j.reserve(bytes, 128, &mut StopAfter(0)).unwrap_err().kind, ErrorKind::Canceled);
    assert_eq!(j.stats(), before);
    let mut scope = j.compose(&source, s.dash, &mut Run).unwrap();
    scope.with_buffers(bytes, ints, &mut Run, |cache, bytes, _, _| {
        assert!(cache.segments().is_empty()); assert_eq!(bytes, &[11]); Ok(())
    }).unwrap();
    drop(scope);
    assert_eq!(j.finish().unwrap().live_bytes, 0);
}

#[test]
fn transient_overlap_admission_and_canceled_growth_leave_original_cache_and_history() {
    let source = owner();
    let (mut j, s, out, _, refs) = grid_job(&source);
    let scratch = j.buffer::<u8>(RasterPool::Scratch, 1 << 20, &mut Run).unwrap();
    j.extend(scratch, &[7; 4096], &mut Run).unwrap();
    let before = j.stats();
    // Retained payload alone is not enough: metadata/source/old scratch stay live.
    assert_eq!(j.reserve(scratch, 1 << 20, &mut Run).unwrap_err().kind, ErrorKind::Limit(LimitKind::ReservedBytes));
    assert_eq!(j.stats(), before);
    let e = j.reserve(scratch, 8192, &mut StopAfter(1)).unwrap_err();
    assert_eq!(e.kind, ErrorKind::Canceled);
    assert_eq!(j.slice(scratch).unwrap(), &[7; 4096]);
    assert_eq!(j.capacity(scratch).unwrap(), 4096);
    assert_eq!(j.stats().live_bytes, before.live_bytes);
    assert!(j.stats().total_allocated_bytes > before.total_allocated_bytes);
    assert!(j.stats().allocation_count > before.allocation_count);
    {
        let mut scope = j.compose(&source, s.dash, &mut Run).unwrap();
        scope.with_buffers(scratch, out, &mut Run, |cache, scratch, output, _| {
            assert_eq!(cache.grid_at(0)?.2, &[0]);
            output[0] = scratch[0]; Ok(())
        }).unwrap();
    }
    assert_eq!(j.slice(refs).unwrap(), &[0]);
    assert_eq!(j.slice(out).unwrap()[0], 7);
    j.release_all().unwrap();
    assert_eq!(j.finish().unwrap().live_bytes, 0);
}

struct MaskPlanner { target: MaskPostTarget, flavor: DecodeFlavor, lease: Option<AllocationLease> }
impl DefinitionPlanner for MaskPlanner {
    fn plan_owned_definition<'src>(&mut self, ctx: &mut DefinitionContext<'src, '_>,
        kind: DefinitionKind, element: ElementId, owner: DefinitionOwner, transform: Affine,
        _: [f64; 4], style: &CssPaint<'src>, control: &mut dyn WorkControl) -> Result<ProductionDefinitionPlan<'src>> {
        owner.validate(ctx)?;
        assert_eq!(kind, DefinitionKind::Mask);
        self.target.validate(ctx, control)?;
        let bytes = std::mem::size_of::<Expansion<'src>>();
        let lease = ctx.ledger.admit(AllocationTag::ReferenceIndex, bytes)?;
        let mut expansions = Vec::new();
        if expansions.try_reserve_exact(1).is_err() {
            ctx.ledger.release(lease)?;
            return Err(Diagnostic { kind: ErrorKind::OutOfMemory, offset: 0 });
        }
        // Exact fixture reservation; reject any allocator excess, never hide it.
        if expansions.capacity() != 1 {
            drop(expansions); ctx.ledger.release(lease)?;
            return Err(Diagnostic { kind: ErrorKind::Limit(LimitKind::ReservedBytes), offset: 0 });
        }
        expansions.push(Expansion { range: TableRange { first: element.0, count: 1 }, transform, viewport: [100., 100.], style: *style,
            flavor: self.flavor, post: PostOperation::AttachTargetedMask(self.target) });
        self.lease = Some(lease);
        Ok(ProductionDefinitionPlan::Definition(DefinitionPlan {
            descriptor: DefinitionDescriptor::Mask(MaskDescriptor { luminance: false, region_object_bounding_box: false, custom_region: false,
                region: [0., 0., 1., 1.], region_transform: identity(), source_offset: 5 }), expansions, post: PostOperation::None }))
    }
}
#[test]
fn actual_mask_planner_shared_validation_nonempty_expansion_wrong_flavor_and_source_coherence() {
    let doc = lexical("M0 0 L1 1 Z");
    let mut source = DecodeSource::new(&doc, &mut Run).unwrap();
    let mut b = build(&doc);
    let root = b.root_id();
    let d = b.begin_draft(root).unwrap();
    let (group, mask, owner) = mask_fixture(&doc, &mut b, &d);
    let shapes = b.scene(root).unwrap().group(group).unwrap().shapes;
    let target = b.mask_post_target(&d, &source, owner, shapes, mask, &mut Run).unwrap();
    // A canonical but unrelated source element cannot authorize this nonempty range.
    let unrelated = ElementId(doc.tokens().iter().rposition(|t| t.local_name.slice(doc.source()) == Some(b"g") && t.kind == vexel_xml::TokenKind::EmptyElement).unwrap());
    let wrong = b.definition_owner(&d, &source, unrelated, DefinitionTarget::Group(group), &mut Run).unwrap();
    assert!(b.mask_post_target(&d, &source, wrong, shapes, mask, &mut Run).is_err());
    let style = source.compute_style(owner.element(), None, &mut Run).unwrap();
    let mut active = ActiveDefinitions::default();
    let mut ledger = ChargeLedger::new(doc.source_owner(), 1 << 20, 65536).unwrap();
    let mut planner = MaskPlanner { target, flavor: DecodeFlavor::Mask, lease: None };
    let mut ctx = DefinitionContext { shared: &mut source, builder: &mut b, scene: root, active: &mut active, ledger: &mut ledger };
    let plan = plan_production_definition(&mut planner, &mut ctx, DefinitionKind::Mask, token(&doc, b"path"), owner,
        identity(), [0., 0., 1., 1.], &style, &mut Run).unwrap();
    let ProductionDefinitionPlan::Definition(plan) = plan else { panic!() };
    assert_eq!(plan.expansions.len(), 1);
    let post = plan.expansions[0].post;
    drop(plan);
    ctx.ledger.release(planner.lease.take().unwrap()).unwrap();
    planner.flavor = DecodeFlavor::Clip { clip_rule: false };
    assert!(plan_production_definition(&mut planner, &mut ctx, DefinitionKind::Mask, token(&doc, b"path"), owner,
        identity(), [0., 0., 1., 1.], &style, &mut Run).is_err());
    ctx.ledger.release(planner.lease.take().unwrap()).unwrap();
    drop(ctx);
    b.apply_mask_post(&d, post, &mut Run).unwrap();
    b.commit_draft(&d, &mut Run).unwrap();
    assert_eq!(b.freeze().unwrap().root().group(group).unwrap().mask, Some(mask));
}

#[test]
fn checked_quantum_and_same_job_work_overflow_stop_before_unreportable_work() {
    let mut q = DecodeQuantum::new(10).unwrap();
    let e = q.atomic(&mut Run, |c| {
        c.charge(WorkKind::TokenVisit, u64::MAX - 1).unwrap();
        c.charge(WorkKind::TokenVisit, 2).map_err(|_| Diagnostic { kind: ErrorKind::Limit(LimitKind::Arithmetic), offset: 0 })
    }).unwrap_err();
    assert_eq!(e.kind, ErrorKind::Limit(LimitKind::Arithmetic));
    assert_eq!(q.work().total_work, u64::MAX - 1);
    let source = owner();
    let (mut j, s, _, _, refs) = grid_job(&source);
    j.charge_work(WorkKind::TokenVisit, u64::MAX).unwrap();
    let before = j.stats();
    assert_eq!(j.compose(&source, s.dash, &mut Run).err().unwrap().kind, ErrorKind::Limit(LimitKind::Arithmetic));
    assert_eq!(j.work().total_work, u64::MAX);
    assert_eq!(j.stats(), before);
    assert_eq!(j.slice(refs).unwrap(), &[0]);
    assert_eq!(j.finish().unwrap().live_bytes, 0);
}
