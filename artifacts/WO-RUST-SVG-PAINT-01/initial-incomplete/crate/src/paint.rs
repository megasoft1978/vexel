use vexel_geometry::{affine_map_point, Edge, Point};
use vexel_scene::{ErrorKind, Paint, PaintKind, PaintStop, Result, Spread};

pub const MAX_STOPS: usize = 65_536;
pub const MAX_GRADIENT_STOPS: usize = 256;
pub const MAX_GRADIENTS: usize = 32_768;
pub const MAX_TOKEN_VISITS: usize = 4_194_304;
pub const CACHE_SLOTS: usize = MAX_GRADIENTS * 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaintError { InvalidArgument, Unsupported, Limit, OutOfMemory, Canceled }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaintDiagnostic { pub error: PaintError, pub offset: usize }

/// Return a nonempty edge slice only when the complete requested range exists.
/// `formed` records whether a valid slice was formed, including empty inputs.
pub fn edge_span<'a>(edges: &'a [Edge], first: usize, count: usize, formed: &mut bool) -> Option<&'a [Edge]> {
    *formed = false;
    if count == 0 { return None; }
    let end = first.checked_add(count)?;
    let result = edges.get(first..end)?;
    *formed = true;
    Some(result)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct CacheKey { definition: usize, width: u64, height: u64 }

/// Per-decode gradient cache. The key includes the complete current resolution
/// context (definition and viewport); shape-specific inverse/bounds are applied
/// after cache lookup and are never shared between shapes.
#[derive(Debug, Default)]
pub struct GradientCache {
    entries: Vec<(CacheKey, Paint, Vec<PaintStop>)>,
    slots: Vec<Option<usize>>,
}

impl GradientCache {
    pub fn len(&self) -> usize { self.entries.len() }
    pub fn is_empty(&self) -> bool { self.entries.is_empty() }
    pub fn allocated_bytes(&self) -> Option<usize> {
        self.entries.capacity().checked_mul(std::mem::size_of::<(CacheKey, Paint, Vec<PaintStop>)>())
            .and_then(|v| self.slots.capacity().checked_mul(std::mem::size_of::<Option<usize>>()).and_then(|s| v.checked_add(s)))
    }
    /// Returns an immutable entry for the exact definition/viewport key.
    pub fn get(&self, definition: usize, viewport: [f64; 2]) -> Option<(Paint, &[PaintStop])> {
        let key = CacheKey { definition, width: viewport[0].to_bits(), height: viewport[1].to_bits() };
        let index = self.find(key)?;
        let (_, paint, stops) = self.entries.get(index)?;
        Some((*paint, stops.as_slice()))
    }
    /// Stores an owned definition result. Cache capacity and all size arithmetic
    /// are fallible, and malformed stop lists are rejected before insertion.
    pub fn insert_resolved(&mut self, definition: usize, viewport: [f64; 2], paint: Paint, stops: Vec<PaintStop>) -> Result<()> {
        if !viewport[0].is_finite() || !viewport[1].is_finite() || !valid_stops(&stops) || stops.len() > MAX_GRADIENT_STOPS {
            return Err(invalid(0));
        }
        let key = CacheKey { definition, width: viewport[0].to_bits(), height: viewport[1].to_bits() };
        if self.find(key).is_some() { return Ok(()); }
        self.insert(key, paint, stops).map(|_| ())
    }
    fn find(&self, key: CacheKey) -> Option<usize> {
        if self.slots.is_empty() { return None; }
        let mut slot = hash(key) & (CACHE_SLOTS - 1);
        for _ in 0..CACHE_SLOTS {
            match self.slots.get(slot).copied().flatten() {
                None => return None,
                Some(index) if self.entries.get(index).is_some_and(|entry| entry.0 == key) => return Some(index),
                _ => slot = (slot + 1) & (CACHE_SLOTS - 1),
            }
        }
        None
    }
    fn insert(&mut self, key: CacheKey, paint: Paint, stops: Vec<PaintStop>) -> Result<usize> {
        if self.entries.len() >= MAX_GRADIENTS { return Err(error(ErrorKind::Limit(vexel_scene::LimitKind::PaintVisits), 0)); }
        if self.slots.is_empty() {
            self.slots.try_reserve_exact(CACHE_SLOTS).map_err(|_| oom(0))?;
            self.slots.resize(CACHE_SLOTS, None);
        }
        let next = self.entries.len();
        self.entries.try_reserve(1).map_err(|_| oom(0))?;
        let mut slot = hash(key) & (CACHE_SLOTS - 1);
        for _ in 0..CACHE_SLOTS {
            if self.slots[slot].is_none() {
                self.entries.push((key, paint, stops));
                self.slots[slot] = Some(next);
                return Ok(next);
            }
            slot = (slot + 1) & (CACHE_SLOTS - 1);
        }
        Err(error(ErrorKind::Limit(vexel_scene::LimitKind::PaintVisits), 0))
    }
}

