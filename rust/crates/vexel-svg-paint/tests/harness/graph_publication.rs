#![forbid(unsafe_code)]
//! Standalone scoped proof using already-built accepted workspace dependencies.
//! Animation parsing is only fixture construction for the real SceneBuilder.
use std::sync::Arc;
use vexel_geometry::affine_identity;
use vexel_scene::*;
use vexel_svg_paint::resolve_cached_fill;
use vexel_xml::XmlSpan;

const SOURCE: &[u8] = b"<svg><linearGradient id='g'><stop stop-color='red'/><stop offset='1' stop-color='blue'/></linearGradient><rect fill='url(#g)'/></svg>";
struct Run;
impl WorkControl for Run {
    fn charge(&mut self, _: WorkKind, _: u64) -> std::result::Result<(), Stop> {
        Ok(())
    }
    fn checkpoint(&mut self) -> std::result::Result<(), Stop> {
        Ok(())
    }
}
struct Polls {
    remaining: usize,
    events: usize,
}
impl Polls {
    fn poll(&mut self) -> std::result::Result<(), Stop> {
        self.events += 1;
        if self.remaining == 0 {
            Err(Stop::Canceled)
        } else {
            self.remaining -= 1;
            Ok(())
        }
    }
}
impl WorkControl for Polls {
    fn charge(&mut self, _: WorkKind, _: u64) -> std::result::Result<(), Stop> {
        self.poll()
    }
    fn checkpoint(&mut self) -> std::result::Result<(), Stop> {
        self.poll()
    }
}
fn viewport() -> Viewport {
    Viewport {
        width: 10,
        height: 10,
        view_x: 0.,
        view_y: 0.,
        view_width: 10.,
        view_height: 10.,
        aspect: Default::default(),
    }
}
fn builder(doc: &LexicalDocument) -> SceneBuilder {
    let source = doc.source_owner().clone();
    let animation = vexel_svg_animation::parse_owned(&source, doc.tokens()).unwrap();
    SceneBuilder::new(source, animation, 1, viewport(), Limits::default()).unwrap()
}
fn fill() -> XmlSpan {
    XmlSpan {
        offset: SOURCE.windows(7).position(|v| v == b"url(#g)").unwrap(),
        length: 7,
    }
}
fn edges() -> [Edge; 1] {
    [Edge {
        a: Point { x: 0., y: 0. },
        b: Point { x: 10., y: 10. },
    }]
}

#[test]
fn real_40000_instance_fill_stroke_handoff_shares_two_graph_stops() {
    let source = Arc::new(SOURCE.to_vec());
    let doc = LexicalDocument::parse(source.clone(), 1 << 20, &mut Run).unwrap();
    let mut shared = DecodeSource::new(&doc, &mut Run).unwrap();
    let mut cache = PaintCache::default();
    let mut ledger = ChargeLedger::new(&source, 1 << 24, 64).unwrap();
    let mut graph = builder(&doc);
    let root = graph.root_id();
    let pointer;
    {
        let cached = resolve_cached_fill(
            &mut shared,
            &mut cache,
            &mut ledger,
            fill(),
            &edges(),
            affine_identity(),
            [10., 10.],
            &mut Run,
        )
        .unwrap();
        pointer = cached.stops.as_ptr();
    }
    let before = ledger.stats();
    for index in 0..40_000 {
        let inverse = Affine {
            a: 0.5,
            d: 0.5,
            e: (index % 2) as f64,
            ..affine_identity()
        };
        let cached = resolve_cached_fill(
            &mut shared,
            &mut cache,
            &mut ledger,
            fill(),
            &edges(),
            inverse,
            [10., 10.],
            &mut Run,
        )
        .unwrap();
        assert_eq!(cached.stops.as_ptr(), pointer);
        if index % 2 == 0 {
            cached.append_fill(&mut graph, root, &mut Run).unwrap();
        } else {
            cached.append_stroke(&mut graph, root, &mut Run).unwrap();
        }
    }
    assert_eq!(ledger.stats(), before);
    let scene = graph.scene(root).unwrap();
    assert_eq!(scene.stops.len(), 2);
    assert_eq!(scene.fill_paints.len(), 20_000);
    assert_eq!(scene.stroke_paints.len(), 20_000);
    assert!(
        scene
            .fill_paints
            .iter()
            .chain(scene.stroke_paints)
            .all(|paint| paint.stops == TableRange { first: 0, count: 2 })
    );
    assert_eq!(cache.entries[0].stops.as_ptr(), pointer);
}

