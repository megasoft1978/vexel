#![forbid(unsafe_code)]

mod file_flow;
mod file_ui;
mod model;
mod preview;
mod preview_app;
mod project_session;
mod render;

use file_flow::FlowOutcome;
use file_ui::FileUi;
use model::{Layout, Mode, Model};
use std::{num::NonZeroUsize, thread, time::Duration};
use vexel_platform_apple::{
    MainThread, NativeApp, NativePanels, PointerKind, WindowConfig, WindowEvent,
};

fn main() {
    let preview = std::env::args_os()
        .skip(1)
        .any(|arg| arg == "--mcp-preview");
    let result = if preview { preview_app::run() } else { run() };
    if let Err(message) = result {
        eprintln!("Vexel shell stopped: {message}");
    }
}
fn run() -> Result<(), &'static str> {
    let main_thread =
        MainThread::acquire().map_err(|_| "launch must begin on the macOS main thread")?;
    let mut app = NativeApp::start(main_thread).map_err(|_| "macOS application startup failed")?;
    let config = WindowConfig {
        title: "Vexel".to_owned(),
        width_points: 1280,
        height_points: 820,
        resizable: true,
        initially_hidden: false,
    };
    let mut window = app
        .create_window(config)
        .map_err(|_| "could not create the Vexel window")?;
    window
        .clear_text_input_state()
        .map_err(|_| "could not establish the shell text-input state")?;
    let (mut logical, mut scale, mut geometry_valid, initial_status) =
        match window.surface_metrics() {
            Ok((points, pixels, scale)) => {
                let logical = checked_points(points.width, points.height).unwrap_or((1280, 820));
                let valid = model::checked_pixel_geometry(logical, scale as u32)
                    == Some((pixels.width, pixels.height))
                    && Layout::new(logical.0, logical.1, Mode::Draw).is_some();
                (
                    logical,
                    scale,
                    valid,
                    if valid {
                        "No document open"
                    } else {
                        "Initial window geometry is unsupported"
                    },
                )
            }
            Err(_) => (
                (1280, 820),
                1.0,
                false,
                "Initial window geometry is unsupported",
            ),
        };
    let dark = window.is_dark_appearance().unwrap_or(true);
    let mut model = Model::new(dark);
    model.status = initial_status;
    let mut files =
        FileUi::new(NativePanels::new(main_thread)).ok_or("blank document unavailable")?;
    files
        .install(&mut app, window.id())
        .map_err(|_| "could not install the File menu")?;
    let mut dirty = true;
    let event_budget = NonZeroUsize::new(128).ok_or("invalid event budget")?;
    let mut closing = false;
    while !closing {
        if dirty || model.dirty {
            let (draw_size, draw_scale) = if geometry_valid {
                (logical, scale as u32)
            } else {
                safe_render_geometry(logical, scale)
            };
            match render::render(draw_size.0, draw_size.1, draw_scale, &model) {
                Ok(frame) => {
                    window
                        .present_rgba(frame)
                        .map_err(|_| "native frame presentation failed")?;
                    window
                        .set_title(files.title())
                        .map_err(|_| "could not update the Vexel window title")?;
                }
                Err(problem) => {
                    eprintln!("Vexel redraw rejected safely: {problem}");
                    model.status = "Redraw unavailable: unsupported window geometry";
                    window
                        .set_title("Vexel — Redraw unavailable".to_owned())
                        .map_err(|_| "redraw failed and the window status could not be updated")?;
                }
            }
            dirty = false;
            model.dirty = false;
        }
        let batch = app
            .poll_events(window.id(), event_budget)
            .map_err(|_| "native event polling failed")?;
        for event in batch.events {
            match event {
                WindowEvent::Open => dirty = true,
                WindowEvent::Focus(focused) => {
                    if focused {
                        window
                            .clear_text_input_state()
                            .map_err(|_| "could not restore the shell text-input state")?;
                    }
                    dirty = true;
                }
                WindowEvent::AppearanceChanged { dark } => {
                    model.dark = dark;
                    dirty = true;
                }
                WindowEvent::Resize { points, pixels } => {
                    if let Some(next) = checked_points(points.width, points.height) {
                        logical = next;
                        dirty = true;
                        geometry_valid = model::checked_pixel_geometry(logical, scale as u32)
                            == Some((pixels.width, pixels.height))
                            && Layout::new(logical.0, logical.1, model.mode).is_some();
                        model.status = if geometry_valid {
                            "No document open"
                        } else {
                            "Resize rejected: unsupported pixel geometry"
                        };
                    } else {
                        geometry_valid = false;
                        dirty = true;
                        model.status = "Resize rejected: invalid point geometry";
                    }
                }
                WindowEvent::BackingScaleChanged {
                    scale: next_scale,
                    pixels,
                } => {
                    if next_scale.is_finite()
                        && (1.0..=3.0).contains(&next_scale)
                        && next_scale.fract() == 0.0
                        && model::checked_pixel_geometry(logical, next_scale as u32)
                            == Some((pixels.width, pixels.height))
                    {
                        scale = next_scale;
                        geometry_valid = Layout::new(logical.0, logical.1, model.mode).is_some();
                        model.status = if geometry_valid {
                            "No document open"
                        } else {
                            "Resize rejected: window is too small for the shell"
                        };
                        dirty = true;
                    } else {
                        geometry_valid = false;
                        dirty = true;
                        model.status = "Backing scale rejected: unsupported pixel geometry";
                    }
                }
                WindowEvent::KeyDown {
                    code, modifiers, ..
                } => {
                    if let Some(outcome) = files.key(code, modifiers) {
                        apply_file_outcome(outcome, &mut model, &files, &app, &mut dirty);
                    } else if model.key(code) {
                        dirty = true;
                    }
                }
                WindowEvent::MenuCommand(tag) => {
                    if let Some(outcome) = files.menu(tag) {
                        apply_file_outcome(outcome, &mut model, &files, &app, &mut dirty);
                    }
                }
                WindowEvent::Pointer(pointer)
                    if pointer.kind == PointerKind::Down && pointer.button == Some(0) =>
                {
                    if pointer_accepts_mode(pointer.button)
                        && pointer.logical.0 >= 0
                        && pointer.logical.1 >= 0
                    {
                        if let Some(layout) = Layout::new(logical.0, logical.1, model.mode) {
                            if model.pointer(
                                &layout,
                                (pointer.logical.0 as u32, pointer.logical.1 as u32),
                            ) {
                                dirty = true;
                            }
                        }
                    }
                }
                WindowEvent::CloseRequested => {
                    if files.close() == FlowOutcome::CloseWindow {
                        closing = true;
                        break;
                    }
                }
                WindowEvent::QueueFault(_) => return Err("native input queue reported a fault"),
                _ => {}
            }
        }
        if !batch.pending {
            thread::sleep(Duration::from_millis(16));
        }
    }
    app.destroy_window(window.id())
        .map_err(|_| "native window shutdown failed")?;
    app.shutdown()
        .map_err(|_| "native application shutdown failed")
}
fn apply_file_outcome(
    outcome: FlowOutcome,
    model: &mut Model,
    files: &FileUi<NativePanels>,
    app: &NativeApp,
    dirty: &mut bool,
) {
    if let Some(status) = outcome.status() {
        model.status = status;
    }
    let _ = files.refresh_menu(app);
    *dirty = true;
}
fn pointer_accepts_mode(button: Option<u8>) -> bool {
    button == Some(0)
}
fn checked_points(width: f64, height: f64) -> Option<(u32, u32)> {
    if !width.is_finite()
        || !height.is_finite()
        || width < 1.0
        || height < 1.0
        || width > 4096.0
        || height > 4096.0
    {
        return None;
    }
    let w = width.round() as u32;
    let h = height.round() as u32;
    ((w as f64 - width).abs() < 0.01 && (h as f64 - height).abs() < 0.01).then_some((w, h))
}

fn safe_render_geometry(points: (u32, u32), scale: f64) -> ((u32, u32), u32) {
    let safe_scale = if scale.is_finite() && (1.0..=3.0).contains(&scale) && scale.fract() == 0.0 {
        scale as u32
    } else {
        1
    };
    let max_points = (4096 / safe_scale).max(440);
    let width = points.0.clamp(440, max_points);
    let height = points.1.clamp(300, max_points);
    ((width, height), safe_scale)
}

#[cfg(test)]
mod tests {
    use super::pointer_accepts_mode;
    #[test]
    fn only_primary_button_can_activate_modes() {
        assert!(pointer_accepts_mode(Some(0)));
        assert!(!pointer_accepts_mode(Some(1)));
        assert!(!pointer_accepts_mode(None));
    }
}
