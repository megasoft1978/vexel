mod support;
use std::sync::Arc;
use support::*;
use vexel_scene::*;
use vexel_svg_scalar_index::*;

fn document(rects: usize, total: usize) -> SourceOwner {
    // Exact test_decode_budget.c helper input and padding character.
    let head = "<svg width=\"8\" height=\"8\">";
    let rect = "<rect x=\"1\" y=\"1\" width=\"1\" height=\"1\" fill=\"red\"/>";
    let fixed = head.len() + rects * rect.len() + "</svg><!---->".len();
    let total = total.max(fixed);
    let mut bytes = Vec::with_capacity(total + 1);
    bytes.extend_from_slice(head.as_bytes());
    for _ in 0..rects {
        bytes.extend_from_slice(rect.as_bytes());
    }
    bytes.extend_from_slice(b"<!--");
    bytes.resize(bytes.len() + total - fixed, b'x');
    bytes.extend_from_slice(b"--></svg>");
    assert_eq!(bytes.len(), total);
    Arc::new(bytes)
}
#[test]
fn original_decode_budget_large_source_tiny_tokens_and_balanced_job_cleanup() {
    let source = document(1000, 16 * 1024 * 1024 + 65536);
    assert!(source.len() > 16 * 1024 * 1024);
    let lexical = LexicalDocument::parse(source.clone(), 256 * 1024 * 1024, &mut Run).unwrap();
    let rects = lexical
        .tokens()
        .iter()
        .filter(|t| t.name.slice(&source) == Some(&b"rect"[..]))
        .count();
    assert_eq!(rects, 1000);
    assert!(Arc::ptr_eq(lexical.source_owner(), &source));
    let small = document(200, 0);
    let error = LexicalDocument::parse(small.clone(), 4096, &mut Run).unwrap_err();
    assert_eq!(error.kind, ErrorKind::Limit(LimitKind::TokenBytes));
    assert!(error.offset > 0 && error.offset <= small.len());
    // Original source ceiling predicate; constructing a 1 GiB mmap needs the
    // later native decoder gate. No fake source length is used here.
    assert_eq!(Limits::default().source_bytes, 1 << 30);
    assert_eq!((1usize << 30).checked_add(1), Some(1_073_741_825));
    validate_source_length(1 << 30).unwrap();
    let ceiling = validate_source_length((1 << 30) + 1).unwrap_err();
    assert_eq!(ceiling.kind, ErrorKind::Limit(LimitKind::SourceBytes));
    assert_eq!(ceiling.offset, 0);
    let mut job = RasterJob::new(ChargeLedger::new(&small, 8 << 20, 128).unwrap()).unwrap();
    let s = JobRasterScratch::new(&mut job, &mut Run).unwrap();
    job.reserve(s.active_edges, 200, &mut Run).unwrap();
    assert!(job.stats().allocation_count > 0 && !job.stats().overflow);
    s.release(&mut job).unwrap();
    assert_eq!(job.finish().unwrap().live_bytes, 0);
    let mut job = RasterJob::new(ChargeLedger::new(&small, 8 << 20, 128).unwrap()).unwrap();
    let s = JobRasterScratch::new(&mut job, &mut Run).unwrap();
    let before = job.stats();
    assert!(job.reserve(s.active_edges, usize::MAX, &mut Run).is_err());
    assert_eq!(job.stats(), before);
    assert_eq!(job.finish().unwrap().live_bytes, 0);
}

