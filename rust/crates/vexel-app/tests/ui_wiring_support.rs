//! Shared rig for the WO-RUST-UI-WIRING-01 tests: a fake panel backend and a
//! window-free shell (model + file session + export runner) driven by
//! synthetic pointer and key events through the same functions `main.rs`
//! calls (`route_pointer` itself lives in the binary and cannot be linked, so
//! `Shell::route` repeats its chrome half: splitter phases, then the press).
#![allow(dead_code)]

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};
use vexel_app::animation::frame;
use vexel_app::chrome::{AppState, chrome_text};
use vexel_app::file_ui::FileUi;
use vexel_app::layout::Layout;
use vexel_app::model::{DragPhase, Mode, Model};
use vexel_app::toolbar_actions::{self, Exports};
use vexel_platform_apple::{
    Modifiers, PanelBackend, PanelResult, PointerEvent, PointerKind, UnsavedChoice, UnsavedText,
};

pub const SIZE: (u32, u32) = (1280, 800);

#[derive(Default)]
pub struct Log {
    /// Suggested names the Save panel was asked with.
    pub saves: Vec<String>,
    pub errors: Vec<(String, String)>,
    /// What the Save panel answers next (`None` = cancel).
    pub answer: Option<PanelResult>,
}

#[derive(Clone, Default)]
pub struct Fake(pub Rc<RefCell<Log>>);

impl PanelBackend for Fake {
    fn open_panel(&self, _: &[&str]) -> PanelResult {
        PanelResult::Cancelled
    }
    fn save_panel(&self, suggested: &str) -> PanelResult {
        let mut log = self.0.borrow_mut();
        log.saves.push(suggested.to_owned());
        log.answer.clone().unwrap_or(PanelResult::Cancelled)
    }
    fn confirm_unsaved(&self, _: &UnsavedText<'_>) -> UnsavedChoice {
        UnsavedChoice::Cancel
    }
    fn show_error(&self, title: &str, message: &str) {
        self.0
            .borrow_mut()
            .errors
            .push((title.to_owned(), message.to_owned()));
    }
}

