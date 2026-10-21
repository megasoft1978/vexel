//! Window events that drive the canvas: zoom, pan, hover and the zoom
//! controls. One entry point so the shell loop needs a single call.
//!
//! Wired: Cmd+scroll and trackpad pinch (zoom about the pointer), two-finger
//! scroll (pan), Space+drag (pan), Cmd+0 (fit), Cmd+1 (100%), Cmd+= / Cmd+-
//! (zoom step), the zoom cluster, and hover outlines for the select tool.

use super::controls::{ControlKind, controls};
use crate::compose::DocumentView;
use vexel_platform_apple::{Modifiers, PointerKind, WindowEvent};
use vexel_ui::Rect;

const KEY_1: u16 = 18;
const KEY_EQUAL: u16 = 24;
const KEY_MINUS: u16 = 27;
const KEY_0: u16 = 29;
const KEY_SPACE: u16 = 49;
/// Zoom per scroll point with Cmd held (exponential).
const SCROLL_ZOOM: f64 = 0.01;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EventOutcome {
    /// The canvas used the event; the shell must not route it further.
    pub consumed: bool,
    /// The window needs a redraw.
    pub redraw: bool,
}

fn used() -> EventOutcome {
    EventOutcome {
        consumed: true,
        redraw: true,
    }
}

/// Applies `event` to the canvas state of `doc`. `select` is true while the
/// select tool is active in Draw mode (hover outlines only exist then).
pub fn handle_event(
    doc: &mut DocumentView,
    event: &WindowEvent,
    stage: Rect,
    select: bool,
) -> EventOutcome {
    let Some(artboard) = doc.artboard() else {
        return EventOutcome::default();
    };
    let size = (f64::from(stage.width), f64::from(stage.height));
    let local = |logical: (i32, i32)| {
        let p = (
            f64::from(logical.0) - f64::from(stage.x),
            f64::from(logical.1) - f64::from(stage.y),
        );
        (p.0 >= 0.0 && p.1 >= 0.0 && p.0 < size.0 && p.1 < size.1).then_some(p)
    };
    match event {
        WindowEvent::Scroll(scroll) => {
            let Some(p) = local(scroll.logical) else {
                return EventOutcome::default();
            };
            if scroll.modifiers.0 & Modifiers::COMMAND != 0 {
                let factor = (scroll.delta.y * SCROLL_ZOOM).exp().clamp(0.5, 2.0);
                doc.canvas.zoom_by(p, factor);
            } else {
                doc.canvas.pan_by((scroll.delta.x, scroll.delta.y));
            }
            used()
        }
        WindowEvent::Magnify {
            logical, amount, ..
        } => match local(*logical) {
            Some(p) => {
                doc.canvas.zoom_by(p, (1.0 + amount).clamp(0.5, 2.0));
                used()
            }
            None => EventOutcome::default(),
        },
        WindowEvent::KeyDown {
            code, modifiers, ..
        } => {
            let m = modifiers.0;
            let command =
                m & Modifiers::COMMAND != 0 && m & (Modifiers::OPTION | Modifiers::CONTROL) == 0;
            if command {
                let centre = (size.0 / 2.0, size.1 / 2.0);
                match *code {
                    KEY_0 => doc.canvas.set_fit(size, artboard),
                    KEY_1 => doc.canvas.set_actual(size, artboard),
                    KEY_EQUAL => doc.canvas.zoom_step(centre, 1),
                    KEY_MINUS => doc.canvas.zoom_step(centre, -1),
                    _ => return EventOutcome::default(),
                }
                return used();
            }
            if *code == KEY_SPACE && m == 0 {
                doc.canvas.space = true;
            }
            EventOutcome::default()
        }
        WindowEvent::KeyUp { code, .. } if *code == KEY_SPACE => {
            doc.canvas.space = false;
            EventOutcome::default()
        }
        WindowEvent::InputReset { .. } => {
            doc.canvas.space = false;
            doc.canvas.drag = None;
            EventOutcome::default()
        }
        WindowEvent::Pointer(pointer) => {
            let at = local(pointer.logical);
            let cluster = controls(stage.width, stage.height, doc.canvas.view.percent());
            let over_cluster = at
                .zip(cluster.as_ref())
                .is_some_and(|(p, c)| c.contains(p.0, p.1));
            match (pointer.kind, at) {
                (PointerKind::Down, Some(p)) if pointer.button == Some(0) => {
                    if over_cluster {
                        let hit = cluster.as_ref().and_then(|c| c.hit(p.0, p.1));
                        let centre = (size.0 / 2.0, size.1 / 2.0);
                        match hit {
                            Some(ControlKind::ZoomOut) => doc.canvas.zoom_step(centre, -1),
                            Some(ControlKind::ZoomIn) => doc.canvas.zoom_step(centre, 1),
                            Some(ControlKind::Label) => doc.canvas.set_actual(size, artboard),
                            Some(ControlKind::Fit) => doc.canvas.set_fit(size, artboard),
                            None => {}
                        }
                        used()
                    } else if doc.canvas.space {
                        doc.canvas.drag = Some(p);
                        used()
                    } else {
                        EventOutcome::default()
                    }
                }
                (PointerKind::Up, _) if doc.canvas.drag.take().is_some() => used(),
                (PointerKind::Move, _) if doc.canvas.drag.is_some() => {
                    if let (Some(last), Some(now)) =
                        (doc.canvas.drag, local_free(pointer.logical, stage))
                    {
                        doc.canvas.pan_by((now.0 - last.0, now.1 - last.1));
                        doc.canvas.drag = Some(now);
                    }
                    used()
                }
                (PointerKind::Move, _) => {
                    let target = at
                        .filter(|_| select && !over_cluster)
                        .map(|_| (pointer.logical.0 as u32, pointer.logical.1 as u32));
                    EventOutcome {
                        consumed: false,
                        redraw: doc.set_hover(stage, target),
                    }
                }
                _ => EventOutcome::default(),
            }
        }
        _ => EventOutcome::default(),
    }
}

/// Stage-local position without the inside-the-stage check (drags continue
/// outside it).
fn local_free(logical: (i32, i32), stage: Rect) -> Option<(f64, f64)> {
    Some((
        f64::from(logical.0) - f64::from(stage.x),
        f64::from(logical.1) - f64::from(stage.y),
    ))
}