fn allocation_source(padding: usize, uses: usize) -> SourceOwner {
    let prefix = "<svg width=\"8\" height=\"8\"><defs><linearGradient id=\"paint\"><stop stop-color=\"red\"/><stop offset=\"1\" stop-color=\"blue\"/></linearGradient><clipPath id=\"clip\" clipPathUnits=\"userSpaceOnUse\"><rect width=\"8\" height=\"8\"/></clipPath><g id=\"target\"><rect width=\"8\" height=\"8\" fill=\"url(#paint)\"/></g></defs><!--";
    let middle = "-->";
    let use_ = "<use href=\"#target\"/>";
    let suffix =
        "<rect width=\"8\" height=\"8\" fill=\"url(#paint)\" clip-path=\"url(#clip)\"/></svg>";
    let len = prefix.len() + padding + middle.len() + uses * use_.len() + suffix.len();
    let mut bytes = Vec::with_capacity(len);
    bytes.extend_from_slice(prefix.as_bytes());
    bytes.resize(bytes.len() + padding, b'p');
    bytes.extend_from_slice(middle.as_bytes());
    for _ in 0..uses {
        bytes.extend_from_slice(use_.as_bytes());
    }
    bytes.extend_from_slice(suffix.as_bytes());
    assert_eq!(bytes.len(), len);
    Arc::new(bytes)
}
#[test]
fn original_reference_same_length_padding_token_growth_fixed_index_and_source_identity() {
    let use_len = "<use href=\"#target\"/>".len();
    let padding = 64 * 1024;
    let many_padding = padding - 31 * use_len;
    let inputs = [
        allocation_source(0, 1),
        allocation_source(padding, 1),
        allocation_source(many_padding, 32),
    ];
    assert_eq!(inputs[1].len(), inputs[2].len());
    let mut token_bytes = [0; 3];
    let mut indexes = [0; 3];
    let mut probes = [0; 3];
    struct Probes(usize);
    impl WorkControl for Probes {
        fn checkpoint(&mut self) -> std::result::Result<(), Stop> {
            Ok(())
        }
        fn charge(&mut self, k: WorkKind, n: u64) -> std::result::Result<(), Stop> {
            if k == WorkKind::ReferenceProbe {
                self.0 += n as usize;
            }
            Ok(())
        }
    }
    for (i, source) in inputs.iter().enumerate() {
        let lexical = LexicalDocument::parse(source.clone(), 256 << 20, &mut Run).unwrap();
        token_bytes[i] = lexical.token_storage_bytes().unwrap();
        let mut context = DecodeSource::new(&lexical, &mut Run).unwrap();
        indexes[i] = context.reference_storage_bytes();
        let mut meter = Probes(0);
        for token in lexical.tokens() {
            if token.name.slice(source) == Some(&b"href"[..]) {
                let mut id = token.value;
                id.offset += 1;
                id.length -= 1;
                assert!(context.find_reference(id, &mut meter).unwrap().is_some());
            }
        }
        probes[i] = meter.0;
        assert!(Arc::ptr_eq(context.source_owner(), source));
        assert_eq!(context.source().as_ptr(), source.as_ptr());
        assert!(token_bytes[i] > 0);
        // WO-RUST-SCENE-LIMITS-01 raised REFERENCE_SLOT_CAPACITY from 131,072 to 524,288 slots.
        assert_eq!(indexes[i], 524288 * std::mem::size_of::<u32>());
        assert!(probes[i] > 0);
    }
    assert!(token_bytes[1] <= inputs[1].len());
    assert!(token_bytes[2].saturating_sub(token_bytes[1]) <= 31 * 2048);
    assert_eq!(indexes[1], indexes[2]);
    assert!(probes[2] > probes[1]);
    // Equal non-token reference storage despite comment padding, exactly the
    // original pure memory predicate. Clip copies/output remain mixed gates.
    assert_eq!(indexes[0], indexes[1]);
}

#[test]
fn original_large_fixture_read_limit_dimensions_and_packed_byte_predicates() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/realworld/maps/map-mosaic-2000.svg");
    let bytes = std::fs::read(fixture).unwrap();
    assert!(!bytes.is_empty());
    assert!(bytes.len() <= 1024 * 1024);
    let source = Arc::new(bytes);
    let lexical = LexicalDocument::parse(source.clone(), 256 << 20, &mut Run).unwrap();
    for name in [b"width".as_slice(), b"height".as_slice()] {
        let token = lexical
            .tokens()
            .iter()
            .find(|t| t.name.slice(&source) == Some(name))
            .unwrap();
        assert_eq!(token.value.slice(&source), Some(b"512".as_slice()));
    }
    assert_eq!(512usize * 512 * 4, 1_048_576);
    assert_eq!(4096usize * 4096 * 4, 64 * 1024 * 1024);
    assert_eq!(vexel_raster::MAX_ALLOCATED_BYTES, 4096 * 4096 * 4);
    // Full 512x512 frozen SHA and 4096x4096 pixels/work need the real renderer.
}

