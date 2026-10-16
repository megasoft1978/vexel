//! Accepted observability hooks observed after real Decoder traversal.
#[path = "support/production.rs"]
mod production;
mod support;

use std::sync::Arc;
use support::Run;
use vexel_scene::*;
use vexel_svg_decode::{Decoder, TraversalDecodeEvent};

struct Decoded {
    graph: SceneGraph,
    history: [usize; 5],
    stats: RenderMemoryStats,
    probes: usize,
    index_bytes: usize,
}

fn decode_with_observability(source: &str) -> Decoded {
    let owner = Arc::new(source.as_bytes().to_vec());
    let mut control = Run::default();
    let document = LexicalDocument::parse(owner.clone(), 1 << 24, &mut control).unwrap();
    let mut shared = DecodeSource::new(&document, &mut control).unwrap();
    let index_bytes = shared.reference_storage_bytes();
    let job = RasterJob::new(ChargeLedger::new(&owner, 1 << 24, 65536).unwrap()).unwrap();
    let mut decoder = Decoder::from_source(
        &document,
        &mut shared,
        1,
        DecodeOptions {
            limits: Limits::default(),
            system_language: b"en",
            media: None,
        },
        &mut control,
    )
    .unwrap();
    let mut operations = production::ProductionFixture::default();
    loop {
        match decoder
            .step_production(64, &mut operations, &job, &mut control)
            .unwrap()
        {
            TraversalDecodeEvent::Complete => break,
            TraversalDecodeEvent::Progress { charged_units } => assert!(charged_units > 0),
            TraversalDecodeEvent::NeedPattern(_) => panic!("unexpected pattern request"),
        }
    }
    let history = decoder.lease_allocation_history();
    let stats = decoder.lease_stats();
    let graph = decoder.finish(&mut control).unwrap();
    let probes = shared.reference_lookup_probes();
    assert!(Arc::ptr_eq(graph.source_owner(), &owner));
    Decoded {
        graph,
        history,
        stats,
        probes,
        index_bytes,
    }
}

fn allocation_source(comment_padding: usize, uses: usize) -> String {
    let mut source = String::from(
        "<svg width='8' height='8'><defs><g id='target'><rect width='1' height='1'/></g></defs><!--",
    );
    source.push_str(&"p".repeat(comment_padding));
    source.push_str("-->");
    for _ in 0..uses {
        source.push_str("<use href='#target'/>");
    }
    source.push_str("</svg>");
    source
}

fn original_memory_source(comment_padding: usize, uses: usize) -> String {
    let prefix = "<svg width=\"8\" height=\"8\"><defs><linearGradient id=\"paint\"><stop stop-color=\"red\"/><stop offset=\"1\" stop-color=\"blue\"/></linearGradient><clipPath id=\"clip\" clipPathUnits=\"userSpaceOnUse\"><rect width=\"8\" height=\"8\"/></clipPath><g id=\"target\"><rect width=\"8\" height=\"8\" fill=\"url(#paint)\"/></g></defs><!--";
    let suffix =
        "--><rect width=\"8\" height=\"8\" fill=\"url(#paint)\" clip-path=\"url(#clip)\"/></svg>";
    let use_bytes = b"<use href=\"#target\"/>";
    let mut source = String::with_capacity(
        prefix
            .len()
            .checked_add(comment_padding)
            .and_then(|n| n.checked_add(suffix.len()))
            .and_then(|n| n.checked_add(uses.checked_mul(use_bytes.len())?))
            .expect("bounded oracle fixture size"),
    );
    source.push_str(prefix);
    source.push_str(&"p".repeat(comment_padding));
    source.push_str("-->");
    for _ in 0..uses {
        source.push_str(std::str::from_utf8(use_bytes).unwrap());
    }
    source.push_str(&suffix[3..]);
    source
}

#[test]
fn original_three_input_source_token_and_reference_accounting_deltas() {
    let many_uses = 32usize;
    let use_len = b"<use href='#target'/>".len();
    let padding = 64 * 1024;
    let padding_many = padding - (many_uses - 1) * use_len;
    let inputs = [
        allocation_source(0, 1),
        allocation_source(padding, 1),
        allocation_source(padding_many, many_uses),
    ];
    assert_eq!(inputs[1].len(), inputs[2].len());
    let decoded: Vec<_> = inputs
        .iter()
        .map(|source| decode_with_observability(source))
        .collect();
    let [small, padded_one, padded_many] = decoded.as_slice() else {
        unreachable!()
    };
    assert_eq!(small.graph.root().shapes.len(), 1);
    assert_eq!(padded_one.graph.root().shapes.len(), 1);
    assert_eq!(padded_many.graph.root().shapes.len(), many_uses);
    assert_eq!(padded_many.graph.root().edges.len(), many_uses * 4);
    assert!(
        padded_many
            .graph
            .root()
            .shapes
            .iter()
            .all(|shape| shape.edges.count == 4)
    );

    for (input, item) in inputs.iter().zip(&decoded) {
        assert_eq!(item.graph.source(), input.as_bytes());
        assert_eq!(item.stats.source_bytes, input.len());
        assert!(item.stats.token_array_bytes > 0);
        assert_eq!(
            item.stats.reference_index_bytes, item.index_bytes,
            "reference index lease must match actual DecodeSource capacity"
        );
        assert!(!item.stats.overflow);
    }

    let small_without_tokens = small.stats.total_allocated_bytes - small.stats.token_array_bytes;
    let padded_without_tokens =
        padded_one.stats.total_allocated_bytes - padded_one.stats.token_array_bytes;
    assert_eq!(small_without_tokens, padded_without_tokens);
    assert_eq!(
        padded_one.stats.reference_index_bytes,
        padded_many.stats.reference_index_bytes
    );
    let token_growth = padded_many
        .stats
        .token_array_bytes
        .saturating_sub(padded_one.stats.token_array_bytes);
    let use_growth_budget = (many_uses - 1) * 2048;
    assert!(token_growth <= use_growth_budget);
    assert!(padded_many.stats.total_allocated_bytes >= padded_one.stats.total_allocated_bytes);
    assert!(padded_many.stats.peak_live_bytes >= padded_one.stats.peak_live_bytes);
    assert!(
        padded_many.stats.peak_live_bytes - padded_one.stats.peak_live_bytes <= use_growth_budget
    );
    assert!(padded_many.probes > padded_one.probes);
}

