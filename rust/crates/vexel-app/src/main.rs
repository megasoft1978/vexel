#![forbid(unsafe_code)]

mod animation;
mod compose;
mod file_flow;
mod file_ui;
mod model;
mod preview;
mod preview_app;
mod project_session;
mod render;
mod session;
mod tools;

use animation::{
    driver::{Driver, TickOutcome},
    job::ExportJob,
    keys::{self, playback_key},
    playback::MonotonicClock,
    strip::{self, Strip, StripPointer},
};
use file_flow::FlowOutcome;
use file_ui::FileUi;
use model::{Layout, Mode, Model, Target};
use preview::UiRequest;
use session::{EditAction, McpHost, McpOutcome, edit_shortcut};
use std::{
    num::NonZeroUsize,
    sync::mpsc::{self, Receiver},
    thread,
    time::Duration,
};
use tools::{Key, Outcome, PointerInput, ToolController};
use vexel_platform_apple::{
    MainThread, Modifiers, NativeApp, NativePanels, PointerEvent, PointerKind, WindowConfig,
    WindowEvent,
};

/// Size of the blank document the MCP preview starts from.
const MCP_CANVAS: (u32, u32) = (preview::CANVAS_SIZE, preview::CANVAS_SIZE);

fn main() {
    let result = match compose::parse_launch(std::env::args_os().skip(1)) {
        // One process, one window: the shell also serves the MCP preview
        // requests through the shared document session.
        compose::Launch::McpPreview => run(None, true),
        compose::Launch::Shell { open } => run(open, false),
        compose::Launch::Usage(message) => Err(message),
    };
    if let Err(message) = result {
        eprintln!("Vexel shell stopped: {message}");
    }
}
fn run(mut pending_open: Option<std::path::PathBuf>, mcp: bool) -> Result<(), &'static str> {
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
    let panels = NativePanels::new(main_thread);
    let mut files = if mcp {
        FileUi::with_size(panels, MCP_CANVAS)
    } else {
        FileUi::new(panels)
    }
    .ok_or("blank document unavailable")?;
    files
        .install(&mut app, window.id())
        .map_err(|_| "could not install the File menu")?;
    let mut doc = compose::DocumentView::new();
    let mut tools = ToolController::new();
    let mcp_rx: Option<Receiver<UiRequest>> = if mcp {
        let (tx, rx) = mpsc::sync_channel(8);
        preview_app::start_server(tx)?;
        Some(rx)
    } else {
        None
    };
    let mut mcp_host = McpHost::new();
    let mut export: Option<ExportJob> = None;
    let mut driver = Driver::new(MonotonicClock::new());
    let mut strip = Strip::default();
    let mut was_playing = false;
    let mut menu_revision = files.document().revision();
    let mut dirty = true;
    let event_budget = NonZeroUsize::new(128).ok_or("invalid event budget")?;
    let mut closing = false;
    while !closing {
        if let Some(done) = export.as_mut().and_then(ExportJob::poll) {
            export = None;
            model.status = if done.starts_with("Exported") {
                "Exported GIF"
            } else {
                "GIF export failed"
            };
            dirty = true;
        }
        if export.is_none() {
            if let Some(UiRequest { operation, reply }) =
                mcp_rx.as_ref().and_then(|rx| rx.try_recv().ok())
            {
                match mcp_host.handle_with(files.document_mut(), operation, &mut driver) {
                    Ok(McpOutcome::Reply(json)) => {
                        let _ = reply.send(Ok(json));
                    }
                    Ok(McpOutcome::Export {
                        source,
                        path,
                        fps,
                        duration,
                    }) => {
                        let policy = vexel_mcp_access::AccessPolicy::from_environment();
                        match ExportJob::spawn(
                            source,
                            policy,
                            path.into(),
                            fps,
                            duration,
                            reply.clone(),
                        ) {
                            Ok(job) => export = Some(job),
                            Err(message) => {
                                let _ = reply.send(Err(message));
                            }
                        }
                    }
                    Err(message) => {
                        let _ = reply.send(Err(message));
                    }
                }
                dirty = true;
            }
        }
        if let Some(layout) = Layout::new(logical.0, logical.1, model.mode) {
            let stage = (layout.stage.width, layout.stage.height);
            if let Some(path) = pending_open.take() {
                doc.open(files.document_mut(), &path, stage);
                dirty = true;
            }
            // Render trigger: the session revision (or stage size) moved.
            let revision = files.document().revision();
            if revision > 0 && !doc.is_current(revision, stage) {
                if let Ok(bytes) = files.document().source_bytes() {
                    let name = files.document().display_name();
                    doc.set_name(name.as_deref().unwrap_or("Untitled"));
                    doc.show(&bytes, revision, stage);
                    dirty = true;
                }
            }
            // Playback: the driver renders the current loop time at stage size
            // once the static compose of this revision is on screen.
            driver.set_size(stage);
            if revision > 0 && doc.is_current(revision, stage) && doc.composed().is_some() {
                if !doc.animated_shown() {
                    driver.invalidate();
                }
                if !driver.knows(revision) {
                    let time = driver.time();
                    let _ = files.document().request_frame(&mut driver, time);
                }
                if let TickOutcome::Frame(animated) = driver.tick(revision, &[]) {
                    if doc.present_animated(animated.revision, animated.frame) {
                        dirty = true;
                    } else {
                        driver.invalidate();
                    }
                }
            }
            if driver.is_playing() != was_playing {
                was_playing = driver.is_playing();
                dirty = true;
            }
        }
        if files.document().revision() != menu_revision {
            menu_revision = files.document().revision();
            let _ = files.refresh_menu(&app);
            dirty = true;
        }
        if doc.poll() {
            dirty = true;
        }
        if dirty || model.dirty {
            let (draw_size, draw_scale) = if geometry_valid {
                (logical, scale as u32)
            } else {
                safe_render_geometry(logical, scale)
            };
            match render::render(draw_size.0, draw_size.1, draw_scale, &model) {
                Ok(frame) => {
                    let frame = match Layout::new(draw_size.0, draw_size.1, model.mode) {
                        Some(layout) => {
                            let frame = compose::blit_document(frame, &doc, &layout, model.dark);
                            match strip::layout(layout.stage).filter(|_| driver.strip_visible()) {
                                Some(bar) => strip::draw(frame, &bar, &driver, model.dark),
                                None => frame,
                            }
                        }
                        None => frame,
                    };
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
                    if let Some(action) = edit_shortcut(code, modifiers) {
                        let _ = match action {
                            EditAction::Undo => files.document_mut().undo(),
                            EditAction::Redo => files.document_mut().redo(),
                        };
                        dirty = true;
                    } else if let Some(outcome) = files.key(code, modifiers) {
                        apply_file_outcome(
                            outcome, &mut model, &files, &app, &mut dirty, &mut tools,
                        );
                    } else if playback_key(code, modifiers)
                        .is_some_and(|key| keys::apply(&mut driver, key, false, tools.owns_keys()))
                    {
                        dirty = true;
                    } else if let Some(key) = tool_key(code, modifiers) {
                        let result = files
                            .document_mut()
                            .edit(|document| tools.key(key, document));
                        dirty |= !matches!(result, Ok(Outcome::Ignored));
                    } else if model.key(code) {
                        dirty = true;
                    }
                }
                WindowEvent::MenuCommand(tag) => {
                    if let Some(outcome) = files.menu(tag) {
                        apply_file_outcome(
                            outcome, &mut model, &files, &app, &mut dirty, &mut tools,
                        );
                    }
                }
                WindowEvent::Pointer(pointer) => {
                    if let Some(layout) = Layout::new(logical.0, logical.1, model.mode) {
                        dirty |= route_pointer(
                            &pointer,
                            &layout,
                            &mut model,
                            &mut doc,
                            &mut files,
                            &mut tools,
                            (&mut driver, &mut strip),
                        );
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
    tools: &mut ToolController,
) {
    if let Some(status) = outcome.status() {
        model.status = status;
    }
    if outcome == FlowOutcome::Opened {
        // Selection ids belong to the replaced document; keep only the tool.
        let tool = tools.tool();
        *tools = ToolController::new();
        tools.set_tool(tool);
    }
    let _ = files.refresh_menu(app);
    *dirty = true;
}
fn pointer_accepts_mode(button: Option<u8>) -> bool {
    button == Some(0)
}

/// Escape, Delete and Return for the tools; Command/Control/Option chords are not tool keys.
fn tool_key(code: u16, modifiers: Modifiers) -> Option<Key> {
    if modifiers.0 & (Modifiers::COMMAND | Modifiers::CONTROL | Modifiers::OPTION) != 0 {
        return None;
    }
    match code {
        53 => Some(Key::Escape),
        51 | 117 => Some(Key::Delete),
        36 | 76 => Some(Key::Enter),
        _ => None,
    }
}

/// Routes one pointer event: mode and tool-rail clicks, the click status line,
/// and (in Draw mode) stage gestures into the tools, whose edits go through
/// the session. True when the window needs a redraw.
fn route_pointer(
    pointer: &PointerEvent,
    layout: &Layout,
    model: &mut Model,
    doc: &mut compose::DocumentView,
    files: &mut FileUi<NativePanels>,
    tools: &mut ToolController,
    (driver, strip): (&mut Driver<MonotonicClock>, &mut Strip),
) -> bool {
    let primary = pointer_accepts_mode(pointer.button);
    let mut redraw = false;
    // The timeline strip overlays the stage bottom and takes its pointer events.
    if let Some(bar) = strip::layout(layout.stage).filter(|_| driver.strip_visible()) {
        let kind = match pointer.kind {
            PointerKind::Down if primary => Some(StripPointer::Down),
            PointerKind::Move => Some(StripPointer::Move),
            PointerKind::Up if primary => Some(StripPointer::Up),
            _ => None,
        };
        if kind.is_some_and(|kind| strip.pointer(kind, pointer.logical, &bar, driver)) {
            return true;
        }
    }
    let point = (pointer.logical.0 >= 0 && pointer.logical.1 >= 0)
        .then(|| (pointer.logical.0 as u32, pointer.logical.1 as u32));
    if pointer.kind == PointerKind::Down && primary {
        if let Some(point) = point {
            if let Some(Target::Tool(tool)) = layout.hit_test(point) {
                tools.set_tool(tool);
                redraw = true;
            }
            redraw |= model.pointer(layout, point) || doc.pointer(layout.stage, point);
        }
    }
    if model.mode != Mode::Draw {
        return redraw;
    }
    let map = doc
        .composed()
        .filter(|c| c.frame.width == layout.stage.width && c.frame.height == layout.stage.height)
        .map(|c| c.map);
    let Some(map) = map else {
        return redraw;
    };
    let canvas = point.and_then(|p| compose::DocumentView::canvas_point(layout.stage, p));
    let (x, y) = canvas.unwrap_or((-1.0, -1.0));
    let mut inputs = Vec::new();
    match pointer.kind {
        PointerKind::Down if primary && canvas.is_some() => {
            inputs.push(PointerInput::Down { x, y });
            if pointer.click_count == 2 {
                inputs.push(PointerInput::DoubleClick { x, y });
            }
        }
        PointerKind::Move => inputs.push(PointerInput::Move { x, y }),
        PointerKind::Up if primary => inputs.push(PointerInput::Up { x, y }),
        _ => {}
    }
    for input in inputs {
        let result = files
            .document_mut()
            .edit(|document| tools.pointer(input, document, &map));
        redraw |= !matches!(result, Ok(Outcome::Ignored));
    }
    redraw
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
