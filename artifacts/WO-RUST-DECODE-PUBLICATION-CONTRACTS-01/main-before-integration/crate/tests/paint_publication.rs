mod support;
use std::sync::Arc;
use support::*;
use vexel_scene::*;
const SVG: &[u8] = br#"<svg><defs><linearGradient id="a"><stop offset="0"/><stop offset="1"/></linearGradient><radialGradient id="b"/></defs></svg>"#;
fn document() -> LexicalDocument {
    let mut bytes = Vec::with_capacity(4096);
    bytes.extend_from_slice(SVG);
    LexicalDocument::parse(Arc::new(bytes), 1 << 20, &mut Run).unwrap()
}
fn scene(doc: &LexicalDocument, limits: Limits) -> SceneBuilder {
    SceneBuilder::new(
        doc.source_owner().clone(),
        animation(doc.source_owner()),
        1,
        viewport(),
        limits,
    )
    .unwrap()
}
fn definition(doc: &LexicalDocument, radial: bool) -> ElementId {
    let name = if radial {
        b"radialGradient".as_slice()
    } else {
        b"linearGradient".as_slice()
    };
    ElementId(
        doc.tokens()
            .iter()
            .position(|t| t.local_name.slice(doc.source()) == Some(name))
            .unwrap(),
    )
}
fn template(doc: &LexicalDocument, radial: bool, viewport: [f64; 2]) -> GradientTemplateId {
    DecodeSource::new(doc, &mut Run)
        .unwrap()
        .gradient_template_id(definition(doc, radial), viewport, &mut Run)
        .unwrap()
}
fn stops() -> [PaintStop; 2] {
    [
        PaintStop {
            offset: 0.,
            rgba: [1, 2, 3, 255],
        },
        PaintStop {
            offset: 1.,
            rgba: [4, 5, 6, 255],
        },
    ]
}
fn resolved(template: &GradientTemplateId) -> ResolvedPaint {
    let mut paint = paint();
    paint.kind = PaintKind::Linear;
    paint.stops = TableRange { first: 0, count: 2 };
    ResolvedPaint {
        template: Some(template.clone()),
        paint,
        stops: stops().to_vec(),
    }
}
#[test]
fn complete_payload_and_borrowed_cache_reuse_across_instances() {
    let doc = document();
    let template = template(&doc, false, [100., 100.]);
    let mut b = scene(
        &doc,
        Limits {
            stops: 2,
            ..Limits::default()
        },
    );
    let root = b.root_id();
    let mut resolved = resolved(&template);
    // A local subrange, not graph indices and not the complete cache vector.
    resolved.stops.insert(
        0,
        PaintStop {
            offset: 0.,
            rgba: [0; 4],
        },
    );
    resolved.paint.stops.first = 1;
    let original = resolved.paint;
    b.append_resolved_fill(root, &resolved, &mut Run).unwrap();
    for n in 0..40000 {
        let mut instance = original;
        instance.bounds[2] = n as f64 + 1.;
        b.append_template_stroke(root, &template, instance, &resolved.stops, &mut Run)
            .unwrap();
    }
    assert_eq!(resolved.paint, original);
    let view = b.scene(root).unwrap();
    assert_eq!(view.stops, stops());
    assert_eq!(view.fill_paints[0].stops, TableRange { first: 0, count: 2 });
    assert_eq!(view.stroke_paints.len(), 40000);
    assert_eq!(view.stroke_paints[39999].bounds[2], 40000.);
    assert!(view
        .stroke_paints
        .iter()
        .all(|p| p.stops == view.fill_paints[0].stops));
    b.freeze().unwrap();
}
#[test]
fn actual_definition_viewport_allocation_and_scene_distinction() {
    let doc = document();
    let a = template(&doc, false, [100., 100.]);
    let same = template(&doc, false, [100., 100.]);
    let viewport_id = template(&doc, false, [101., 100.]);
    let other_def = template(&doc, true, [100., 100.]);
    let other_doc = document();
    let other_source = template(&other_doc, false, [100., 100.]);
    assert!(a.same_template(&same));
    for other in [&viewport_id, &other_def, &other_source] {
        assert!(!a.same_template(other));
    }
    let mut b = scene(&doc, Limits::default());
    let root = b.root_id();
    let child = b.new_scene(viewport()).unwrap();
    let p = b
        .publish_template_stops(root, &a, &stops(), &mut Run)
        .unwrap();
    b.publish_template_stops(root, &viewport_id, &stops(), &mut Run)
        .unwrap();
    b.publish_template_stops(root, &other_def, &stops(), &mut Run)
        .unwrap();
    assert_eq!(b.scene(root).unwrap().stops.len(), 6);
    assert!(b
        .publish_template_stops(root, &other_source, &stops(), &mut Run)
        .is_err());
    assert!(b.published_stop_range(child, &p, &mut Run).is_err());
    b.publish_template_stops(child, &a, &stops(), &mut Run)
        .unwrap();
    assert_eq!(b.scene(child).unwrap().stops.len(), 2);
    let b2 = scene(&doc, Limits::default());
    assert!(b2.published_stop_range(b2.root_id(), &p, &mut Run).is_err());
    b.freeze().unwrap();
}
#[test]
fn ordinary_rollback_preserves_prefix_and_independent_child_rejects_stale_reuse() {
    let doc = document();
    let a = template(&doc, false, [100., 100.]);
    let later = template(&doc, false, [101., 100.]);
    let mut b = scene(&doc, Limits::default());
    let root = b.root_id();
    let prefix = b
        .publish_template_stops(root, &a, &stops(), &mut Run)
        .unwrap();
    b.append_stops(root, &stops(), &mut Run).unwrap();
    let mark = b.mark(root).unwrap();
    let stale = b
        .publish_template_stops(root, &later, &stops(), &mut Run)
        .unwrap();
    let child = b.new_scene(viewport()).unwrap();
    let cp = b
        .publish_template_stops(child, &later, &stops(), &mut Run)
        .unwrap();
    b.rollback(mark).unwrap();
    assert_eq!(
        b.published_stop_range(root, &prefix, &mut Run)
            .unwrap()
            .first,
        0
    );
    assert_eq!(
        b.published_stop_range(child, &cp, &mut Run).unwrap().count,
        2
    );
    assert!(b.published_stop_range(root, &stale, &mut Run).is_err());
    let mut changed = stops();
    changed[0].rgba = [99; 4];
    let fresh = b
        .publish_template_stops(root, &later, &changed, &mut Run)
        .unwrap();
    assert_eq!(
        b.published_stop_range(root, &fresh, &mut Run)
            .unwrap()
            .first,
        4
    );
    assert!(b.published_stop_range(root, &stale, &mut Run).is_err());
    b.publish_template_stops(root, &a, &stops(), &mut Run)
        .unwrap();
    assert_eq!(b.scene(root).unwrap().stops.len(), 6);
    b.freeze().unwrap();
}
#[test]
fn nested_commit_then_parent_abort_drops_descendants_and_metadata() {
    let doc = document();
    let a = template(&doc, false, [100., 100.]);
    let mut b = scene(&doc, Limits::default());
    let root = b.root_id();
    let prefix = b
        .publish_template_stops(root, &a, &stops(), &mut Run)
        .unwrap();
    let parent = b.begin_draft(root).unwrap();
    let child = b.new_scene(viewport()).unwrap();
    let nested = b.begin_draft(child).unwrap();
    let cp = b
        .publish_template_stops(child, &a, &stops(), &mut Run)
        .unwrap();
    assert!(b
        .publish_template_stops(root, &a, &stops(), &mut Run)
        .is_err());
    let grandchild = b.new_scene(viewport()).unwrap();
    b.commit_draft(&nested, &mut Run).unwrap();
    b.publish_template_stops(root, &a, &stops(), &mut Run)
        .unwrap();
    let before = b.memory_stats();
    b.rollback_draft(parent).unwrap();
    assert!(b.scene(child).is_err());
    assert!(b.scene(grandchild).is_err());
    let reused = b.new_scene(viewport()).unwrap();
    assert_ne!(child, reused);
    assert!(b.published_stop_range(reused, &cp, &mut Run).is_err());
    b.publish_template_stops(reused, &a, &stops(), &mut Run)
        .unwrap();
    assert_eq!(
        b.published_stop_range(root, &prefix, &mut Run)
            .unwrap()
            .first,
        0
    );
    assert!(b.memory_stats().total_allocated_bytes >= before.total_allocated_bytes);
    b.freeze().unwrap();
}
#[test]
fn sorted_out_of_order_keys_and_zero_stop_suffix_rollback() {
    let doc = document();
    let mut b = scene(&doc, Limits::default());
    let root = b.root_id();
    let high = template(&doc, false, [300., 100.]);
    let low = template(&doc, false, [100., 100.]);
    let retained = b
        .publish_template_stops(root, &high, &[], &mut Run)
        .unwrap();
    let mark = b.mark(root).unwrap();
    let removed = b.publish_template_stops(root, &low, &[], &mut Run).unwrap();
    b.rollback(mark).unwrap();
    assert!(b.published_stop_range(root, &retained, &mut Run).is_ok());
    assert!(b.published_stop_range(root, &removed, &mut Run).is_err());
    b.publish_template_stops(root, &low, &stops(), &mut Run)
        .unwrap();
    assert!(b.published_stop_range(root, &removed, &mut Run).is_err());
    b.freeze().unwrap();
}
#[test]
fn same_key_conflict_rejected_without_corrupting_original() {
    let doc = document();
    let a = template(&doc, false, [100., 100.]);
    let mut b = scene(&doc, Limits::default());
    let root = b.root_id();
    let p = b
        .publish_template_stops(root, &a, &stops(), &mut Run)
        .unwrap();
    let before = b.memory_stats();
    let mut conflict = stops();
    conflict[1].rgba = [0; 4];
    assert!(b
        .publish_template_stops(root, &a, &conflict, &mut Run)
        .is_err());
    assert_eq!(b.memory_stats(), before);
    assert_eq!(b.scene(root).unwrap().stops, stops());
    assert_eq!(b.published_stop_range(root, &p, &mut Run).unwrap().count, 2);
    b.freeze().unwrap();
}
#[test]
fn cancellation_at_every_charge_preserves_tables_and_freeze_accounting() {
    let doc = document();
    let a = template(&doc, false, [100., 100.]);
    let mut failures = 0;
    let mut successes = 0;
    for budget in 0..150 {
        let mut b = scene(&doc, Limits::default());
        let root = b.root_id();
        let r = b.append_resolved_fill(root, &resolved(&a), &mut StopAfter(budget));
        if let Err(e) = r {
            assert_eq!(e.kind, ErrorKind::Canceled);
            failures += 1;
            assert!(b.scene(root).unwrap().stops.is_empty());
            assert!(b.scene(root).unwrap().fill_paints.is_empty());
            b.append_resolved_fill(root, &resolved(&a), &mut Run)
                .unwrap();
        } else {
            successes += 1;
        }
        assert_eq!(b.scene(root).unwrap().stops.len(), 2);
        b.freeze().unwrap();
    }
    assert!(failures > 0 && successes > 0);
}
#[test]
fn cancellation_at_checkpoints_including_after_publication_is_atomic() {
    struct Checkpoints(usize);
    impl WorkControl for Checkpoints {
        fn charge(&mut self, _: WorkKind, _: u64) -> std::result::Result<(), Stop> {
            Ok(())
        }
        fn checkpoint(&mut self) -> std::result::Result<(), Stop> {
            if self.0 == 0 {
                Err(Stop::Canceled)
            } else {
                self.0 -= 1;
                Ok(())
            }
        }
    }
    let doc = document();
    let a = template(&doc, false, [100., 100.]);
    for budget in 0..12 {
        let mut b = scene(&doc, Limits::default());
        let root = b.root_id();
        if let Err(e) = b.append_resolved_stroke(root, &resolved(&a), &mut Checkpoints(budget)) {
            assert_eq!(e.kind, ErrorKind::Canceled);
            assert!(b.scene(root).unwrap().stops.is_empty());
            assert!(b.scene(root).unwrap().stroke_paints.is_empty());
        }
        b.append_resolved_fill(root, &resolved(&a), &mut Run)
            .unwrap();
        assert_eq!(b.scene(root).unwrap().stops.len(), 2);
        b.freeze().unwrap();
    }
}
#[test]
fn invalid_sources_keys_payloads_and_local_ranges_never_publish() {
    let doc = document();
    let shared = DecodeSource::new(&doc, &mut Run).unwrap();
    for v in [[0., 1.], [-1., 1.], [f64::NAN, 1.], [1., f64::INFINITY]] {
        assert!(shared
            .gradient_template_id(definition(&doc, false), v, &mut Run)
            .is_err());
    }
    assert!(shared
        .gradient_template_id(ElementId(0), [1., 1.], &mut Run)
        .is_err());
    assert!(shared
        .gradient_template_id(ElementId(usize::MAX), [1., 1.], &mut Run)
        .is_err());
    let a = template(&doc, false, [100., 100.]);
    let radial = template(&doc, true, [100., 100.]);
    let mut b = scene(&doc, Limits::default());
    let root = b.root_id();
    let mut missing = resolved(&a);
    missing.template = None;
    assert!(b.append_resolved_fill(root, &missing, &mut Run).is_err());
    assert!(b
        .append_resolved_fill(root, &resolved(&radial), &mut Run)
        .is_err());
    let mut r = resolved(&a);
    r.paint.stops.first = usize::MAX;
    assert!(b.append_resolved_fill(root, &r, &mut Run).is_err());
    r = resolved(&a);
    r.paint.user_inverse.a = f64::NAN;
    assert!(b.append_resolved_fill(root, &r, &mut Run).is_err());
    for invalid in [
        vec![PaintStop {
            offset: -0.1,
            rgba: [0; 4],
        }],
        vec![PaintStop {
            offset: f64::NAN,
            rgba: [0; 4],
        }],
        stops().into_iter().rev().collect(),
        vec![stops()[0]; 257],
    ] {
        assert!(b
            .publish_template_stops(root, &a, &invalid, &mut Run)
            .is_err());
    }
    assert!(b.scene(root).unwrap().stops.is_empty());
    b.freeze().unwrap();
}
#[test]
fn full_reference_stop_and_gradient_limits_no_instance_budget_regression() {
    let doc = document();
    let shared = DecodeSource::new(&doc, &mut Run).unwrap();
    let mut b = scene(&doc, Limits::default());
    let root = b.root_id();
    for n in 0..32768 {
        let a = shared
            .gradient_template_id(definition(&doc, false), [n as f64 + 1., 100.], &mut Run)
            .unwrap();
        b.publish_template_stops(root, &a, &stops(), &mut Run)
            .unwrap();
    }
    assert_eq!(b.scene(root).unwrap().stops.len(), 65536);
    let first = shared
        .gradient_template_id(definition(&doc, false), [1., 100.], &mut Run)
        .unwrap();
    b.append_resolved_fill(root, &resolved(&first), &mut Run)
        .unwrap();
    let extra = shared
        .gradient_template_id(definition(&doc, false), [32769., 100.], &mut Run)
        .unwrap();
    assert_eq!(
        b.publish_template_stops(root, &extra, &stops(), &mut Run)
            .unwrap_err()
            .kind,
        ErrorKind::Limit(LimitKind::PaintVisits)
    );
    assert_eq!(
        b.append_stops(root, &stops(), &mut Run).unwrap_err().kind,
        ErrorKind::Limit(LimitKind::PaintStops)
    );
    b.freeze().unwrap();
}
#[test]
fn stop_byte_work_and_byte_budget_failure_are_transactional() {
    use vexel_base::{BudgetControl, WorkLimits};
    let doc = document();
    let a = template(&doc, false, [100., 100.]);
    let mut b = scene(&doc, Limits::default());
    let root = b.root_id();
    let cancel = std::sync::atomic::AtomicBool::new(false);
    let mut c = BudgetControl::with_limits(
        &cancel,
        u64::MAX,
        WorkLimits::unlimited().with_limit(WorkKind::SourceRead, 0),
    );
    assert!(b
        .publish_template_stops(root, &a, &stops(), &mut c)
        .is_err());
    assert!(b.scene(root).unwrap().stops.is_empty());
    let baseline = b.memory_stats().live_bytes;
    let mut restricted = scene(
        &doc,
        Limits {
            reserved_bytes: baseline,
            ..Limits::default()
        },
    );
    let root = restricted.root_id();
    let before = restricted.memory_stats();
    assert_eq!(
        restricted
            .publish_template_stops(root, &a, &stops(), &mut Run)
            .unwrap_err()
            .kind,
        ErrorKind::Limit(LimitKind::ReservedBytes)
    );
    assert_eq!(restricted.memory_stats(), before);
    restricted.freeze().unwrap();
    b.freeze().unwrap();
}
#[test]
fn source_is_retained_once_and_no_mutable_escape() {
    let doc = document();
    let a = template(&doc, false, [100., 100.]);
    let mut retained = doc.source_owner().clone();
    assert!(Arc::get_mut(&mut retained).is_none());
    let mut b = scene(&doc, Limits::default());
    let root = b.root_id();
    let before = b.memory_stats();
    for n in 0..20 {
        let key = template(&doc, false, [n as f64 + 1., 100.]);
        b.publish_template_stops(root, &key, &stops(), &mut Run)
            .unwrap();
    }
    assert_eq!(b.memory_stats().source_bytes, 4096);
    assert!(b.memory_stats().live_bytes > before.live_bytes);
    let graph = b.freeze().unwrap();
    assert_eq!(graph.memory_stats().source_bytes, 4096);
    drop(doc);
    drop(retained);
    assert!(a.same_template(&a.clone()));
    assert_eq!(graph.source(), SVG);
}

