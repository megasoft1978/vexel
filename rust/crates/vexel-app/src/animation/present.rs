//! Preview-window raster through the real pipeline (decode, `evaluate_frame`,
//! `vexel-render`). The decoded scene is cached per source on the UI thread.

use super::frame::{AnimatedScene, AnimationError, decode, over_white};
use std::cell::RefCell;
use std::sync::atomic::AtomicBool;

const SIDE: usize = 512;

thread_local! {
    static CACHE: RefCell<Option<(Vec<u8>, Result<AnimatedScene, AnimationError>)>> =
        const { RefCell::new(None) };
}

/// Opaque 512x512 RGBA (composited over white) of `source` at `seconds`.
/// A document with nothing drawable is a blank white canvas; every other
/// failure is returned.
pub fn raster_at(source: &[u8], seconds: f64) -> Result<Vec<u8>, &'static str> {
    CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if cache.as_ref().is_none_or(|(cached, _)| cached != source) {
            *cache = Some((source.to_vec(), decode(source)));
        }
        let Some((_, scene)) = cache.as_ref() else {
            return Err("preview frame rendering failed");
        };
        match scene {
            Err(AnimationError::NotDrawable) => Ok(vec![255; SIDE * SIDE * 4]),
            Err(_) => Err("preview frame rendering failed"),
            Ok(scene) => {
                let (width, height) = scene.size();
                if (width as usize, height as usize) != (SIDE, SIDE) {
                    return Err("preview frame rendering failed");
                }
                let time = if scene.loop_seconds() > 0.0 {
                    seconds
                } else {
                    0.0
                };
                let tile = scene
                    .render(time, &AtomicBool::new(false))
                    .map_err(|_| "preview frame rendering failed")?;
                Ok(over_white(&tile))
            }
        }
    })
}
