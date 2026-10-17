//! WO-RUST-UI-WIRING-01 acceptance 7: keyboard activation of the toolbar.
mod ui_wiring_support;
use std::cell::Cell;
use std::rc::Rc;
use ui_wiring_support::*;
use vexel_app::layout::{Button, Focus};
use vexel_app::model::{ExportFormat, Mode, Target};
use vexel_app::motion::{MotionAdapter, control_id};
use vexel_platform_apple::{Modifiers, PanelResult};
use vexel_ui::motion::Clock;

const TAB: u16 = 48;
const SPACE: u16 = 49;
const RETURN: u16 = 36;
const ESCAPE: u16 = 53;
const DOWN: u16 = 125;
const UP: u16 = 126;
const SHIFT: u32 = Modifiers::SHIFT;

#[derive(Clone)]
struct TestClock(Rc<Cell<u64>>);
impl Clock for TestClock {
    fn now_ns(&self) -> u64 {
        self.0.get()
    }
}

fn label(shell: &Shell) -> String {
    match (shell.model.focus_button, shell.model.ui.focus) {
        (Some(b), _) => format!("{b:?}"),
        (None, Some(Focus::Segmented)) => "Mode".to_owned(),
        (None, Some(Focus::Section(i))) => format!("Section{i}"),
        (None, None) => "None".to_owned(),
    }
}

/// Tab stops from the page until focus leaves the toolbar for the sections.
fn toolbar_stops(shell: &mut Shell) -> Vec<String> {
    let mut seen = Vec::new();
    for _ in 0..12 {
        shell.key(TAB, 0);
        let l = label(shell);
        if l.starts_with("Section") || l == "None" {
            break;
        }
        seen.push(l);
    }
    seen
}

fn focus(shell: &mut Shell, button: Button) {
    for _ in 0..20 {
        if shell.model.focus_button == Some(button) {
            return;
        }
        shell.key(TAB, 0);
    }
    panic!("{button:?} never received focus");
}

fn edited() -> Shell {
    let mut shell = Shell::new();
    shell.edit("a", 10.0);
    shell.edit("b", 50.0);
    shell
}

#[test]
fn tab_visits_mode_then_each_enabled_button_in_order() {
    let mut shell = edited();
    shell.press_button(UNDO);
    assert_eq!(
        toolbar_stops(&mut shell),
        ["Mode", "Undo", "Redo", "Save", "Export", "ToggleInspector"]
    );
    // Redo disabled after a redo: skipped.
    let mut shell = edited();
    assert_eq!(
        toolbar_stops(&mut shell),
        ["Mode", "Undo", "Save", "Export", "ToggleInspector"]
    );
    // Clean, no history: only Export and the toggle remain.
    let mut shell = Shell::new();
    shell.open(&fixture("v1/compose/compose-v1.svg"));
    assert!(!shell.files.document().is_dirty());
    assert_eq!(
        toolbar_stops(&mut shell),
        ["Mode", "Export", "ToggleInspector"]
    );
    // Nothing open: the mode control and the toggle.
    let mut shell = Shell::new();
    assert_eq!(toolbar_stops(&mut shell), ["Mode", "ToggleInspector"]);
    // Shift+Tab from the page lands on the last stop.
    let mut shell = edited();
    shell.key(TAB, SHIFT);
    assert!(label(&shell).starts_with("Section"));
}

