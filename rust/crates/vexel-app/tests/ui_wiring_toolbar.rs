//! WO-RUST-UI-WIRING-01 acceptance 1, 2, 3 and 6: toolbar buttons, history
//! truth, Save, inspector toggle, section headers and splitters.
mod ui_wiring_support;
use ui_wiring_support::*;
use vexel_app::animation::driver::Driver;
use vexel_app::animation::playback::MonotonicClock;
use vexel_app::layout::{INSPECTOR_MAX, INSPECTOR_MIN, SECTIONS};
use vexel_app::model::Target;
use vexel_app::preview::{Operation, Shape};
use vexel_app::session::McpHost;
use vexel_platform_apple::{PanelResult, PointerKind};

fn enabled(shell: &Shell, index: usize) -> bool {
    let l = shell.layout();
    let button = [
        vexel_app::layout::Button::Undo,
        vexel_app::layout::Button::Redo,
        vexel_app::layout::Button::Save,
        vexel_app::layout::Button::Export,
        vexel_app::layout::Button::ToggleInspector,
    ][index];
    shell.model.target_enabled(&l, Target::Button(button))
}

fn two_edits() -> Shell {
    let mut shell = Shell::new();
    shell.edit("a", 10.0);
    shell.edit("b", 50.0);
    shell
}

#[test]
fn enabled_buttons_act_and_disabled_buttons_change_nothing() {
    // Nothing open: Undo, Redo, Save and Export are disabled and inert.
    let mut closed = Shell::new();
    let before = closed.source();
    for index in [UNDO, REDO, SAVE, EXPORT] {
        assert!(!enabled(&closed, index), "button {index}");
        closed.press_button(index);
        assert_eq!(closed.source(), before);
        assert!(!closed.model.export_open);
        assert_eq!(closed.model.status, "");
    }
    assert!(closed.log.borrow().saves.is_empty());

    let mut shell = two_edits();
    assert_eq!(shell.cursor(), (2, 2));
    assert!(enabled(&shell, UNDO) && !enabled(&shell, REDO));
    // Redo is disabled with nothing undone: the press changes nothing.
    let src = shell.source();
    shell.press_button(REDO);
    assert_eq!((shell.source(), shell.cursor()), (src.clone(), (2, 2)));

    // Undo reverts exactly one edit, Redo reapplies it.
    shell.press_button(UNDO);
    assert_eq!(shell.cursor(), (1, 2));
    assert_ne!(shell.source(), src);
    assert!(!String::from_utf8_lossy(&shell.source()).contains("id=\"b\""));
    assert_eq!(shell.model.status, "Undid last change");
    shell.press_button(REDO);
    assert_eq!(shell.cursor(), (2, 2));
    assert_eq!(shell.source(), src);

    // Export opens the popover.
    assert!(enabled(&shell, EXPORT));
    shell.press_button(EXPORT);
    assert!(shell.model.export_open);
    shell.press_button(EXPORT);
    assert!(!shell.model.export_open, "second press closes it");

    // Save writes the file and clears dirty.
    let dir = Dir::new();
    let path = dir.file("one.vexel");
    shell.log.borrow_mut().answer = Some(PanelResult::Path(path.to_str().unwrap().into()));
    assert!(enabled(&shell, SAVE));
    shell.press_button(SAVE);
    assert!(path.exists());
    assert!(!shell.files.document().is_dirty());
    assert!(!enabled(&shell, SAVE), "clean documents disable Save");
    assert_eq!(shell.model.status, "Saved");
    let saves = shell.log.borrow().saves.len();
    shell.press_button(SAVE);
    assert_eq!(
        shell.log.borrow().saves.len(),
        saves,
        "disabled Save is inert"
    );
}

