//! Legacy stand-alone MCP preview window. `--mcp-preview` now runs the shell
//! (see `session::McpHost`); only `start_server` is still used.
#![allow(dead_code)]
#[cfg(test)]
use crate::preview;
use crate::{
    animation::{
        self,
        job::ExportJob,
        playback::{LoopClock, MonotonicClock},
    },
    model::{Layout, Mode, Model, checked_pixel_geometry},
    preview::{LiveBackend, Operation, PreviewState, UiRequest},
};
use std::{
    io::{BufReader, BufWriter},
    num::NonZeroUsize,
    sync::mpsc::{self, SyncSender, TryRecvError},
    thread,
    time::Duration,
};
use vexel_platform_apple::{
    MainThread, NativeApp, RgbaFrame, TextEmphasis, TextFace, WindowConfig, WindowEvent,
    draw_text_rgba,
};
use vexel_ui::{Theme, theme_tokens};

const CHANNEL_CAPACITY: usize = 8;

struct Pending {
    state: Option<PreviewState>,
    reply: SyncSender<Result<String, &'static str>>,
    response: String,
}

pub fn run() -> Result<(), &'static str> {
    let main_thread =
        MainThread::acquire().map_err(|_| "launch must begin on the macOS main thread")?;
    let mut app = NativeApp::start(main_thread).map_err(|_| "macOS application startup failed")?;
    let mut window = app
        .create_window(WindowConfig {
            title: "Vexel — MCP basic-shape preview".to_owned(),
            width_points: 1280,
            height_points: 820,
            resizable: true,
            initially_hidden: false,
        })
        .map_err(|_| "could not create the Vexel preview window")?;
    window
        .clear_text_input_state()
        .map_err(|_| "could not establish the preview text-input state")?;
    let (mut width, mut height, mut backing, initial_dark) = match window.surface_metrics() {
        Ok((points, pixels, scale)) => {
            let w = points.width.round() as u32;
            let h = points.height.round() as u32;
            if checked_pixel_geometry((w, h), scale as u32) != Some((pixels.width, pixels.height)) {
                return Err("native preview window has unsupported geometry");
            }
            (
                w,
                h,
                scale as u32,
                window.is_dark_appearance().unwrap_or(true),
            )
        }
        Err(_) => (1280, 820, 1, true),
    };
    let mut ui_model = Model::new(initial_dark);
    ui_model.mode = Mode::Draw;
    let mut state =
        PreviewState::new().map_err(|_| "could not create the blank preview document")?;
    let (tx, rx) = mpsc::sync_channel::<UiRequest>(CHANNEL_CAPACITY);
    start_server(tx)?;
    let event_budget = NonZeroUsize::new(128).ok_or("invalid event budget")?;
    let mut dirty = true;
    let mut closing = false;
    let mut pending: Option<Pending> = None;
    let mut status = String::from("Waiting for MCP client");
    let clock = MonotonicClock::new();
    let mut playback_epoch = clock.now_seconds();
    let mut last_frame = playback_epoch;
    let mut export: Option<ExportJob> = None;
    while !closing {
        if let Some(done) = export.as_mut().and_then(ExportJob::poll) {
            export = None;
            status = done;
            dirty = true;
        }
        if state.playback().0 && clock.now_seconds() - last_frame >= 0.033 {
            let duration = state.playback().2;
            if duration > 0.0 {
                state.set_frame_time(clock.now_seconds() - playback_epoch);
                status = format!("Playing · {:.2}s / {:.2}s", state.playback().1, duration);
                dirty = true;
            }
            last_frame = clock.now_seconds();
        }
        if pending.is_none() && export.is_none() {
            match rx.try_recv() {
                Ok(UiRequest {
                    operation:
                        Operation::Export {
                            path,
                            fps,
                            duration,
                        },
                    reply,
                }) => match start_export(&state, path, fps, duration, reply) {
                    Ok(job) => {
                        export = Some(job);
                        status = "Exporting GIF…".to_owned();
                        dirty = true;
                    }
                    Err((reply, message)) => {
                        let _ = reply.send(Err(message));
                    }
                },
                Ok(request) => {
                    handle_request(request, &state, &mut pending, &mut status, &mut dirty)?
                }
                Err(TryRecvError::Disconnected) => status = "MCP stdio disconnected".to_owned(),
                Err(TryRecvError::Empty) => {}
            }
        }
        if dirty {
            let shown = pending
                .as_ref()
                .and_then(|p| p.state.as_ref())
                .unwrap_or(&state);
            let frame = preview_frame(width, height, backing, &ui_model, shown, &status)
                .map_err(|_| "preview frame rendering failed")?;
            if let Err(error) = window.present_rgba(frame) {
                eprintln!("Vexel preview frame presentation failed: {error:?}");
                if let Some(p) = pending.take() {
                    let _ = p.reply.send(Err("native frame presentation failed"));
                }
                return Err("native frame presentation failed");
            }
            if let Some(p) = pending.take() {
                if let Some(committed) = p.state {
                    state = committed;
                    if state.playback().0 {
                        playback_epoch = clock.now_seconds() - state.playback().1;
                    }
                }
                let _ = p.reply.send(Ok(p.response));
            }
            dirty = false;
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
                            .map_err(|_| "could not restore preview text input")?;
                    }
                    dirty = true;
                }
                WindowEvent::AppearanceChanged { dark } => {
                    ui_model.dark = dark;
                    dirty = true;
                }
                WindowEvent::Resize { points, pixels } => {
                    let w = points.width.round() as u32;
                    let h = points.height.round() as u32;
                    if checked_pixel_geometry((w, h), backing)
                        == Some((pixels.width, pixels.height))
                    {
                        width = w;
                        height = h;
                        dirty = true;
                    }
                }
                WindowEvent::BackingScaleChanged { scale, pixels } => {
                    let next = scale as u32;
                    if scale.is_finite()
                        && scale.fract() == 0.0
                        && checked_pixel_geometry((width, height), next)
                            == Some((pixels.width, pixels.height))
                    {
                        backing = next;
                        dirty = true;
                    }
                }
                WindowEvent::CloseRequested => {
                    closing = true;
                    break;
                }
                WindowEvent::QueueFault(_) => return Err("native input queue reported a fault"),
                _ => {}
            }
        }
        if !batch.pending {
            thread::sleep(Duration::from_millis(16));
        }
    }
    if let Some(p) = pending.take() {
        let _ = p.reply.send(Err("preview window is closed"));
    }
    if let Some(job) = export.as_ref() {
        job.cancel();
    }
    app.destroy_window(window.id())
        .map_err(|_| "native preview shutdown failed")?;
    app.shutdown()
        .map_err(|_| "native application shutdown failed")
}

