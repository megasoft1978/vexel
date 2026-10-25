//! Dirty-rectangle recompose: after every seeded random edit (including undo
//! and redo) the cached compose equals a full compose byte for byte. The
//! number of sequences is `VEXEL_DIRTY_SEQS` (default 6; 300 is the full run).
//! `dirty_bench` (ignored, release) prints per-edit p50/p95 before and after.

use super::{DiffCache, compose_document, compose_document_cached};
use std::fmt::Write as _;
use std::sync::Arc;
use std::time::Instant;

struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 33
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
    fn f(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (self.next() % 10_000) as f64 / 10_000.0 * (hi - lo)
    }
}

#[derive(Clone)]
struct Sh {
    kind: usize,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    fill: usize,
    stroke: Option<usize>,
    sw: f64,
    op: f64,
    rot: f64,
}

#[derive(Clone)]
struct Gr {
    wrap: bool,
    opacity: f64,
    clip: Option<usize>,
    items: Vec<Sh>,
}

#[derive(Clone)]
struct Scn {
    size: f64,
    groups: Vec<Gr>,
}

const COLORS: [&str; 6] = [
    "#e63946", "#2a9d8f", "#264653", "#f4a261", "#8338ec", "#ffd166",
];

fn shape(r: &mut Lcg, size: f64, profile: &str) -> Sh {
    let stroke_only = profile == "stroke";
    Sh {
        kind: if stroke_only { 2 } else { r.below(3) },
        x: r.f(0.0, size - 30.0),
        y: r.f(0.0, size - 30.0),
        w: r.f(6.0, 30.0),
        h: r.f(6.0, 30.0),
        fill: if stroke_only { 8 } else { r.below(9) },
        stroke: (stroke_only || r.below(3) == 0).then(|| r.below(6)),
        sw: r.f(1.0, 4.0),
        op: if r.below(6) == 0 { 0.6 } else { 1.0 },
        rot: if r.below(3) == 0 { r.f(0.0, 90.0) } else { 0.0 },
    }
}

fn scene(seed: u64, n: usize, size: f64, profile: &str) -> Scn {
    let mut r = Lcg(seed);
    let mut groups = Vec::new();
    let mut left = n;
    while left > 0 {
        let take = left.min(1 + r.below(8));
        left -= take;
        let (wrap, opacity, clip) = match profile {
            "clip" => (true, 1.0, Some(r.below(2))),
            "mix" => match r.below(4) {
                0 => (true, 0.5, None),
                1 => (true, 1.0, Some(r.below(2))),
                2 => (true, 1.0, None),
                _ => (false, 1.0, None),
            },
            _ => (false, 1.0, None),
        };
        let items = (0..take).map(|_| shape(&mut r, size, profile)).collect();
        groups.push(Gr {
            wrap,
            opacity,
            clip,
            items,
        });
    }
    Scn { size, groups }
}