#[test]
fn logarithmic_lookup_reverse_insertion_and_cancelled_hit_keep_prefix() {
    struct Probes(usize);
    impl WorkControl for Probes {
        fn charge(&mut self, kind: WorkKind, units: u64) -> std::result::Result<(), Stop> {
            if kind == WorkKind::ReferenceProbe {
                self.0 += units as usize;
            }
            Ok(())
        }
        fn checkpoint(&mut self) -> std::result::Result<(), Stop> {
            Ok(())
        }
    }
    let doc = document();
    let shared = DecodeSource::new(&doc, &mut Run).unwrap();
    let mut b = scene(&doc, Limits::default());
    let root = b.root_id();
    for n in (0..1024).rev() {
        let a = shared
            .gradient_template_id(definition(&doc, false), [n as f64 + 1., 100.], &mut Run)
            .unwrap();
        b.publish_template_stops(root, &a, &stops(), &mut Run)
            .unwrap();
    }
    let middle = template(&doc, false, [513., 100.]);
    let mut probes = Probes(0);
    let p = b
        .publish_template_stops(root, &middle, &stops(), &mut probes)
        .unwrap();
    assert!(probes.0 <= 11);
    let prefix = b.published_stop_range(root, &p, &mut Run).unwrap();
    let before = b.memory_stats();
    assert!(b
        .publish_template_stops(root, &middle, &stops(), &mut StopAfter(4))
        .is_err());
    assert_eq!(b.memory_stats(), before);
    assert_eq!(b.published_stop_range(root, &p, &mut Run).unwrap(), prefix);
    assert_eq!(b.scene(root).unwrap().stops.len(), 2048);
    b.freeze().unwrap();
}
#[test]
fn nested_child_abort_reclaims_publications_parent_commit_and_freeze_cancel() {
    let doc = document();
    let a = template(&doc, false, [100., 100.]);
    let mut b = scene(&doc, Limits::default());
    let root = b.root_id();
    let parent = b.begin_draft(root).unwrap();
    let child = b.new_scene(viewport()).unwrap();
    let nested = b.begin_draft(child).unwrap();
    let old = b
        .publish_template_stops(child, &a, &stops(), &mut Run)
        .unwrap();
    let leaf = b.new_scene(viewport()).unwrap();
    b.rollback_draft(nested).unwrap();
    assert!(b.scene(leaf).is_err());
    assert!(b.published_stop_range(child, &old, &mut Run).is_err());
    assert!(b.scene(child).unwrap().stops.is_empty());
    assert!(b
        .publish_template_stops(child, &a, &stops(), &mut Run)
        .is_err());
    let retry = b.begin_draft(child).unwrap();
    let fresh = b
        .publish_template_stops(child, &a, &stops(), &mut Run)
        .unwrap();
    assert!(b.published_stop_range(child, &fresh, &mut Run).is_ok());
    assert!(b.published_stop_range(child, &old, &mut Run).is_err());
    b.commit_draft(&retry, &mut Run).unwrap();
    b.seal(child, &mut Run).unwrap();
    b.commit_draft(&parent, &mut Run).unwrap();
    b.freeze().unwrap();
    let mut b = scene(&doc, Limits::default());
    let root = b.root_id();
    b.publish_template_stops(root, &a, &[], &mut Run).unwrap();
    assert_eq!(
        b.freeze_controlled(&mut StopAfter(1)).unwrap_err().kind,
        ErrorKind::Canceled
    );
}
