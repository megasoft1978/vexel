use super::*;
use crate::builder::NoStop;
use crate::memory::{bytes, reserve};
fn builder() -> (SceneBuilder, LexicalDocument) {
    let source = Arc::new(b"<svg><linearGradient/></svg>".to_vec());
    let doc = LexicalDocument::parse(source.clone(), 1 << 20, &mut NoStop).unwrap();
    let animation = vexel_svg_animation::parse_owned(&source, doc.tokens()).unwrap();
    let viewport = Viewport {
        width: 100,
        height: 100,
        view_x: 0.,
        view_y: 0.,
        view_width: 100.,
        view_height: 100.,
        aspect: Default::default(),
    };
    (
        SceneBuilder::new(source, animation, 1, viewport, Limits::default()).unwrap(),
        doc,
    )
}
fn key(doc: &LexicalDocument, n: usize) -> GradientTemplateId {
    let shared = DecodeSource::new(doc, &mut NoStop).unwrap();
    shared
        .gradient_template_id(ElementId(1), [n as f64 + 1., 100.], &mut NoStop)
        .unwrap()
}
fn measured(b: &SceneBuilder) -> usize {
    let mut result = b.animation.storage_bytes().unwrap() + bytes(&b.scenes).unwrap();
    for scene in &b.scenes {
        result += scene.storage_bytes().unwrap();
    }
    result
}
#[test]
fn actual_metadata_capacity_overlap_history_and_retained_failed_reserve() {
    let (mut b, doc) = builder();
    let root = b.root_id();
    let initial = b.memory_stats();
    // Admit only a first metadata reserve, then fail the stop reserve. Actual
    // unused metadata capacity/history must remain measurable with zero records.
    b.limits.reserved_bytes = initial.live_bytes + 8 * std::mem::size_of::<Publication>();
    let stops = [
        PaintStop {
            offset: 0.,
            rgba: [0; 4],
        },
        PaintStop {
            offset: 1.,
            rgba: [255; 4],
        },
    ];
    assert_eq!(
        b.publish_template_stops(root, &key(&doc, 0), &stops, &mut NoStop)
            .unwrap_err()
            .kind,
        ErrorKind::Limit(LimitKind::ReservedBytes)
    );
    assert!(b.scenes[0].publications.is_empty());
    assert!(b.scenes[0].stops.is_empty());
    assert!(b.scenes[0].publications.capacity() >= 1);
    assert_eq!(b.stats.live_bytes, measured(&b));
    assert_eq!(
        b.stats.live_bytes - initial.live_bytes,
        bytes(&b.scenes[0].publications).unwrap()
    );
    assert_eq!(
        b.stats.total_allocated_bytes - initial.total_allocated_bytes,
        bytes(&b.scenes[0].publications).unwrap()
    );
    b.limits.reserved_bytes = usize::MAX;
    let prefix = b
        .publish_template_stops(root, &key(&doc, 0), &stops, &mut NoStop)
        .unwrap();
    let mark = b.mark(root).unwrap();
    let mut growths = 0;
    for n in 1..20 {
        let before = b.stats;
        let capacity =
            bytes(&b.scenes[0].publications).unwrap() + bytes(&b.scenes[0].stops).unwrap();
        b.publish_template_stops(root, &key(&doc, n), &stops, &mut NoStop)
            .unwrap();
        assert_eq!(b.stats.live_bytes, measured(&b));
        let newcapacity =
            bytes(&b.scenes[0].publications).unwrap() + bytes(&b.scenes[0].stops).unwrap();
        assert_eq!(
            b.stats.live_bytes - before.live_bytes,
            newcapacity - capacity
        );
        if newcapacity > capacity {
            growths += 1;
            assert!(b.stats.peak_live_bytes > b.stats.live_bytes); // old/new overlap
            assert!(
                b.stats.total_allocated_bytes - before.total_allocated_bytes
                    > newcapacity - capacity
            );
        }
    }
    assert!(growths > 1);
    let retained = b.stats;
    b.rollback(mark).unwrap();
    assert_eq!(b.scenes[0].publications.len(), 1);
    assert_eq!(b.stats.live_bytes, retained.live_bytes);
    assert_eq!(
        b.stats.total_allocated_bytes,
        retained.total_allocated_bytes
    );
    assert_eq!(
        b.published_stop_range(root, &prefix, &mut NoStop)
            .unwrap()
            .count,
        2
    );
    assert_eq!(b.stats.live_bytes, measured(&b));
    b.freeze().unwrap();
}
#[test]
fn real_fallible_reserve_capacity_failure_preserves_existing_buffer() {
    // Vec::try_reserve_exact returns CapacityOverflow (mapped to OOM), without
    // an allocation attempt of impossible size. Exercise the actual failure
    // boundary rather than a synthetic allocator/provider or success stub.
    let mut values = vec![42u8];
    let before_capacity = values.capacity();
    let error = reserve(&mut values, isize::MAX as usize, usize::MAX, usize::MAX).unwrap_err();
    assert_eq!(error.kind, ErrorKind::OutOfMemory);
    assert_eq!(values, [42]);
    assert_eq!(values.capacity(), before_capacity);
}
