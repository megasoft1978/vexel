#![forbid(unsafe_code)]

mod mcp_pump;

use mcp_pump::McpPump;
use std::{
    num::NonZeroUsize,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};
use vexel_app::animation::{
    driver::Driver,
    keys::{self, playback_key},
    native_export::NativeExportSource,
    playback::MonotonicClock,
    stage,
    strip::{self, Strip, StripPointer},
};
use vexel_app::chrome::{self, AppState};
use vexel_app::file_flow::{FlowOutcome, ShortcutDedup, action_for_tag};
use vexel_app::file_ui::FileUi;
use vexel_app::model::{self, DragPhase, Layout, Mode, Model, Target, Tool};
use vexel_app::motion::MotionAdapter;
use vexel_app::native_animation::{TimelineSurface, host::NativeAnimationHost};
use vexel_app::native_timeline::interaction::Phase as NativeTimelinePhase;
use vexel_app::preview;
use vexel_app::project_session::shortcut_action;
use vexel_app::session::{McpGrants, edit_shortcut};
use vexel_app::toolbar_actions::{self, Exports};
use vexel_app::tools::{Key, Outcome, PointerInput, ToolController};
use vexel_app::{canvas, compose, overlay, preview_app, render, timeline};
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
        compose::Launch::McpPreview => run(None, Some(McpGrants::none())),
        compose::Launch::McpPreviewGranted(grants) => run(None, Some(grants)),
        compose::Launch::Shell { open } => run(open, None),
        compose::Launch::Usage(message) => Err(message),
    };
    if let Err(message) = result {
        eprintln!("Vexel shell stopped: {message}");
    }
}
fn run(
    mut pending_open: Option<std::path::PathBuf>,
    grants: Option<McpGrants>,
) -> Result<(), &'static str> {
    let mcp = grants.is_some();
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
                    && Model::new(true).layout(logical.0, logical.1).is_some();
                (
                    logical,
                    scale,
                    valid,
                    if valid {
                        ""
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
    let mcp_rx = if mcp {
        let (tx, rx) = mpsc::sync_channel(8);
        preview_app::start_server(tx)?;
        Some(rx)
    } else {
        None
    };
    let mut mcp_pump = McpPump::new(mcp_rx, grants.unwrap_or_default());
    model.mcp = mcp_pump.idle_state();
    let mut exports = Exports::default();
    let mut driver = Driver::new(MonotonicClock::new());
    let mut native = NativeAnimationHost::new(MonotonicClock::new());
    let mut strip = Strip::default();
    let mut was_playing = false;
    let mut native_loop: (u64, Option<f64>) = (u64::MAX, None);
    let mut dedup = ShortcutDedup::new();
    let started = Instant::now();
    let mut menu_revision = files.document().state_revision();
    let mut menu_save_enabled = files.document().is_dirty() || files.document().path().is_none();
    // No platform accessor for reduce-motion yet (gap): constant false.
    let mut motion = MotionAdapter::new(false);
    let mut dirty = true;
    let event_budget = NonZeroUsize::new(128).ok_or("invalid event budget")?;
    let mut closing = false;
    while !closing {
        if let Some(done) = exports.poll() {
            model.status = done;
            dirty = true;
        }
        if mcp_pump.poll(&mut files, &mut native, &mut driver, &mut tools, &mut model) {
            dirty = true;
        }
        if let Some(status) = files.tick_recovery(Instant::now()) {
            model.status = status.user_message();
            dirty = true;
        }
        if let Some(layout) = model.layout(logical.0, logical.1) {
            let stage = doc.compose_target(layout.stage, scale as u32);
            if model.mode != Mode::Animate {
                native.deactivate();
                if native.surface() == TimelineSurface::Native {
                    dirty |= doc.restore_static();
                }
            }
            if let Some(path) = pending_open.take() {
                files.open_startup(&mut doc, &path, stage);
                reset_animation_state(&mut native, &mut driver);
                dirty = true;
            }
            let prepared = native.prepare_document(files.document(), &mut doc);
            dirty |= prepared;
            let stage_changed = if native.surface() == TimelineSurface::Native {
                stage::sync_native(&mut driver, files.document(), &mut doc, stage)
            } else {
                stage::sync(&mut driver, files.document(), &mut doc, stage)
            };
            if stage_changed {
                dirty = true;
            }
            if model.mode == Mode::Animate && native.surface() == TimelineSurface::Native {
                if let Some(composed) = doc.composed() {
                    if let Some(frame) = native.tick(
                        files.document(),
                        (composed.frame.width, composed.frame.height),
                    ) {
                        dirty |= doc.present_animated(files.document().revision(), frame);
                    }
                }
            }
            if driver.is_playing() != was_playing {
                was_playing = driver.is_playing();
                dirty = true;
            }
        }
        let native_snapshot = if model.mode == Mode::Animate {
            native
                .snapshot(
                    files.document(),
                    doc.selected_target_id().map(str::to_owned),
                )
                .ok()
        } else {
            None
        };
        {
            let revision = files.document().revision();
            if revision == 0 {
                if model.timeline.model.open {
                    model.timeline.clear();
                    dirty = true;
                }
            } else if driver.knows(revision) && model.timeline.model.revision != Some(revision) {
                if let Ok(bytes) = files.document().source_bytes() {
                    model
                        .timeline
                        .set_document(revision, &bytes, driver.loop_seconds());
                    dirty = true;
                }
            }
            if model.timeline.model.open && model.timeline.model.sync_playback(&driver) {
                dirty = true;
            }
        }
        let save_enabled = files.document().is_dirty() || files.document().path().is_none();
        if files.document().state_revision() != menu_revision || save_enabled != menu_save_enabled {
            menu_revision = files.document().state_revision();
            menu_save_enabled = save_enabled;
            let _ = files.refresh_menu(&app);
            dirty = true;
        }
        if doc.poll() {
            dirty = true;
        }
        let mut app_state = AppState::from_session(files.document(), &doc);
        if driver.knows(files.document().revision()) {
            app_state.loop_seconds = driver.loop_seconds();
        }
        // Native timelines have no SMIL loop; they still export as GIF.
        let state_revision = files.document().state_revision();
        if native_loop.0 != state_revision {
            native_loop = (
                state_revision,
                NativeExportSource::from_document(files.document())
                    .and_then(|source| source.loop_seconds()),
            );
        }
        if let Some(seconds) = native_loop.1 {
            app_state.loop_seconds = seconds;
        }
        let chrome_text = chrome::chrome_text(&app_state);
        if chrome_text != model.chrome {
            model.chrome = chrome_text;
            dirty = true;
        }
        dirty |= model.sync_history(files.document());
        motion.sync_focus(model.focus_id());
        dirty |= motion.redraw_after().is_some() | motion.poll_tooltip();
        if dirty || model.dirty {
            let (draw_size, draw_scale) = if geometry_valid {
                (logical, scale as u32)
            } else {
                safe_render_geometry(logical, scale)
            };
            match render::render_with(
                draw_size.0,
                draw_size.1,
                draw_scale,
                &model,
                &motion.frame(model.dark, model.mode),
            ) {
                Ok(frame) => {
                    let frame = match model.layout(draw_size.0, draw_size.1) {
                        Some(layout) => {
                            let frame = compose::blit_document(frame, &doc, &layout, model.dark);
                            let bar = strip::layout(layout.stage)
                                .filter(|_| driver.strip_visible() && layout.footer.is_none());
                            let frame = match bar {
                                Some(bar) => strip::draw(frame, &bar, &driver, model.dark),
                                None => frame,
                            };
                            let frame = match (layout.footer, native_snapshot.as_ref()) {
                                (Some(footer), Some(snapshot)) => {
                                    native.paint_frame(frame, footer, snapshot, model.dark)
                                }
                                _ => frame,
                            };
                            overlay::overlay_tooltip(frame, &model, &layout, motion.tooltip())
                        }
                        None => frame,
                    };
                    window
                        .present_rgba(frame)
                        .map_err(|_| "native frame presentation failed")?;
                    window
                        .set_title(model.chrome.title.clone())
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
            if let Some(layout) = model.layout(logical.0, logical.1) {
                let select = model.mode == Mode::Draw && tools.tool() == Tool::Select;
                // Events under an open popover belong to the popover, not the canvas.
                let under = matches!(&event, WindowEvent::Pointer(p)
                    if p.logical.0 >= 0 && p.logical.1 >= 0
                        && model.popover_hit(&layout, (p.logical.0 as u32, p.logical.1 as u32)));
                let used = if under {
                    Default::default()
                } else {
                    canvas::handle_event(&mut doc, &event, layout.stage, select)
                };
                dirty |= used.redraw;
                if used.consumed {
                    // The canvas took the event (pan/zoom/cluster); chrome hover and press
                    // state must still follow the pointer.
                    if let WindowEvent::Pointer(pointer) = &event {
                        let primary =
                            vexel_app::native_animation::host::pointer_accepts_mode(pointer.button);
                        dirty |= motion
                            .pointer_in(&layout, pointer.kind, pointer.logical, primary, &model)
                            .1;
                    }
                    continue;
                }
            }
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
                            && model.layout(logical.0, logical.1).is_some();
                        model.status = if geometry_valid {
                            ""
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
                        geometry_valid = model.layout(logical.0, logical.1).is_some();
                        model.status = if geometry_valid {
                            ""
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
                    let layout = model.layout(logical.0, logical.1);
                    if layout.as_ref().is_some_and(|l| {
                        toolbar_actions::key(
                            l,
                            &mut model,
                            &mut files,
                            &mut exports,
                            code,
                            modifiers,
                        )
                    }) {
                        let _ = files.refresh_menu(&app);
                        dirty = true;
                    } else if let Some(action) = edit_shortcut(code, modifiers) {
                        toolbar_actions::edit(&mut model, files.document_mut(), action);
                        dirty = true;
                    } else if let Some(outcome) =
                        file_key(&mut files, &mut dedup, started.elapsed(), code, modifiers)
                    {
                        apply_file_outcome(
                            outcome, &mut model, &files, &app, &mut dirty, &mut tools, &mut doc,
                        );
                        if outcome == FlowOutcome::Opened {
                            reset_animation_state(&mut native, &mut driver);
                        }
                    } else if model.mode == Mode::Animate
                        && modifiers.0
                            & (Modifiers::COMMAND | Modifiers::CONTROL | Modifiers::OPTION)
                            == 0
                        && vexel_app::native_animation::host::key_for_code(code).is_some_and(
                            |key| {
                                let previous = native.surface();
                                let handled = native
                                    .key(
                                        key,
                                        &mut files,
                                        doc.selected_target_id().map(str::to_owned),
                                    )
                                    .unwrap_or(false);
                                if previous != native.surface() {
                                    doc.restore_static();
                                    driver.pause();
                                    driver.invalidate();
                                }
                                handled
                            },
                        )
                    {
                        dirty = true;
                    } else if model.mode == Mode::Animate
                        && native.surface() == TimelineSurface::Smil
                        && timeline::key(code, modifiers, &mut driver)
                    {
                        dirty = true;
                    } else if playback_key(code, modifiers)
                        .is_some_and(|key| keys::apply(&mut driver, key, false, tools.owns_keys()))
                    {
                        dirty = true;
                    } else if let Some(key) = tool_key(code, modifiers) {
                        if key == Key::Escape {
                            doc.deselect();
                        }
                        let result = files
                            .document_mut()
                            .edit(|document| tools.key(key, document));
                        dirty |= !matches!(result, Ok(Outcome::Ignored));
                    } else if code == 48 && motion.tab(modifiers.0 & Modifiers::SHIFT != 0) {
                        dirty = true;
                    } else if model.key(code) {
                        dirty = true;
                    }
                }
                WindowEvent::MenuCommand(tag) => {
                    if let Some(outcome) = file_menu(&mut files, &mut dedup, started.elapsed(), tag)
                    {
                        apply_file_outcome(
                            outcome, &mut model, &files, &app, &mut dirty, &mut tools, &mut doc,
                        );
                        if outcome == FlowOutcome::Opened {
                            reset_animation_state(&mut native, &mut driver);
                        }
                    }
                }
                WindowEvent::Pointer(pointer) => {
                    if let Some(layout) = model.layout(logical.0, logical.1) {
                        let primary =
                            vexel_app::native_animation::host::pointer_accepts_mode(pointer.button);
                        dirty |= motion
                            .pointer_in(&layout, pointer.kind, pointer.logical, primary, &model)
                            .1;
                        let native_phase = (model.mode == Mode::Animate)
                            .then(|| match pointer.kind {
                                PointerKind::Down if primary => Some(NativeTimelinePhase::Down),
                                PointerKind::Move => Some(NativeTimelinePhase::Move),
                                PointerKind::Up if primary => Some(NativeTimelinePhase::Up),
                                _ => None,
                            })
                            .flatten();
                        let native_handled = native_phase.is_some_and(|phase| {
                            let previous = native.surface();
                            let handled = native
                                .pointer(
                                    phase,
                                    pointer.logical,
                                    layout.footer.unwrap_or_default(),
                                    &mut files,
                                    doc.selected_target_id().map(str::to_owned),
                                )
                                .unwrap_or(false);
                            if previous != native.surface() {
                                doc.restore_static();
                                driver.pause();
                                driver.invalidate();
                            }
                            handled
                        });
                        if native_handled {
                            dirty = true;
                            if pointer.kind == PointerKind::Down {
                                let _ = files.refresh_menu(&app);
                            }
                            continue;
                        }
                        dirty |= route_pointer(
                            &pointer,
                            &layout,
                            &mut model,
                            &mut doc,
                            &mut files,
                            &mut tools,
                            (&mut driver, &mut strip),
                            native.surface(),
                            &mut exports,
                        );
                        if pointer.kind == PointerKind::Down {
                            let _ = files.refresh_menu(&app);
                        }
                    }
                }
                WindowEvent::Scroll(scroll) => {
                    if let Some(footer) = model.layout(logical.0, logical.1).and_then(|l| l.footer)
                    {
                        if native.surface() == TimelineSurface::Smil
                            && let Some(moved) =
                                model.timeline.wheel(footer, scroll.logical, scroll.delta.y)
                        {
                            dirty |= moved;
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
fn file_key(
    files: &mut FileUi<NativePanels>,
    dedup: &mut ShortcutDedup,
    now: Duration,
    code: u16,
    modifiers: Modifiers,
) -> Option<FlowOutcome> {
    let action = shortcut_action(code, modifiers)?;
    if !dedup.allow_key(action, now) {
        return Some(FlowOutcome::Cancelled);
    }
    files.key(code, modifiers)
}

fn file_menu(
    files: &mut FileUi<NativePanels>,
    dedup: &mut ShortcutDedup,
    now: Duration,
    tag: u32,
) -> Option<FlowOutcome> {
    let action = action_for_tag(tag)?;
    if !dedup.allow_menu(action, now) {
        return Some(FlowOutcome::Cancelled);
    }
    files.menu(tag)
}
fn apply_file_outcome(
    outcome: FlowOutcome,
    model: &mut Model,
    files: &FileUi<NativePanels>,
    app: &NativeApp,
    dirty: &mut bool,
    tools: &mut ToolController,
    doc: &mut compose::DocumentView,
) {
    if let Some(status) = outcome.status() {
        model.status = status;
    }
    if outcome == FlowOutcome::Opened {
        doc.deselect();
        // Selection ids belong to the replaced document; keep only the tool.
        let tool = tools.tool();
        *tools = ToolController::new();
        tools.set_tool(tool);
    }
    let _ = files.refresh_menu(app);
    *dirty = true;
}
fn reset_animation_state(
    native: &mut NativeAnimationHost<MonotonicClock>,
    driver: &mut Driver<MonotonicClock>,
) {
    native.document_replaced();
    driver.pause();
    driver.invalidate();
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

fn route_pointer(
    pointer: &PointerEvent,
    layout: &Layout,
    model: &mut Model,
    doc: &mut compose::DocumentView,
    files: &mut FileUi<NativePanels>,
    tools: &mut ToolController,
    (driver, strip): (&mut Driver<MonotonicClock>, &mut Strip),
    surface: TimelineSurface,
    exports: &mut Exports,
) -> bool {
    let primary = vexel_app::native_animation::host::pointer_accepts_mode(pointer.button);
    let mut redraw = false;
    if let Some(footer) = layout
        .footer
        .filter(|_| model.mode == Mode::Animate && surface == TimelineSurface::Smil)
    {
        let phase = match pointer.kind {
            PointerKind::Down if primary => Some(timeline::Phase::Down),
            PointerKind::Move => Some(timeline::Phase::Move),
            PointerKind::Up if primary => Some(timeline::Phase::Up),
            _ => None,
        };
        let tl = timeline::layout::TimelineLayout::new(footer);
        if let Some(out) = phase.map(|p| model.timeline.pointer(p, pointer.logical, &tl, driver)) {
            redraw |= out.redraw;
            if out.consumed {
                return true;
            }
        }
    }
    // The timeline strip overlays the stage bottom (Draw mode) and takes its pointer events.
    if let Some(bar) =
        strip::layout(layout.stage).filter(|_| driver.strip_visible() && layout.footer.is_none())
    {
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
    let phase = match pointer.kind {
        PointerKind::Down if primary => Some(DragPhase::Down),
        PointerKind::Move if model.ui.dragging() => Some(DragPhase::Move),
        PointerKind::Up if model.ui.dragging() => Some(DragPhase::Up),
        _ => None,
    };
    if let Some(phase) = phase {
        let window = (layout.width, layout.height);
        if model.splitter(phase, pointer.logical, layout, window) {
            return true;
        }
    }
    let point = (pointer.logical.0 >= 0 && pointer.logical.1 >= 0)
        .then(|| (pointer.logical.0 as u32, pointer.logical.1 as u32));
    if pointer.kind == PointerKind::Down && primary {
        if let Some(point) = point {
            if let Some(Target::SectionHeader(index @ 5..=20)) =
                layout.hit_test_menus(point, model.ui.mcp_open, model.export_open)
            {
                return toolbar_actions::inspector_edit(index, model, files, tools);
            }
            if let Some(Target::Tool(tool)) = layout.hit_test(point) {
                tools.set_tool(tool);
                model.tool = tool;
                redraw = true;
            }
            let select = tools.tool() == Tool::Select;
            redraw |= toolbar_actions::press(layout, point, model, files, exports)
                || doc.pointer_with(layout.stage, point, select);
        }
    }
    if model.mode != Mode::Draw {
        return redraw;
    }
    let Some(map) = doc.view_map() else {
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
    let max_points = (4096 / safe_scale).max(720);
    let width = points.0.clamp(720, max_points);
    let height = points.1.clamp(480, max_points);
    ((width, height), safe_scale)
}

#[cfg(test)]
mod tests {
    use vexel_app::native_animation::host::pointer_accepts_mode;
    #[test]
    fn only_primary_button_can_activate_modes() {
        assert!(pointer_accepts_mode(Some(0)));
        assert!(!pointer_accepts_mode(Some(1)));
        assert!(!pointer_accepts_mode(None));
    }
}
