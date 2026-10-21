//! Toolbar wiring: Undo, Redo, Save and Export presses (pointer and keyboard)
//! executed against the document session, and the Export popover's three
//! file writers.
//!
//! `main.rs` calls [`press`] and [`key`]; everything here is generic over the
//! panel backend so the headless tests drive it with a fake.
//!
//! Export grants nothing wider than the chosen file: GIF and PNG render into a
//! fresh private staging directory that is the only grant of a one-off
//! `AccessPolicy`, then the finished file is renamed over the chosen path and
//! the directory removed. SVG writes a temp sibling and renames it.

use crate::animation::{
    driver::Driver, frame, job::ExportJob, native_export::NativeExportSource,
    playback::MonotonicClock,
};
use crate::file_ui::FileUi;
use crate::history;
use crate::layout::Layout;
use crate::model::{ButtonAction, ExportFormat, Model};
use crate::preview::Operation;
use crate::session::{AppDocument, EditAction, McpGrants, McpHost};
use crate::tools::ToolController;
use std::ffi::OsString;
use std::fs;
use std::io::Write;
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::os::unix::fs::DirBuilderExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::sync_channel;
use vexel_mcp_access::AccessPolicy;
use vexel_platform_apple::{Modifiers, PanelBackend, PanelResult};

#[path = "inspector_ext_edit.rs"]
mod ext_edit;
pub use ext_edit::apply as apply_ext_control;

/// Frames per second of a GIF export (the exporter caps at 30).
pub const GIF_FPS: u32 = 24;
const KEY_SPACE: u16 = 49;
const KEY_RETURN: u16 = 36;
const KEY_DOWN: u16 = 125;
const KEY_UP: u16 = 126;
const KEY_ESCAPE: u16 = 53;

/// Undo or redo through the one shared path (button and Cmd+Z / Shift+Cmd+Z):
/// updates the document, the availability, the status line and the redraw
/// flag. True when the document changed.
pub fn edit(model: &mut Model, doc: &mut AppDocument, action: EditAction) -> bool {
    let changed = history::apply(doc, action);
    model.status = match (action, changed) {
        (EditAction::Undo, true) => "Undid last change",
        (EditAction::Undo, false) => "Nothing to undo",
        (EditAction::Redo, true) => "Redid change",
        (EditAction::Redo, false) => "Nothing to redo",
    };
    model.sync_history(doc);
    model.dirty = true;
    changed
}

/// Runs one toolbar action.
pub fn run<B: PanelBackend>(
    action: ButtonAction,
    model: &mut Model,
    files: &mut FileUi<B>,
    exports: &mut Exports,
) {
    match action {
        ButtonAction::Undo => {
            edit(model, files.document_mut(), EditAction::Undo);
        }
        ButtonAction::Redo => {
            edit(model, files.document_mut(), EditAction::Redo);
        }
        ButtonAction::Save => {
            let outcome = files.save();
            if let Some(status) = outcome.status() {
                model.status = status;
            }
            model.dirty = true;
        }
        ButtonAction::Export(format) => exports.start(format, model, files),
    }
}

/// A primary press at `point`. True when the press was consumed (it changed
/// the chrome, fired an action or landed under an open popover).
pub fn press<B: PanelBackend>(
    layout: &Layout,
    point: (u32, u32),
    model: &mut Model,
    files: &mut FileUi<B>,
    exports: &mut Exports,
) -> bool {
    let under = model.popover_hit(layout, point);
    let (changed, action) = model.pointer_action(layout, point);
    if let Some(action) = action {
        run(action, model, files, exports);
    }
    changed || under || action.is_some()
}

