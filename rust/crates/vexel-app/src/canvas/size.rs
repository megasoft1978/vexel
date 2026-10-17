//! How large the cached compose of the artboard must be for a zoom.
//!
//! The compose is the artboard rendered at `zoom * backing` pixels per document
//! pixel and is resampled for presentation. It is only redone when the wanted
//! resolution leaves a band around the cached one, so ordinary zooming and
//! panning never recompose.

/// Largest compose, in pixels (64 MiB of RGBA) and per axis.
pub const MAX_COMPOSE_PIXELS: f64 = 16.0 * 1024.0 * 1024.0;
pub const MAX_COMPOSE_AXIS: f64 = 8192.0;
/// The cache is kept while the wanted resolution is within
/// `[cached / SHRINK_LIMIT, cached * GROW_LIMIT]`.
pub const GROW_LIMIT: f64 = 2.0;
pub const SHRINK_LIMIT: f64 = 4.0;

/// Device pixel size of the artboard at `zoom` on a backing of `backing` pixels
/// per point. Shared by the compose and the presenter so both agree exactly.
pub fn artboard_pixels(natural: (f64, f64), zoom: f64, backing: u32) -> (u32, u32) {
    let r = zoom * f64::from(backing);
    let px = |n: f64| (n * r).round().clamp(1.0, 1.0e6) as u32;
    (px(natural.0), px(natural.1))
}

/// True when a frame of `frame` pixels is the artboard `natural` at some
/// uniform resolution (so it can be stretched over the artboard undistorted).
pub fn frame_matches(frame: (u32, u32), natural: (f64, f64)) -> bool {
    if frame.0 == 0 || frame.1 == 0 || !(natural.0 > 0.0 && natural.1 > 0.0) {
        return false;
    }
    let (rx, ry) = (
        f64::from(frame.0) / natural.0,
        f64::from(frame.1) / natural.1,
    );
    let tolerance = (1.0 / f64::from(frame.0.min(frame.1))).max(0.02);
    (rx / ry - 1.0).abs() <= tolerance
}

/// The compose size to request. `current` is the size of the cache now.
pub fn target_compose_size(
    natural: (f64, f64),
    zoom: f64,
    backing: u32,
    current: Option<(u32, u32)>,
) -> (u32, u32) {
    let wanted = zoom * f64::from(backing);
    if let Some(size) = current.filter(|s| frame_matches(*s, natural)) {
        let cached = f64::from(size.0) / natural.0;
        if wanted <= cached * GROW_LIMIT && wanted >= cached / SHRINK_LIMIT {
            return size;
        }
    }
    let (w, h) = (natural.0 * wanted, natural.1 * wanted);
    let limit = (MAX_COMPOSE_AXIS / w)
        .min(MAX_COMPOSE_AXIS / h)
        .min((MAX_COMPOSE_PIXELS / (w * h)).sqrt())
        .min(1.0);
    let px = |v: f64| (v * limit).round().max(1.0) as u32;
    (px(w), px(h))
}