type Reply = SyncSender<Result<String, &'static str>>;

/// Starts the background GIF export of the committed preview document inside
/// a granted directory; on failure the unsent reply is handed back.
fn start_export(
    state: &PreviewState,
    path: String,
    fps: u32,
    duration: f64,
    reply: Reply,
) -> Result<ExportJob, (Reply, &'static str)> {
    let source = match state.source() {
        Ok(source) => source,
        Err(message) => return Err((reply, message)),
    };
    let policy = vexel_mcp_access::AccessPolicy::from_environment();
    ExportJob::spawn(source, policy, path.into(), fps, duration, reply.clone())
        .map_err(|message| (reply, message))
}

pub fn start_server(requests: SyncSender<UiRequest>) -> Result<(), &'static str> {
    thread::Builder::new()
        .name("vexel-mcp-stdio".to_owned())
        .spawn(move || {
            let mut input = BufReader::new(std::io::stdin().lock());
            let mut output = BufWriter::new(std::io::stdout().lock());
            let mut session = vexel_protocol::ProtocolSession::live_preview();
            let mut backend = LiveBackend::new(requests);
            if let Err(error) =
                vexel_mcp_stdio::serve(&mut input, &mut output, &mut session, &mut backend)
            {
                eprintln!("Vexel MCP preview stopped: {error}");
            }
        })
        .map(|_| ())
        .map_err(|_| "could not start the MCP stdio worker")
}

fn handle_request(
    request: UiRequest,
    state: &PreviewState,
    pending: &mut Option<Pending>,
    status: &mut String,
    dirty: &mut bool,
) -> Result<(), &'static str> {
    let UiRequest { operation, reply } = request;
    match operation {
        operation @ (Operation::List | Operation::Info | Operation::AnimationList) => {
            let response = match state.read_result(&operation) {
                Ok(response) => response,
                Err(error) => {
                    let _ = reply.send(Err(error));
                    return Ok(());
                }
            };
            *status = if matches!(operation, Operation::List) {
                "Listed preview documents".to_owned()
            } else {
                "Inspected preview document".to_owned()
            };
            *pending = Some(Pending {
                state: None,
                reply,
                response,
            });
            *dirty = true;
        }
        operation => {
            let (draft, message) = match state.stage(&operation) {
                Ok(value) => value,
                Err(error) => {
                    let _ = reply.send(Err(error));
                    return Ok(());
                }
            };
            let count = draft.shapes().len();
            let safe_message = match message.as_str() {
                "Shape added" => "Shape added",
                "Animation added" => "Animation added",
                "Animation playback updated" => "Animation playback updated",
                "Undo complete" => "Undo complete",
                "Redo complete" => "Redo complete",
                _ => "Preview updated",
            };
            let response = match &operation {
                Operation::Control(_) => draft.read_result(&operation)?,
                Operation::Add(_) => format!(
                    "{{\"ok\":true,\"shapeCount\":{count},\"elementId\":\"{}\",\"message\":\"{safe_message}\"}}",
                    draft
                        .element_ids()
                        .last()
                        .ok_or("preview stable ID missing")?
                ),
                _ => {
                    format!("{{\"ok\":true,\"shapeCount\":{count},\"message\":\"{safe_message}\"}}")
                }
            };
            *status = if matches!(operation, Operation::Control(_)) {
                let (playing, time, duration) = draft.playback();
                format!(
                    "{} · {:.2}s / {:.2}s",
                    if playing { "Playing" } else { "Paused" },
                    time,
                    duration
                )
            } else {
                format!(
                    "{safe_message} · {count} visible shape{}",
                    if count == 1 { "" } else { "s" }
                )
            };
            *pending = Some(Pending {
                state: Some(draft),
                reply,
                response,
            });
            *dirty = true;
        }
    }
    Ok(())
}

