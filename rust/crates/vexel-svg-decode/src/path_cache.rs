//! Process-wide cache of parsed path data, so an unchanged `d` attribute is not
//! re-tokenized and re-flattened on every decode (animation ticks, edits).
//!
//! Entries hold the path in LOCAL space. The parser flattens in local space and
//! only maps each edge endpoint through the transform afterwards; the
//! transform enters the flattening solely through its tolerance. So the key is
//! the exact `d` bytes (hashed, then verified byte-for-byte) and the parse
//! options. The tolerance is not part of the key but a validity window: the
//! parse records the range of tolerances that reproduce all of its curve
//! flattening decisions (arcs pin the exact value), so a slowly zooming
//! ancestor keeps hitting. An animated ancestor translation or rotation never
//! invalidates an entry: a hit maps the cached edges with the current
//! transform (the same `affine_map_point` the parser uses). The document
//! carries absolute source offsets as provenance; a hit rebases them to the
//! requesting element's offset. A hit is also accounted: the fresh parse's
//! per-kind work charges are stored and replayed through the caller's
//! `WorkControl`, so limits, cancellation and typed errors match an uncached
//! parse. Only successful parses of `d` values of at least `MIN_D_BYTES` are
//! stored.
//!
//! Memory is bounded: entries are charged by owned bytes (`d` copy plus the
//! document vectors). When the total exceeds `CAP_BYTES` the least recently
//! used entries are evicted until it is back under `LOW_WATER`. A single
//! document above `MAX_ENTRY_BYTES` is never cached.
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use vexel_base::{Span, Stop, WorkControl, WorkKind};
use vexel_geometry::{Affine, Edge, affine_map_point};
use vexel_svg_path::{ParseOptions, PathDocument};

/// Total cache budget (owned bytes) and the level eviction drains down to.
/// Sized above the working set of a dense cartoon shot (about 31 MiB on the
/// heaviest): a cyclic access pattern larger than the cap would evict every
/// entry just before its reuse and miss on each tick.
const CAP_BYTES: usize = 128 * 1024 * 1024;
const LOW_WATER: usize = 96 * 1024 * 1024;
/// Shortest `d` worth caching: parse cost scales with length, and the many
/// short paths of dense scenes would only duplicate memory for little gain.
const MIN_D_BYTES: usize = 512;
/// Largest single entry; bigger documents bypass the cache.
const MAX_ENTRY_BYTES: usize = 2 * 1024 * 1024;

#[derive(Hash, PartialEq, Eq)]
struct Key {
    hash: u64,
    len: usize,
    options: [usize; 3],
}

/// Work charged by the fresh parse, per `WorkKind`. A hit replays it through
/// the caller's control, so limits, cancellation and typed errors behave as
/// for an uncached parse.
pub(crate) type Charges = [u64; WorkKind::ALL.len()];

/// Forwards to the real control while totalling what the parse charged.
pub(crate) struct Recorder<'a> {
    pub inner: &'a mut dyn WorkControl,
    pub charges: Charges,
}

impl WorkControl for Recorder<'_> {
    fn charge(&mut self, kind: WorkKind, units: u64) -> Result<(), Stop> {
        self.inner.charge(kind, units)?;
        let slot = &mut self.charges[kind as usize];
        *slot = slot.saturating_add(units);
        Ok(())
    }
    fn checkpoint(&mut self) -> Result<(), Stop> {
        self.inner.checkpoint()
    }
}

/// Charges a hit's recorded totals (and a checkpoint) to `control`.
pub(crate) fn replay(charges: &Charges, control: &mut dyn WorkControl) -> Result<(), Stop> {
    control.checkpoint()?;
    for kind in WorkKind::ALL {
        let units = charges[kind as usize];
        if units != 0 {
            control.charge(kind, units)?;
        }
    }
    control.checkpoint()
}

struct Entry {
    charges: Charges,
    /// Flatness range `lo <= tol < hi` this entry's flattening is valid for.
    window: (f64, f64),
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

fn key(d: &[u8], o: ParseOptions) -> Key {
    Key {
        hash: hash(d),
        len: d.len(),
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

/// Returns the cached local-space document, rebased to `span.start`.
pub(crate) fn get(
    source: &[u8],
    span: Span,
    tolerance: f64,
    o: ParseOptions,
) -> Option<(PathDocument, Charges)> {
    let d = source
        .get(span.start..span.end)
        .filter(|d| d.len() >= MIN_D_BYTES)?;
    let k = key(d, o);
    let (doc, base, charges) = {
        let mut c = cache().lock().ok()?;
        c.clock += 1;
        let clock = c.clock;
        let e = c.map.get_mut(&k)?;
        if *e.d != *d || tolerance < e.window.0 || tolerance >= e.window.1 {
            return None;
        }
        e.used = clock;
        (e.doc.clone(), e.base, e.charges)
    };
    Some((rebase(&doc, base, span.start), charges))
}

/// Stores a successful local-space parse of `source[span]`.
pub(crate) fn put(
    source: &[u8],
    span: Span,
    window: (f64, f64),
    o: ParseOptions,
    doc: &PathDocument,
    charges: Charges,
) {
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
    let k = key(d, o);
    let entry_doc = Arc::new(doc.clone());
    let Ok(mut c) = cache().lock() else { return };
    c.clock += 1;
    let used = c.clock;
    let entry = Entry {
        charges,
        window,
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

/// Maps a local-space document's edges through `t` in place. `None` when a
/// point fails the parser's mapping rules; callers then parse directly to get
/// the exact typed error.
pub(crate) fn map_edges(doc: &mut PathDocument, t: Affine) -> Option<()> {
    for e in &mut doc.edges {
        e.a = affine_map_point(t, e.a)?;
        e.b = affine_map_point(t, e.b)?;
    }
    Some(())
}

/// Whether a `d` span is long enough to be worth caching.
pub(crate) fn cacheable(span: Span) -> bool {
    span.end.saturating_sub(span.start) >= MIN_D_BYTES
}
