//! Capacity journal proofs are exact lease accounting, not allocator/RSS claims.
#[path = "support/production.rs"]
mod production;
mod support;
use std::sync::Arc;
use support::Run;
use vexel_scene::*;
use vexel_svg_decode::{Decoder, DecoderState, TraversalDecodeEvent as Event};
#[test]
fn canonical_source_token_index_and_stack_capacity_are_retained_and_admitted() {
    let source = format!(
        "<svg width='8' height='8'><defs><g id='target'/></defs>{}<rect width='1' height='1'/>{}</svg>",
        "<g>".repeat(60),
        "</g>".repeat(60)
    );
    let mut c = Run::default();
    let owner = Arc::new(source.as_bytes().to_vec());
    let doc = LexicalDocument::parse(owner.clone(), 1 << 24, &mut c).unwrap();
    let mut shared = DecodeSource::new(&doc, &mut c).unwrap();
    let reference_bytes = shared.reference_storage_bytes();
    let mut d = Decoder::from_source(
        &doc,
        &mut shared,
        1,
        DecodeOptions {
            limits: Limits::default(),
            system_language: b"en",
            media: None,
        },
        &mut c,
    )
    .unwrap();
    let initial = d.lease_stats();
    assert_eq!(initial.source_bytes, owner.capacity());
    assert_eq!(
        initial.token_array_bytes,
        doc.token_storage_bytes().unwrap()
    );
    assert_eq!(initial.reference_index_bytes, reference_bytes);
    assert_eq!(initial.clip_copy_bytes, 0);
    while d.state() == DecoderState::Running {
        d.step(1, &mut support::RejectOperations, &mut c).unwrap();
        let stats = d.lease_stats();
        assert!(!stats.overflow);
        assert!(stats.peak_live_bytes >= stats.live_bytes);
        assert!(
            stats.live_bytes
                >= stats.source_bytes + stats.token_array_bytes + stats.reference_index_bytes
        );
    }
    let final_stats = d.lease_stats();
    assert!(final_stats.allocation_count > initial.allocation_count);
    let g = d.finish(&mut c).unwrap();
    assert!(Arc::ptr_eq(g.source_owner(), &owner));
    assert_eq!(g.root().shapes.len(), 1);
}
#[test]
fn actual_clip_temporary_capacity_has_no_live_leases_after_parent_completion() {
    let mut clip_copy_history = Vec::new();
    for count in [1, 16] {
        let source = format!(
            "<svg width=\"8\" height=\"8\"><defs><clipPath id=\"c\" clipPathUnits=\"objectBoundingBox\"><rect width=\"0.5\" height=\"1\"/></clipPath></defs><g clip-path=\"url(#c)\">{}</g></svg>",
            "<rect width=\"4\" height=\"4\" fill=\"red\"/>".repeat(count)
        );
        let mut c = Run::default();
        let doc =
            LexicalDocument::parse(Arc::new(source.as_bytes().to_vec()), 1 << 24, &mut c).unwrap();
        let mut d = Decoder::new(
            &doc,
            17,
            DecodeOptions {
                limits: Limits::default(),
                system_language: b"en",
                media: None,
            },
            &mut c,
        )
        .unwrap();
        let job =
            RasterJob::new(ChargeLedger::new(doc.source_owner(), 1 << 24, 65536).unwrap()).unwrap();
        let mut ops = production::ProductionFixture::default();
        loop {
            match d.step_production(1, &mut ops, &job, &mut c).unwrap() {
                Event::Complete => break,
                Event::Progress { charged_units } => assert!(charged_units > 0),
                _ => panic!("clip has no pattern render request"),
            }
        }
        let stats = d.lease_stats();
        let history = d.lease_allocation_history();
        assert_eq!(stats.clip_copy_bytes, 0);
        assert!(history[AllocationTag::ClipCopy as usize] > 0);
        assert!(history[AllocationTag::ClipCopy as usize] <= 512 * 1024);
        clip_copy_history.push(history[AllocationTag::ClipCopy as usize]);
        assert!(!stats.overflow);
        let graph = d.finish(&mut c).unwrap();
        assert_eq!(graph.root().clips.len(), 1);
        assert_eq!(graph.root().shapes.len(), count);
    }
    assert_eq!(clip_copy_history[0], clip_copy_history[1]);
}