/// Applies one of the inspector's preset fill, stroke, width, opacity, gradient,
/// dash or transform edits.
pub fn inspector_edit<B: PanelBackend>(
    target_index: usize,
    model: &mut Model,
    files: &mut FileUi<B>,
    tools: &mut ToolController,
) -> bool {
    if let Some(control) = crate::layout::ExtControl::from_index(target_index) {
        return ext_edit::run(control, model, files, tools);
    }
    let width_index = target_index.checked_sub(17).filter(|i| *i < 4);
    let width = width_index.map(|i| [1.0, 2.0, 4.0, 8.0][i]);
    let stroke = if (11..17).contains(&target_index) || width_index.is_some() {
        true
    } else if (5..11).contains(&target_index) {
        false
    } else {
        return false;
    };
    let colour = if (5..17).contains(&target_index) {
        crate::model::INSPECTOR_SWATCHES
            .get(if target_index < 11 {
                target_index - 5
            } else {
                target_index - 11
            })
            .copied()
    } else {
        None
    };
    if (5..17).contains(&target_index) && colour.is_none() {
        return false;
    }
    let selected_stroke = match &model.chrome.inspector {
        crate::chrome::Inspector::Shape(props) => {
            let value = props.stroke_text();
            (value != "none").then_some(value)
        }
        crate::chrome::Inspector::NoSelection => None,
    };
    let current_width = match &model.chrome.inspector {
        crate::chrome::Inspector::Shape(props) if props.stroke_width > 0.0 => props.stroke_width,
        _ => tools.stroke().1,
    };
    let paint = colour
        .or_else(|| selected_stroke.as_deref())
        .unwrap_or_else(|| tools.stroke().0)
        .to_owned();
    let chosen_width = width.unwrap_or(current_width);
    let edited = files.document_mut().edit(|document| {
        if stroke {
            tools.set_stroke(document, &paint, chosen_width).map(|_| ())
        } else {
            tools.set_fill(document, &paint).map(|_| ())
        }
    });
    model.status = if edited.is_ok() {
        if width.is_some() {
            "Updated stroke width"
        } else {
            "Updated shape color"
        }
    } else if width.is_some() {
        "Could not update stroke width"
    } else {
        "Could not update shape color"
    };
    model.dirty = true;
    true
}

/// Keyboard navigation of the toolbar: Tab stops, Space/Return on a focused
/// button or popover row, Up/Down inside the popover, Escape. True when the
/// key was consumed.
pub fn key<B: PanelBackend>(
    layout: &Layout,
    model: &mut Model,
    files: &mut FileUi<B>,
    exports: &mut Exports,
    code: u16,
    modifiers: Modifiers,
) -> bool {
    let chord = modifiers.0 & (Modifiers::COMMAND | Modifiers::CONTROL | Modifiers::OPTION) != 0;
    if chord {
        return false;
    }
    let shift = modifiers.0 & Modifiers::SHIFT != 0;
    match code {
        KEY_ESCAPE if model.close_export() => true,
        KEY_SPACE | KEY_RETURN if model.focus_button.is_some() || model.export_open => {
            if let Some(action) = model.activate_focused(layout) {
                run(action, model, files, exports);
            }
            model.dirty = true;
            true
        }
        KEY_DOWN | KEY_UP if model.export_open => model.menu_nav(code == KEY_DOWN, layout),
        _ => model.nav(
            code,
            shift,
            layout.inspector.is_some(),
            layout.width >= crate::layout::INSPECTOR_MIN_WINDOW,
        ),
    }
}

// ---- Export ----

static COUNTER: AtomicU32 = AtomicU32::new(0);

/// In-flight and finished exports of the window.
#[derive(Default)]
pub struct Exports {
    run: Option<GifRun>,
}

struct GifRun {
    job: ExportJob,
    stage: Stage,
    target: PathBuf,
}

