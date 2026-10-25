//! GPU display route (ADR-0035) for the compose worker and playback ticks.
//!
//! # Where it can run
//!
//! Structurally only through [`try_render`], which `compose_owned` calls when
//! it is handed a [`DiffCache::display`] cache. Exports, `render_png`, MCP
//! tools and goldens use no cache or `DiffCache::default()` and never reach
//! this module (`tests_gpu::export_paths_do_not_reference_the_route` scans for
//! it). On top of that the route is off in every process until the shell binary
//! opts in ([`app_launch`]), so library tests that compare CPU bytes cannot
//! take it by accident.
//!
//! # Decision per revision
//!
//! GPU when the process opted in (or a test/bench forces it), `VEXEL_GPU` is
//! not `0`, the latch is open, the scene lowers (solid or gradient fills only,
//! see `gpu_adapter`), `classify` says supported, a device exists, and the
//! cost model ([`super::route_policy`]) estimates the GPU frame cheaper than
//! the CPU differential tick (dirty area and shapes, measured times, with
//! hysteresis). Any other case renders on the CPU unchanged. Measured CPU tick
//! times are fed in by [`record_cpu_tick`]. A GPU frame replaces the diff
//! cache's previous frame ([`DiffCache::invalidate`]), so a later CPU frame is
//! a full render, never a patch over GPU pixels.
//!
//! # Errors
//!
//! A runtime GPU error renders that frame on the CPU and counts toward the
//! session latch ([`LATCH_ERROR_LIMIT`]); at the limit the route stays off.
//! Errors go to the debug profile (`gpu.error`, `gpu.latched`), not stderr.

use super::diff::{DiffCache, device_rect};
use super::gpu_adapter::{Lowered, lower};
use super::layers::Geo;
use super::render_scene::{Drawn, geo};
use super::route_policy::{Load, Policy, Route, Verdict};
use super::shapes::{Entry, OwnedShape, prepare};
use std::cell::{Cell, RefCell};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Instant;
use vexel_gpu_scene::policy::{ENV_SWITCH, FallbackLatch, env_enables_gpu};
use vexel_gpu_scene::run::{ComputeBackend, GpuRasterizer, Output, RenderError};
use vexel_gpu_scene::{Capability, GpuError, GpuScene, UnsupportedReason, classify};
use vexel_raster::Tile;
use vexel_render::{PreserveAspect, ViewBox};
use vexel_scene::{PaintKind, SceneView, ViewportRead};

static OPTED_IN: AtomicBool = AtomicBool::new(false);
/// Per thread, so a test forcing the route never changes what other threads
/// (workers of concurrent tests) do. 0 = follow opt-in, 1 = forced off, 2 = forced on without the cost and cache
/// heuristics (tests), 3 = on with the shell's heuristics (benches).
static FRAMES: AtomicU64 = AtomicU64::new(0);
static SESSION: Mutex<Session> = Mutex::new(Session::new());

/// Why the last frame of this thread took the path it did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    /// Not a display cache, or no frame yet.
    None,
    /// Route off: not opted in, `VEXEL_GPU=0` or the latch tripped.
    Off,
    /// Scene has a feature the lowering does not take (named).
    Cpu(&'static str),
    /// Lowered but the classifier refused it.
    Unsupported(UnsupportedReason),
    /// Eligible, but no usable device on this thread.
    NoDevice,
    /// Rendered on the GPU.
    Gpu,
    /// Runtime GPU error; the frame was rendered on the CPU.
    Error,
}

/// Ticks a scene verdict (lowering, classify, work) is reused before it is
/// recomputed; the entry count changing also refreshes it.
const VERDICT_TICKS: u32 = 30;
/// Past this many edges the previous shapes are not kept for dirty tracking.
const MAX_TRACKED_EDGES: usize = 1_500_000;

/// Per worker thread: calibration, hysteresis and the last scene seen.
struct Tracker {
    policy: Policy,
    prev: Vec<OwnedShape>,
    verdict: Option<(usize, u32, Verdict<UnsupportedReason>)>,
    /// Load of the last tick that went to the CPU, awaiting its timing.
    pending_cpu: Option<Load>,
    last_gpu: bool,
}

impl Tracker {
    const fn new() -> Self {
        Self {
            policy: Policy::new(),
            prev: Vec::new(),
            verdict: None,
            pending_cpu: None,
            last_gpu: false,
        }
    }
}

/// Feeds the measured wall time (ms) of the CPU tick that followed the last
/// `try_render` returning `None`; refines the CPU estimate.
pub(super) fn record_cpu_tick(ms: f64) {
    TRACKER.with(|t| {
        let mut t = t.borrow_mut();
        if let Some(load) = t.pending_cpu.take() {
            t.policy.observe_cpu(ms, &load);
        }
    });
}

