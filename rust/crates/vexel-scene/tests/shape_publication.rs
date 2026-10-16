mod support;
use support::*;
use vexel_scene::*;

struct CancelAfterEdges(usize);
impl WorkControl for CancelAfterEdges {
    fn charge(&mut self, kind: WorkKind, _: u64) -> std::result::Result<(), Stop> {
        if kind == WorkKind::Edge {
            if self.0 == 0 {
                return Err(Stop::Canceled);
            }
            self.0 -= 1;
        }
        Ok(())
    }
    fn checkpoint(&mut self) -> std::result::Result<(), Stop> {
        Ok(())
    }
}

struct PanicAfterEdges(usize);
impl WorkControl for PanicAfterEdges {
    fn charge(&mut self, kind: WorkKind, _: u64) -> std::result::Result<(), Stop> {
        if kind == WorkKind::Edge {
            if self.0 == 0 {
                panic!("injected edge-publication panic");
            }
            self.0 -= 1;
        }
        Ok(())
    }
    fn checkpoint(&mut self) -> std::result::Result<(), Stop> {
        Ok(())
    }
}

fn draft_request(
    b: &mut SceneBuilder,
    control: &mut dyn WorkControl,
) -> (DraftToken, RequestToken) {
    let draft = b.begin_draft(b.root_id()).unwrap();
    let request = b.begin_request(&draft, control).unwrap();
    (draft, request)
}

fn new_request() -> (SceneBuilder, DraftToken, RequestToken, Run) {
    let mut builder = builder();
    let mut control = Run;
    let draft = builder.begin_draft(builder.root_id()).unwrap();
    let request = builder.begin_request(&draft, &mut control).unwrap();
    (builder, draft, request, control)
}

fn finish_request(
    mut builder: SceneBuilder,
    draft: DraftToken,
    request: RequestToken,
    control: &mut dyn WorkControl,
) -> SceneGraph {
    builder.commit_request(&draft, request, control).unwrap();
    builder.commit_draft(&draft, control).unwrap();
    builder.freeze().unwrap()
}

#[test]
fn admitted_shape_is_real_request_scoped_and_commits() {
    let (mut b, draft, request, mut c) = new_request();
    let token = b.admit_shape(&draft, request, 6, &mut c).unwrap();
    let id = b
        .append_admitted_shape(&draft, &token, shape(b.source_owner()), &mut c)
        .unwrap();
    let graph = finish_request(b, draft, request, &mut c);
    let view = graph.root();
    assert_eq!(view.shape_id(0), Some(id));
    assert_eq!(view.shapes.len(), 1);
}

#[test]
fn unresolved_admission_blocks_commit_and_cancel_releases_it() {
    let (mut b, draft, request, mut c) = new_request();
    let token = b.admit_shape(&draft, request, 9, &mut c).unwrap();
    let error = b.commit_request(&draft, request, &mut c).unwrap_err();
    assert_eq!(error.offset, 9);
    assert_eq!(error.kind, ErrorKind::InvalidArgument);
    b.cancel_shape_admission(&draft, &token).unwrap();
    assert!(
        finish_request(b, draft, request, &mut c)
            .root()
            .shapes
            .is_empty()
    );
}

#[test]
fn admissions_reserve_distinct_slots_and_direct_append_respects_them() {
    let (mut b, draft, request, mut c) = new_request();
    let first = b.admit_shape(&draft, request, 10, &mut c).unwrap();
    let second = b.admit_shape(&draft, request, 11, &mut c).unwrap();
    let direct = b
        .append_shape(b.root_id(), shape(b.source_owner()), &mut c)
        .unwrap();
    let a = b
        .append_admitted_shape(&draft, &second, shape(b.source_owner()), &mut c)
        .unwrap();
    let z = b
        .append_admitted_shape(&draft, &first, shape(b.source_owner()), &mut c)
        .unwrap();
    assert_ne!(direct, a);
    assert_ne!(a, z);
    b.commit_request(&draft, request, &mut c).unwrap();
    b.commit_draft(&draft, &mut c).unwrap();
    let root = b.root_id();
    assert_eq!(b.freeze().unwrap().scene(root).unwrap().shapes.len(), 3);
}