fn preview_frame(
    width: u32,
    height: u32,
    scale: u32,
    model: &Model,
    state: &PreviewState,
    status: &str,
) -> Result<RgbaFrame, &'static str> {
    let base = crate::render::render(width, height, scale, model)?;
    let mut pixels = Vec::new();
    pixels
        .try_reserve_exact(base.pixels().len())
        .map_err(|_| "preview frame allocation failed")?;
    pixels.extend_from_slice(base.pixels());
    let layout = Layout::new(width, height, Mode::Draw)
        .ok_or("preview window is below minimum layout size")?;
    let tokens = theme_tokens(
        if model.dark {
            Theme::Dark
        } else {
            Theme::Light
        },
        false,
    );
    fill(
        &mut pixels,
        base.width(),
        base.height(),
        scale,
        (0, 0, width, 48),
        tokens.window,
    );
    text(
        &mut pixels,
        base.width(),
        base.height(),
        scale,
        "Vexel",
        (18, 8, 86, 30),
        tokens.text_primary,
        16,
    )?;
    text(
        &mut pixels,
        base.width(),
        base.height(),
        scale,
        "Live animation preview",
        (110, 11, 300, 26),
        tokens.text_secondary,
        12,
    )?;
    let activity_y = height.saturating_sub(98);
    fill(
        &mut pixels,
        base.width(),
        base.height(),
        scale,
        (0, 48, width, activity_y.saturating_sub(48)),
        tokens.window,
    );
    fill(
        &mut pixels,
        base.width(),
        base.height(),
        scale,
        (
            layout.stage.x,
            layout.stage.y,
            layout.stage.width,
            layout.stage.height,
        ),
        tokens.canvas_well,
    );
    let pad = 24u32;
    let size = layout
        .stage
        .width
        .saturating_sub(pad * 2)
        .min(layout.stage.height.saturating_sub(pad * 2))
        .min(512)
        .max(1);
    let x = layout.stage.x + (layout.stage.width - size) / 2;
    let y = layout.stage.y + (layout.stage.height - size) / 2;
    fill(
        &mut pixels,
        base.width(),
        base.height(),
        scale,
        (x, y, size, size),
        0xFFFFFFFF,
    );
    let raster = animation::present::raster_at(&state.source()?, state.playback().1)?;
    for dy in 0..size {
        let sy = (dy as usize * 512 / size as usize).min(511);
        for dx in 0..size {
            let sx = (dx as usize * 512 / size as usize).min(511);
            let from = (sy * 512 + sx) * 4;
            for py in 0..scale {
                for px in 0..scale {
                    let at = (((y + dy) * scale + py) as usize * base.width() as usize
                        + ((x + dx) * scale + px) as usize)
                        * 4;
                    pixels[at..at + 4].copy_from_slice(&raster[from..from + 4]);
                }
            }
        }
    }
    fill(
        &mut pixels,
        base.width(),
        base.height(),
        scale,
        (0, activity_y, width, 70),
        tokens.window,
    );
    text(
        &mut pixels,
        base.width(),
        base.height(),
        scale,
        "MCP activity",
        (16, activity_y + 8, 160, 22),
        tokens.text_primary,
        12,
    )?;
    text(
        &mut pixels,
        base.width(),
        base.height(),
        scale,
        status,
        (16, activity_y + 34, width.saturating_sub(32), 24),
        tokens.text_secondary,
        11,
    )?;
    fill(
        &mut pixels,
        base.width(),
        base.height(),
        scale,
        (0, height.saturating_sub(28), width, 28),
        tokens.panel_raised,
    );
    text(
        &mut pixels,
        base.width(),
        base.height(),
        scale,
        if state.is_dirty() {
            "Preview document · unsaved"
        } else {
            "Blank 512 × 512 preview"
        },
        (14, height.saturating_sub(26), 300, 23),
        tokens.text_secondary,
        11,
    )?;
    RgbaFrame::new(base.width(), base.height(), f64::from(scale), pixels)
        .map_err(|_| "preview frame construction failed")
}

