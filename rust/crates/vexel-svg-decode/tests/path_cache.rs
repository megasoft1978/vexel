//! The path cache returns exactly what a fresh parse would, including source
//! provenance rebased to the requesting element, and stays within its bound.
use std::sync::Mutex;

use vexel_base::Span;
use vexel_geometry::Affine;
use vexel_svg_decode::{element_geometry, path_cache::cached_bytes, source::attribute};
use vexel_xml::{XmlLimits, tokenize};

const D: &str = "M10 10 L 60.5 12 C70 20 80 30 90 40 q 3 4 5 6 z m 1 1 l 2 2";

/// `D` padded past the cache's minimum size.
fn long_d() -> String {
    format!("{D} {}", "l 1.5 2.5 ".repeat(60))
}

fn geometry_of(
    source: &str,
    t: Affine,
) -> (vexel_svg_path::PathDocument, vexel_svg_path::PathDocument) {
    let bytes = source.as_bytes();
    let (tokens, result) = tokenize(bytes, XmlLimits::default());
    let _ = result;
    let element = (0..tokens.len())
        .find(|&i| tokens[i].local_name.slice(bytes) == Some(b"path".as_slice()))
        .unwrap();
    let got = element_geometry(bytes, &tokens, element, t, [100.0, 100.0]).unwrap();
    let d = attribute(bytes, &tokens, element, b"d", None).unwrap();
    let span = Span {
        start: d.offset,
        end: d.end().unwrap(),
    };
    let fresh =
        vexel_svg_path::parse_path(bytes, span, t, vexel_svg_decode::path_options()).unwrap();
    (got.path, fresh)
}

#[test]
fn hit_is_identical_to_a_fresh_parse_at_a_shifted_offset() {
    let t = Affine {
        a: 1.25,
        b: 0.0,
        c: 0.0,
        d: 1.25,
        e: 3.0,
        f: -2.0,
    };
    let a = format!("<svg><path d=\"{}\"/></svg>", long_d());
    let b = format!(
        "<svg><g id=\"padding-moves-the-path\"/><path fill=\"red\" d=\"{}\"/></svg>",
        long_d()
    );
    let (miss, fresh_a) = geometry_of(&a, t);
    assert_eq!(miss, fresh_a);
    let before = cached_bytes();
    assert!(before > 0);
    let (hit, fresh_b) = geometry_of(&b, t);
    assert_eq!(hit, fresh_b, "rebased provenance must match a fresh parse");
    assert_ne!(hit.numbers[0].source, miss.numbers[0].source);
}

#[test]
fn transform_is_part_of_the_key() {
    let src = format!("<svg><path d=\"{} h 7\"/></svg>", long_d());
    let one = Affine {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: 1.0,
        e: 0.0,
        f: 0.0,
    };
    let two = Affine { a: 2.0, ..one };
    let (a, fa) = geometry_of(&src, one);
    let (b, fb) = geometry_of(&src, two);
    assert_eq!((a.clone(), b.clone()), (fa, fb));
    assert_ne!(a.edges, b.edges);
}

/// The cache is process-global: the two tests that size it run one at a time.
static GUARD: Mutex<()> = Mutex::new(());

#[test]
fn cache_stays_bounded() {
    let _guard = GUARD.lock().unwrap_or_else(|e| e.into_inner());
    let t = Affine {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: 1.0,
        e: 0.0,
        f: 0.0,
    };
    let before = cached_bytes();
    // About 32 KiB per entry: enough to pass the cap and force eviction.
    for i in 0..6000 {
        let d: String = (0..200).map(|k| format!("L{} {} ", i + k, k * 3)).collect();
        let src = format!("<svg><path d=\"M0 0 {d}\"/></svg>");
        geometry_of(&src, t);
    }
    assert!(cached_bytes() <= 128 * 1024 * 1024);
    // 6000 entries of at least 31 KiB would exceed this, so eviction ran.
    assert!(
        cached_bytes() < 6000 * 31 * 1024,
        "eviction must run (before {before}, after {})",
        cached_bytes()
    );
}