impl Drop for GifRun {
    fn drop(&mut self) {
        // Cancel, give the worker a short bounded time to stop writing, then remove the
        // private staging directory so quitting mid-export never leaves it behind.
        self.job.cancel();
        if self.job.is_running() {
            for _ in 0..200 {
                if self.job.poll().is_some() {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }
        self.stage.discard();
    }
}

/// A fresh, private, empty directory beside the target; the only directory a
/// one-off policy grants. `file` is where the exporter writes.
pub struct Stage {
    pub dir: PathBuf,
    pub file: PathBuf,
}

impl Stage {
    pub fn create(target: &Path) -> Result<Self, &'static str> {
        let parent = match target.parent() {
            Some(p) if !p.as_os_str().is_empty() => p,
            _ => Path::new("."),
        };
        let name = target.file_name().ok_or(PATH_BAD)?;
        let dir = parent.join(format!(
            ".vexel-export-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::DirBuilder::new()
            .mode(0o700)
            .create(&dir)
            .map_err(|_| WRITE_FAILED)?;
        let file = dir.join(name);
        Ok(Self { dir, file })
    }

    /// The policy that grants this directory and nothing else (no environment
    /// value, no config file).
    pub fn policy(&self) -> Result<AccessPolicy, &'static str> {
        McpGrants::from_dirs([&self.dir], None)
            .map(|grants| grants.policy_with(None, None))
            .map_err(|_| PATH_BAD)
    }

    /// Moves the staged file over `target` and removes the directory.
    fn publish(self, target: &Path) -> Result<(), &'static str> {
        let moved = fs::rename(&self.file, target);
        self.discard();
        moved.map_err(|_| WRITE_FAILED)
    }

    pub fn discard(&self) {
        let _ = fs::remove_file(&self.file);
        let _ = fs::remove_dir(&self.dir);
    }
}

const PATH_BAD: &str = "Export path is not usable";
const WRITE_FAILED: &str = "Export could not write the file";

impl Exports {
    /// True while a GIF is rendering.
    pub fn busy(&self) -> bool {
        self.run.is_some()
    }

    /// Finishes a GIF export once its job ends; the status line to show.
    pub fn poll(&mut self) -> Option<&'static str> {
        let status = self.run.as_mut()?.job.poll()?;
        let run = self.run.take()?;
        let GifRun { stage, target, .. } = &run;
        let done = if status.starts_with("Exported") {
            let moved = fs::rename(&stage.file, target);
            stage.discard();
            if moved.is_ok() {
                "Exported GIF"
            } else {
                WRITE_FAILED
            }
        } else {
            stage.discard();
            "GIF export failed"
        };
        Some(done)
    }

    /// Asks for a file name with the Save panel, then writes the export.
    pub fn start<B: PanelBackend>(
        &mut self,
        format: ExportFormat,
        model: &mut Model,
        files: &mut FileUi<B>,
    ) {
        model.dirty = true;
        if self.busy() {
            model.status = "An export is already running";
            return;
        }
        let suggested = suggested_name(files.document(), format);
        let chosen = match files.backend().save_panel(&suggested) {
            PanelResult::Cancelled => return,
            PanelResult::Error => {
                files.backend().show_error("Could not export", PATH_BAD);
                model.status = PATH_BAD;
                return;
            }
            PanelResult::Path(path) => path,
        };
        let Some(target) = export_target(chosen, format) else {
            files.backend().show_error("Could not export", PATH_BAD);
            model.status = PATH_BAD;
            return;
        };
        let result = match format {
            ExportFormat::Svg => write_svg(files.document(), &target).map(|()| "Exported SVG"),
            ExportFormat::Png => write_png(files.document_mut(), &target).map(|()| "Exported PNG"),
            ExportFormat::Gif => self.spawn_gif(files.document(), target),
        };
        model.status = match result {
            Ok(done) => done,
            Err(message) => {
                files.backend().show_error("Could not export", message);
                message
            }
        };
    }

    /// Native timeline: rendered and written synchronously. Otherwise the SMIL
    /// animation renders on a worker. Returns the status line.
    fn spawn_gif(
        &mut self,
        doc: &AppDocument,
        target: PathBuf,
    ) -> Result<&'static str, &'static str> {
        if let Some(native) = NativeExportSource::from_document(doc) {
            return write_native_gif(&native, target).map(|()| "Exported GIF");
        }
        self.spawn_smil_gif(doc, target).map(|()| "Exporting GIF")
    }

    fn spawn_smil_gif(&mut self, doc: &AppDocument, target: PathBuf) -> Result<(), &'static str> {
        let source = doc.source_bytes().map_err(|_| WRITE_FAILED)?;
        let scene = frame::decode(&source).map_err(|_| "Export could not read the animation")?;
        let duration = scene.loop_seconds();
        drop(scene);
        if duration <= 0.0 {
            return Err("Export needs an animation");
        }
        let stage = Stage::create(&target)?;
        let spawned = stage.policy().and_then(|policy| {
            let (reply, _unused) = sync_channel(1);
            ExportJob::spawn(
                source,
                policy,
                OsString::from(stage.file.as_os_str()),
                GIF_FPS,
                duration,
                reply,
            )
        });
        match spawned {
            Ok(job) => {
                self.run = Some(GifRun { job, stage, target });
                Ok(())
            }
            Err(message) => {
                stage.discard();
                Err(message)
            }
        }
    }
}

