# WO-RUST-V1-NATIVE-PANELS-01 result (candidate, worker claude-sonnet-5-5)

Prerequisite checked: `NSURL` is in `vexel-platform-apple/Cargo.toml` and `docs/dependencies/objc2-macos.lock.json` (ADR-0030). No manifest, lock or doc edited; `cargo tree` shows no new package.

## Implemented
- `vexel-platform-apple/src/panels/`: `open_panel`, `save_panel`, `confirm_unsaved`, `show_error` (main-thread re-checked, error value off-main), `PanelResult`, `UnsavedChoice`, `UnsavedText`, `PanelBackend` trait, `NativePanels`, `checked_path` (<=1024 bytes, no NUL), `with_project_extension`.
- `native_window/file_menu.rs`: File menu (NSMenu/NSMenuItem) from application-supplied specs; activation enqueues new `WindowEvent::MenuCommand(tag)` on the existing queue; `NativeApp::install_file_menu`, `set_file_menu_enabled`.
- `vexel-app/src/file_flow/`: pure `handle`, `handle_close`, one `FILE_MENU` table (titles, keys, key codes), `menu_enabled`.
- `vexel-app/src/file_ui.rs` glue; `main.rs` edits: KeyDown via `shortcut_action`, `MenuCommand`, `CloseRequested`, window title.

## Verified (frozen commands, all exit 0)
check; `vexel-app` tests 45 pass (14 new); `vexel-platform-apple` tests 48+5+1+6 pass (4 new); fmt --check; wo_validate --ready; ledger validate; git diff --check. Logs in this directory.

## Disclosures
- Unsaved-changes prompt is a modal NSAlert (3 buttons), not a window-attached sheet.
- Open panel offers `vexel` only (APP-COMPOSE-01 not integrated).
- main.rs starts with a blank clean 800x600 untitled document so Save/Save As are reachable; the existing shell still draws "No document open" and does not render the opened document (compose order owns rendering).
- Panel file types use deprecated `setAllowedFileTypes` (no UTType framework in the lock).

## Manual live-check list: UNEXECUTED
1. Cmd+S on new document opens Save panel, creates `.vexel`, title shows name, Cmd+S again silent.
2. Cmd+Shift+S always opens panel; cancel changes nothing.
3. Cmd+O opens Open panel filtered to `.vexel`, loads and redraws.
4. Open/close with unsaved edits shows the three-choice alert.
5. Corrupt `.vexel` shows error alert, document kept.
6. File menu shows shortcuts and enable/disable; panels appear over the custom window without focus loss or crash.