#[test]
fn buttons_and_shortcuts_agree_through_a_scripted_sequence() {
    let mut by_button = Shell::new();
    let mut by_key = Shell::new();
    let rect = |s: &mut Shell, id: &str, x: f64| s.edit(id, x);
    let check = |a: &Shell, b: &Shell, step: &str| {
        assert_eq!(a.source(), b.source(), "{step}: source");
        assert_eq!(a.cursor(), b.cursor(), "{step}: cursor");
        for s in [a, b] {
            let (cursor, count) = s.cursor();
            assert_eq!(enabled(s, UNDO), cursor > 0, "{step}: undo");
            assert_eq!(enabled(s, REDO), cursor < count, "{step}: redo");
        }
    };
    for s in [&mut by_button, &mut by_key] {
        rect(s, "a", 10.0);
        rect(s, "b", 50.0);
    }
    check(&by_button, &by_key, "two edits");
    for step in ["undo 1", "undo 2"] {
        by_button.press_button(UNDO);
        by_key.cmd_z(false);
        check(&by_button, &by_key, step);
        assert!(enabled(&by_button, REDO), "{step}: redo enabled");
    }
    by_button.press_button(REDO);
    by_key.cmd_z(true);
    check(&by_button, &by_key, "redo");
    for s in [&mut by_button, &mut by_key] {
        rect(s, "c", 90.0);
    }
    check(&by_button, &by_key, "final edit");
    assert!(
        !enabled(&by_button, REDO),
        "redo disabled after the last edit"
    );
    assert_eq!(by_button.cursor(), (2, 2));

    // A load resets availability (the core history starts empty).
    by_button.press_button(UNDO);
    assert!(enabled(&by_button, REDO));
    by_button.open(&fixture("v1/compose/compose-v1.svg"));
    assert_eq!(by_button.cursor(), (0, 0));
    assert!(!enabled(&by_button, UNDO) && !enabled(&by_button, REDO));

    // An MCP edit after an undo clears redo.
    by_key.cmd_z(false);
    assert!(enabled(&by_key, REDO));
    let mut host = McpHost::new();
    let mut driver = Driver::new(MonotonicClock::new());
    let rect = Shape::Rect {
        x: 1.0,
        y: 2.0,
        width: 3.0,
        height: 4.0,
        rgb: 0xff0000,
    };
    host.handle_with(
        by_key.files.document_mut(),
        Operation::Add(rect),
        &mut driver,
    )
    .unwrap();
    by_key.refresh();
    assert!(!enabled(&by_key, REDO), "MCP edit resets redo");
    let (cursor, count) = by_key.cursor();
    assert_eq!(cursor, count);

    let history = include_str!("../src/history.rs");
    assert!(!history.contains("redo_depth") && !history.contains("struct History"));
}

#[test]
fn save_click_matches_cmd_s_and_handles_untitled_cancel_and_failure() {
    // Titled document: the click and Cmd+S write identical project bytes.
    let dir = Dir::new();
    let mut clicked = two_edits();
    let mut keyed = two_edits();
    let (a, b) = (dir.file("a.vexel"), dir.file("b.vexel"));
    for (shell, path) in [(&mut clicked, &a), (&mut keyed, &b)] {
        shell.log.borrow_mut().answer = Some(PanelResult::Path(path.to_str().unwrap().into()));
        shell.cmd_s();
        shell.edit("c", 80.0);
        assert!(shell.files.document().is_dirty());
    }
    clicked.press_button(SAVE);
    keyed.cmd_s();
    assert_eq!(std::fs::read(&a).unwrap(), std::fs::read(&b).unwrap());
    assert!(!clicked.files.document().is_dirty());
    assert_eq!(
        clicked.log.borrow().saves.len(),
        1,
        "titled Save asks nothing"
    );

    // Untitled: opens the Save panel once; cancel leaves dirty set.
    let mut untitled = two_edits();
    untitled.press_button(SAVE);
    assert_eq!(untitled.log.borrow().saves.len(), 1);
    assert_eq!(untitled.log.borrow().saves[0], "Untitled.vexel");
    assert!(untitled.files.document().is_dirty());
    assert!(enabled(&untitled, SAVE));

    // A failing backend shows the alert and keeps the document.
    let source = untitled.source();
    untitled.log.borrow_mut().answer = Some(PanelResult::Error);
    untitled.press_button(SAVE);
    assert_eq!(untitled.log.borrow().errors.len(), 1);
    assert_eq!(untitled.log.borrow().errors[0].0, "Could not save");
    assert_eq!(untitled.source(), source);
    assert!(untitled.files.document().is_dirty());
    // A path that cannot be written fails the same way.
    let bad = dir.file("missing-dir/x.vexel");
    untitled.log.borrow_mut().answer = Some(PanelResult::Path(bad.to_str().unwrap().into()));
    untitled.press_button(SAVE);
    assert_eq!(untitled.log.borrow().errors.len(), 2);
    assert!(untitled.files.document().is_dirty());
    assert_eq!(untitled.source(), source);
}