fn fill(pixels: &mut [u8], w: u32, h: u32, scale: u32, rect: (u32, u32, u32, u32), color: u32) {
    let (x, y, rw, rh) = rect;
    let rgba = [
        (color >> 24) as u8,
        (color >> 16) as u8,
        (color >> 8) as u8,
        color as u8,
    ];
    for py in y.saturating_mul(scale)..y.saturating_add(rh).saturating_mul(scale).min(h) {
        for px in x.saturating_mul(scale)..x.saturating_add(rw).saturating_mul(scale).min(w) {
            let at = (py as usize * w as usize + px as usize) * 4;
            pixels[at..at + 4].copy_from_slice(&rgba);
        }
    }
}
fn text(
    pixels: &mut [u8],
    w: u32,
    h: u32,
    scale: u32,
    value: &str,
    rect: (u32, u32, u32, u32),
    color: u32,
    size: u32,
) -> Result<(), &'static str> {
    let (x, y, rw, rh) = rect;
    draw_text_rgba(
        pixels,
        w,
        h,
        (w as usize) * 4,
        scale,
        value,
        (x * scale, y * scale, rw * scale, rh * scale),
        color,
        TextFace::System,
        TextEmphasis::Regular,
        size * scale,
    )
    .map_err(|_| "native preview text rendering failed")
}

#[cfg(test)]
mod tests {
    use super::*;
    use vexel_json::JsonDocument;
    use vexel_protocol::ProtocolBackend;
    #[test]
    fn reply_follows_staged_state_and_is_only_committed_for_frame() {
        let state = PreviewState::new().unwrap();
        let (tx, rx) = mpsc::sync_channel(1);
        let request = UiRequest {
            operation: Operation::Add(preview::Shape::Rect {
                x: 2.,
                y: 2.,
                width: 4.,
                height: 4.,
                rgb: 0,
            }),
            reply: tx,
        };
        let mut pending = None;
        let mut status = String::new();
        let mut dirty = false;
        handle_request(request, &state, &mut pending, &mut status, &mut dirty).unwrap();
        assert!(dirty);
        assert!(state.shapes().is_empty());
        let staged = pending.take().unwrap();
        assert_eq!(staged.state.as_ref().unwrap().shapes().len(), 1);
        assert!(staged.response.contains("vexel-object-1"));
        staged.reply.send(Ok(staged.response)).unwrap();
        assert!(rx.recv().unwrap().unwrap().contains("shapeCount"));
    }

    #[test]
    fn backend_call_waits_until_the_ui_reports_frame_presentation() {
        let line = br##"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"element_add","arguments":{"documentId":"preview-document","parentId":"root","template":"rect","attributes":{"x":"1","y":"2","width":"3","height":"4","fill":"#00FF00"}}}}"##;
        let doc = JsonDocument::parse_owned(line).unwrap();
        let args = doc
            .object_get(doc.root(), b"params")
            .and_then(|p| doc.object_get(p, b"arguments"))
            .unwrap();
        let parsed = super::preview::parse_operation(&doc, args, "element_add");
        assert!(parsed.is_ok(), "preview request mapping failed: {parsed:?}");
        let (tx, rx) = mpsc::sync_channel(CHANNEL_CAPACITY);
        let worker = thread::spawn(move || {
            let mut backend = LiveBackend::new(tx);
            backend.call_tool(&doc, args, "element_add").unwrap()
        });
        let request = rx.recv().unwrap();
        assert!(matches!(&request.operation, Operation::Add(_)));
        assert!(!worker.is_finished(), "the tool must await its UI reply");
        let blank = PreviewState::new().unwrap();
        let staged = blank.stage(&request.operation).unwrap().0;
        assert_eq!(
            blank.shapes().len(),
            0,
            "staged edits are not yet committed"
        );
        let raster = preview::rasterize(staged.shapes()).unwrap();
        assert!(raster.iter().any(|&byte| byte != 255));
        request
            .reply
            .send(Ok(format!(
                "{{\"ok\":true,\"shapeCount\":{},\"elementId\":\"{}\"}}",
                staged.shapes().len(),
                staged.element_ids()[0]
            )))
            .unwrap();
        let response = worker.join().unwrap();
        assert!(!response.is_error);
        assert!(
            String::from_utf8(response.structured_json)
                .unwrap()
                .contains("shapeCount")
        );
    }
}