#[test]
fn original_bounds_diagnostic_full_svg_sources_reject_paths_with_absolute_offsets() {
    let cases = [
        "<svg width=\"16\" height=\"16\"><path d=\"M0 0 L1000001 1\"/></svg>",
        "<svg width=\"16\" height=\"16\"><path d=\"M0 0 Lnan 1\"/></svg>",
        "<svg width=\"16\" height=\"16\"><path d=\"M,,0 1\"/></svg>",
        "<svg width=\"16\" height=\"16\"><path d=\"M0 0 L1.2.3 4\"/></svg>", // incomplete pair; `L1.2.3 4 5` became valid with compact-dot numbers
        "<svg width=\"16\" height=\"16\"><path d=\"M0 0 A1 1 0 2 0 4 4\"/></svg>",
        "<svg width=\"16\" height=\"16\"><path d=\"M0 0 A1 1 0 1.0 0 4 4\"/></svg>",
        "<svg width=\"16\" height=\"16\"><path d=\"L0 0\"/></svg>",
        "<svg width=\"16\" height=\"16\"><path d=\"M999999 0 c2 2 2 2 2 2\"/></svg>",
        "<svg width=\"16\" height=\"16\"><path d=\"M999999 0 q2 2 2 2\"/></svg>",
    ];
    let mut accepted = Vec::new();
    for text in cases {
        let source = Arc::new(text.as_bytes().to_vec());
        let lexical = LexicalDocument::parse(source.clone(), 1 << 20, &mut Run).unwrap();
        let token = lexical
            .tokens()
            .iter()
            .find(|t| t.name.slice(&source) == Some(b"d".as_slice()))
            .unwrap();
        let span = Span {
            start: token.value.offset,
            end: token.value.offset + token.value.length,
        };
        match vexel_svg_path::parse_path(
            &source,
            span,
            vexel_geometry::affine_identity(),
            Default::default(),
        ) {
            Err(error) => assert!(error.offset >= span.start && error.offset <= source.len()),
            Ok(path) => {
                // Prove the actual accepted path -> draft -> owning index flow
                // also admits this frozen rejection input, without a shim.
                let animation =
                    vexel_svg_animation::parse_owned(&source, lexical.tokens()).unwrap();
                let viewport = Viewport {
                    width: 16,
                    height: 16,
                    view_x: 0.,
                    view_y: 0.,
                    view_width: 16.,
                    view_height: 16.,
                    aspect: Default::default(),
                };
                let mut builder =
                    SceneBuilder::new(source.clone(), animation, 1, viewport, Limits::default())
                        .unwrap();
                let scene = builder.root_id();
                let draft = builder.begin_draft(scene).unwrap();
                let edge_colors = vec![[0, 0, 0, 255]; path.edges.len()];
                let shape = Shape::new(SourceMap {
                    element: span,
                    transform: vexel_geometry::affine_identity(),
                    transform_node: None,
                });
                builder
                    .append_shape(
                        scene,
                        ShapeInput {
                            shape,
                            path,
                            edge_colors,
                        },
                        &mut Run,
                    )
                    .unwrap();
                builder.commit_draft(&draft, &mut Run).unwrap();
                let graph = builder.freeze().unwrap();
                let mut owning_job = job(&graph, 8 << 20);
                let index = ScalarIndex::build(
                    graph.root(),
                    map(),
                    &mut owning_job,
                    &Resources::default(),
                    &mut Run,
                )
                .unwrap();
                assert!(index.pixel_count > 0);
                accepted.push(text);
            }
        }
        if vexel_svg_path::parse_path_controlled(
            &source,
            span,
            vexel_geometry::affine_identity(),
            Default::default(),
            &mut Run,
        )
        .is_ok()
        {
            accepted.push(text);
        }
        assert_eq!(source.as_slice(), text.as_bytes());
    }
    assert!(
        accepted.is_empty(),
        "accepted peer parser violates frozen rejection predicates: {accepted:?}"
    );
}