#[test]
fn shape_limit_preflight_keeps_source_offset_and_active_prefix() {
    let source = owner();
    let animation = animation(&source);
    let mut limits = Limits::default();
    limits.shapes = 1;
    let mut b = SceneBuilder::new(source, animation, 7, viewport(), limits).unwrap();
    let mut c = Run;
    let (draft, request) = draft_request(&mut b, &mut c);
    let first = b.admit_shape(&draft, request, 13, &mut c).unwrap();
    let error = b.admit_shape(&draft, request, 17, &mut c).unwrap_err();
    assert_eq!(error.kind, ErrorKind::Limit(LimitKind::Shapes));
    assert_eq!(error.offset, 17);
    b.cancel_shape_admission(&draft, &first).unwrap();
    assert!(
        finish_request(b, draft, request, &mut c)
            .root()
            .shapes
            .is_empty()
    );
}

#[test]
fn shape_capacity_preflight_wins_before_path_provider_and_malformed_stroke() {
    let source = owner();
    let mut limits = Limits::default();
    limits.shapes = 1;
    let mut b =
        SceneBuilder::new(source.clone(), animation(&source), 7, viewport(), limits).unwrap();
    let mut c = Run;
    let (draft, request) = draft_request(&mut b, &mut c);
    let reserved = b.admit_shape(&draft, request, 11, &mut c).unwrap();
    let malformed_shape = Shape::new(SourceMap {
        element: Span { start: 5, end: 31 },
        transform: vexel_geometry::affine_identity(),
        transform_node: None,
    });
    let mut malformed_shape = malformed_shape;
    malformed_shape.stroke.width = f64::NAN;
    let mut provider_calls = 0;
    let error = match b.admit_shape(&draft, request, 19, &mut c) {
        Err(error) => error,
        Ok(admission) => {
            provider_calls += 1;
            let path_start = source.windows(2).position(|bytes| bytes == b"M0").unwrap();
            let path = vexel_svg_path::parse_path(
                &source,
                Span {
                    start: path_start,
                    end: path_start + b"M0 0 L1 1 2 3 Z".len(),
                },
                vexel_geometry::affine_identity(),
                Default::default(),
            )
            .unwrap();
            let edge_colors = (0..path.edges.len())
                .map(|index| [index as u8, 2, 3, 255])
                .collect();
            b.append_admitted_shape(
                &draft,
                &admission,
                ShapeInput {
                    shape: malformed_shape,
                    path,
                    edge_colors,
                },
                &mut c,
            )
            .unwrap_err()
        }
    };
    assert_eq!(error.kind, ErrorKind::Limit(LimitKind::Shapes));
    assert_eq!(error.offset, 19);
    assert_eq!(provider_calls, 0);
    b.cancel_shape_admission(&draft, &reserved).unwrap();
    assert!(
        finish_request(b, draft, request, &mut c)
            .root()
            .shapes
            .is_empty()
    );
}

#[test]
fn admission_table_probes_obey_work_budget_without_mutating_active_prefix() {
    let (mut b, draft, request, mut c) = new_request();
    let first = b.admit_shape(&draft, request, 8, &mut c).unwrap();
    let error = b
        .admit_shape(&draft, request, 18, &mut StopAfter(0))
        .unwrap_err();
    assert_eq!(error.kind, ErrorKind::Canceled);
    assert_eq!(error.offset, 18);
    assert_eq!(
        b.commit_request(&draft, request, &mut c)
            .unwrap_err()
            .offset,
        8
    );
    b.cancel_shape_admission(&draft, &first).unwrap();
    assert!(
        finish_request(b, draft, request, &mut c)
            .root()
            .shapes
            .is_empty()
    );
}

#[test]
fn request_rollback_cleans_dropped_admission_and_cancel_precedes_allocation() {
    let (mut b, draft, request, mut c) = new_request();
    let token = b.admit_shape(&draft, request, 5, &mut c).unwrap();
    drop(token);
    assert!(b.rollback_request(&draft, request).is_ok());
    b.commit_draft(&draft, &mut c).unwrap();
    assert!(b.freeze().is_ok());

    let mut b = builder();
    let mut c = StopAfter(0);
    let draft = b.begin_draft(b.root_id()).unwrap();
    let request = b.begin_request(&draft, &mut Run).unwrap();
    let before = b.memory_stats();
    assert_eq!(
        b.admit_shape(&draft, request, 7, &mut c).unwrap_err().kind,
        ErrorKind::Canceled
    );
    assert_eq!(b.memory_stats(), before);
    b.rollback_request(&draft, request).unwrap();
    b.rollback_draft(draft).unwrap();
}

