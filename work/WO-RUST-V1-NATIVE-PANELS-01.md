---
id: WO-RUST-V1-NATIVE-PANELS-01
revision: 1
status: READY
milestone: V1-SVG-COMPOSITION
owner: project-owner
executor: gpt-6-luna
base_sha: aefb9b70659a8d71165b70ff879964dbdf5328fc
spec_sha256: 1f1d4888f6620279d4050ce0aee355f0188613b2cc2625952390c1c7236638e9
parallel_safe: false
network: false
---
# Native Open / Save / Save As panels and Cmd+S, Cmd+Shift+S, Cmd+O wiring

## Goal
Drive the existing window-free `vexel-app` `project_session` module
(`ProjectSession`, `FileAction`, `shortcut_action`, `unsaved_changes_prompt`,
`SessionError::user_message`, accepted by WO-RUST-V1-SAVE-01) from the native
window: Open..., Save, Save As... panels, the unsaved-changes sheet and an error
alert, reachable by key shortcut and File menu items. Covers walkthrough step 4
(save/reopen) of docs/product/vexel-v1-vertical-slice.md at the UI edge only.
No change to the project format, session semantics or MCP.

## Dependencies
Integrated `project_session` (present at base_sha). Does not require
APP-COMPOSE-01; if it is integrated first, Open reuses its `open_path` for
`.svg` selections and this order only adds the `.vexel` route (disclose).
Read first: `rust/crates/vexel-app/src/project_session/{mod,actions}.rs`,
`rust/crates/vexel-app/src/main.rs` (event loop, `WindowEvent::KeyDown`,
`CloseRequested`), `rust/crates/vexel-platform-apple/src/native_window/`,
`src/platform/macos/platform_dialog.{h,m}` (C reference behaviour only; it is
not linked and must not be called), ADR-0026, ADR-0027,
`docs/dependencies/objc2-native-window.lock.json`.

## Coordinator-only prerequisite (check first, stop if unmet)
The lock already enables `objc2-app-kit` features `NSOpenPanel`, `NSSavePanel`,
`NSAlert`, `NSMenu`, `NSMenuItem` (Cargo.toml of `vexel-platform-apple` and the
lock agree). It does NOT enable `objc2-foundation` `NSURL` (panel results are
`NSURL`) nor `NSPathUtilities`. Before this order starts, the coordinator must
land an ADR/lock amendment (ADR-0027 style) adding exactly the Foundation
features the worker needs (expected: `NSURL`; the worker reports any further
feature as a blocker) plus matching `vexel-platform-apple/Cargo.toml` and
`Cargo.lock` entries. If `NSURL` is absent from the lock at start, stop and
report; the worker must not edit manifests, locks or docs, or add any package.
No new crate or version is expected, only a feature on a locked crate.

## Assumptions
- Panels and alerts run modally on the main thread only, through the existing
  `MainThread` marker; calls from another thread return an error, never panic.
- The platform crate returns plain bytes (UTF-8 path) and small enums; no
  Objective-C pointer escapes the crate (ADR-0024/0026).
- Menu activation and shortcuts both produce a `FileAction`; Cmd+S etc. go
  through the existing `shortcut_action`, menu items through the same enum.
- Panel file type is `vexel` for Save/Save As; Open accepts `vexel` (and `svg`
  only when APP-COMPOSE-01 is integrated).

## Scope
1. `vexel-platform-apple`: safe, main-thread-only functions
   `open_panel(MainThread, &[&str]) -> PanelResult`,
   `save_panel(MainThread, suggested_name: &str) -> PanelResult` and
   `confirm_unsaved(MainThread, &Prompt-like text) -> UnsavedChoice`
   (Save / DontSave / Cancel), `show_error(MainThread, title, message)`, where
   `PanelResult` is `Path(Vec<u8>) | Cancelled | Error`. Path bounded to 1024
   bytes, rejects NUL; save panel appends `.vexel` when missing. A small
   pure `PanelBackend` trait (or function table) lets the app swap in a fake.
2. File menu (Open..., Save, Save As...) with Cmd+O, Cmd+S, Cmd+Shift+S key
   equivalents, built with `NSMenu`/`NSMenuItem`; activation enqueues a
   `WindowEvent`-level file action on the existing queue (bounded, existing
   fault policy). Items are enabled/disabled by a pure function of
   `ProjectSession` state (Save disabled when clean and has a path).
3. `vexel-app`: a pure `file_flow` module: `handle(action, &mut ProjectSession,
   &dyn PanelBackend) -> FlowOutcome` implementing: Save (no path -> Save As
   panel), Save As, Open (dirty -> unsaved prompt -> Save/DontSave/Cancel, then
   panel, then `ProjectSession::open`), window close (`CloseRequested`) with the
   same unsaved prompt. Errors map to `SessionError::user_message` in the alert
   and the status area. Cancel at any step changes nothing.
4. Wire `main.rs` `KeyDown` through `shortcut_action` and menu events through
   `file_flow::handle`; after a successful open the new document replaces the
   session and the frame is marked dirty for redraw. Existing `--mcp-preview`
   and shell behaviour unchanged.