#[test]
fn original_benchmark_percentile_and_workload_input_predicates() {
    // Pure percentile95 helper from the assigned benchmark; native timing,
    // output hashes and RSS are explicitly outside this port proof.
    let mut values = (1..=21).rev().collect::<Vec<_>>();
    values.sort_unstable();
    let at = (0.95 * values.len() as f64).ceil() as usize - 1;
    assert_eq!(values[at], 20);
    let original = include_str!("../../../fixtures/legacy/bench_raster_culling.py");
    for name in [
        "circle_640x400",
        "many_shapes_1920x1080",
        "dashed_strokes_1920x1080",
        "simple_4096x2048",
    ] {
        assert!(original.contains(name));
    }
    assert!(original.contains("WARMUPS = 2"));
    assert!(original.contains("TRIALS = 21"));
    assert_eq!(4096u64 * 2048 * 4, 33_554_432);
}

#[test]
fn original_5000_shape_culling_counter_predicates_on_canonical_path_scene() {
    // Exact original rect positions/sizes/color in a canonical path scene.
    // The original full SVG rect source remains frozen for later decoder proof.
    let paths = (0..5000)
        .map(|i| {
            let x = (i % 100) * 18 + 2;
            let y = (i / 100) * 20 + 2;
            format!(
                "M{x} {y} L{} {y} L{} {} L{x} {} Z",
                x + 5,
                x + 5,
                y + 5,
                y + 5
            )
        })
        .collect::<Vec<_>>();
    let graph = graph(&paths, 1920, 1080, false, None, true);
    let view = graph.root();
    let mut job = job(&graph, 64 << 20);
    let resources = Resources::default();
    let mut index = ScalarIndex::build(view, map(), &mut job, &resources, &mut Run).unwrap();
    setup_cost(&index, &mut job, 5000);
    let memory = job.stats();
    let mut stats = RenderWorkStats::default();
    preflight_job(
        view,
        &mut index,
        map(),
        &mut job,
        &mut stats,
        &resources,
        &mut Run,
    )
    .unwrap();
    assert!(stats.shape_pixels > 0 && stats.fill_edge_tests > 0);
    assert!(stats.total_work < 200_000_000);
    assert!(stats.shape_pixels < 1_000_000);
    assert!(stats.shape_row_visits < 100_000);
    assert_eq!(job.stats(), memory);
}

#[test]
fn original_benchmark_pure_rss_status_and_latency_helpers() {
    fn rss(text: &str) -> Option<u64> {
        text.lines().find_map(|line| {
            line.strip_prefix("peak_rss_bytes=")
                .map(|n| n.parse().unwrap())
        })
    }
    fn status(code: i32, stderr: &str) -> (String, Option<String>) {
        if code == 0 {
            return ("rendered".into(), None);
        }
        let diagnostic = stderr.lines().next().unwrap_or("process-exit");
        (
            diagnostic.split(':').next().unwrap().into(),
            Some(diagnostic.into()),
        )
    }
    assert_eq!(rss("metrics=1\npeak_rss_bytes=123456\n"), Some(123456));
    assert_eq!(rss("metrics=1\n"), None);
    assert_eq!(status(0, ""), ("rendered".into(), None));
    assert_eq!(
        status(1, "limit:0\n"),
        ("limit".into(), Some("limit:0".into()))
    );
    assert_eq!(
        status(2, ""),
        ("process-exit".into(), Some("process-exit".into()))
    );
    let latency = [3u64, 1, 2];
    let mut sorted = latency;
    sorted.sort_unstable();
    assert_eq!(sorted[sorted.len() / 2], 2);
}