#[test]
fn canceled_append_consumes_only_its_admission_and_request_can_retry() {
    let (mut b, draft, request, mut c) = new_request();
    let source = b.source_owner().clone();
    let first = b.admit_shape(&draft, request, 3, &mut c).unwrap();
    let first_id = b
        .append_admitted_shape(&draft, &first, shape(&source), &mut c)
        .unwrap();

    let canceled = b.admit_shape(&draft, request, 9, &mut c).unwrap();
    let error = b
        .append_admitted_shape(&draft, &canceled, shape(&source), &mut StopAfter(0))
        .unwrap_err();
    assert_eq!(error.kind, ErrorKind::Canceled);

    let retry = b.admit_shape(&draft, request, 12, &mut c).unwrap();
    let retry_id = b
        .append_admitted_shape(&draft, &retry, shape(&source), &mut c)
        .unwrap();
    let graph = finish_request(b, draft, request, &mut c);
    let root = graph.root();
    assert_eq!(root.shapes.len(), 2);
    assert_eq!(root.shape_id(0), Some(first_id));
    assert_eq!(root.shape_id(1), Some(retry_id));
}

#[test]
fn partial_publication_cancellation_preserves_another_reserved_shape() {
    let (mut b, draft, request, mut c) = new_request();
    let source = b.source_owner().clone();
    let interrupted = b.admit_shape(&draft, request, 6, &mut c).unwrap();
    let survivor = b.admit_shape(&draft, request, 12, &mut c).unwrap();
    let input = shape(&source);
    let edge_count = input.path.edges.len();
    assert!(edge_count >= 2);
    let error = b
        .append_admitted_shape(
            &draft,
            &interrupted,
            input,
            &mut CancelAfterEdges(edge_count + 1),
        )
        .unwrap_err();
    assert_eq!(error.kind, ErrorKind::Canceled);
    let id = b
        .append_admitted_shape(&draft, &survivor, shape(&source), &mut c)
        .unwrap();
    let graph = finish_request(b, draft, request, &mut c);
    let root = graph.root();
    assert_eq!(root.shapes.len(), 1);
    assert_eq!(root.shape_id(0), Some(id));
}

#[test]
fn panic_during_partial_publication_requires_request_rollback() {
    let (mut b, draft, request, mut c) = new_request();
    let source = b.source_owner().clone();
    let admission = b.admit_shape(&draft, request, 14, &mut c).unwrap();
    let input = shape(&source);
    let edge_count = input.path.edges.len();
    assert!(edge_count >= 2);
    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = b.append_admitted_shape(
            &draft,
            &admission,
            input,
            &mut PanicAfterEdges(edge_count + 1),
        );
    }));
    assert!(panic.is_err());
    assert_eq!(
        b.commit_request(&draft, request, &mut c).unwrap_err().kind,
        ErrorKind::InvalidArgument
    );
    b.rollback_request(&draft, request).unwrap();
    b.commit_draft(&draft, &mut c).unwrap();
    let graph = b.freeze().unwrap();
    assert!(graph.root().shapes.is_empty());
}

#[test]
fn foreign_builder_and_cross_draft_cannot_consume_an_admission() {
    let (mut b, draft, request, mut c) = new_request();
    let admission = b.admit_shape(&draft, request, 10, &mut c).unwrap();
    let source = b.source_owner().clone();
    let child_scene = b.new_scene(viewport()).unwrap();
    let child = b.begin_draft(child_scene).unwrap();
    assert_eq!(
        b.append_admitted_shape(&child, &admission, shape(&source), &mut c)
            .unwrap_err()
            .kind,
        ErrorKind::InvalidArgument
    );
    b.rollback_draft(child).unwrap();
    let id = b
        .append_admitted_shape(&draft, &admission, shape(&source), &mut c)
        .unwrap();
    assert_eq!(
        finish_request(b, draft, request, &mut c).root().shape_id(0),
        Some(id)
    );

    let mut origin = builder();
    let (origin_draft, origin_request) = draft_request(&mut origin, &mut c);
    let foreign_admission = origin
        .admit_shape(&origin_draft, origin_request, 15, &mut c)
        .unwrap();
    let mut foreign = builder();
    let (foreign_draft, foreign_request) = draft_request(&mut foreign, &mut c);
    assert_eq!(
        foreign
            .append_admitted_shape(
                &foreign_draft,
                &foreign_admission,
                shape(foreign.source_owner()),
                &mut c,
            )
            .unwrap_err()
            .kind,
        ErrorKind::InvalidArgument
    );
    foreign
        .rollback_request(&foreign_draft, foreign_request)
        .unwrap();
    foreign.rollback_draft(foreign_draft).unwrap();
    origin
        .cancel_shape_admission(&origin_draft, &foreign_admission)
        .unwrap();
    origin
        .commit_request(&origin_draft, origin_request, &mut c)
        .unwrap();
    origin.commit_draft(&origin_draft, &mut c).unwrap();
    assert!(origin.freeze().unwrap().root().shapes.is_empty());
}