## Non-goals
Recent files, drag-and-drop open, autosave/recovery, multi-window/document,
export dialogs (PNG/GIF/SVG), sandbox/security-scoped bookmarks, localization,
Save As of the legacy `.svg` target, restyling the custom shell.

## Files allowed to touch
- `rust/crates/vexel-platform-apple/src/**` and `tests/**` (not `build.rs`
  unless the prerequisite proves it needs nothing new; otherwise report)
- `rust/crates/vexel-app/src/**`, `rust/crates/vexel-app/tests/**`
- `rust/fixtures/v1/panels/**` (new fixtures only)
- `artifacts/WO-RUST-V1-NATIVE-PANELS-01/**`

## Protected paths
Root and `rust/` `Cargo.toml`/`Cargo.lock`, every `Cargo.toml` (coordinator edits
manifests), `docs/**` including `docs/dependencies/**` and ADRs, all other
crates, `vexel-project*`, `vexel-v1-document`, C/ObjC under `src/`, `tests/`,
goldens, oracles, `STATE.md`, `LEDGER*.md`, `ledger.jsonl`, other work orders,
git HEAD/index. No network, installs, GUI launch, commit, stage, reset or
deletion. `unsafe` only where `vexel-platform-apple` already permits it for
objc2 calls and only inside its existing safe-boundary pattern; none in
`vexel-app`. Files under 800 lines.

## Acceptance criteria
1. Fake-backend `file_flow` tests: Save with no path invokes the save panel then
   writes the project; Save with a path writes without any panel; Save As always
   shows the panel and updates the session path; cancel leaves path, bytes and
   dirty flag unchanged.
2. Open on a clean session loads a fixture `.vexel` (`source()` equals the
   saved bytes); Open on a dirty session shows the prompt first and each of
   Save, Don't Save, Cancel produces the specified result (Save failing keeps
   the current document and aborts the open).
3. Close request: clean closes; dirty prompts; Cancel keeps the window open.
4. Failure paths from `SessionError` (truncated, bad magic, wrong version,
   missing file, unwritable directory) produce an alert carrying
   `user_message()` text and leave the live document and file unchanged.
5. Shortcut table test: Cmd+S, Cmd+Shift+S, Cmd+O map to Save, SaveAs, Open;
   Cmd+Option+S, Ctrl+S and plain S map to nothing; the menu item key
   equivalents and titles come from one table the test also reads. Enable/
   disable function covered by table tests.
6. Platform crate: bounded-path and NUL rejection, `.vexel` extension
   appending, and off-main-thread rejection are tested without opening any
   panel; no test creates an `NSApplication` window or shows a panel.
7. All new tests run without a display; existing `vexel-app` and
   `vexel-platform-apple` tests still pass (counts may only increase).
8. `cargo tree -p vexel-app -p vexel-platform-apple` shows no new package
   beyond those already locked; manifests untouched by the worker.
9. `result.md` (<=400 words) lists verified items and the manual live-check
   list below as UNEXECUTED.

If an AC cannot be met, stop with the exact blocker; no test weakening.

## Manual live-check list (coordinator, isolated desktop session, not worker)
1. Cmd+S on a new document opens the Save panel; saving creates a `.vexel`
   file; title/status shows the name; Cmd+S again saves silently.
2. Cmd+Shift+S always opens the panel; cancel changes nothing.
3. Cmd+O opens the Open panel, filters to `.vexel`, loads and redraws.
4. Open or close with unsaved edits shows the sheet with the three choices.
5. A deliberately corrupt `.vexel` shows the error alert and keeps the document.
6. File menu items show the shortcuts and enable/disable as specified; panels
   appear over the custom window without focus loss or crash.

## Verification (frozen; repo-local CARGO_HOME/CARGO_TARGET_DIR, locked, offline)
- `cargo +stable check -p vexel-app -p vexel-platform-apple --locked --offline`
- `cargo +stable test -p vexel-app --locked --offline`
- `cargo +stable test -p vexel-platform-apple --locked --offline`
- `cargo +stable fmt --manifest-path Cargo.toml --package vexel-app --package vexel-platform-apple -- --check`
- `python3 scripts/wo_validate.py WO-RUST-V1-NATIVE-PANELS-01 --ready`
- `python3 scripts/ledger.py validate ledger.jsonl`
- `git diff --check`

## Evidence
`artifacts/WO-RUST-V1-NATIVE-PANELS-01/`: `result.md`, `verification.json`,
`ac-map.json`, `changed-path-audit.json`.

## Worker and review
Worker: gpt-6-luna High, Standard speed, one writer, existing allowance only; stop
on quota errors. Candidate only until a fresh read-only reviewer (separate
session, Luna or Sol Medium, diff + order only) reruns the frozen commands and
checks each AC. The coordinator integrates the exact patch, performs the manual
live-check list, then updates STATE.md, LEDGER.md and ledger.jsonl.
