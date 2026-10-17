//! WO-RUST-UI-WIRING-01 acceptance 4 and 5: the Export popover, its three
//! writers and popover dismissal.
mod ui_wiring_support;
use std::path::Path;
use ui_wiring_support::*;
use vexel_app::layout::Button;
use vexel_app::model::{ExportFormat, Target};
use vexel_app::toolbar_actions::Stage;
use vexel_platform_apple::PanelResult;

const GIF: usize = 0;
const SVG: usize = 1;
const PNG: usize = 2;

fn animated() -> Shell {
    let mut shell = Shell::new();
    shell.open(&fixture("v1/anim/loop.svg"));
    shell
}

fn answer(shell: &Shell, path: &Path) {
    shell.log.borrow_mut().answer = Some(PanelResult::Path(path.to_str().unwrap().into()));
}

fn pick(shell: &mut Shell, item: usize) {
    shell.press_button(EXPORT);
    assert!(shell.model.export_open);
    shell.press_item(item);
    assert!(!shell.model.export_open, "a row closes the popover");
}

#[test]
fn each_row_asks_for_its_extension_and_writes_only_the_chosen_file() {
    let dir = Dir::new();
    let mut shell = animated();

    answer(&shell, &dir.file("clip.gif"));
    pick(&mut shell, GIF);
    assert_eq!(shell.finish_export(), "Exported GIF");
    let gif = std::fs::read(dir.file("clip.gif")).unwrap();
    assert_eq!(&gif[..6], b"GIF89a");
    assert_eq!(
        dir.names(),
        ["clip.gif"],
        "no staging directory or temp file left"
    );

    answer(&shell, &dir.file("art.svg"));
    pick(&mut shell, SVG);
    assert_eq!(
        std::fs::read(dir.file("art.svg")).unwrap(),
        shell.source(),
        "SVG is the document source"
    );

    answer(&shell, &dir.file("shot.png"));
    pick(&mut shell, PNG);
    let png = std::fs::read(dir.file("shot.png")).unwrap();
    assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
    let size = |at: usize| u32::from_be_bytes(png[at..at + 4].try_into().unwrap());
    assert_eq!((size(16), size(20)), (64, 48), "document size");
    assert_eq!(dir.names(), ["art.svg", "clip.gif", "shot.png"]);

    let saves = shell.log.borrow().saves.clone();
    assert_eq!(saves.len(), 3);
    for (name, format) in saves.iter().zip(ExportFormat::ALL) {
        assert_eq!(name, &format!("loop.{}", format.extension()));
    }
    assert_eq!(shell.model.status, "Exported PNG");
}

#[test]
fn the_project_panel_suffix_is_removed_and_extensions_are_ensured() {
    let dir = Dir::new();
    let mut shell = animated();
    // The native panel is a project panel and appends `.vexel`.
    answer(&shell, &dir.file("one.svg.vexel"));
    pick(&mut shell, SVG);
    answer(&shell, &dir.file("two.vexel"));
    pick(&mut shell, SVG);
    answer(&shell, &dir.file("three"));
    pick(&mut shell, SVG);
    answer(&shell, &dir.file("four.GIF.vexel"));
    pick(&mut shell, GIF);
    shell.finish_export();
    assert_eq!(dir.names(), ["four.GIF", "one.svg", "three.svg", "two.svg"]);
}