/// A dense shot re-parses about 1100 paths (31 MiB) per tick; a cache smaller
/// than that working set evicts each entry just before its reuse.
#[test]
fn cache_holds_a_cartoon_shot_working_set() {
    let _guard = GUARD.lock().unwrap_or_else(|e| e.into_inner());
    let t = Affine {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: 1.0,
        e: 0.0,
        f: 0.0,
    };
    let sources: Vec<String> = (0..1300)
        .map(|i| {
            let d: String = (0..200)
                .map(|k| format!("L{} {} ", i * 7 + k, k * 3 + 1))
                .collect();
            format!("<svg><path d=\"M0 0 {d}\"/></svg>")
        })
        .collect();
    for src in &sources {
        geometry_of(src, t);
    }
    let filled = cached_bytes();
    assert!(filled >= 40 * 1024 * 1024, "{filled}");
    // Second pass: every shot path must still be cached, so nothing is added
    // or evicted.
    for src in &sources {
        let (got, fresh) = geometry_of(src, t);
        assert_eq!(got, fresh);
    }
    assert_eq!(cached_bytes(), filled, "working set was evicted");
}

// ---- accounting: a hit is charged exactly like the parse it replaces ----

use vexel_base::{Stop, WorkControl, WorkKind};

/// Total-work meter with an optional cap and a cancel flag.
struct Meter {
    cap: u64,
    used: u64,
    checkpoints: u32,
    cancel: bool,
}

impl Meter {
    fn new(cap: u64) -> Self {
        Self {
            cap,
            used: 0,
            checkpoints: 0,
            cancel: false,
        }
    }
}

impl WorkControl for Meter {
    fn charge(&mut self, _: WorkKind, units: u64) -> Result<(), Stop> {
        self.used += units;
        if self.used > self.cap {
            Err(Stop::Limit)
        } else {
            Ok(())
        }
    }
    fn checkpoint(&mut self) -> Result<(), Stop> {
        self.checkpoints += 1;
        if self.cancel {
            Err(Stop::Canceled)
        } else {
            Ok(())
        }
    }
}

fn run(source: &str, meter: &mut Meter) -> Result<usize, String> {
    let bytes = source.as_bytes();
    let (tokens, _) = tokenize(bytes, XmlLimits::default());
    let element = (0..tokens.len())
        .find(|&i| tokens[i].local_name.slice(bytes) == Some(b"path".as_slice()))
        .unwrap();
    let t = Affine {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: 1.0,
        e: 0.0,
        f: 0.0,
    };
    vexel_svg_decode::element_geometry_controlled(bytes, &tokens, element, t, [100.0, 100.0], meter)
        .map(|g| g.path.edges.len())
        .map_err(|e| format!("{:?}", e.kind))
}

fn unique_path(tag: u32) -> String {
    let d: String = (0..90)
        .map(|k| format!("L{} {} ", tag + k, k * 3 + 1))
        .collect();
    format!("<svg><path d=\"M0 0 {d}\"/></svg>")
}

#[test]
fn hit_fails_like_a_fresh_parse_under_a_small_budget() {
    let src = unique_path(9001);
    let mut open = Meter::new(u64::MAX);
    let fresh_ok = run(&src, &mut open).unwrap();
    let full = open.used;
    assert!(full > 0);
    // Cached now: a budget below the fresh cost must fail identically to an
    // uncached run of the same source (a never-cached 1-byte-longer variant
    // is not comparable, so compare against the cold failure recorded on a
    // sibling source parsed before it was cached).
    let cold = unique_path(9002);
    let mut tiny = Meter::new(full / 2);
    let cold_err = run(&cold, &mut tiny).unwrap_err();
    let mut tiny = Meter::new(full / 2);
    let hit_err = run(&src, &mut tiny).unwrap_err();
    assert_eq!(hit_err, cold_err);
    // And the same budget that suffices for the fresh parse suffices for a hit.
    let mut exact = Meter::new(full);
    assert_eq!(run(&src, &mut exact), Ok(fresh_ok));
    assert_eq!(exact.used, full, "hit charges exactly the fresh totals");
}

#[test]
fn cancellation_fires_on_a_hit() {
    let src = unique_path(9100);
    run(&src, &mut Meter::new(u64::MAX)).unwrap();
    let mut meter = Meter::new(u64::MAX);
    meter.cancel = true;
    assert!(run(&src, &mut meter).is_err());
    assert!(meter.checkpoints >= 1);
}