fn svg(s: &Scn) -> Vec<u8> {
    let z = s.size;
    let mut o = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{z}\" height=\"{z}\" viewBox=\"0 0 {z} {z}\"><defs>\
<linearGradient id=\"g1\" x1=\"0\" y1=\"0\" x2=\"1\" y2=\"1\"><stop offset=\"0\" stop-color=\"#f00\"/><stop offset=\"1\" stop-color=\"#00f\"/></linearGradient>\
<radialGradient id=\"g2\"><stop offset=\"0\" stop-color=\"#fff\"/><stop offset=\"1\" stop-color=\"#093\"/></radialGradient>\
<clipPath id=\"c0\"><rect x=\"0\" y=\"0\" width=\"{h}\" height=\"{z}\"/></clipPath>\
<clipPath id=\"c1\"><circle cx=\"{h}\" cy=\"{h}\" r=\"{q}\"/></clipPath></defs>",
        h = z / 2.0,
        q = z / 3.0
    );
    for g in &s.groups {
        if g.wrap {
            let clip = g
                .clip
                .map_or(String::new(), |c| format!(" clip-path=\"url(#c{c})\""));
            let _ = write!(o, "<g opacity=\"{}\"{clip}>", g.opacity);
        }
        for sh in &g.items {
            let fill = match sh.fill {
                0..=5 => COLORS[sh.fill].to_owned(),
                6 | 7 => format!("url(#g{})", sh.fill - 5),
                _ => "none".to_owned(),
            };
            let stroke = sh.stroke.map_or(String::new(), |c| {
                format!(" stroke=\"{}\" stroke-width=\"{:.2}\"", COLORS[c], sh.sw)
            });
            let (cx, cy) = (sh.x + sh.w / 2.0, sh.y + sh.h / 2.0);
            let rot = if sh.rot == 0.0 {
                String::new()
            } else {
                format!(" transform=\"rotate({:.2} {cx:.2} {cy:.2})\"", sh.rot)
            };
            let tail = format!(" fill=\"{fill}\"{stroke} opacity=\"{}\"{rot}/>", sh.op);
            match sh.kind {
                0 => {
                    let _ = write!(
                        o,
                        "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\"{tail}",
                        sh.x, sh.y, sh.w, sh.h
                    );
                }
                1 => {
                    let _ = write!(
                        o,
                        "<circle cx=\"{cx:.2}\" cy=\"{cy:.2}\" r=\"{:.2}\"{tail}",
                        sh.w / 2.0
                    );
                }
                _ => {
                    let _ = write!(
                        o,
                        "<path d=\"M{:.2} {:.2} L{:.2} {:.2} L{:.2} {:.2}\"{tail}",
                        sh.x,
                        sh.y,
                        sh.x + sh.w,
                        sh.y + sh.h,
                        sh.x,
                        sh.y + 2.0 * sh.h
                    );
                }
            }
        }
        if g.wrap {
            o.push_str("</g>");
        }
    }
    o.push_str("</svg>");
    o.into_bytes()
}

fn pick(s: &Scn, r: &mut Lcg) -> (usize, usize) {
    let g = r.below(s.groups.len());
    (g, r.below(s.groups[g].items.len()))
}

/// One random edit; returns false when it did not apply.
fn edit(s: &mut Scn, r: &mut Lcg, profile: &str) -> bool {
    let (g, i) = pick(s, r);
    let size = s.size;
    match r.below(9) {
        0 | 1 => {
            let sh = &mut s.groups[g].items[i];
            let (dx, dy) = (r.f(-12.0, 12.0), r.f(-12.0, 12.0));
            sh.x = (sh.x + dx).clamp(-10.0, size);
            sh.y = (sh.y + dy).clamp(-10.0, size);
        }
        2 => s.groups[g].items[i].fill = r.below(9),
        3 => s.groups[g].items[i].op = [1.0, 0.8, 0.4][r.below(3)],
        4 => {
            if !s.groups[g].wrap {
                return false;
            }
            s.groups[g].opacity = [1.0, 0.7, 0.3][r.below(3)];
        }
        5 => s.groups[g].items[i].rot = r.f(0.0, 180.0),
        6 => {
            let sh = &mut s.groups[g].items[i];
            sh.w = r.f(4.0, 40.0);
            sh.h = r.f(4.0, 40.0);
        }
        7 => {
            if s.groups.iter().map(|g| g.items.len()).sum::<usize>() < 4 {
                return false;
            }
            s.groups[g].items.remove(i);
            if s.groups[g].items.is_empty() {
                s.groups.remove(g);
            }
        }
        _ => {
            let sh = shape(r, size, profile);
            let at = r.below(s.groups[g].items.len() + 1);
            s.groups[g].items.insert(at, sh);
        }
    }
    true
}

fn same(a: &super::Composed, b: &super::Composed, what: &str) {
    assert_eq!(
        a.frame.disclosures, b.frame.disclosures,
        "{what} disclosures"
    );
    assert_eq!(a.shapes, b.shapes, "{what} shapes");
    assert_eq!(a.target_ids, b.target_ids, "{what} ids");
    assert!(a.frame.rgba == b.frame.rgba, "{what}: pixels differ");
}

