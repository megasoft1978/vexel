---
id: WO-UI-01
revision: 2
status: READY
milestone: M6
owner: project-owner
executor: deepseek-flash
base_sha: e31084f396d4d6ddabcfffb99f346d741f3a8349
spec_sha256: 2c5b47bef3e920d2d90b77126f7b6fd47a159fe4775eb9532edcd631995e4595
parallel_safe: false
network: false
---
# Goal
Adopt the first two Blender-alignment patterns from `docs/adr/0017-ui-direction-blender-aligned.md` and `docs/ui/blender-alignment-review.md`: **task workspaces** in the top bar, and **one consolidated status bar** replacing the duplicated tool-hint row.

# Context pointers
- Read: `AGENTS.md`, `OWNER-DIRECTIVES.md`, `docs/adr/0016-product-target-and-gates.md`, `docs/adr/0017-ui-direction-blender-aligned.md`, `docs/ui/blender-alignment-review.md`, `docs/ui/editor-interaction.md`, `docs/ui/README.md`.
- Current chrome: `src/app/viewer_chrome_structure.c`, `viewer_chrome_render.c`, `viewer_chrome.h`, `viewer_editor_internal.h` (layout constants), `src/ui/ui_core.h` (widget tree, roles, states), `src/platform/platform_settings.h`.
- Current fixtures: `tests/ui/chrome-golden/`, `tests/ui/style-golden/`, `tests/ui/command-overlay-golden/`, `tests/editor/chrome-golden/`; verifiers `tests/ui/verify_chrome.py`, `tests/ui/verify_style_goldens.py`, `tests/editor/verify_chrome.py`.
- Reference behaviour: Blender Topbar workspaces and Status Bar, cited in the review document.

# Existing contracts and invariants
- macOS only, C17, custom-drawn chrome, no new dependency, MPL-2.0. Objective-C stays inside `src/platform/macos/`.
- The retained widget tree keeps stable IDs, bounded depth and full UTF-8 semantic text; every interactive control stays keyboard reachable and exposes role, label, value and state. Existing semantic-group assertions in `tests/test_w15_accessibility_localization.py` must keep passing or be extended, never weakened.
- **Workspace switching never touches the document.** No `document_edit` call, no source byte change, and an unchanged file must still save byte-identically after any sequence of workspace switches.
- The command palette, rulers/guides, snapping, timeline, selection and accessibility behaviour already merged must keep working.
- Source files stay under 800 lines; split rather than grow.
- Existing golden fixtures may change only with an explicit baseline-change record listing each changed file, the reason, and a side-by-side comparison; a new fixture is preferred over rewriting an unrelated one.

