//! One shell step that keeps the stage, the playback driver and the timeline
//! text in step with the session document for a given stage size. The event
//! loop calls it every iteration; tests call it headlessly.

use super::driver::{Driver, TickOutcome};
use super::playback::LoopClock;
use crate::compose::DocumentView;
use crate::session::AppDocument;

/// Starts the compose when the revision or stage size moved, decodes the
/// animation for the revision (so the loop length is known before the static
/// compose lands), and presents the driver frame once the static compose of
/// this revision and stage is on screen. True when the screen needs a redraw.
pub fn sync<C: LoopClock>(
    driver: &mut Driver<C>,
    app: &AppDocument,
    view: &mut DocumentView,
    stage: (u32, u32),
) -> bool {
    let mut dirty = false;
    let revision = app.revision();
    if revision > 0 && !view.is_current(revision, stage) {
        if let Ok(bytes) = app.source_bytes() {
            let name = app.display_name();
            view.set_name(name.as_deref().unwrap_or("Untitled"));
            view.show(&bytes, revision, stage);
            dirty = true;
        }
    }
    driver.set_size(stage);
    if revision > 0 && !driver.knows(revision) {
        if let Ok(bytes) = app.source_bytes() {
            driver.sync(revision, &bytes);
            dirty = true;
        }
    }
    if revision > 0 && view.is_current(revision, stage) && view.composed().is_some() {
        if !view.animated_shown() {
            driver.invalidate();
        }
        if let TickOutcome::Frame(animated) = driver.tick(revision, &[]) {
            if view.present_animated(animated.revision, animated.frame) {
                dirty = true;
            } else {
                driver.invalidate();
            }
        }
    }
    dirty
}