/// Shapes of `entries` that changed against `prev`, and the pixels of the
/// union of their old and new device bounds (the differential repaint area).
fn dirty(prev: &[OwnedShape], cur: &[&OwnedShape], geo: &Geo, w: u32, h: u32) -> (f64, usize) {
    if prev.len() != cur.len() {
        return (f64::from(w) * f64::from(h), cur.len());
    }
    let mut count = 0;
    let mut rect = None;
    for (old, new) in prev.iter().zip(cur) {
        if old == *new {
            continue;
        }
        count += 1;
        for shape in [old, *new] {
            if let Some(r) = device_rect(shape, geo, w, h) {
                rect = Some(rect.map_or(r, |u: super::diff::Rect| u.union(r)));
            }
        }
    }
    let area = rect.map_or(0.0, |r| f64::from(r.x1 - r.x0) * f64::from(r.y1 - r.y0));
    (area, count)
}

fn forced() -> bool {
    OVERRIDE.with(Cell::get) == 2
}

thread_local! {
    static OVERRIDE: Cell<u8> = const { Cell::new(0) };
    static LAST_WORK: Cell<f64> = const { Cell::new(0.0) };
    static TRACKER: RefCell<Tracker> = const { RefCell::new(Tracker::new()) };
    static LAST: Cell<Decision> = const { Cell::new(Decision::None) };
}

fn note(decision: Decision) {
    LAST.with(|last| last.set(decision));
}

/// The decision taken by the last display frame composed on this thread.
pub fn last_decision() -> Decision {
    LAST.with(Cell::get)
}

/// Work estimate of the last lowered scene on this thread.
#[cfg(test)]
pub fn last_work() -> f64 {
    LAST_WORK.with(Cell::get)
}

/// GPU frames rendered by this process.
pub fn gpu_frames() -> u64 {
    FRAMES.load(Ordering::Relaxed)
}

/// The shell's policy with the route on, for benches (`VEXEL_GPU=0` and the
/// latch still apply).
pub fn set_gpu_display_policy() {
    OVERRIDE.with(|o| o.set(3));
}

/// Forces the route on or off (`None`: follow the shell's opt-in). Forced on
/// skips the cost and cache heuristics so tests see every eligible frame.
pub fn set_gpu_display(force: Option<bool>) {
    OVERRIDE.with(|o| {
        o.set(match force {
            None => 0,
            Some(false) => 1,
            Some(true) => 2,
        });
    });
}

/// What a shell status line shows: `VEXEL_GPU: on|off|latched`.
pub fn status() -> &'static str {
    if lock_session().latch.is_off() {
        "VEXEL_GPU: latched off"
    } else if active() {
        "VEXEL_GPU: on"
    } else {
        "VEXEL_GPU: off"
    }
}

/// [`super::parse_launch`] for the shell binary: also opts the process in to
/// the GPU display route.
pub fn app_launch<I>(args: I) -> super::Launch
where
    I: IntoIterator<Item = std::ffi::OsString>,
{
    OPTED_IN.store(true, Ordering::SeqCst);
    super::parse_launch(args)
}

fn env_on() -> bool {
    use std::sync::OnceLock;
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| env_enables_gpu(std::env::var(ENV_SWITCH).ok().as_deref()))
}

fn active() -> bool {
    let wanted = match OVERRIDE.with(Cell::get) {
        1 => false,
        2 | 3 => true,
        _ => OPTED_IN.load(Ordering::SeqCst),
    };
    wanted && env_on() && !lock_session().latch.is_off()
}

/// Session error accounting.
#[derive(Debug)]
pub(super) struct Session {
    pub latch: FallbackLatch,
}

impl Session {
    pub(super) const fn new() -> Self {
        Self {
            latch: FallbackLatch::new(),
        }
    }

    /// Counts a runtime error; logs to the debug profile, once more when the
    /// latch trips.
    pub(super) fn error(&mut self, _error: GpuError) {
        let was_off = self.latch.is_off();
        self.latch.record_error();
        crate::profile::record("gpu.error", 1.0);
        if !was_off && self.latch.is_off() {
            crate::profile::record("gpu.latched", 1.0);
        }
    }
}

fn lock_session() -> std::sync::MutexGuard<'static, Session> {
    SESSION.lock().unwrap_or_else(|p| p.into_inner())
}

/// Clears the process latch (tests).
pub fn reset_latch() {
    lock_session().latch = FallbackLatch::new();
}