# Files allowed to touch
- `src/app/**` (chrome, editor, runtime, settings wiring)
- `src/ui/**`
- `src/platform/**` (settings persistence only)
- `app/**` (only if a target's source list changes)
- `tests/ui/**` (excluding `tests/oracle/**` and `tests/golden/**`)
- `tests/editor/**`
- `tests/test_ui_chrome_golden.py`
- `tests/test_w15_accessibility_localization.py`
- `CMakeLists.txt`
- `docs/ui/**`
- `artifacts/WO-UI-01/**`
- `STATE.md`
- `LEDGER.md`
- `ledger.jsonl`
- `work/WO-UI-01.md` (this order's own frozen spec)

# Protected paths
- `OWNER-DIRECTIVES.md`, `AGENTS.md`, `docs/adr/**`, `tests/oracle/**`, `tests/golden/**`, `bench/**`, `reports/**`, all prior artifacts, `src/svg/**`, `src/raster/**`, `src/document/**` other than call sites that must read the current selection, and every path outside the allowlist.

# Required implementation
1. Add a **workspace strip** to the editor top bar with exactly four workspaces: `Draw`, `Nodes`, `Animate`, `Inspect`. The strip shows the active workspace distinctly, is reachable by pointer and by keyboard, and exposes each workspace as a widget-tree node with a stable ID, role, label, checked state, and `⌃⇥`-style cycle hints.
2. Make each workspace a **declarative record of pane visibility and active inspector group** rather than imperative show/hide calls. `Draw`: structure panel + inspector, no timeline, inspector group `Object`. `Nodes`: structure panel + inspector focused on geometry/path, no timeline. `Animate`: structure panel + timeline + inspector. `Inspect`: structure panel + inspector, timeline hidden, inspector group `Document`. Store the table in one place and drive both layout and the semantic tree from it.
3. **Persist the active workspace** through the existing settings mechanism, defaulting to `Draw`. An unknown or missing stored value must fall back to `Draw` without error and without a settings write storm. Record the settings key name in `docs/ui/README.md`.
4. **Consolidate status and feedback into one status bar.** Remove the duplicated full-width tool-hint row. The status bar gains three regions: left = active-tool hint and shortcut, centre = progress message with a cancel affordance when an operation is in progress and transient report text otherwise, right = readouts (selection count, zoom, document size). Long-running operations already present (open/index, export, render) must route their message into the centre region rather than into the removed hint row or the window status text.
5. Add focused tests covering: the four workspaces exist with the documented pane visibility; switching changes exactly the documented panes; cycling wraps in both directions; the active workspace persists across a settings save/load round-trip; an unknown stored name falls back to `Draw`; the status bar exposes hint, progress and readout regions and the removed hint row no longer exists; and a workspace-switch sequence leaves the document bytes unchanged.
6. Update `docs/ui/editor-interaction.md` with the workspace shortcuts and the status-bar regions, and `docs/ui/README.md` with the settings key.
7. Update or add deterministic fixtures for the chrome in dark and light at 1× and 2×, and record the baseline change for **every** changed fixture, manifests included: the reason, the before/after SHA-256, and a red-highlight comparison for each PNG.
8. The tab strip exposes the cycle shortcut and the workspace applies a real inspector section; the status bar's centre region offers a cancel affordance while an operation runs, and its message is not repeated in the right-hand readout.

# Acceptance criteria (machine-checkable)
- AC-1 `cmake --preset mac-debug && cmake --build --preset mac-debug && ctest --test-dir build/mac-debug --output-on-failure` exits 0.
- AC-2 `./dev verify` exits 0 (Debug and Release builds, all CTest, all Python tests).
- AC-3 `./dev t --all` exits 0.
- AC-4 The new focused workspace/status tests are registered under CTest. Because the workspace model does not exist at the base revision, record the pre-change failure as the compile-time diagnostic from building `tests/editor/viewer_workspace_test.c` against `base_sha` (`viewer_workspace.h` not found) in `artifacts/WO-UI-01/pre-change.md`, instead of a runtime failure.
- AC-5 `python3 -m unittest tests.test_ui_chrome_golden` and `python3 -m unittest tests.test_w15_accessibility_localization` exit 0.
- AC-6 `./dev editor-smoke` exits 0 and switches workspaces from the keyboard in a real window: it sends the cycle shortcut, asserts the newly active tab through the live layout dump, cycles back with wrap, and the run starts from a pinned workspace so a remembered user choice cannot change the fixture.
- AC-7 Workspace switching leaves the document unchanged, verified through a real save: the smoke switches through every workspace and the save assertion that follows compares the file written to disk with the previously saved bytes.
- AC-8 `artifacts/WO-UI-01/golden-changes.md` lists every changed or added fixture with its reason and side-by-side comparison; no fixture changed without an entry.
- AC-9 `python3 scripts/ledger.py validate ledger.jsonl`, `python3 scripts/wo_validate.py WO-UI-01 --ready` and `git diff --check` exit 0.
- AC-10 A fresh read-only verifier reports pass against the exact candidate commit.

# Verification commands
Run sequentially from the repository root unless the command changes directory itself.

```text
cmake --preset mac-debug
cmake --build --preset mac-debug
ctest --test-dir build/mac-debug --output-on-failure
./dev verify
./dev t --all
python3 -m unittest tests.test_ui_chrome_golden
python3 -m unittest tests.test_w15_accessibility_localization
./dev editor-smoke
python3 scripts/ledger.py validate ledger.jsonl
python3 scripts/wo_validate.py WO-UI-01 --ready
git diff --check
```

# Non-goals
No pane splitting/joining, no per-pane editor-type switcher, no properties icon tabs, no tool-options footer, no Adjust Last Operation panel, no log pane, no density token rework, no change to any editing semantic, renderer, parser, rasteriser, document model or oracle. No pie menus, no keymap editor, no theme matrix (see ADR-0017).

# Budget
Executor max runs: 3; verifier max rounds: 2; elapsed hard limit: 240 min; dependency/network: none.

# Failure and escalation
If persisting the workspace would require changing the settings file format incompatibly, stop and record the required migration in `artifacts/WO-UI-01/` for planner review rather than silently dropping existing preferences. If consolidating the status bar requires routing operation progress from a component this order may not touch, record the exact call site and implement the centre region with the messages available, leaving the remaining routing to a follow-up order.

# Output
One implementation commit using `feat(ui): add task workspaces and a single status bar`. Include the focused-test, golden-change and document-invariance evidence under `artifacts/WO-UI-01/`, update `STATE.md` and both ledgers, and provide the schema-valid executor report.