#[test]
fn inspector_toggle_headers_and_splitter_work() {
    let mut shell = two_edits();
    let wide = shell.layout();
    assert!(wide.inspector.is_some());
    shell.press_button(TOGGLE);
    assert!(!shell.model.ui.inspector_visible);
    let narrow = shell.layout();
    assert!(narrow.inspector.is_none());
    assert!(
        narrow.stage.width > wide.stage.width,
        "stage takes the space"
    );
    assert!(
        enabled(&shell, TOGGLE),
        "a hidden inspector can be shown again"
    );
    shell.press_button(TOGGLE);
    assert_eq!(shell.layout(), wide);

    // Each section header toggles exactly its own section.
    for i in 0..SECTIONS {
        let l = shell.layout();
        let h = l.section_headers[i];
        if h.width == 0 {
            continue;
        }
        let before = shell.model.ui.collapsed;
        shell.down((h.x + h.width / 2, h.y + h.height / 2));
        let mut want = before;
        want[i] = !want[i];
        assert_eq!(shell.model.ui.collapsed, want, "header {i}");
        shell.down((h.x + h.width / 2, h.y + h.height / 2));
        assert_eq!(shell.model.ui.collapsed, before, "header {i} restored");
    }

    // Splitter: Down / Move / Up resizes inside the clamps.
    let l = shell.layout();
    let s = l.inspector_splitter.unwrap();
    let start = l.inspector.unwrap().width;
    let at = ((s.x + 1) as i32, (s.y + 100) as i32);
    shell.route(&Shell::event(PointerKind::Down, at, Some(0)));
    assert!(shell.model.ui.dragging());
    shell.route(&Shell::event(PointerKind::Move, (at.0 - 30, at.1), Some(0)));
    assert_eq!(shell.layout().inspector.unwrap().width, start + 30);
    shell.route(&Shell::event(PointerKind::Move, (-500, at.1), Some(0)));
    assert_eq!(shell.layout().inspector.unwrap().width, INSPECTOR_MAX);
    shell.route(&Shell::event(PointerKind::Move, (5000, at.1), Some(0)));
    assert_eq!(shell.layout().inspector.unwrap().width, INSPECTOR_MIN);
    shell.route(&Shell::event(PointerKind::Up, (-40, 9000), Some(0)));
    assert!(
        !shell.model.ui.dragging(),
        "Up outside the window ends the drag"
    );
    let width = shell.layout().inspector.unwrap().width;
    shell.route(&Shell::event(PointerKind::Move, (10, 10), Some(0)));
    assert_eq!(shell.layout().inspector.unwrap().width, width);
}

#[test]
fn seeded_pointer_and_key_sequences_never_panic() {
    let mut seed = 0x2545_f491_4f6c_dd1du64;
    let mut next = move |n: u64| {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed % n
    };
    let mut shell = two_edits();
    let dir = Dir::new();
    shell.log.borrow_mut().answer = Some(PanelResult::Path(
        dir.file("fuzz.vexel").to_str().unwrap().into(),
    ));
    for _ in 0..2000 {
        for _ in 0..8 {
            let at = (next(1500) as i32 - 100, next(950) as i32 - 100);
            let kind = [PointerKind::Down, PointerKind::Move, PointerKind::Up][next(3) as usize];
            let button = [Some(0), Some(0), Some(1), None][next(4) as usize];
            shell.route(&Shell::event(kind, at, button));
            if next(6) == 0 {
                shell.key(
                    [48, 49, 36, 53, 125, 126, 123, 124, 6][next(9) as usize],
                    next(16) as u32,
                );
            }
        }
        shell.route(&Shell::event(PointerKind::Up, (0, 0), Some(0)));
        let l = shell.layout();
        assert!(l.stage.width > 0 && l.stage.height > 0);
        if shell.model.ui.inspector_visible {
            let w = l.inspector.map_or(INSPECTOR_MIN, |r| r.width);
            assert!((INSPECTOR_MIN..=INSPECTOR_MAX).contains(&w));
        }
        let (cursor, count) = shell.cursor();
        assert!(cursor <= count);
    }
}