fn run_sequence(seed: u64, profile: &str, edits: usize, partials: &mut u32) {
    let mut r = Lcg(seed ^ 0x9e37);
    let mut now = scene(seed, 24 + r.below(24), 160.0, profile);
    let (mut undo, mut redo): (Vec<Scn>, Vec<Scn>) = (Vec::new(), Vec::new());
    let mut cache = DiffCache::default();
    let check = |s: &Scn, cache: &mut DiffCache, what: &str| {
        let bytes = Arc::new(svg(s));
        let full = compose_document(&bytes, 160, 160, &|| false).unwrap();
        let fast = compose_document_cached(bytes, 160, 160, &|| false, cache).unwrap();
        same(&full, &fast, what);
    };
    check(&now, &mut cache, &format!("{profile}#{seed} open"));
    for step in 0..edits {
        let op = r.below(12);
        let what = format!("{profile}#{seed} step {step}");
        if op == 10
            && let Some(prev) = undo.pop()
        {
            redo.push(std::mem::replace(&mut now, prev));
        } else if op == 11
            && let Some(next) = redo.pop()
        {
            undo.push(std::mem::replace(&mut now, next));
        } else {
            let before = now.clone();
            if !edit(&mut now, &mut r, profile) {
                continue;
            }
            undo.push(before);
            redo.clear();
        }
        check(&now, &mut cache, &what);
    }
    *partials += cache.partial_frames();
}

#[test]
fn cached_compose_matches_full_compose_after_every_edit() {
    let sequences: u64 = std::env::var("VEXEL_DIRTY_SEQS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(6);
    let mut partials = 0;
    for seq in 0..sequences {
        let profile = ["mix", "stroke", "clip"][(seq % 3) as usize];
        run_sequence(0xD1_0000 + seq, profile, 14, &mut partials);
    }
    assert!(partials > 0, "no frame took the partial path");
}

#[test]
fn unsupported_scenes_fall_back_to_full_compose() {
    let a = br##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64"><defs><mask id="m"><rect width="32" height="64" fill="#fff"/></mask></defs><rect x="4" y="4" width="50" height="50" fill="#c00" mask="url(#m)"/><circle cx="40" cy="40" r="8" fill="#00c"/></svg>"##;
    let b = String::from_utf8_lossy(a).replace("cx=\"40\"", "cx=\"44\"");
    let mut cache = DiffCache::default();
    for bytes in [a.to_vec(), b.into_bytes()] {
        let full = compose_document(&bytes, 64, 64, &|| false).unwrap();
        cache.rearm();
        let fast = compose_document_cached(Arc::new(bytes), 64, 64, &|| false, &mut cache).unwrap();
        same(&full, &fast, "mask scene");
        assert!(cache.disabled(), "mask scene must not be diffed");
    }
    assert_eq!(cache.partial_frames(), 0);
}

fn percentile(v: &mut [f64], p: f64) -> f64 {
    v.sort_by(f64::total_cmp);
    v[((v.len() - 1) as f64 * p).round() as usize]
}

/// `cargo test -p vexel-app --release --lib dirty_bench -- --ignored --nocapture`
#[test]
#[ignore = "benchmark: run in release"]
fn dirty_bench() {
    println!("scene   N    | full p50/p95 ms | dirty p50/p95 ms | speedup");
    for profile in ["mix", "stroke", "clip"] {
        for n in [200, 1000, 5000] {
            let mut r = Lcg(77);
            let mut s = scene(5 + n as u64, n, 900.0, profile);
            let mut cache = DiffCache::default();
            let warm = Arc::new(svg(&s));
            compose_document_cached(warm, 900, 900, &|| false, &mut cache).unwrap();
            let (mut full, mut fast) = (Vec::new(), Vec::new());
            for _ in 0..15 {
                let g = r.below(s.groups.len());
                let i = r.below(s.groups[g].items.len());
                s.groups[g].items[i].x += 9.0;
                s.groups[g].items[i].y += 5.0;
                let bytes = Arc::new(svg(&s));
                let t = Instant::now();
                let a = compose_document_cached(bytes.clone(), 900, 900, &|| false, &mut cache)
                    .unwrap();
                fast.push(t.elapsed().as_secs_f64() * 1e3);
                let t = Instant::now();
                let b = compose_document(&bytes, 900, 900, &|| false).unwrap();
                full.push(t.elapsed().as_secs_f64() * 1e3);
                assert!(a.frame.rgba == b.frame.rgba, "{profile} {n}: pixels differ");
            }
            let (f50, f95) = (percentile(&mut full, 0.5), percentile(&mut full, 0.95));
            let (d50, d95) = (percentile(&mut fast, 0.5), percentile(&mut fast, 0.95));
            println!(
                "{profile:6} {n:5} | {f50:8.1}/{f95:8.1} | {d50:8.1}/{d95:8.1} | x{:.1} (partial {})",
                f50 / d50,
                cache.partial_frames()
            );
        }
    }
}
