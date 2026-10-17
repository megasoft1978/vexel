---
id: WO-RUST-UI-WIRING-01
revision: 1
status: READY
milestone: UI-MODERN-2026-10
owner: project-owner
executor: gpt-6-luna
base_sha: 6c7544de24d19e75cca67c7f48dfe9ce5077e634
spec_sha256: d2ef82260ea3dad77eb6db6cda1144641e03911fa61b14e174cd7de6bfc06d53
parallel_safe: False
network: false
---
# UI wiring: toolbar buttons, history truth, Export menu, keyboard activation

## Goal
Live review (`docs/archive/live/2026-10-17-ui-upgrade-final-draw-light.png`)
shows Undo, Redo, Save, Export and the inspector toggle drawn, hovered and
enabled correctly, but `route_pointer` in `main.rs` only handles
`Target::Tool`; `Model::pointer` handles only modes, section headers,
ToggleInspector and Mcp. Clicking Undo/Redo/Save/Export does nothing. Make
every drawn control do what it says, by pointer and keyboard. `vexel-app` only.

## Dependencies
CHROME, MOTION, POLISH-01 merged (HEAD). The TIMELINE-02 order runs concurrently
and owns `src/timeline/**`, `src/animation/**` and `layout/state.rs`
footer code: do not touch them. Read first: `main.rs` (`route_pointer`,
`edit_shortcut`, `file_key`, `apply_file_outcome`), `model.rs`
(`target_enabled`, `pointer`, `focus_key`), `history.rs`, `file_ui.rs`,
`file_flow/{mod,menu}.rs`, `layout/{mod,state}.rs`, `motion/`,
`animation/export.rs`, `session/{mcp_tools,mcp_png}.rs`.

## Assumptions
- `V1Document::history_count()`/`history_cursor()` exist (vexel-v1-document
  lib.rs:134-139; POLISH-01 wrongly said no accessor). Undo is available iff
  `cursor > 0`, Redo iff `cursor < count`; the app-side `History` redo
  counter is removed and `Model::target_enabled` reads the document. Dirty
  stays separate and only gates Save.
- Save: Save enabled iff open and dirty, same as today; click runs the same
  `FileAction` as Cmd+S through `FileUi` (untitled doc opens the native Save
  panel). Save As stays a menu/Cmd+Shift+S action.
- Export has no UI-callable path today: `export_gif` needs an `AccessPolicy`
  grant and `export_svg` returns inline JSON only. Minimal design: Export click
  opens a 3-item popover (Export as GIF..., SVG..., PNG...) drawn like the MCP
  popover (new `layout/menu.rs`, reuse popover tokens). Each item opens the
  native Save panel from the panel backend (extension preset), builds a one-path
  grant for the chosen file only, then calls the existing exporter (GIF: loop
  duration, 24 fps cap 30; PNG: document size). SVG writes `source_bytes()` with
  the same temp-sibling-then-rename rule. GIF item disabled when the document
  has no animation. Cancel changes nothing. If a PNG path cannot be reached
  without editing a protected path, STOP and report; ship GIF and SVG.
- Keyboard: Tab stops for Undo, Redo, Save, Export, ToggleInspector join
  `FOCUS_ORDER` after the mode control (disabled buttons skipped); Enter/Space
  on a focused button activates it exactly like a click; Escape closes the
  Export popover and returns focus to Export.

## Scope
1. `Model` gains `fn button(&mut self, b: Button, layout) -> Option<ButtonAction>`
   (`Undo | Redo | Save | Export(Format)`) returned to `main.rs`, which executes
   it through the existing paths (`document_mut().undo()/redo()`, `files`).
   `route_pointer` calls it on Down for enabled buttons only (at most 40 changed
   `main.rs` lines; logic lives in `model.rs`/new `src/toolbar_actions.rs`).
2. Undo/Redo by button and by Cmd+Z / Cmd+Shift+Z share one function; every
   path updates dirty, status text and redraw.
3. Delete `history.rs` redo counter; enablement from core cursor.
4. Export popover, hit-testing under the popover (`hit_test_with`), dismiss on
   outside click, hover and focus ids in the motion registry.
5. ToggleInspector, section header collapse/expand, splitter drag already have
   handlers: add the tests below to prove them, fixing any defect found.

## Non-goals
New export formats or options, Save As button, MCP behaviour, tools, tooltips
text changes, timeline/footer, other crates, persistence of UI state.

## Files allowed to touch
- `rust/crates/vexel-app/src/{main.rs,model.rs,history.rs,file_ui.rs,toolbar_actions.rs}`
  (`main.rs` call sites only), `src/motion/**`, `src/layout/{mod.rs,menu.rs,paint.rs}`
  except footer/status-hint hunks, single-line `lib.rs` `mod` additions
- `rust/crates/vexel-app/tests/ui_wiring_*.rs` (new), `rust/fixtures/ui/wiring/**`
  (new), `artifacts/WO-RUST-UI-WIRING-01/**`