/// A temporary directory removed on drop.
pub struct Dir(pub PathBuf);
impl Dir {
    pub fn new() -> Self {
        static N: AtomicU32 = AtomicU32::new(0);
        let path = std::env::temp_dir().join(format!(
            "vexel-ui-wiring-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
    pub fn file(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
    /// Sorted names of everything in the directory (hidden entries included).
    pub fn names(&self) -> Vec<String> {
        let mut v: Vec<String> = std::fs::read_dir(&self.0)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        v.sort();
        v
    }
}
impl Drop for Dir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

pub fn fixture(rel: &str) -> PathBuf {
    PathBuf::from(format!(
        "{}/../../fixtures/{rel}",
        env!("CARGO_MANIFEST_DIR")
    ))
}

pub struct Shell {
    pub model: Model,
    pub files: FileUi<Fake>,
    pub exports: Exports,
    pub log: Rc<RefCell<Log>>,
    loop_cache: Option<(u64, f64)>,
}

impl Shell {
    pub fn new() -> Self {
        let fake = Fake::default();
        let log = fake.0.clone();
        Self {
            model: Model::new(false),
            files: FileUi::new(fake).unwrap(),
            exports: Exports::default(),
            log,
            loop_cache: None,
        }
    }

    /// Opens `path` in the session and refreshes the chrome like the loop does.
    pub fn open(&mut self, path: &Path) {
        self.files.document_mut().open_path(path).unwrap();
        self.refresh();
    }

    /// One rectangle added through the session (an edit).
    pub fn edit(&mut self, id: &str, x: f64) {
        self.files
            .document_mut()
            .edit(|d| d.add_rectangle_with_id(x, 10.0, x + 20.0, 40.0, "#ff0000", id))
            .unwrap();
        self.refresh();
    }

    /// What `main.rs` recomputes each loop: chrome text and history availability.
    pub fn refresh(&mut self) {
        let doc = self.files.document();
        let loop_seconds = match self.loop_cache {
            Some((rev, secs)) if rev == doc.revision() => secs,
            _ => {
                let source = doc.source_bytes().unwrap();
                let secs = frame::decode(&source).map_or(0.0, |s| s.loop_seconds());
                self.loop_cache = Some((doc.revision(), secs));
                secs
            }
        };
        let state = AppState {
            open: doc.revision() > 0,
            name: doc.display_name(),
            path: doc
                .origin_path()
                .map(|p| String::from_utf8_lossy(p).into_owned()),
            size: doc.size(),
            dirty: doc.is_dirty(),
            loop_seconds,
            zoom_percent: 100,
            ..AppState::default()
        };
        self.model.chrome = chrome_text(&state);
        self.model.sync_history(doc.document());
    }

    pub fn layout(&self) -> Layout {
        self.model.layout(SIZE.0, SIZE.1).unwrap()
    }

    pub fn source(&self) -> Vec<u8> {
        self.files.document().source_bytes().unwrap()
    }

    pub fn cursor(&self) -> (usize, usize) {
        let d = self.files.document().document();
        (d.history_cursor(), d.history_count())
    }

    pub fn event(kind: PointerKind, at: (i32, i32), button: Option<u8>) -> PointerEvent {
        PointerEvent {
            kind,
            scale: 1.0,
            logical: at,
            at: vexel_platform_apple::Point {
                x: f64::from(at.0),
                y: f64::from(at.1),
            },
            button,
            click_count: 1,
            modifiers: Modifiers(0),
            timestamp_seconds: 0.0,
        }
    }

    /// The chrome half of `route_pointer`: splitter phases first, then (on a
    /// primary Down) the toolbar press. True when a redraw is needed.
    pub fn route(&mut self, pointer: &PointerEvent) -> bool {
        let layout = self.layout();
        let primary = pointer.button == Some(0);
        let phase = match pointer.kind {
            PointerKind::Down if primary => Some(DragPhase::Down),
            PointerKind::Move if self.model.ui.dragging() => Some(DragPhase::Move),
            PointerKind::Up if self.model.ui.dragging() => Some(DragPhase::Up),
            _ => None,
        };
        if let Some(phase) = phase {
            let window = (layout.width, layout.height);
            if self.model.splitter(phase, pointer.logical, &layout, window) {
                return true;
            }
        }
        let mut redraw = false;
        if pointer.kind == PointerKind::Down && primary {
            if pointer.logical.0 >= 0 && pointer.logical.1 >= 0 {
                let point = (pointer.logical.0 as u32, pointer.logical.1 as u32);
                redraw |= toolbar_actions::press(
                    &layout,
                    point,
                    &mut self.model,
                    &mut self.files,
                    &mut self.exports,
                );
            }
        }
        self.refresh();
        redraw
    }

    pub fn down(&mut self, at: (u32, u32)) -> bool {
        let e = Self::event(PointerKind::Down, (at.0 as i32, at.1 as i32), Some(0));
        self.route(&e)
    }

    /// Down on the centre of toolbar button `index` (Undo, Redo, Save, Export,
    /// inspector toggle).
    pub fn press_button(&mut self, index: usize) -> bool {
        let r = self.layout().buttons[index];
        self.down((r.x + r.width / 2, r.y + r.height / 2))
    }

    /// Down on the centre of Export popover row `index`.
    pub fn press_item(&mut self, index: usize) -> bool {
        let r = self.layout().export_items()[index];
        self.down((r.x + r.width / 2, r.y + r.height / 2))
    }

    /// Cmd+Z / Shift+Cmd+Z the way `main.rs` routes them.
    pub fn cmd_z(&mut self, shift: bool) {
        let mods = Modifiers(Modifiers::COMMAND | if shift { Modifiers::SHIFT } else { 0 });
        let action = vexel_app::session::edit_shortcut(KEY_Z, mods).unwrap();
        toolbar_actions::edit(&mut self.model, self.files.document_mut(), action);
        self.refresh();
    }

    /// Cmd+S the way `main.rs` routes it (through the file flow).
    pub fn cmd_s(&mut self) {
        let mods = Modifiers(Modifiers::COMMAND);
        self.files.key(KEY_S, mods);
        self.refresh();
    }

    pub fn key(&mut self, code: u16, modifiers: u32) -> bool {
        let layout = self.layout();
        let used = toolbar_actions::key(
            &layout,
            &mut self.model,
            &mut self.files,
            &mut self.exports,
            code,
            Modifiers(modifiers),
        );
        self.refresh();
        used
    }

    /// Waits for a running GIF export and returns its final status.
    pub fn finish_export(&mut self) -> &'static str {
        let deadline = Instant::now() + Duration::from_secs(60);
        loop {
            if let Some(done) = self.exports.poll() {
                self.model.status = done;
                return done;
            }
            assert!(Instant::now() < deadline, "export timed out");
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    pub fn set_mode(&mut self, mode: Mode) {
        self.model.mode = mode;
    }
}

pub const UNDO: usize = 0;
pub const REDO: usize = 1;
pub const SAVE: usize = 2;
pub const EXPORT: usize = 3;
pub const TOGGLE: usize = 4;
pub const KEY_Z: u16 = 6;
pub const KEY_S: u16 = 1;