/// Renders `scene` through `rasterizer`; `None` means "use the CPU". Runtime
/// errors are counted in `session`.
pub(super) fn render_checked<B: ComputeBackend>(
    rasterizer: &mut GpuRasterizer<B>,
    scene: &GpuScene,
    session: &mut Session,
) -> Option<Vec<u8>> {
    match rasterizer.render_timed(scene, Output::Straight) {
        Ok((pixels, times)) => {
            crate::profile::record("gpu.encode", times.encode.as_secs_f64() * 1000.0);
            note(Decision::Gpu);
            Some(pixels)
        }
        Err(RenderError::Unsupported(reason)) => {
            note(Decision::Unsupported(reason));
            None
        }
        Err(RenderError::Gpu(error)) => {
            session.error(error);
            note(Decision::Error);
            None
        }
    }
}

#[cfg(target_os = "macos")]
type Rasterizer = GpuRasterizer<super::gpu_adapter::MetalBackend>;

enum Lane {
    Unborn,
    Absent,
    #[cfg(target_os = "macos")]
    Ready(Box<Rasterizer>),
}

thread_local! {
    // Metal buffers are not `Send`: device and rasterizer live on the one
    // worker thread that uses them.
    static LANE: RefCell<Lane> = const { RefCell::new(Lane::Unborn) };
}

fn with_lane(scene: &GpuScene) -> Option<Vec<u8>> {
    LANE.with(|lane| {
        let mut lane = lane.borrow_mut();
        #[cfg(target_os = "macos")]
        if matches!(*lane, Lane::Unborn) {
            *lane = match super::gpu_adapter::MetalBackend::new() {
                Some(backend) => Lane::Ready(Box::new(GpuRasterizer::new(backend))),
                None => Lane::Absent,
            };
        }
        match &mut *lane {
            #[cfg(target_os = "macos")]
            Lane::Ready(rasterizer) => render_checked(rasterizer, scene, &mut lock_session()),
            _ => {
                let _ = scene;
                note(Decision::NoDevice);
                None
            }
        }
    })
}