#[test]
fn canceled_admission_retains_real_capacity_and_reuses_it_without_new_growth() {
    let (mut b, draft, request, mut c) = new_request();
    let before = b.memory_stats();
    let admission = b.admit_shape(&draft, request, 4, &mut c).unwrap();
    let grown = b.memory_stats();
    assert!(grown.live_bytes > before.live_bytes);
    assert!(grown.total_allocated_bytes > before.total_allocated_bytes);
    b.cancel_shape_admission(&draft, &admission).unwrap();
    assert_eq!(b.memory_stats(), grown);

    let reused = b.admit_shape(&draft, request, 7, &mut c).unwrap();
    assert_eq!(
        b.memory_stats().total_allocated_bytes,
        grown.total_allocated_bytes
    );
    b.cancel_shape_admission(&draft, &reused).unwrap();
    assert!(
        finish_request(b, draft, request, &mut c)
            .root()
            .shapes
            .is_empty()
    );
}

#[test]
fn equal_bytes_from_another_immutable_source_owner_cannot_authorize_geometry() {
    let source = std::sync::Arc::new(br#"<svg><path d="M0 0 L1 1 2 3 Z"></path></svg>"#.to_vec());
    let foreign = std::sync::Arc::new(source.as_ref().clone());
    let mut c = Run;
    let document = LexicalDocument::parse(foreign.clone(), 1 << 20, &mut c).unwrap();
    let decoded = DecodeSource::new(&document, &mut c).unwrap();
    let element = ElementId(1);
    let view = decoded
        .element(element, document.tokens().len(), &mut c)
        .unwrap();
    let geometry = decoded.canonical_path_source(view, &mut c).unwrap();
    let mut b = SceneBuilder::new(
        source.clone(),
        animation(&source),
        7,
        viewport(),
        Limits::default(),
    )
    .unwrap();
    let (draft, request) = draft_request(&mut b, &mut c);
    assert_eq!(
        b.admit_shape_with_source(&draft, request, 0, geometry, &mut c)
            .unwrap_err()
            .kind,
        ErrorKind::InvalidArgument
    );
    b.rollback_request(&draft, request).unwrap();
    b.rollback_draft(draft).unwrap();
}

#[test]
fn canonical_path_proof_rejects_wrong_kind_and_out_of_range_elements() {
    let source = std::sync::Arc::new(
        br##"<svg><path d="M0 0 L1 1 2 3 Z"/><use href="#p"/></svg>"##.to_vec(),
    );
    let mut c = Run;
    let document = LexicalDocument::parse(source, 1 << 20, &mut c).unwrap();
    let decoded = DecodeSource::new(&document, &mut c).unwrap();
    let use_element = document
        .tokens()
        .iter()
        .enumerate()
        .find(|(_, token)| {
            matches!(
                token.kind,
                vexel_xml::TokenKind::StartElement | vexel_xml::TokenKind::EmptyElement
            ) && token
                .local_name
                .slice(decoded.source())
                .is_some_and(|name| name == b"use")
        })
        .map(|(index, _)| ElementId(index))
        .unwrap();
    let use_view = decoded
        .element(use_element, document.tokens().len(), &mut c)
        .unwrap();
    assert_eq!(
        decoded
            .canonical_path_source(use_view, &mut c)
            .unwrap_err()
            .kind,
        ErrorKind::InvalidArgument
    );
    assert_eq!(
        decoded
            .element(ElementId(usize::MAX), document.tokens().len(), &mut c)
            .unwrap_err()
            .kind,
        ErrorKind::InvalidArgument
    );
}

#[test]
fn parent_request_owns_nested_child_admission_and_rolls_child_back() {
    let mut b = builder();
    let mut c = Run;
    let source = b.source_owner().clone();
    let (draft, request) = draft_request(&mut b, &mut c);
    let child_scene = b.new_scene(viewport()).unwrap();
    let child = b.begin_draft(child_scene).unwrap();
    let admission = b.admit_shape(&child, request, 12, &mut c).unwrap();
    b.append_admitted_shape(&child, &admission, shape(&source), &mut c)
        .unwrap();
    b.commit_draft(&child, &mut c).unwrap();
    b.rollback_request(&draft, request).unwrap();
    b.commit_draft(&draft, &mut c).unwrap();
    let graph = b.freeze().unwrap();
    assert!(graph.scene(child_scene).is_none());
    assert!(graph.root().shapes.is_empty());
}

#[test]
fn outer_build_mark_remains_rollbackable_after_admission_consumption() {
    let mut b = builder();
    let root = b.root_id();
    let mark = b.mark(root).unwrap();
    let mut c = Run;
    let (draft, request) = draft_request(&mut b, &mut c);
    let admission = b.admit_shape(&draft, request, 8, &mut c).unwrap();
    let source = b.source_owner().clone();
    b.append_admitted_shape(&draft, &admission, shape(&source), &mut c)
        .unwrap();
    b.commit_request(&draft, request, &mut c).unwrap();
    b.commit_draft(&draft, &mut c).unwrap();
    b.rollback(mark).unwrap();
    let graph = b.freeze().unwrap();
    assert!(graph.root().shapes.is_empty());
}

#[test]
fn actual_capacity_failure_is_pregeometry_and_reports_the_shape_source() {
    let source = owner();
    let measure = SceneBuilder::new(
        source.clone(),
        animation(&source),
        7,
        viewport(),
        Limits::default(),
    )
    .unwrap();
    let budget = measure.memory_stats().live_bytes;
    drop(measure);
    let mut limits = Limits::default();
    limits.reserved_bytes = budget;
    let source = owner();
    let mut b =
        SceneBuilder::new(source.clone(), animation(&source), 7, viewport(), limits).unwrap();
    let mut c = Run;
    let (draft, request) = draft_request(&mut b, &mut c);
    let before = b.memory_stats();
    let error = b.admit_shape(&draft, request, 23, &mut c).unwrap_err();
    assert_eq!(error.kind, ErrorKind::Limit(LimitKind::ReservedBytes));
    assert_eq!(error.offset, 23);
    assert_eq!(b.memory_stats(), before);
    b.rollback_request(&draft, request).unwrap();
    b.rollback_draft(draft).unwrap();
}

#[test]
fn capacity_failure_after_shape_growth_preserves_serial_accounting() {
    let source = owner();
    let baseline = SceneBuilder::new(
        source.clone(),
        animation(&source),
        7,
        viewport(),
        Limits::default(),
    )
    .unwrap();
    let initial_live = baseline.memory_stats().live_bytes;
    drop(baseline);

    let mut limits = Limits::default();
    limits.reserved_bytes = initial_live
        .checked_add(std::mem::size_of::<ShapeAdmission>() * 8)
        .unwrap();
    let mut b =
        SceneBuilder::new(source.clone(), animation(&source), 7, viewport(), limits).unwrap();
    let mut c = Run;
    let (draft, request) = draft_request(&mut b, &mut c);
    let before = b.memory_stats();
    let error = b.admit_shape(&draft, request, 29, &mut c).unwrap_err();
    let after = b.memory_stats();
    assert_eq!(error.kind, ErrorKind::Limit(LimitKind::ReservedBytes));
    assert_eq!(error.offset, 29);
    assert_eq!(after.allocation_count, before.allocation_count + 2);
    assert!(after.live_bytes > before.live_bytes);
    assert!(after.total_allocated_bytes > before.total_allocated_bytes);
    b.rollback_request(&draft, request).unwrap();
    b.rollback_draft(draft).unwrap();
}

#[test]
fn abort_and_a_later_request_cannot_replay_a_stale_admission() {
    let mut b = builder();
    let mut c = Run;
    let draft = b.begin_draft(b.root_id()).unwrap();
    let first = b.begin_request(&draft, &mut c).unwrap();
    let stale = b.admit_shape(&draft, first, 4, &mut c).unwrap();
    b.abort_request(&draft).unwrap();
    let second = b.begin_request(&draft, &mut c).unwrap();
    let source = b.source_owner().clone();
    let error = b
        .append_admitted_shape(&draft, &stale, shape(&source), &mut c)
        .unwrap_err();
    assert_eq!(error.kind, ErrorKind::InvalidArgument);
    b.commit_request(&draft, second, &mut c).unwrap();
    b.commit_draft(&draft, &mut c).unwrap();
    assert!(b.freeze().is_ok());
}