#[test]
fn many_repeated_cached_paths_stay_under_the_aggregate_cap() {
    let src = unique_path(9200);
    let mut probe = Meter::new(u64::MAX);
    run(&src, &mut probe).unwrap();
    let per = probe.used;
    let mut meter = Meter::new(per * 1000);
    let mut done = 0;
    for _ in 0..1500 {
        if run(&src, &mut meter).is_err() {
            break;
        }
        done += 1;
    }
    assert_eq!(done, 1000, "work cap applies across repeated cached parses");
}

#[test]
fn twenty_random_ancestor_transforms_match_fresh_decodes() {
    let src = format!(
        "<svg><path d=\"{} A 20 10 30 1 0 90 40 a 5 5 0 0 1 9 9 Q 1 2 3 4 T 5 6\"/></svg>",
        long_d()
    );
    let bytes = src.as_bytes();
    let (tokens, _) = tokenize(bytes, XmlLimits::default());
    let element = (0..tokens.len())
        .find(|&i| tokens[i].local_name.slice(bytes) == Some(b"path".as_slice()))
        .unwrap();
    let d = attribute(bytes, &tokens, element, b"d", None).unwrap();
    let span = Span {
        start: d.offset,
        end: d.end().unwrap(),
    };
    let mut seed = 0x2545_F491_4F6C_DD1Du64;
    let mut next = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed >> 11) as f64 / (1u64 << 53) as f64
    };
    let (mut failures, mut successes) = (0, 0);
    for _ in 0..20 {
        let angle = next() * std::f64::consts::TAU;
        let scale = 0.05 + next() * 9000.0f64.min(40.0 + next() * 8000.0);
        let t = Affine {
            a: scale * angle.cos(),
            b: scale * angle.sin(),
            c: -scale * angle.sin() * (0.5 + next()),
            d: scale * angle.cos() * (0.5 + next()),
            e: next() * 2000.0 - 1000.0,
            f: next() * 2000.0 - 1000.0,
        };
        let fresh = vexel_svg_path::parse_path(bytes, span, t, vexel_svg_decode::path_options());
        let got = element_geometry(bytes, &tokens, element, t, [100.0, 100.0]).map(|g| g.path);
        match (fresh, got) {
            (Ok(f), Ok(g)) => {
                assert_eq!(g, f);
                successes += 1;
            }
            (Err(_), Err(_)) => failures += 1,
            (f, g) => panic!("fresh {f:?} vs cached {g:?}"),
        }
    }
    assert!(successes > 5, "{successes} ok / {failures} err");
}

#[test]
fn slow_zoom_and_pan_sequence_matches_fresh_parses() {
    let d: String = (0..160)
        .map(|k| {
            let x = f64::from(k) * 7.31;
            format!(
                "C{} {} {} {} {} {} ",
                x + 3.3,
                40.7 - x * 0.1,
                x + 9.9,
                90.1,
                x + 14.3,
                20.5 + x * 0.3
            )
        })
        .collect();
    let src = format!("<svg><path d=\"M0 0 {d}\"/></svg>");
    let bytes = src.as_bytes();
    let (tokens, _) = tokenize(bytes, XmlLimits::default());
    let element = (0..tokens.len())
        .find(|&i| tokens[i].local_name.slice(bytes) == Some(b"path".as_slice()))
        .unwrap();
    let dspan = attribute(bytes, &tokens, element, b"d", None).unwrap();
    let span = Span {
        start: dspan.offset,
        end: dspan.end().unwrap(),
    };
    for step in 0..300 {
        let zoom = 0.585 + f64::from(step) * 0.0004;
        let (sin, cos) = (f64::from(step) * 0.001).sin_cos();
        let t = Affine {
            a: zoom * cos,
            b: zoom * sin,
            c: -zoom * sin,
            d: zoom * cos,
            e: f64::from(step) * 0.37,
            f: -3.0,
        };
        let fresh =
            vexel_svg_path::parse_path(bytes, span, t, vexel_svg_decode::path_options()).unwrap();
        let got = element_geometry(bytes, &tokens, element, t, [100.0, 100.0]).unwrap();
        assert_eq!(got.path, fresh, "step {step}");
    }
}