fn hash(key: CacheKey) -> usize {
    let mut x = (key.definition as u64) ^ key.width ^ key.height.wrapping_add(0x9e3779b97f4a7c15);
    x ^= x >> 30; x = x.wrapping_mul(0xbf58476d1ce4e5b9);
    x ^= x >> 27; x = x.wrapping_mul(0x94d049bb133111eb); x ^= x >> 31;
    x as usize
}

fn error(kind: ErrorKind, offset: usize) -> vexel_scene::Diagnostic { vexel_scene::Diagnostic { kind, offset } }
fn oom(offset: usize) -> vexel_scene::Diagnostic { error(ErrorKind::OutOfMemory, offset) }
fn invalid(offset: usize) -> vexel_scene::Diagnostic { error(ErrorKind::InvalidArgument, offset) }
/// Sample a validated paint. Invalid stop ranges or nonfinite intermediate
/// geometry leave `rgba` untouched, matching the C routine's no-write returns.
pub fn sample(paint: &Paint, stops: &[PaintStop], x: f64, y: f64, rgba: &mut [u8;4]) {
    let Some(end) = paint.stops.first.checked_add(paint.stops.count) else { return; };
    if paint.stops.count < 2 || end > stops.len() || !valid_stops(&stops[paint.stops.first..end]) { return; }
    let Some(local) = affine_map_point(paint.user_inverse, Point { x, y }) else { return; };
    let (mut gx, mut gy) = (local.x, local.y);
    if paint.object_bounding_box {
        let (w, h) = (paint.bounds[2] - paint.bounds[0], paint.bounds[3] - paint.bounds[1]);
        if !w.is_finite() || !h.is_finite() || w <= 0.0 || h <= 0.0 { return; }
        gx = (local.x - paint.bounds[0]) / w;
        gy = (local.y - paint.bounds[1]) / h;
    }
    let Some(point) = affine_map_point(paint.gradient_inverse, Point { x: gx, y: gy }) else { return; };
    gx = point.x; gy = point.y;
    let t = match paint.kind {
        PaintKind::Linear => {
            let dx = paint.coordinates[2] - paint.coordinates[0];
            let dy = paint.coordinates[3] - paint.coordinates[1];
            let len = dx * dx + dy * dy;
            if len == 0.0 { 1.0 } else { ((gx - paint.coordinates[0]) * dx + (gy - paint.coordinates[1]) * dy) / len }
        }
        PaintKind::Radial => radial_t(paint, gx, gy),
        _ => return,
    };
    if !t.is_finite() { return; }
    let t = spread(t, paint.spread);
    let section = &stops[paint.stops.first..end];
    if t <= section[0].offset { *rgba = section[0].rgba; return; }
    let last = section.len() - 1;
    if t >= section[last].offset { *rgba = section[last].rgba; return; }
    let mut right = 1;
    while right < last && t > section[right].offset { right += 1; }
    let left = right - 1;
    let range = section[right].offset - section[left].offset;
    let amount = if range <= 0.0 { 1.0 } else { (t - section[left].offset) / range };
    let mut out = [0;4];
    for c in 0..4 { out[c] = ((1.0 - amount) * f64::from(section[left].rgba[c]) + amount * f64::from(section[right].rgba[c])).round().clamp(0.0,255.0) as u8; }
    *rgba = out;
}

fn valid_stops(stops: &[PaintStop]) -> bool {
    stops.len() >= 2 && stops.iter().all(|s| s.offset.is_finite() && (0.0..=1.0).contains(&s.offset)) && stops.windows(2).all(|w| w[1].offset >= w[0].offset)
}
fn spread(t: f64, spread: Spread) -> f64 {
    match spread {
        Spread::Pad => t,
        Spread::Repeat => t - t.floor(),
        Spread::Reflect => { let mut v = t % 2.0; if v < 0.0 { v += 2.0; } if v > 1.0 { 2.0-v } else { v } }
    }
}
fn radial_t(paint: &Paint, gx: f64, gy: f64) -> f64 {
    let (cx,cy,r,fx,fy) = (paint.coordinates[0],paint.coordinates[1],paint.coordinates[2],paint.coordinates[3],paint.coordinates[4]);
    let (cdx,cdy,pdx,pdy) = (cx-fx,cy-fy,gx-fx,gy-fy);
    let (a,b,c) = (cdx*cdx+cdy*cdy-r*r, -2.0*(pdx*cdx+pdy*cdy), pdx*pdx+pdy*pdy);
    if a.abs() <= 1e-15 { if b.abs() <= 1e-15 { return f64::NAN; } return -c/b; }
    let d = b*b-4.0*a*c;
    if d < 0.0 || !d.is_finite() { return f64::NAN; }
    let root=d.sqrt(); let t0=(-b-root)/(2.0*a); let t1=(-b+root)/(2.0*a);
    let t=t0.max(t1); if t < 0.0 { t0.min(t1) } else { t }
}

/// Cache destroy is ordinary owned Rust destruction; no release callback exists.
pub fn destroy_cache(cache: GradientCache) { drop(cache); }