/// Cheap scan of the raw scene before any work: the first thing the lowering
/// cannot take, so CPU-only scenes pay almost nothing.
fn prescan<V: ViewportRead, A: Copy>(scene: &SceneView<'_, V, A>) -> Option<&'static str> {
    if scene.shapes.is_empty() {
        return Some("empty");
    }
    if scene.groups.iter().any(|g| {
        g.filter.is_some()
            || g.mask.is_some()
            || g.blend_mode != 0
            || !(0.0..=1.0).contains(&g.opacity)
    }) {
        return Some("group");
    }
    for shape in scene.shapes {
        if shape.image.is_some() {
            return Some("image");
        }
        if let Some(paint) = shape.stroke_paint.and_then(|i| scene.stroke_paints.get(i))
            && (paint.pattern.is_some()
                || !matches!(
                    paint.kind,
                    PaintKind::Solid | PaintKind::Linear | PaintKind::Radial
                ))
        {
            return Some("pattern");
        }
        if let Some(paint) = shape.fill_paint.and_then(|i| scene.fill_paints.get(i))
            && (paint.pattern.is_some()
                || !matches!(
                    paint.kind,
                    PaintKind::Solid | PaintKind::Linear | PaintKind::Radial
                ))
        {
            return Some("pattern");
        }
    }
    None
}

/// Lowers and classifies; the lowered scene is returned when supported.
fn judge(
    entries: &[Entry<'_>],
    groups: &[vexel_scene::OpacityGroup],
    clips: &[vexel_scene::Clip],
    geometry: &Geo,
    tile: &Tile,
) -> (Verdict<UnsupportedReason>, Option<GpuScene>) {
    let Some(Lowered {
        scene,
        work,
        extra_entries,
    }) = lower(
        entries,
        groups,
        clips,
        geometry,
        tile.width(),
        tile.height(),
    )
    else {
        return (Verdict::Cpu("lowering"), None);
    };
    match classify(&scene) {
        Capability::Unsupported(reason) => (Verdict::Unsupported(reason), None),
        _ => (
            Verdict::Gpu {
                work,
                extra_entries,
            },
            Some(scene),
        ),
    }
}

/// Tries the GPU route for the root scene. `Some` means `tile` holds the
/// finished frame; `None` leaves `tile` untouched for the CPU renderer.
pub(super) fn try_render<V: ViewportRead, A: Copy>(
    scene: &SceneView<'_, V, A>,
    view: ViewBox,
    aspect: Option<PreserveAspect>,
    tile: &mut Tile,
    cache: &mut DiffCache,
) -> Option<Drawn> {
    if !cache.gpu_allowed() {
        return None;
    }
    if !active() {
        note(Decision::Off);
        return None;
    }
    let cold = cache.gpu_candidate();
    if let Some(why) = prescan(scene) {
        note(Decision::Cpu(why));
        return None;
    }
    let entries = prepare(scene).ok()?;
    let geometry = geo(view, aspect, tile);
    let (w, h) = (tile.width(), tile.height());
    let forced = forced();
    let cur: Vec<&OwnedShape> = entries
        .iter()
        .filter_map(|e| match e {
            Entry::Draw(p) => Some(&p.shape),
            Entry::Skip(_) => None,
        })
        .collect();
    let mut lowered = None;
    let (route, load) = TRACKER.with(|t| {
        let t = &mut *t.borrow_mut();
        let (dirty_px, dirty_shapes) = dirty(&t.prev, &cur, &geometry, w, h);
        let edges: usize = cur.iter().map(|s| s.edges.len()).sum();
        t.prev = if edges <= MAX_TRACKED_EDGES {
            cur.iter().map(|s| (*s).clone()).collect()
        } else {
            Vec::new()
        };
        let reuse = !forced
            && matches!(t.verdict, Some((n, age, _)) if n == entries.len() && age < VERDICT_TICKS);
        let verdict = if reuse {
            let (n, age, v) = t.verdict.take()?;
            t.verdict = Some((n, age + 1, v));
            v
        } else {
            let (v, scene) = judge(&entries, scene.groups, scene.clips, &geometry, tile);
            lowered = scene;
            t.verdict = Some((entries.len(), 0, v));
            v
        };
        let (work, extra_entries) = match verdict {
            Verdict::Gpu {
                work,
                extra_entries,
            } => (work, extra_entries),
            Verdict::Cpu(why) => {
                note(Decision::Cpu(why));
                t.policy.force_cpu();
                t.last_gpu = false;
                return None;
            }
            Verdict::Unsupported(reason) => {
                note(Decision::Unsupported(reason));
                t.policy.force_cpu();
                t.last_gpu = false;
                return None;
            }
        };
        LAST_WORK.with(|w| w.set(work));
        let load = Load {
            canvas_px: f64::from(w) * f64::from(h),
            work,
            entries: cur.len(),
            extra_entries,
            dirty_px,
            dirty_shapes,
            warm: !cold && !t.last_gpu && !cache.disabled(),
            undiffable: cache.disabled(),
        };
        let route = if forced {
            Route::Gpu
        } else {
            let estimates = t.policy.estimate(&load);
            t.policy.decide(estimates)
        };
        Some((route, load))
    })?;
    if route == Route::Cpu {
        TRACKER.with(|t| {
            let mut t = t.borrow_mut();
            t.pending_cpu = Some(load);
            t.last_gpu = false;
        });
        note(Decision::Cpu(if cold { "light" } else { "diff" }));
        return None;
    }
    let gpu_scene = match lowered {
        Some(s) => s,
        None => match judge(&entries, scene.groups, scene.clips, &geometry, tile) {
            (_, Some(s)) => s,
            (Verdict::Unsupported(r), None) => {
                note(Decision::Unsupported(r));
                TRACKER.with(|t| t.borrow_mut().policy.force_cpu());
                return None;
            }
            _ => {
                note(Decision::Cpu("lowering"));
                TRACKER.with(|t| t.borrow_mut().policy.force_cpu());
                return None;
            }
        },
    };
    let started = Instant::now();
    let Some(pixels) = with_lane(&gpu_scene) else {
        TRACKER.with(|t| {
            let mut t = t.borrow_mut();
            t.policy.force_cpu();
            t.last_gpu = false;
        });
        return None;
    };
    if pixels.len() != tile.rgba().len() {
        return None;
    }
    tile.rgba_mut().copy_from_slice(&pixels);
    let ms = started.elapsed().as_secs_f64() * 1000.0;
    TRACKER.with(|t| {
        let mut t = t.borrow_mut();
        t.policy.observe_gpu(ms, &load);
        t.last_gpu = true;
        t.pending_cpu = None;
    });
    FRAMES.fetch_add(1, Ordering::Relaxed);
    cache.invalidate();
    let shapes = entries
        .into_iter()
        .filter_map(|entry| match entry {
            Entry::Draw(prepared) => Some(prepared.shape),
            Entry::Skip(_) => None,
        })
        .collect();
    Some(Drawn {
        shapes,
        skipped: Vec::new(),
    })
}

/// Counts one runtime error on the process session (tests).
#[cfg(test)]
pub(super) fn record_error_for_test() {
    lock_session().error(GpuError::CommandFailed);
}