#[test]
fn original_reference_memory_decoder_owned_predicates_include_actual_clip_copy_history() {
    let many_uses = 32usize;
    let use_len = b"<use href=\"#target\"/>".len();
    let padding = 64 * 1024;
    let padding_many = padding - (many_uses - 1) * use_len;
    let inputs = [
        original_memory_source(0, 1),
        original_memory_source(padding, 1),
        original_memory_source(padding_many, many_uses),
    ];
    assert_eq!(inputs[1].len(), inputs[2].len());
    let decoded: Vec<_> = inputs
        .iter()
        .map(|source| decode_with_observability(source))
        .collect();
    let [small, padded_one, padded_many] = decoded.as_slice() else {
        unreachable!()
    };
    assert_eq!(small.graph.root().shapes.len(), 2);
    assert_eq!(padded_one.graph.root().shapes.len(), 2);
    assert_eq!(padded_many.graph.root().shapes.len(), many_uses + 1);
    assert_eq!(small.graph.root().clips.len(), 1);
    assert_eq!(padded_one.graph.root().clips.len(), 1);
    assert_eq!(padded_many.graph.root().clips.len(), 1);

    for (input, item) in inputs.iter().zip(&decoded) {
        assert_eq!(item.graph.source(), input.as_bytes());
        assert_eq!(item.stats.source_bytes, input.len());
        assert!(item.stats.peak_live_bytes > 0);
        assert!(item.stats.allocation_count > 0);
        // ChargeLedger reports source capacity separately from cumulative
        // admitted payloads; include both when comparing to peak live bytes.
        let admitted_plus_source = item
            .stats
            .total_allocated_bytes
            .checked_add(item.stats.source_bytes)
            .expect("bounded oracle fixture accounting");
        assert!(admitted_plus_source >= item.stats.peak_live_bytes);
        assert!(item.stats.token_array_bytes > 0);
        assert!(item.history[AllocationTag::ClipCopy as usize] > 0);
        assert!(item.history[AllocationTag::ClipCopy as usize] <= 512 * 1024);
        assert!(item.index_bytes > 0);
        assert_eq!(item.stats.reference_index_bytes, item.index_bytes);
        assert!(item.probes > 0);
        assert!(!item.stats.overflow);
        assert_eq!(
            item.graph.root().shapes.len(),
            item.graph.root().edges.len() / 4
        );
    }

    let small_non_token = small.stats.total_allocated_bytes - small.stats.token_array_bytes;
    let padded_non_token =
        padded_one.stats.total_allocated_bytes - padded_one.stats.token_array_bytes;
    assert_eq!(small_non_token, padded_non_token);
    let use_growth = (many_uses - 1) * 2048;
    let token_growth = padded_many
        .stats
        .token_array_bytes
        .saturating_sub(padded_one.stats.token_array_bytes);
    assert!(token_growth <= use_growth);
    assert_eq!(
        padded_many.history[AllocationTag::ClipCopy as usize],
        padded_one.history[AllocationTag::ClipCopy as usize],
        "one clipped output is copied once regardless of unrelated use expansion"
    );
    assert!(padded_many.stats.peak_live_bytes >= padded_one.stats.peak_live_bytes);
    assert!(padded_many.stats.total_allocated_bytes >= padded_one.stats.total_allocated_bytes);
    assert!(padded_many.stats.peak_live_bytes - padded_one.stats.peak_live_bytes <= use_growth);
    assert_eq!(
        padded_one.stats.reference_index_bytes,
        padded_many.stats.reference_index_bytes
    );
    assert!(padded_many.probes > padded_one.probes);
}

#[test]
fn reference_probes_and_index_capacity_come_from_decoder_traversal() {
    let source = format!(
        "<svg width=\"16\" height=\"16\">{}{}</svg>",
        "<rect width=\"1\" height=\"1\" fill=\"url(#late)\"/>".repeat(512),
        "<defs><linearGradient id=\"late\"><stop stop-color=\"red\"/><stop offset=\"1\" stop-color=\"blue\"/></linearGradient></defs>"
    );
    let decoded = decode_with_observability(&source);
    assert_eq!(decoded.graph.root().shapes.len(), 512);
    assert!((512..=1024).contains(&decoded.probes));
    assert_eq!(
        decoded.history[AllocationTag::ReferenceIndex as usize],
        decoded.index_bytes
    );
    assert_eq!(decoded.graph.source(), source.as_bytes());
}

#[test]
fn opacity_group_range_and_context_are_published() {
    let source = "<svg width='8' height='8'><g opacity='0.5'><rect width='2' height='2'/><rect x='2' width='2' height='2'/></g></svg>";
    let graph = support::decode(source).unwrap();
    let root = graph.root();
    assert_eq!(root.shapes.len(), 2);
    assert_eq!(root.groups.len(), 1);
    let outer = root
        .groups
        .iter()
        .find(|group| group.parent.is_none())
        .unwrap();
    assert_eq!(outer.shapes, TableRange { first: 0, count: 2 });
    assert_eq!(outer.opacity, 0.5);
    assert_eq!(outer.blend_mode, 0);
    assert!(!outer.isolation);
}