#[test]
fn space_and_return_activate_like_a_click() {
    for key in [SPACE, RETURN] {
        let mut by_key = edited();
        let mut by_click = edited();
        focus(&mut by_key, Button::Undo);
        assert!(by_key.key(key, 0));
        by_click.press_button(UNDO);
        assert_eq!(by_key.cursor(), (1, 2));
        assert_eq!(by_key.source(), by_click.source());
        focus(&mut by_key, Button::Redo);
        by_key.key(key, 0);
        by_click.press_button(REDO);
        assert_eq!(by_key.cursor(), (2, 2));
        assert_eq!(by_key.source(), by_click.source());

        // Save: the untitled document opens the (fake) Save panel once.
        let dir = Dir::new();
        by_key.log.borrow_mut().answer = Some(PanelResult::Path(
            dir.file("k.vexel").to_str().unwrap().into(),
        ));
        focus(&mut by_key, Button::Save);
        by_key.key(key, 0);
        assert_eq!(by_key.log.borrow().saves.len(), 1);
        assert!(dir.file("k.vexel").exists());
        assert!(!by_key.files.document().is_dirty());

        // Inspector toggle.
        focus(&mut by_key, Button::ToggleInspector);
        by_key.key(key, 0);
        assert!(!by_key.model.ui.inspector_visible);
        by_key.key(key, 0);
        assert!(by_key.model.ui.inspector_visible);

        // Export: opens the popover; Down/Up move the row; the key on a row
        // writes that format; Escape closes and returns focus.
        focus(&mut by_key, Button::Export);
        by_key.key(key, 0);
        assert!(by_key.model.export_open);
        assert_eq!(
            by_key.model.menu_focus,
            Some(1),
            "GIF is disabled for a static document"
        );
        by_key.key(DOWN, 0);
        assert_eq!(by_key.model.menu_focus, Some(2));
        by_key.key(DOWN, 0);
        assert_eq!(
            by_key.model.menu_focus,
            Some(1),
            "wraps past the disabled row"
        );
        by_key.key(UP, 0);
        assert_eq!(by_key.model.menu_focus, Some(2));
        by_key.log.borrow_mut().answer = Some(PanelResult::Path(
            dir.file("k.png").to_str().unwrap().into(),
        ));
        by_key.key(key, 0);
        assert!(dir.file("k.png").exists());
        assert!(!by_key.model.export_open);
        assert_eq!(by_key.model.focus_button, Some(Button::Export));
        by_key.key(key, 0);
        assert!(by_key.model.export_open);
        assert!(by_key.key(ESCAPE, 0));
        assert!(!by_key.model.export_open);
        assert_eq!(by_key.model.focus_button, Some(Button::Export));
    }
}

#[test]
fn command_chords_do_not_activate_a_focused_button() {
    let mut shell = edited();
    focus(&mut shell, Button::Undo);
    assert!(!shell.key(RETURN, Modifiers::COMMAND));
    assert_eq!(shell.cursor(), (2, 2));
}

#[test]
fn every_focus_stop_shows_a_ring_in_its_control_visual() {
    let time = Rc::new(Cell::new(1_000_000_000));
    let mut motion = MotionAdapter::with_clock(TestClock(time.clone()), false);
    let mut shell = edited();
    shell.press_button(UNDO);
    let buttons = [
        Button::Undo,
        Button::Redo,
        Button::Save,
        Button::Export,
        Button::ToggleInspector,
    ];
    for button in buttons {
        focus(&mut shell, button);
        motion.sync_focus(shell.model.focus_id());
        time.set(time.get() + 200_000_000);
        let id = control_id(Target::Button(button));
        let v = motion.frame(false, Mode::Draw).get(id);
        assert_eq!(v.focus, 1.0, "{button:?}");
        assert_ne!(v.ring & 255, 0, "{button:?} ring visible");
    }
    for format in ExportFormat::ALL {
        let id = control_id(Target::MenuItem(format));
        assert_eq!(
            motion.frame(false, Mode::Draw).get(id).focus,
            0.0,
            "{format:?} idle"
        );
    }
    // A menu row takes the ring while it holds focus.
    focus(&mut shell, Button::Export);
    shell.key(RETURN, 0);
    motion.sync_focus(shell.model.focus_id());
    time.set(time.get() + 200_000_000);
    let row = control_id(Target::MenuItem(ExportFormat::Svg));
    let v = motion.frame(false, Mode::Draw).get(row);
    assert_eq!(v.focus, 1.0);
    assert_ne!(v.ring & 255, 0);
    // Leaving focus clears every ring.
    shell.key(ESCAPE, 0);
    shell.key(ESCAPE, 0);
    motion.sync_focus(shell.model.focus_id());
    time.set(time.get() + 200_000_000);
    for button in buttons {
        let id = control_id(Target::Button(button));
        assert_eq!(motion.frame(false, Mode::Draw).get(id).focus, 0.0);
    }
}
