//! Explicit public fixtures, not product resolver/decoder registrations.
mod support;
use support::*;
use std::sync::Arc;
use vexel_scene::*;
use vexel_geometry::affine_identity as identity;
fn controlled(stop: Stop) -> Diagnostic {
    Diagnostic { kind: match stop { Stop::Canceled => ErrorKind::Canceled,
        Stop::Limit => ErrorKind::Limit(LimitKind::Work),
        Stop::Overflow => ErrorKind::Limit(LimitKind::Arithmetic) }, offset: 0 }
}

fn add_shape(b: &mut SceneBuilder, scene: SceneId) -> ShapeId {
    let mut input = shape(b.source_owner());
    let source = b.source_owner();
    let start = source.windows(5).position(|w| w == b"<path").unwrap();
    let end = start + source[start..].windows(2).position(|w| w == b"/>").unwrap() + 2;
    input.shape.source.element = Span { start, end };
    b.append_shape(scene, input, &mut Run).unwrap()
}
fn doc() -> LexicalDocument {
    LexicalDocument::parse(Arc::new(br#"<svg><linearGradient id="g"/><g/><pattern/><path d="M0 0 L1 1 2 3 Z"/></svg>"#.to_vec()),
        1 << 20, &mut Run).unwrap()
}
fn from_doc(doc: &LexicalDocument) -> SceneBuilder {
    SceneBuilder::new(doc.source_owner().clone(), animation(doc.source_owner()), 7,
        viewport(), Limits::default()).unwrap()
}
fn element(doc: &LexicalDocument, name: &[u8]) -> ElementId {
    ElementId(doc.tokens().iter().position(|t| t.local_name.slice(doc.source()) == Some(name)).unwrap())
}
fn two_stops() -> [PaintStop; 2] {
    [PaintStop { offset: 0., rgba: [0, 0, 0, 255] },
        PaintStop { offset: 1., rgba: [255, 255, 255, 255] }]
}
fn gradient() -> Paint {
    Paint { kind: PaintKind::Linear, stops: TableRange { first: 0, count: 2 }, ..paint() }
}
#[test]
fn request_late_rollback_retains_prefix_identity_children_source_and_usability() {
    let d = doc();
    let mut b = from_doc(&d);
    let root = b.root_id();
    let draft = b.begin_draft(root).unwrap();
    let prefix = add_shape(&mut b, root);
    let old = *b.scene(root).unwrap().shape(prefix).unwrap();
    let child = b.new_scene(viewport()).unwrap();
    b.seal(child, &mut Run).unwrap();
    let source = DecodeSource::new(&d, &mut Run).unwrap();
    let template = source.gradient_template_id(element(&d, b"linearGradient"), [100., 100.], &mut Run).unwrap();
    let published = b.publish_template_stops(root, &template, &two_stops(), &mut Run).unwrap();
    let stats = b.memory_stats();
    let request = b.begin_request(&draft, &mut Run).unwrap();
    let suffix = add_shape(&mut b, root);
    b.append_image(root, image(), &mut Run).unwrap();
    let removed_child = b.new_scene(viewport()).unwrap();
    b.seal(removed_child, &mut Run).unwrap();
    b.record_ignored(IgnoredKind::Meta, 0).unwrap();
    let live = b.memory_stats().live_bytes;
    assert_eq!(b.commit_request(&draft, request, &mut StopAfter(0)).unwrap_err().kind, ErrorKind::Canceled);
    b.rollback_request(&draft, request).unwrap();
    let view = b.scene(root).unwrap();
    assert_eq!(view.shape(prefix), Some(&old));
    assert!(view.shape(suffix).is_none());
    assert!(view.images.is_empty());
    assert_eq!(view.ignored(IgnoredKind::Meta).count, 0);
    assert!(b.scene(child).unwrap().scene(child).is_some());
    assert!(b.scene(removed_child).is_err());
    assert_eq!(b.published_stop_range(root, &published, &mut Run).unwrap().count, 2);
    assert!(b.memory_stats().live_bytes < live);
    assert!(b.memory_stats().live_bytes >= stats.live_bytes); // spare capacity retained
    assert!(b.memory_stats().total_allocated_bytes > stats.total_allocated_bytes);
    let mut updated = old;
    updated.stroke.width = 9.;
    b.finalize_shape(&draft, prefix, updated, &mut Run).unwrap();
    let reused = add_shape(&mut b, root);
    assert_eq!(reused.index(), suffix.index());
    assert_ne!(reused, suffix);
    b.commit_draft(&draft, &mut Run).unwrap();
    assert_eq!(b.freeze().unwrap().root().shape(prefix).unwrap().stroke.width, 9.);
}
#[test]
fn request_commit_double_stale_foreign_reentrant_and_abandoned_cleanup() {
    let mut b = builder();
    let root = b.root_id();
    let draft = b.begin_draft(root).unwrap();
    let r = b.begin_request(&draft, &mut Run).unwrap();
    assert!(b.begin_request(&draft, &mut Run).is_err());
    let child = b.new_scene(viewport()).unwrap();
    assert!(b.begin_draft(root).is_err());
    let nested = b.begin_draft(child).unwrap();
    assert!(b.abort_request(&draft).is_err());
    b.commit_draft(&nested, &mut Run).unwrap();
    assert!(b.commit_draft(&draft, &mut Run).is_err());
    assert!(b.seal(root, &mut Run).is_err());
    let mut other = builder();
    let od = other.begin_draft(other.root_id()).unwrap();
    let or = other.begin_request(&od, &mut Run).unwrap();
    assert!(b.rollback_request(&draft, or).is_err());
    assert!(other.rollback_request(&od, r).is_err());
    let id = add_shape(&mut b, root);
    b.commit_request(&draft, r, &mut Run).unwrap();
    assert!(b.commit_request(&draft, r, &mut Run).is_err());
    assert!(b.rollback_request(&draft, r).is_err());
    let next = b.begin_request(&draft, &mut Run).unwrap();
    assert!(b.rollback_request(&draft, r).is_err());
    b.abort_request(&draft).unwrap();
    assert!(b.commit_request(&draft, next, &mut Run).is_err());
    assert!(b.scene(root).unwrap().shape(id).is_some());
    b.commit_draft(&draft, &mut Run).unwrap();
    other.abort_request(&od).unwrap();
    other.rollback_draft(od).unwrap();
    assert!(b.freeze().is_ok());
}
#[test]
fn request_cannot_escape_via_draft_rollback_or_freeze() {
    let mut b = builder();
    let d = b.begin_draft(b.root_id()).unwrap();
    b.begin_request(&d, &mut Run).unwrap();
    assert!(b.rollback_draft(d).is_err());
    assert!(b.freeze().is_err());
}
#[test]
fn every_prefix_draft_mutator_is_guarded_during_request() {
    let mut b = builder();
    let root = b.root_id();
    let d = b.begin_draft(root).unwrap();
    let s = add_shape(&mut b, root);
    let t = b.append_transform(root, transform(None), &mut Run).unwrap();
    let c = b.append_clip(root, clip(), &mut Run).unwrap();
    let child = b.new_scene(viewport()).unwrap();
    b.seal(child, &mut Run).unwrap();
    let m = b.append_mask(root, mask(child), &mut Run).unwrap();
    let g = b.append_group(root, group(), &mut Run).unwrap();
    let f = b.append_filter(root, filter(), &[primitive()], &mut Run).unwrap();
    let fp = b.append_fill_paint(root, paint(), &mut Run).unwrap();
    let sp = b.append_stroke_paint(root, paint(), &mut Run).unwrap();
    let r = b.begin_request(&d, &mut Run).unwrap();
    let sv = *b.scene(root).unwrap().shape(s).unwrap();
    assert!(b.finalize_shape(&d, s, sv, &mut Run).is_err());
    assert!(b.finalize_shape_colors(&d, s, &vec![[0; 4]; sv.edges.count], &mut Run).is_err());
    assert!(b.finalize_transform(&d, t, transform(None), &mut Run).is_err());
    assert!(b.finalize_clip(&d, c, clip(), &mut Run).is_err());
    assert!(b.finalize_mask(&d, m, mask(child), &mut Run).is_err());
    assert!(b.finalize_group(&d, g, group(), &mut Run).is_err());
    assert!(b.finalize_filter(&d, f, filter(), &[primitive()], &mut Run).is_err());
    assert!(b.finalize_fill_paint(&d, fp, paint(), &mut Run).is_err());
    assert!(b.finalize_stroke_paint(&d, sp, paint(), &mut Run).is_err());
    b.rollback_request(&d, r).unwrap();
    b.finalize_group(&d, g, group(), &mut Run).unwrap();
    b.commit_draft(&d, &mut Run).unwrap();
    assert!(b.freeze().is_ok());
}
#[test]
fn request_suffix_template_publication_and_image_pixel_budget_restore() {
    let d = doc();
    let mut b = from_doc(&d);
    let root = b.root_id();
    let draft = b.begin_draft(root).unwrap();
    let source = DecodeSource::new(&d, &mut Run).unwrap();
    let template = source.gradient_template_id(element(&d, b"linearGradient"), [100., 100.], &mut Run).unwrap();
    let r = b.begin_request(&draft, &mut Run).unwrap();
    let witness = b.publish_template_stops(root, &template, &two_stops(), &mut Run).unwrap();
    b.append_template_fill(root, &template, gradient(), &two_stops(), &mut Run).unwrap();
    b.append_image(root, image(), &mut Run).unwrap();
    b.rollback_request(&draft, r).unwrap();
    assert!(b.published_stop_range(root, &witness, &mut Run).is_err());
    assert!(b.scene(root).unwrap().fill_paints.is_empty());
    let new = b.publish_template_stops(root, &template, &two_stops(), &mut Run).unwrap();
    assert!(b.published_stop_range(root, &witness, &mut Run).is_err());
    assert_eq!(b.published_stop_range(root, &new, &mut Run).unwrap().first, 0);
    b.append_image(root, image(), &mut Run).unwrap();
    b.commit_draft(&draft, &mut Run).unwrap();
    assert!(b.freeze().is_ok());
}
#[test]
fn failed_append_inside_request_still_allows_atomic_cleanup() {
    let mut b = builder();
    let root = b.root_id();
    let d = b.begin_draft(root).unwrap();
    let prefix = add_shape(&mut b, root);
    let r = b.begin_request(&d, &mut Run).unwrap();
    let input = shape(b.source_owner());
    assert!(b.append_shape(root, input, &mut StopAfter(15)).is_err());
    b.rollback_request(&d, r).unwrap();
    assert!(b.scene(root).unwrap().shape(prefix).is_some());
    b.finalize_shape(&d, prefix, *b.scene(root).unwrap().shape(prefix).unwrap(), &mut Run).unwrap();
    b.commit_draft(&d, &mut Run).unwrap();
}
#[test]
fn empty_source_group_filter_owner_and_later_child_range_are_real() {
    let d = doc();
    let mut source = DecodeSource::new(&d, &mut Run).unwrap();
    let mut b = from_doc(&d);
    let root = b.root_id();
    let draft = b.begin_draft(root).unwrap();
    let group_id = b.append_group(root, group(), &mut Run).unwrap();
    let owner = b.definition_owner(&draft, &source, element(&d, b"g"),
        DefinitionTarget::Group(group_id), &mut Run).unwrap();
    let mut active = ActiveDefinitions::default();
    let mut ledger = ChargeLedger::new(d.source_owner(), 1 << 20, 65536).unwrap();
    owner.validate(&DefinitionContext { shared: &mut source, builder: &mut b, scene: root,
        active: &mut active, ledger: &mut ledger }).unwrap();
    assert!(b.scene(root).unwrap().shapes.is_empty());
    let fid = b.publish_group_filter(&draft, owner, filter(), &[primitive()], &mut Run).unwrap();
    assert_eq!(b.scene(root).unwrap().group(group_id).unwrap().filter, Some(fid));
    let child = add_shape(&mut b, root);
    let mut value = *b.scene(root).unwrap().group(group_id).unwrap();
    value.shapes = TableRange { first: 0, count: 1 };
    b.finalize_group(&draft, group_id, value, &mut Run).unwrap();
    b.commit_draft(&draft, &mut Run).unwrap();
    let graph = b.freeze().unwrap();
    assert_eq!(graph.root().group(group_id).unwrap().shapes.count, 1);
    assert!(graph.root().shape(child).is_some());
}
#[test]
fn group_owner_foreign_stale_prefix_and_canceled_publication_denied() {
    let d = doc();
    let source = DecodeSource::new(&d, &mut Run).unwrap();
    let mut b = from_doc(&d);
    let root = b.root_id();
    let draft = b.begin_draft(root).unwrap();
    let g = b.append_group(root, group(), &mut Run).unwrap();
    let own = b.definition_owner(&draft, &source, element(&d, b"g"), DefinitionTarget::Group(g), &mut Run).unwrap();
    let mut other = from_doc(&d);
    let od = other.begin_draft(other.root_id()).unwrap();
    assert!(other.publish_group_filter(&od, own, filter(), &[], &mut Run).is_err());
    assert!(b.publish_group_filter(&draft, own, filter(), &[primitive()], &mut StopAfter(1)).is_err());
    assert!(b.scene(root).unwrap().filters.is_empty());
    let r = b.begin_request(&draft, &mut Run).unwrap();
    assert!(b.publish_group_filter(&draft, own, filter(), &[], &mut Run).is_err());
    b.rollback_request(&draft, r).unwrap();
    b.rollback_draft(draft).unwrap();
    let new = b.begin_draft(root).unwrap();
    b.append_group(root, group(), &mut Run).unwrap();
    assert!(b.publish_group_filter(&new, own, filter(), &[], &mut Run).is_err());
}

/// Cache-owned fixture uses actual public preallocation; not an SVG resolver.
struct FixtureResolver { template: GradientTemplateId, stops: LedgerBuffer<PaintStop>, source: SourceOwner }
impl CachedPaintResolver for FixtureResolver {
    fn resolve_cached<'cache>(&'cache mut self, source: &mut DecodeSource<'_>, ledger: &mut ChargeLedger,
        _: vexel_xml::XmlSpan, _: &[Edge], _: Affine, _: [f64; 2],
        control: &mut dyn WorkControl) -> Result<CachedPaint<'cache>>
    {
        ledger.require_owner(ledger.owner_identity(), &self.source)?;
        assert!(Arc::ptr_eq(source.source_owner(), &self.source));
        for _ in self.stops.values(ledger)? {
            control.charge(WorkKind::PaintVisit, 1).map_err(controlled)?;
        }
        Ok(CachedPaint::Gradient { template: &self.template, paint: gradient(), stops: self.stops.values(ledger)? })
    }
}
#[test]
fn production_delegate_40000_instances_share_metered_cache_and_published_stops() {
    let d = doc();
    let mut source = DecodeSource::new(&d, &mut Run).unwrap();
    let mut ledger = ChargeLedger::new(d.source_owner(), 16 << 20, 65536).unwrap();
    let mut stops = ledger.allocate_buffer(AllocationTag::ReferenceIndex, 2, &mut Run).unwrap();
    stops.append(&ledger, &two_stops(), &mut Run).unwrap();
    let ptr = stops.values(&ledger).unwrap().as_ptr();
    let mut resolver = FixtureResolver { template: source.gradient_template_id(element(&d, b"linearGradient"),
        [100., 100.], &mut Run).unwrap(), stops, source: d.source_owner().clone() };
    let mut b = from_doc(&d);
    let root = b.root_id();
    let old = ledger.stats();
    let value = vexel_xml::XmlSpan { offset: 0, length: 0 };
    let mut published_ptr = std::ptr::null();
    for n in 0..40000 {
        for channel in [PaintChannel::Fill, PaintChannel::Stroke] {
            resolve_publish_cached(&mut resolver, &mut source, &mut ledger, &mut b, root,
                value, &[], identity(), [100., 100.], channel, &mut Run).unwrap();
        }
        assert_eq!(resolver.stops.values(&ledger).unwrap().as_ptr(), ptr);
        let current = b.scene(root).unwrap().stops.as_ptr();
        if n == 0 { published_ptr = current; }
        assert_eq!(current, published_ptr);
    }
    assert_eq!(ledger.stats(), old);
    assert_eq!(b.scene(root).unwrap().stops.len(), 2);
    assert_eq!(b.scene(root).unwrap().fill_paints.len(), 40000);
    assert_eq!(b.scene(root).unwrap().stroke_paints.len(), 40000);
    let before = b.scene(root).unwrap().fill_paints.len();
    assert!(resolve_publish_cached(&mut resolver, &mut source, &mut ledger, &mut b, root,
        value, &[], identity(), [100., 100.], PaintChannel::Fill, &mut StopAfter(1)).is_err());
    assert_eq!(b.scene(root).unwrap().fill_paints.len(), before);
    resolver.stops.release(&mut ledger).unwrap();
    assert!(b.freeze().is_ok());
}
#[test]
fn production_delegate_wrong_source_and_plain_gradient_cannot_fall_back() {
    let d = doc();
    let mut b = from_doc(&d);
    let root = b.root_id();
    assert!(b.append_cached_paint(root, CachedPaint::Plain(gradient()), PaintChannel::Fill, &mut Run).is_err());
    let other = doc();
    let source = DecodeSource::new(&other, &mut Run).unwrap();
    let template = source.gradient_template_id(element(&other, b"linearGradient"), [100., 100.], &mut Run).unwrap();
    assert!(b.append_cached_paint(root, CachedPaint::Gradient { template: &template, paint: gradient(),
        stops: &two_stops() }, PaintChannel::Stroke, &mut Run).is_err());
    assert!(b.scene(root).unwrap().stops.is_empty());
}
fn job(source: &SourceOwner, budget: usize, max: usize) -> RasterJob {
    RasterJob::new(ChargeLedger::new(source, budget, max).unwrap()).unwrap()
}
fn output(job: &mut RasterJob, generation: u64) -> JobRenderOutput {
    let pixels = job.buffer(RasterPool::Output, 16 * 1024 * 1024, &mut Run).unwrap();
    job.reserve(pixels, 4, &mut Run).unwrap();
    job.extend(pixels, &[1, 2, 3, 255], &mut Run).unwrap();
    JobRenderOutput { generation, pixels, width: 1, height: 1, stride: 4 }
}
fn descriptor(d: &LexicalDocument) -> PatternDescriptor {
    PatternDescriptor { definition: element(d, b"pattern"), width: 1, height: 1,
        tile_width: 2., tile_height: 4., content_transform: Affine { e: 3., f: 4., ..identity() }, error_offset: 0 }
}
fn mapping(source: SourceMap) -> PatternMapping {
    PatternMapping { tile_origin: [6., 8.], user_to_base: Affine { a: 0.5, d: 0.25, e: -6., f: -8., ..identity() },
        pattern_transform: Affine { a: 2., d: 4., ..identity() },
        user_inverse: Affine { a: 0.5, d: 0.5, ..identity() }, viewport: [100., 100.],
        bounds: [1., 2., 10., 20.], source_map: source, opacity: 0.75 }
}
#[test]
fn computed_pattern_mapping_transfers_same_job_pixels_and_attaches_fill_and_stroke() {
    let d = doc();
    let mut b = from_doc(&d);
    let root = b.root_id();
    let draft = b.begin_draft(root).unwrap();
    let mut j = job(d.source_owner(), 16 << 20, 65536);
    let mut tickets = PatternTickets::new().unwrap();
    for channel in [PaintChannel::Fill, PaintChannel::Stroke] {
        let s = add_shape(&mut b, root);
        let map = mapping(b.scene(root).unwrap().shape(s).unwrap().source);
        let child = b.new_scene(viewport()).unwrap();
        b.seal(child, &mut Run).unwrap();
        let req = tickets.request_mapped(&b, &draft, &DecodeSource::new(&d, &mut Run).unwrap(), child, descriptor(&d), map, s, channel, &j, &mut Run).unwrap();
        assert_eq!(req.content_transform, descriptor(&d).content_transform);
        let ticket = req.ticket;
        let out = output(&mut j, 7);
        let ptr = j.slice(out.pixels).unwrap().as_ptr();
        let (pid, index) = tickets.complete_mapped(ticket, &mut b, &draft, &mut j, Ok(out), &mut Run).unwrap();
        assert!(!tickets.outstanding());
        assert!(j.slice(out.pixels).is_err());
        let view = b.scene(root).unwrap();
        let pattern = view.pattern(pid).unwrap();
        assert_eq!(pattern.tile.rgba().as_ptr(), ptr);
        assert_eq!(pattern.content_transform, descriptor(&d).content_transform);
        let paint = match channel { PaintChannel::Fill => &view.fill_paints[index], PaintChannel::Stroke => &view.stroke_paints[index] };
        assert_eq!(paint.pattern, Some(pid));
        assert_eq!(paint.pattern_inverse, Affine { a: 0.25, d: 0.0625, e: -3., f: -2., ..identity() });
        assert_eq!(paint.pattern_inverse, map.pixel_inverse(descriptor(&d)).unwrap());
        assert_eq!(paint.user_inverse, map.user_inverse);
        assert_eq!(paint.bounds, map.bounds);
        assert_eq!(paint.pattern_opacity, map.opacity);
        assert_eq!(paint.coordinates[..4], [6., 8., 2., 4.]);
        let shape = view.shape(s).unwrap();
        assert_eq!(match channel { PaintChannel::Fill => shape.fill_paint, PaintChannel::Stroke => shape.stroke_paint }, Some(index));
        assert!(tickets.complete_mapped(ticket, &mut b, &draft, &mut j, Ok(out), &mut Run).is_err());
    }
    assert_eq!(j.stats().output_bytes, 0);
    b.commit_draft(&draft, &mut Run).unwrap();
    assert!(b.freeze().is_ok());
}
#[test]
fn pattern_wrong_ticket_job_dimensions_mapping_child_and_legacy_escape_denied() {
    let d = doc();
    let mut b = from_doc(&d);
    let root = b.root_id();
    let draft = b.begin_draft(root).unwrap();
    let s = add_shape(&mut b, root);
    let map = mapping(b.scene(root).unwrap().shape(s).unwrap().source);
    let child = b.new_scene(viewport()).unwrap();
    let mut j = job(d.source_owner(), 16 << 20, 65536);
    let mut tickets = PatternTickets::new().unwrap();
    assert!(tickets.request_mapped(&b, &draft, &DecodeSource::new(&d, &mut Run).unwrap(), child, descriptor(&d), map, s, PaintChannel::Fill, &j, &mut Run).is_err());
    b.seal(child, &mut Run).unwrap();
    let mut invalid_map = map;
    invalid_map.user_to_base.e = 0.;
    assert!(tickets.request_mapped(&b, &draft, &DecodeSource::new(&d, &mut Run).unwrap(), child, descriptor(&d), invalid_map, s, PaintChannel::Fill, &j, &mut Run).is_err());
    let ticket = tickets.request_mapped(&b, &draft, &DecodeSource::new(&d, &mut Run).unwrap(), child, descriptor(&d), map, s, PaintChannel::Fill, &j, &mut Run).unwrap().ticket;
    assert!(tickets.complete(ticket, pattern().tile, &mut Run).is_err());
    let mut peer = PatternTickets::new().unwrap();
    let foreign = peer.request(b.scene(child).unwrap(), descriptor(&d), &mut Run).unwrap().ticket;
    let out = output(&mut j, 7);
    assert!(tickets.complete_mapped(foreign, &mut b, &draft, &mut j, Ok(out), &mut Run).is_err());
    let mut other = job(d.source_owner(), 16 << 20, 65536);
    let wrong_job = output(&mut other, 7);
    assert!(tickets.complete_mapped(ticket, &mut b, &draft, &mut other, Ok(wrong_job), &mut Run).is_err());
    let wrong_dimensions = JobRenderOutput { width: 2, stride: 8, ..out };
    assert!(tickets.complete_mapped(ticket, &mut b, &draft, &mut j, Ok(wrong_dimensions), &mut Run).is_err());
    assert!(tickets.outstanding());
    tickets.complete_mapped(ticket, &mut b, &draft, &mut j, Ok(out), &mut Run).unwrap();
    other.release(wrong_job.pixels).unwrap();
}
#[test]
fn pattern_request_rollback_cancellation_and_lower_error_cleanup() {
    let d = doc();
    let mut b = from_doc(&d);
    let root = b.root_id();
    let draft = b.begin_draft(root).unwrap();
    let mut j = job(d.source_owner(), 16 << 20, 65536);
    let mut tickets = PatternTickets::new().unwrap();
    let r = b.begin_request(&draft, &mut Run).unwrap();
    let s = add_shape(&mut b, root);
    let map = mapping(b.scene(root).unwrap().shape(s).unwrap().source);
    let child = b.new_scene(viewport()).unwrap();
    b.seal(child, &mut Run).unwrap();
    let ticket = tickets.request_mapped(&b, &draft, &DecodeSource::new(&d, &mut Run).unwrap(), child, descriptor(&d), map, s, PaintChannel::Fill, &j, &mut Run).unwrap().ticket;
    let out = output(&mut j, 7);
    assert_eq!(tickets.complete_mapped(ticket, &mut b, &draft, &mut j, Ok(out), &mut StopAfter(0)).unwrap_err().kind, ErrorKind::Canceled);
    assert!(!tickets.outstanding());
    assert!(j.slice(out.pixels).is_err());
    assert!(b.scene(root).unwrap().patterns.is_empty());
    // A completed ticket is stale; request rollback removes its shape/child.
    b.rollback_request(&draft, r).unwrap();
    assert!(b.scene(child).is_err());
    let r = b.begin_request(&draft, &mut Run).unwrap();
    let s = add_shape(&mut b, root);
    let child = b.new_scene(viewport()).unwrap();
    b.seal(child, &mut Run).unwrap();
    let ticket = tickets.request_mapped(&b, &draft, &DecodeSource::new(&d, &mut Run).unwrap(), child, descriptor(&d), map, s, PaintChannel::Stroke, &j, &mut Run).unwrap().ticket;
    let error = Diagnostic { kind: ErrorKind::Number, offset: 7 };
    assert_eq!(tickets.complete_mapped(ticket, &mut b, &draft, &mut j, Err(error), &mut Run).unwrap_err(), error);
    assert!(!tickets.outstanding());
    b.rollback_request(&draft, r).unwrap();
    b.commit_draft(&draft, &mut Run).unwrap();
}
#[test]
fn mapped_pattern_late_cancellation_unwinds_published_pattern_and_paint() {
    // Sweep real publication work boundaries, including after tile transfer.
    for budget in 0..35 {
        let d = doc();
        let mut b = from_doc(&d);
        let root = b.root_id();
        let draft = b.begin_draft(root).unwrap();
        let s = add_shape(&mut b, root);
        let original = *b.scene(root).unwrap().shape(s).unwrap();
        let map = mapping(original.source);
        let child = b.new_scene(viewport()).unwrap();
        b.seal(child, &mut Run).unwrap();
        let mut j = job(d.source_owner(), 16 << 20, 65536);
        let mut tickets = PatternTickets::new().unwrap();
        let ticket = tickets.request_mapped(&b, &draft, &DecodeSource::new(&d, &mut Run).unwrap(), child, descriptor(&d), map, s, PaintChannel::Fill, &j, &mut Run).unwrap().ticket;
        let out = output(&mut j, 7);
        let result = tickets.complete_mapped(ticket, &mut b, &draft, &mut j, Ok(out), &mut StopAfter(budget));
        if result.is_err() {
            assert_eq!(b.scene(root).unwrap().shape(s), Some(&original));
            assert!(b.scene(root).unwrap().patterns.is_empty());
            assert!(b.scene(root).unwrap().fill_paints.is_empty());
        }
        assert!(!tickets.outstanding());
        assert!(j.slice(out.pixels).is_err());
        b.commit_draft(&draft, &mut Run).unwrap();
        assert!(b.freeze().is_ok());
    }
}
#[test]
fn public_buffer_preallocation_identity_retained_capacity_and_double_release() {
    let source = owner();
    let mut ledger = ChargeLedger::new(&source, 1 << 20, 65536).unwrap();
    let identity = ledger.owner_identity();
    let before = ledger.stats();
    ledger.require_owner(identity, &source).unwrap();
    assert_eq!(ledger.stats(), before); // no zero-byte identity lease
    let mut values = ledger.allocate_buffer::<u32>(AllocationTag::ReferenceIndex, 16, &mut Run).unwrap();
    assert_eq!(ledger.stats().reference_index_bytes, values.capacity(&ledger).unwrap() * 4);
    values.append(&ledger, &[3, 4, 5], &mut Run).unwrap();
    let stats = ledger.stats();
    values.values_mut(&ledger).unwrap()[0] = 7;
    values.truncate(&ledger, 2).unwrap();
    assert_eq!(values.values(&ledger).unwrap(), &[7, 4]);
    assert_eq!(ledger.stats(), stats);
    values.release(&mut ledger).unwrap();
    assert!(values.release(&mut ledger).is_err());
    assert!(values.values(&ledger).is_err());
    assert_eq!(ledger.stats().reference_index_bytes, 0);
    assert!(ledger.stats().live_bytes > source.capacity()); // honest retained registry
    assert_eq!(ledger.stats().total_allocated_bytes, stats.total_allocated_bytes);
}
#[test]
fn public_buffer_rejects_budget_before_physical_and_distinguishes_native_overflow() {
    let source = Arc::new(Vec::new());
    let mut ledger = ChargeLedger::new(&source, 4096, 65536).unwrap();
    // Warm registry so payload rejection does not confuse retained metadata history.
    let mut warm = ledger.allocate_buffer::<u8>(AllocationTag::None, 0, &mut Run).unwrap();
    warm.release(&mut ledger).unwrap();
    let old = ledger.stats();
    assert_eq!(ledger.allocate_buffer::<u8>(AllocationTag::Output, 4097, &mut Run).unwrap_err().kind,
        ErrorKind::Limit(LimitKind::ReservedBytes));
    assert_eq!(ledger.stats(), old);
    assert_eq!(ledger.allocate_buffer::<u64>(AllocationTag::Output, usize::MAX, &mut Run).unwrap_err().kind,
        ErrorKind::Limit(LimitKind::Arithmetic));
    assert!(ledger.stats().overflow);
    let mut huge = ChargeLedger::new(&source, usize::MAX, 65536).unwrap();
    let mut warm = huge.allocate_buffer::<u8>(AllocationTag::None, 0, &mut Run).unwrap();
    warm.release(&mut huge).unwrap();
    let old = huge.stats();
    assert_eq!(huge.allocate_buffer::<u8>(AllocationTag::Output, isize::MAX as usize + 1, &mut Run).unwrap_err().kind,
        ErrorKind::OutOfMemory); // real Vec CapacityOverflow, no physical payload reserve
    assert_eq!(huge.stats().total_allocated_bytes, old.total_allocated_bytes);
    assert_eq!(huge.stats().live_bytes, old.live_bytes);
}
struct CheckpointCancel(usize);
impl WorkControl for CheckpointCancel {
    fn charge(&mut self, _: WorkKind, _: u64) -> std::result::Result<(), Stop> { Ok(()) }
    fn checkpoint(&mut self) -> std::result::Result<(), Stop> {
        if self.0 == 0 { Err(Stop::Canceled) } else { self.0 -= 1; Ok(()) }
    }
}
#[test]
fn public_buffer_after_allocation_cancel_and_in_loop_atomic_copy() {
    let source = owner();
    let mut ledger = ChargeLedger::new(&source, 1 << 20, 65536).unwrap();
    let before = ledger.stats();
    assert_eq!(ledger.allocate_buffer::<u8>(AllocationTag::Output, 1024, &mut CheckpointCancel(1)).unwrap_err().kind, ErrorKind::Canceled);
    assert_eq!(ledger.stats().output_bytes, 0);
    assert!(ledger.stats().total_allocated_bytes >= before.total_allocated_bytes + 1024);
    let mut values = ledger.allocate_buffer::<u32>(AllocationTag::Output, 32, &mut Run).unwrap();
    values.append(&ledger, &[9, 8], &mut Run).unwrap();
    assert!(values.append(&ledger, &[7, 6, 5, 4], &mut StopAfter(2)).is_err());
    assert_eq!(values.values(&ledger).unwrap(), &[9, 8]);
    let capacity = values.capacity(&ledger).unwrap();
    let old = ledger.stats();
    assert!(values.replace_capacity(&mut ledger, AllocationTag::Output, 64, &mut StopAfter(1)).is_err());
    assert_eq!(values.values(&ledger).unwrap(), &[9, 8]);
    assert_eq!(values.capacity(&ledger).unwrap(), capacity);
    assert_eq!(ledger.stats().output_bytes, old.output_bytes);
    assert!(ledger.stats().total_allocated_bytes > old.total_allocated_bytes);
    values.release(&mut ledger).unwrap();
}
#[test]
fn public_buffer_old_new_overlap_wrong_source_job_and_record_limit() {
    let source = Arc::new(Vec::new());
    let mut ledger = ChargeLedger::new(&source, 8192, 65536).unwrap();
    let mut v = ledger.allocate_buffer::<u8>(AllocationTag::Output, 4096, &mut Run).unwrap();
    v.append(&ledger, &[1, 2, 3], &mut Run).unwrap();
    let old = ledger.stats();
    assert!(v.replace_capacity(&mut ledger, AllocationTag::Output, 4096, &mut Run).is_err());
    assert_eq!(ledger.stats().output_bytes, old.output_bytes);
    assert_eq!(v.values(&ledger).unwrap(), &[1, 2, 3]);
    let mut other = ChargeLedger::new(&source, 8192, 65536).unwrap();
    assert!(v.values(&other).is_err());
    assert!(v.release(&mut other).is_err());
    assert!(ledger.require_owner(other.owner_identity(), &source).is_err());
    assert!(ledger.require_owner(ledger.owner_identity(), &Arc::new(Vec::new())).is_err());
    v.release(&mut ledger).unwrap();
    // Exact query active-record limit: initial leases and replacement overlap.
    let mut capped = ChargeLedger::new(&source, 16 << 20, 65536).unwrap();
    let mut buffers = Vec::new(); // TEST descriptor list, not production registry
    for _ in 0..65536 { buffers.push(capped.allocate_buffer::<u8>(AllocationTag::None, 0, &mut Run).unwrap()); }
    let registry_live = capped.stats().live_bytes;
    assert!(capped.allocate_buffer::<u8>(AllocationTag::None, 0, &mut Run).is_err());
    assert!(buffers[0].replace_capacity(&mut capped, AllocationTag::None, 0, &mut Run).is_err());
    buffers[1].release(&mut capped).unwrap();
    buffers[0].replace_capacity(&mut capped, AllocationTag::None, 1, &mut Run).unwrap();
    for b in &mut buffers { if b.values(&capped).is_ok() { b.release(&mut capped).unwrap(); } }
    assert_eq!(capped.stats().live_bytes, registry_live);
}
#[test]
fn request_preserves_parent_context_with_real_nested_child_drafts() {
    let mut b = builder();
    let root = b.root_id();
    let outer = b.begin_draft(root).unwrap();
    let prefix_child = b.new_scene(viewport()).unwrap();
    let r = b.begin_request(&outer, &mut Run).unwrap();
    assert!(b.begin_draft(prefix_child).is_err());
    assert!(b.seal(prefix_child, &mut Run).is_err());
    let child = b.new_scene(viewport()).unwrap();
    let nested = b.begin_draft(child).unwrap();
    let id = add_shape(&mut b, child);
    b.append_image(child, image(), &mut Run).unwrap();
    assert!(b.begin_request(&nested, &mut Run).is_err());
    assert!(b.rollback_request(&outer, r).is_err());
    b.commit_draft(&nested, &mut Run).unwrap();
    b.seal(child, &mut Run).unwrap();
    let before = b.memory_stats();
    b.rollback_request(&outer, r).unwrap();
    assert!(b.scene(child).is_err());
    assert!(b.scene(prefix_child).is_ok());
    assert!(b.memory_stats().live_bytes < before.live_bytes);
    let reused_child = b.new_scene(viewport()).unwrap();
    assert_eq!(reused_child.index(), child.index());
    assert_ne!(reused_child, child);
    assert!(b.scene(reused_child).unwrap().shape(id).is_none());
    b.seal(prefix_child, &mut Run).unwrap();
    b.seal(reused_child, &mut Run).unwrap();
    b.commit_draft(&outer, &mut Run).unwrap();
    assert!(b.freeze().is_ok());
}
#[test]
fn internal_paint_cancel_preserves_prefix_witness_and_request_rollbackability() {
    for budget in 0..35 {
        let d = doc();
        let source = DecodeSource::new(&d, &mut Run).unwrap();
        let def = element(&d, b"linearGradient");
        let prefix = source.gradient_template_id(def, [100., 100.], &mut Run).unwrap();
        let suffix = source.gradient_template_id(def, [200., 200.], &mut Run).unwrap();
        let mut b = from_doc(&d);
        let root = b.root_id();
        let draft = b.begin_draft(root).unwrap();
        let old = b.publish_template_stops(root, &prefix, &two_stops(), &mut Run).unwrap();
        let r = b.begin_request(&draft, &mut Run).unwrap();
        let result = b.append_template_fill(root, &suffix, gradient(), &two_stops(), &mut StopAfter(budget));
        if result.is_err() {
            assert_eq!(b.scene(root).unwrap().stops.len(), 2);
            assert!(b.scene(root).unwrap().fill_paints.is_empty());
        }
        assert_eq!(b.published_stop_range(root, &old, &mut Run).unwrap().count, 2);
        b.rollback_request(&draft, r).unwrap();
        assert_eq!(b.scene(root).unwrap().stops.len(), 2);
        b.commit_draft(&draft, &mut Run).unwrap();
        assert!(b.freeze().is_ok());
    }
}
/// Explicit planner fixture exercises the full typed owner and nonempty source
/// expansion. Full parsing/planning remains the lower writer's scope.
struct FilterPlanner;
impl DefinitionPlanner for FilterPlanner {
    fn plan_owned_definition<'src>(&mut self, ctx: &mut DefinitionContext<'src, '_>,
        _: DefinitionKind, element: ElementId, owner: DefinitionOwner, transform: Affine,
        _: [f64; 4], style: &CssPaint<'src>, control: &mut dyn WorkControl)
        -> Result<ProductionDefinitionPlan<'src>>
    {
        owner.validate(ctx)?;
        control.charge(WorkKind::FilterPixel, 1).map_err(controlled)?;
        Ok(ProductionDefinitionPlan::Definition(DefinitionPlan {
            descriptor: DefinitionDescriptor::Filter { filter: filter(), primitives: vec![primitive()] },
            expansions: vec![Expansion { range: TableRange { first: element.0, count: 1 }, transform,
                viewport: [100., 100.], style: *style, flavor: DecodeFlavor::Normal,
                post: PostOperation::FinishGroup(match owner.target() { DefinitionTarget::Group(g) => g, _ => unreachable!() }) }],
            post: PostOperation::None,
        }))
    }
}
#[test]
fn full_typed_planner_fixture_flows_into_actual_empty_group_filter_publication() {
    let d = doc();
    let mut source = DecodeSource::new(&d, &mut Run).unwrap();
    let elem = element(&d, b"g");
    let style = source.compute_style(elem, None, &mut Run).unwrap();
    let mut b = from_doc(&d);
    let root = b.root_id();
    let draft = b.begin_draft(root).unwrap();
    let g = b.append_group(root, group(), &mut Run).unwrap();
    let owner = b.definition_owner(&draft, &source, elem, DefinitionTarget::Group(g), &mut Run).unwrap();
    let mut active = ActiveDefinitions::default();
    let mut ledger = ChargeLedger::new(d.source_owner(), 1 << 20, 65536).unwrap();
    let mut ctx = DefinitionContext { shared: &mut source, builder: &mut b, scene: root,
        active: &mut active, ledger: &mut ledger };
    let plan = plan_production_definition(&mut FilterPlanner, &mut ctx, DefinitionKind::Filter,
        elem, owner, identity(), [0., 0., 1., 1.], &style, &mut Run).unwrap();
    let ProductionDefinitionPlan::Definition(plan) = plan else { panic!() };
    let DefinitionDescriptor::Filter { filter, primitives } = plan.descriptor else { panic!() };
    assert_eq!(plan.expansions.len(), 1);
    assert!(plan_production_definition(&mut FilterPlanner, &mut ctx, DefinitionKind::Filter,
        elem, owner, identity(), [0., 0., 1., 1.], &style, &mut StopAfter(0)).is_err());
    assert!(plan_production_definition(&mut FilterPlanner, &mut ctx, DefinitionKind::Marker,
        elem, owner, identity(), [0., 0., 1., 1.], &style, &mut Run).is_err());
    b.publish_group_filter(&draft, owner, filter, &primitives, &mut Run).unwrap();
    b.commit_draft(&draft, &mut Run).unwrap();
    let graph = b.freeze().unwrap();
    assert!(graph.root().shapes.is_empty());
    assert_eq!(graph.root().filters.len(), 1);
    assert_eq!(graph.root().filter_primitives, &[primitive()]);
}
#[test]
fn stale_mapped_scope_reclaims_own_output_and_invalidates_ticket() {
    let d = doc();
    let mut b = from_doc(&d);
    let root = b.root_id();
    let draft = b.begin_draft(root).unwrap();
    let r = b.begin_request(&draft, &mut Run).unwrap();
    let s = add_shape(&mut b, root);
    let map = mapping(b.scene(root).unwrap().shape(s).unwrap().source);
    let child = b.new_scene(viewport()).unwrap();
    b.seal(child, &mut Run).unwrap();
    let mut j = job(d.source_owner(), 16 << 20, 65536);
    let mut tickets = PatternTickets::new().unwrap();
    let ticket = tickets.request_mapped(&b, &draft, &DecodeSource::new(&d, &mut Run).unwrap(), child,
        descriptor(&d), map, s, PaintChannel::Fill, &j, &mut Run).unwrap().ticket;
    let out = output(&mut j, 7);
    b.rollback_request(&draft, r).unwrap();
    assert!(tickets.complete_mapped(ticket, &mut b, &draft, &mut j, Ok(out), &mut Run).is_err());
    assert!(j.slice(out.pixels).is_err());
    assert!(!tickets.outstanding());
    assert_eq!(j.stats().output_bytes, 0);
    b.commit_draft(&draft, &mut Run).unwrap();
}
#[test]
fn public_buffer_successful_growth_shrink_overlap_and_reuse_are_actual() {
    let source = owner();
    let mut ledger = ChargeLedger::new(&source, 1 << 20, 65536).unwrap();
    let mut v = ledger.allocate_buffer::<u32>(AllocationTag::Output, 16, &mut Run).unwrap();
    v.append(&ledger, &[7, 8, 9], &mut Run).unwrap();
    let old = ledger.stats();
    v.replace_capacity(&mut ledger, AllocationTag::Output, 64, &mut Run).unwrap();
    assert_eq!(v.values(&ledger).unwrap(), &[7, 8, 9]);
    let actual = v.capacity(&ledger).unwrap() * 4;
    assert_eq!(ledger.stats().output_bytes, actual);
    assert!(ledger.stats().peak_live_bytes >= old.live_bytes + actual);
    let grown = ledger.stats();
    assert!(v.replace_capacity(&mut ledger, AllocationTag::Output, 2, &mut Run).is_err());
    assert_eq!(ledger.stats(), grown);
    v.replace_capacity(&mut ledger, AllocationTag::Output, 3, &mut Run).unwrap();
    assert_eq!(v.values(&ledger).unwrap(), &[7, 8, 9]);
    assert!(v.capacity(&ledger).unwrap() < 64);
    v.release(&mut ledger).unwrap();
    let mut reuse = ledger.allocate_buffer::<u8>(AllocationTag::Output, 3, &mut Run).unwrap();
    assert!(v.values(&ledger).is_err());
    reuse.release(&mut ledger).unwrap();
}
struct LoopLimit;
impl WorkControl for LoopLimit {
    fn checkpoint(&mut self) -> std::result::Result<(), Stop> { Ok(()) }
    fn charge(&mut self, _: WorkKind, _: u64) -> std::result::Result<(), Stop> { Err(Stop::Limit) }
}
#[test]
fn cached_delegate_borrow_outlives_temporary_source_and_ledger_borrows_without_copy() {
    let d = doc();
    let mut ledger = ChargeLedger::new(d.source_owner(), 1 << 20, 65536).unwrap();
    let mut b = from_doc(&d);
    let root = b.root_id();
    let mut stops = ledger.allocate_buffer(AllocationTag::ReferenceIndex, 2, &mut Run).unwrap();
    stops.append(&ledger, &two_stops(), &mut Run).unwrap();
    let template = DecodeSource::new(&d, &mut Run).unwrap().gradient_template_id(
        element(&d, b"linearGradient"), [100., 100.], &mut Run).unwrap();
    let mut resolver = FixtureResolver { template, stops, source: d.source_owner().clone() };
    let value = vexel_xml::XmlSpan { offset: 0, length: 0 };
    let cached = {
        let mut temporary = DecodeSource::new(&d, &mut Run).unwrap();
        resolver.resolve_cached(&mut temporary, &mut ledger, value, &[], identity(), [100., 100.], &mut Run).unwrap()
    };
    drop(d);
    let before = ledger.stats(); // independent temporary mutable ledger borrow has ended
    b.append_cached_paint(root, cached, PaintChannel::Fill, &mut Run).unwrap();
    assert_eq!(ledger.stats(), before);
    let d = LexicalDocument::parse(resolver.source.clone(), 1 << 20, &mut Run).unwrap();
    let mut source = DecodeSource::new(&d, &mut Run).unwrap();
    let old = b.scene(root).unwrap().fill_paints.len();
    assert_eq!(resolve_publish_cached(&mut resolver, &mut source, &mut ledger, &mut b, root,
        value, &[], identity(), [100., 100.], PaintChannel::Fill, &mut LoopLimit).unwrap_err().kind,
        ErrorKind::Limit(LimitKind::Work));
    assert_eq!(b.scene(root).unwrap().fill_paints.len(), old);
    resolver.stops.release(&mut ledger).unwrap();
}