#[test]
fn cached_publication_survives_rollback_without_stale_graph_ranges_and_owned_payload_is_compatible()
{
    let source = Arc::new(SOURCE.to_vec());
    let doc = LexicalDocument::parse(source.clone(), 1 << 20, &mut Run).unwrap();
    let mut shared = DecodeSource::new(&doc, &mut Run).unwrap();
    let mut cache = PaintCache::default();
    let mut ledger = ChargeLedger::new(&source, 1 << 24, 64).unwrap();
    let cached = resolve_cached_fill(
        &mut shared,
        &mut cache,
        &mut ledger,
        fill(),
        &edges(),
        affine_identity(),
        [10., 10.],
        &mut Run,
    )
    .unwrap();
    let pointer = cached.stops.as_ptr();
    let mut graph = builder(&doc);
    let root = graph.root_id();
    let mark = graph.mark(root).unwrap();
    cached.append_fill(&mut graph, root, &mut Run).unwrap();
    let old = graph
        .publish_template_stops(root, &cached.template, cached.stops, &mut Run)
        .unwrap();
    graph.rollback(mark).unwrap();
    assert!(graph.published_stop_range(root, &old, &mut Run).is_err());
    let draft = graph.begin_draft(root).unwrap();
    cached.append_stroke(&mut graph, root, &mut Run).unwrap();
    graph.rollback_draft(draft).unwrap();
    assert!(graph.scene(root).unwrap().stops.is_empty());
    cached.append_stroke(&mut graph, root, &mut Run).unwrap();
    let owned = cached.to_owned(&mut ledger, &mut Run).unwrap();
    graph
        .append_resolved_fill(root, owned.as_resolved(), &mut Run)
        .unwrap();
    owned.release(&mut ledger).unwrap();
    assert_eq!(graph.scene(root).unwrap().stops.len(), 2);
    assert_eq!(graph.scene(root).unwrap().stroke_paints.len(), 1);
    assert_eq!(graph.scene(root).unwrap().fill_paints.len(), 1);
    let child = graph.new_scene(viewport()).unwrap();
    cached.append_fill(&mut graph, child, &mut Run).unwrap();
    assert_eq!(graph.scene(child).unwrap().stops.len(), 2);
    assert_eq!(graph.scene(root).unwrap().stops.len(), 2);
    assert_eq!(cached.stops.as_ptr(), pointer);
    let other_doc = LexicalDocument::parse(Arc::new(SOURCE.to_vec()), 1 << 20, &mut Run).unwrap();
    let mut wrong_graph = builder(&other_doc);
    let wrong_root = wrong_graph.root_id();
    assert_eq!(
        cached
            .append_fill(&mut wrong_graph, wrong_root, &mut Run)
            .unwrap_err()
            .kind,
        ErrorKind::InvalidArgument
    );
    assert!(wrong_graph.scene(wrong_root).unwrap().stops.is_empty());
}

#[test]
fn every_public_handoff_cancellation_position_cleans_logical_publication() {
    let source = Arc::new(SOURCE.to_vec());
    let doc = LexicalDocument::parse(source.clone(), 1 << 20, &mut Run).unwrap();
    let mut shared = DecodeSource::new(&doc, &mut Run).unwrap();
    let mut cache = PaintCache::default();
    let mut ledger = ChargeLedger::new(&source, 1 << 24, 64).unwrap();
    let cached = resolve_cached_fill(
        &mut shared,
        &mut cache,
        &mut ledger,
        fill(),
        &edges(),
        affine_identity(),
        [10., 10.],
        &mut Run,
    )
    .unwrap();
    let mut measure = builder(&doc);
    let root = measure.root_id();
    let mut polls = Polls {
        remaining: usize::MAX,
        events: 0,
    };
    cached.append_fill(&mut measure, root, &mut polls).unwrap();
    for at in 0..polls.events {
        let mut graph = builder(&doc);
        let root = graph.root_id();
        let mut cancel = Polls {
            remaining: at,
            events: 0,
        };
        assert_eq!(
            cached
                .append_fill(&mut graph, root, &mut cancel)
                .unwrap_err()
                .kind,
            ErrorKind::Canceled
        );
        assert!(graph.scene(root).unwrap().stops.is_empty());
        assert!(graph.scene(root).unwrap().fill_paints.is_empty());
        cached.append_stroke(&mut graph, root, &mut Run).unwrap();
        assert_eq!(graph.scene(root).unwrap().stops.len(), 2);
    }
}