/// Renders the native timeline into a private staging file, then publishes it.
fn write_native_gif(native: &NativeExportSource, target: PathBuf) -> Result<(), &'static str> {
    let gif = native
        .export_gif()
        .map_err(|_| "Export could not render the timeline")?;
    let stage = Stage::create(&target)?;
    let written = fs::File::create(&stage.file)
        .and_then(|mut file| file.write_all(&gif.bytes).and_then(|()| file.sync_all()));
    if written.is_err() {
        stage.discard();
        return Err(WRITE_FAILED);
    }
    stage.publish(&target)
}

/// Save-panel suggestion: the document name without its extension.
fn suggested_name(doc: &AppDocument, format: ExportFormat) -> String {
    let name = doc.display_name().unwrap_or_else(|| "Untitled".to_owned());
    let stem = name
        .rsplit_once('.')
        .map_or(name.as_str(), |(stem, _)| stem);
    format!(
        "{}.{}",
        if stem.is_empty() { "Untitled" } else { stem },
        format.extension()
    )
}

/// The path to write. The native Save panel is a project panel and appends
/// `.vexel`; that suffix is removed again, then the format's extension is
/// ensured. `None` for an unusable name.
fn export_target(mut chosen: Vec<u8>, format: ExportFormat) -> Option<PathBuf> {
    let has = |bytes: &[u8], ext: &str| {
        bytes.len() > ext.len() + 1
            && bytes[bytes.len() - ext.len() - 1] == b'.'
            && bytes[bytes.len() - ext.len()..].eq_ignore_ascii_case(ext.as_bytes())
    };
    if has(&chosen, "vexel") {
        chosen.truncate(chosen.len() - ".vexel".len());
    }
    if !has(&chosen, format.extension()) {
        chosen.push(b'.');
        chosen.extend_from_slice(format.extension().as_bytes());
    }
    let leaf = chosen.rsplit(|b| *b == b'/').next()?;
    (leaf.len() > 1 + format.extension().len() && !chosen.contains(&0))
        .then(|| PathBuf::from(OsString::from_vec(chosen)))
}

/// SVG: the document source verbatim, through a temp sibling renamed into place.
fn write_svg(doc: &AppDocument, target: &Path) -> Result<(), &'static str> {
    let bytes = doc.source_bytes().map_err(|_| WRITE_FAILED)?;
    let name = target.file_name().ok_or(PATH_BAD)?;
    let mut temp_name = OsString::from(".");
    temp_name.push(name);
    temp_name.push(format!(
        ".vexel-export-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let temp = target.with_file_name(temp_name);
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)
        .map_err(|_| WRITE_FAILED)?;
    let written = file
        .write_all(&bytes)
        .and_then(|()| file.sync_all())
        .and_then(|()| {
            drop(file);
            fs::rename(&temp, target)
        });
    if written.is_err() {
        let _ = fs::remove_file(&temp);
        return Err(WRITE_FAILED);
    }
    Ok(())
}

/// PNG: the MCP `render_png` writer (document size, time 0) pointed at a
/// staging directory, then renamed over the chosen path.
fn write_png(doc: &mut AppDocument, target: &Path) -> Result<(), &'static str> {
    let stage = Stage::create(target)?;
    let rendered = stage.policy().and_then(|policy| {
        let path = std::str::from_utf8(stage.file.as_os_str().as_bytes())
            .map_err(|_| PATH_BAD)?
            .to_owned();
        let mut driver = Driver::new(MonotonicClock::new());
        McpHost::with_policy(policy)
            .handle_with(
                doc,
                Operation::RenderPng {
                    path,
                    width: None,
                    height: None,
                },
                &mut driver,
            )
            .map(|_| ())
            .map_err(|_| "Export could not render the PNG")
    });
    match rendered {
        Ok(()) => stage.publish(target),
        Err(message) => {
            stage.discard();
            Err(message)
        }
    }
}
