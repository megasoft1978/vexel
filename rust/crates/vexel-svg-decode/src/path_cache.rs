//! Process-wide cache of parsed path data, so an unchanged `d` attribute is not
//! re-parsed and re-flattened on every decode (animation ticks, edits).
//!
//! Key: the exact `d` bytes (hashed, then verified byte-for-byte), the affine
//! transform bits and the parse options: the same inputs that determine the
//! parse. The parsed document carries absolute source offsets as provenance;
//! a hit rebases them to the requesting element's offset, so the result is
//! identical to a fresh parse. Only successful parses of `d` values of at least `MIN_D_BYTES` are stored.
//!
//! Memory is bounded: entries are charged by owned bytes (`d` copy plus the
//! document vectors). When the total exceeds `CAP_BYTES` the least recently
//! used entries are evicted until it is back under `LOW_WATER`. A single
//! document above `MAX_ENTRY_BYTES` is never cached.
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use vexel_base::Span;
use vexel_geometry::{Affine, Edge};
use vexel_svg_path::{ParseOptions, PathDocument};

/// Total cache budget (owned bytes) and the level eviction drains down to.
const CAP_BYTES: usize = 24 * 1024 * 1024;
const LOW_WATER: usize = 16 * 1024 * 1024;
/// Shortest `d` worth caching: parse cost scales with length, and the many
/// short paths of dense scenes would only duplicate memory for little gain.
const MIN_D_BYTES: usize = 512;
/// Largest single entry; bigger documents bypass the cache.
const MAX_ENTRY_BYTES: usize = 2 * 1024 * 1024;

#[derive(Hash, PartialEq, Eq)]
struct Key {
    hash: u64,
    len: usize,
    transform: [u64; 6],
    options: [usize; 3],
}

struct Entry {
    d: Box<[u8]>,
    /// Absolute offset of `d` in the source the document was parsed from.
    base: usize,
    doc: Arc<PathDocument>,
    bytes: usize,
    used: u64,
}

#[derive(Default)]
struct Cache {
    map: HashMap<Key, Entry>,
    bytes: usize,
    clock: u64,
}

fn cache() -> &'static Mutex<Cache> {
    static CACHE: OnceLock<Mutex<Cache>> = OnceLock::new();
    CACHE.get_or_init(Default::default)
}

fn hash(bytes: &[u8]) -> u64 {
    const K: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut h = 0xcbf2_9ce4_8422_2325u64 ^ bytes.len() as u64;
    let mut chunks = bytes.chunks_exact(8);
    for c in &mut chunks {
        let v = u64::from_le_bytes([c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7]]);
        h = (h ^ v).wrapping_mul(K).rotate_left(29);
    }
    for &b in chunks.remainder() {
        h = (h ^ u64::from(b)).wrapping_mul(K).rotate_left(11);
    }
    h ^ (h >> 32)
}

fn key(d: &[u8], t: Affine, o: ParseOptions) -> Key {
    Key {
        hash: hash(d),
        len: d.len(),
        transform: [t.a, t.b, t.c, t.d, t.e, t.f].map(f64::to_bits),
        options: [o.max_commands, o.max_edges, o.max_numbers],
    }
}

fn doc_bytes(d: &PathDocument) -> usize {
    d.edges.len() * size_of::<Edge>()
        + d.subpaths.len() * size_of::<vexel_svg_path::PathSubpath>()
        + d.segments.len() * size_of::<vexel_svg_path::PathSegment>()
        + d.numbers.len() * size_of::<vexel_svg_path::PathNumber>()
}

fn shift(v: usize, from: usize, to: usize) -> usize {
    v.wrapping_add(to).wrapping_sub(from)
}

fn rebase(doc: &PathDocument, from: usize, to: usize) -> PathDocument {
    let mut out = doc.clone();
    if from != to {
        let span = |s: &mut Span| {
            s.start = shift(s.start, from, to);
            s.end = shift(s.end, from, to);
        };
        for n in &mut out.numbers {
            span(&mut n.source);
        }
        for s in &mut out.segments {
            span(&mut s.source);
            if let Some(o) = s.command_offset.as_mut() {
                *o = shift(*o, from, to);
            }
        }
    }
    out
}

/// Returns the cached document for these inputs, rebased to `span.start`.
pub(crate) fn get(source: &[u8], span: Span, t: Affine, o: ParseOptions) -> Option<PathDocument> {
    let d = source
        .get(span.start..span.end)
        .filter(|d| d.len() >= MIN_D_BYTES)?;
    let k = key(d, t, o);
    let (doc, base) = {
        let mut c = cache().lock().ok()?;
        c.clock += 1;
        let clock = c.clock;
        let e = c.map.get_mut(&k)?;
        if *e.d != *d {
            return None;
        }
        e.used = clock;
        (e.doc.clone(), e.base)
    };
    Some(rebase(&doc, base, span.start))
}

/// Stores a successful parse of `source[span]`.
pub(crate) fn put(source: &[u8], span: Span, t: Affine, o: ParseOptions, doc: &PathDocument) {
    let Some(d) = source
        .get(span.start..span.end)
        .filter(|d| d.len() >= MIN_D_BYTES)
    else {
        return;
    };
    let bytes = doc_bytes(doc) + d.len();
    if bytes > MAX_ENTRY_BYTES {
        return;
    }
    let k = key(d, t, o);
    let entry_doc = Arc::new(doc.clone());
    let Ok(mut c) = cache().lock() else { return };
    c.clock += 1;
    let used = c.clock;
    let entry = Entry {
        d: d.into(),
        base: span.start,
        doc: entry_doc,
        bytes,
        used,
    };
    c.bytes += bytes;
    if let Some(old) = c.map.insert(k, entry) {
        c.bytes -= old.bytes;
    }
    if c.bytes > CAP_BYTES {
        let mut order: Vec<(u64, usize, u64)> = Vec::new();
        order.extend(c.map.iter().map(|(k, e)| (e.used, e.bytes, k.hash)));
        order.sort_unstable();
        let mut victims = std::collections::HashSet::new();
        let mut total = c.bytes;
        for (_, bytes, h) in order {
            if total <= LOW_WATER {
                break;
            }
            total -= bytes;
            victims.insert(h);
        }
        c.map.retain(|k, _| !victims.contains(&k.hash));
        c.bytes = c.map.values().map(|e| e.bytes).sum();
    }
}

/// Cached bytes (for tests and diagnostics).
pub fn cached_bytes() -> usize {
    cache().lock().map_or(0, |c| c.bytes)
}