#[test]
fn cancel_and_failures_write_nothing_and_change_nothing() {
    let dir = Dir::new();
    let mut shell = animated();
    let source = shell.source();
    for item in [GIF, SVG, PNG] {
        shell.log.borrow_mut().answer = None;
        shell.model.status = "";
        pick(&mut shell, item);
        assert_eq!(shell.model.status, "", "cancel leaves the status alone");
        assert!(!shell.exports.busy());
    }
    assert!(dir.names().is_empty());
    assert_eq!(shell.source(), source);
    // A panel error shows the alert.
    shell.log.borrow_mut().answer = Some(PanelResult::Error);
    pick(&mut shell, SVG);
    assert_eq!(shell.log.borrow().errors.len(), 1);
    // An unwritable folder fails without leaving files behind.
    answer(&shell, &dir.file("nope/x.svg"));
    pick(&mut shell, SVG);
    assert_eq!(shell.log.borrow().errors.len(), 2);
    answer(&shell, &dir.file("nope/x.gif"));
    pick(&mut shell, GIF);
    assert_eq!(shell.log.borrow().errors.len(), 3);
    assert!(dir.names().is_empty());
}

#[test]
fn gif_row_is_disabled_for_a_static_document_and_inert() {
    let mut shell = Shell::new();
    shell.edit("a", 10.0);
    let l = shell.layout();
    assert!(
        !shell
            .model
            .target_enabled(&l, Target::MenuItem(ExportFormat::Gif))
    );
    assert!(
        shell
            .model
            .target_enabled(&l, Target::MenuItem(ExportFormat::Svg))
    );
    assert!(
        shell
            .model
            .target_enabled(&l, Target::MenuItem(ExportFormat::Png))
    );
    shell.press_button(EXPORT);
    shell.press_item(GIF);
    assert!(shell.log.borrow().saves.is_empty());
    assert!(
        shell.model.export_open,
        "a disabled row leaves the popover open"
    );
    assert!(!shell.exports.busy());
}

#[test]
fn the_one_off_grant_covers_the_staging_directory_only() {
    let dir = Dir::new();
    let target = dir.file("clip.gif");
    let stage = Stage::create(&target).unwrap();
    let policy = stage.policy().unwrap();
    assert!(policy.resolve_save_path(&stage.file).is_ok());
    for outside in [
        target.clone(),
        dir.file("other.gif"),
        dir.0.clone(),
        dir.file("sub/x.gif"),
        stage.dir.join("../clip.gif"),
    ] {
        let verdict = policy.resolve_save_path(&outside);
        assert!(verdict.is_err(), "{outside:?} -> {verdict:?}");
    }
    assert_eq!(
        std::fs::read_dir(&stage.dir).unwrap().count(),
        0,
        "fresh and empty"
    );
    stage.discard();
    assert!(dir.names().is_empty());
}

#[test]
fn popover_dismissal_hit_testing_and_exclusion() {
    let mut shell = animated();
    shell.press_button(EXPORT);
    let l = shell.layout();
    let pop = l.export_popover();
    // The popover sits over the inspector header strip.
    let under = (pop.x + pop.width / 2, pop.y + 1);
    assert!(l.section_headers[0].y <= under.1);
    assert_eq!(l.hit_test_menus(under, false, true), None);
    let collapsed = shell.model.ui.collapsed;
    assert!(shell.down(under), "the click is consumed by the popover");
    assert!(shell.model.export_open && shell.model.ui.collapsed == collapsed);
    // A row hit.
    let row = l.export_items()[SVG];
    assert_eq!(
        l.hit_test_menus((row.x + 3, row.y + 3), false, true),
        Some(Target::MenuItem(ExportFormat::Svg))
    );
    // Outside click closes it and does nothing else.
    shell.down((l.stage.x + 20, l.stage.y + 300));
    assert!(!shell.model.export_open);
    // Escape closes it and returns focus to Export.
    shell.press_button(EXPORT);
    assert!(shell.key(53, 0));
    assert!(!shell.model.export_open);
    assert_eq!(shell.model.focus_button, Some(Button::Export));
    // Opening MCP closes Export and the reverse.
    shell.press_button(EXPORT);
    let mcp = l.mcp;
    shell.down((mcp.x + mcp.width / 2, mcp.y + mcp.height / 2));
    assert!(shell.model.ui.mcp_open && !shell.model.export_open);
    shell.press_button(EXPORT);
    assert!(shell.model.export_open && !shell.model.ui.mcp_open);
}