## Protected paths
Every `Cargo.toml`, lock files, other crates (including `vexel-v1-document`,
`vexel-ui`), `src/timeline/**`, `src/animation/**`, `src/layout/state.rs`,
`chrome.rs`, C/ObjC, `tests/` other than the new files, accepted goldens and
oracles, `docs/**`, `STATE.md`, `LEDGER*.md`, `ledger.jsonl`, other work orders,
git HEAD/index. No unsafe, new deps, network, installs, GUI launch by the
worker, commit, stage, reset, deletion. Files under 800 lines.

## Acceptance criteria (headless)
1. Synthetic `PointerEvent` Down on the centre of each enabled toolbar button
   through `route_pointer` (fake panels, 1280x800): Undo reverts one edit,
   Redo reapplies it, Save writes the file and clears dirty, Export opens the
   popover; the same events on disabled buttons change nothing.
2. Button Undo/Redo and Cmd+Z / Cmd+Shift+Z produce byte-identical
   `source_bytes()` and `history_cursor()` after the same scripted sequence
   (edit, edit, undo, undo, redo, edit): redo enabled after each undo, disabled
   after the final edit; load and MCP edits reset it. Enablement equals
   `cursor>0` / `cursor<count` at every step; `history.rs` holds no counter.
3. Save click on a titled doc writes the same bytes as Cmd+S; on an untitled
   doc it opens the (fake) Save panel once; cancel leaves dirty set; a failing
   backend shows the alert and keeps the document.
4. Export: each menu item calls the fake Save panel with the right extension;
   GIF/SVG/PNG files are written only to the chosen path (temp sibling renamed;
   read back: GIF89a header, SVG equals source, PNG signature and size); cancel
   writes nothing; GIF item disabled for a static document; no grant widening
   beyond the single path (assert policy).
5. Popover: a click outside closes it; a click under it hits the popover; Escape
   closes it; opening MCP closes Export and the reverse.
6. ToggleInspector flips visibility and the stage width changes accordingly;
   clicking each section header toggles exactly that section and not others;
   splitter Down/Move/Up changes inspector width within clamps and ends cleanly
   on Up outside the window; 2,000 seeded pointer sequences never panic.
7. Keyboard: Tab visits mode, Undo, Redo, Save, Export, inspector toggle in
   order, skipping disabled; Enter and Space on each activate like a click;
   Escape closes the popover; focus ring visible on each (ControlVisual test).
8. Golden candidates: 1280x800 light and dark with Export popover open, plus
   a hover frame on Undo, 1x and 2x, re-blessed with `VEXEL_BLESS_GOLDENS=1`
   into `artifacts/WO-RUST-UI-WIRING-01/review/` and recorded in `result.md`
   (which existing goldens changed, and why). Never accepted by the worker.
9. Existing tests pass, changed pins listed; no new deps; no unsafe.

If an AC cannot be met, stop with the exact blocker; no test weakening.

## Verification (frozen; repo-local CARGO_HOME/CARGO_TARGET_DIR, locked, offline)
- `cargo +stable check -p vexel-ui -p vexel-app --locked --offline`
- `cargo +stable test -p vexel-ui --locked --offline`
- `cargo +stable test -p vexel-app --locked --offline`
- `cargo +stable fmt --manifest-path Cargo.toml --package vexel-ui --package vexel-app -- --check`
- `python3 scripts/wo_validate.py WO-RUST-UI-WIRING-01 --ready`
- `python3 scripts/ledger.py validate ledger.jsonl`
- `git diff --check`

## Live check (coordinator, a person at the Mac)
Open compose-v1.svg, light and dark. Find the window with Quartz
`CGWindowListCopyWindowInfo`, `screencapture -x -l <id>`, view each PNG. Draw a
rectangle: Undo button reverts it, Redo restores it, Cmd+Z/Cmd+Shift+Z agree,
Save clears the dirty dot (check the file), Export shows the menu and a real
GIF, SVG and PNG open in Preview, Tab/Enter/Space drive the toolbar, the
inspector toggle and section headers respond, splitters drag. Record as
`docs/archive/live/YYYY-MM-DD-ui-wiring-01-<what>.png` with README lines
(shown, verified, unverified), then accept goldens.

## Evidence
`artifacts/WO-RUST-UI-WIRING-01/`: `result.md` (<=400 words, incl. the export
path chosen per format and any STOP), `verification.json`, `ac-map.json`,
`changed-path-audit.json`, `review/`.

## Worker and review
Worker: gpt-6-luna High, one writer, existing allowance only; stop on quota
errors. Candidate until a fresh read-only reviewer reruns the frozen commands
and checks each AC. The coordinator integrates, merges after or with TIMELINE-02
(disjoint paths), does the live check, accepts goldens, then updates STATE.md,
LEDGER.md and ledger.jsonl.
