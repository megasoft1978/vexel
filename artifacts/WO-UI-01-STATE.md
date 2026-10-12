# WO-UI-01 in-progress state (round 1)

Committed as a WIP checkpoint because the round budget ended mid-implementation.
This file is deleted when the work order is completed.

## Done

- `src/app/viewer_workspace.{h,c}`: the workspace model — `draw`, `nodes`,
  `animate`, `inspect`, each with pane visibility, an inspector group and a
  stable settings name; `viewer_workspace_from_name` (unknown → default),
  `viewer_workspace_cycle` (wraps both ways). Registered in `CMakeLists.txt`
  for `vexel_editor_ui` and `viewer_chrome_layout_fuzz`.
- `viewer_editor_internal.h`: `workspace` state, the workspace-changed callback,
  and new IDs `EDITOR_NODE_WORKSPACES` (24510), `EDITOR_BUTTON_WORKSPACE_BASE`
  (24520), `EDITOR_NODE_STATUS_HINT` (24530), `EDITOR_NODE_STATUS_REPORT`
  (24531), `EDITOR_NODE_STATUS_READOUT` (24532).
- `viewer_chrome_editor.c`: the workspace tab strip replaces the duplicated tool
  hint in the options band (tab per workspace, `UI_STATE_CHECKED` on the active
  one); the status bar now has three regions — hint (left), report (centre),
  readout (right).
- `viewer_editor.{h,c}`: `viewer_editor_set_workspace`, `viewer_editor_cycle_workspace`,
  `viewer_editor_apply_workspace_name`, `viewer_editor_set_workspace_callback`,
  `viewer_editor_workspace`. The editor core takes no platform dependency; the
  native layer persists through the callback.
- `viewer_editor_view.c`: workspace tab dispatch.
- `viewer_editor_input.c`: `Control-Tab` / `Control-Shift-Tab` cycles workspaces.
- `platform_settings.{h,m}` + `viewer.c`: the workspace name is stored and
  restored (honouring `rememberWorkspace`), via `platform_settings_store_workspace`
  and a runtime callback.
- Debug build passes with warnings-as-errors.

## Not done (next)

1. Golden baseline updates with the baseline-change record (`artifacts/WO-UI-01/golden-changes.md`).
   Five tests fail today, all because the chrome intentionally changed:
   `editor-chrome`, `viewer-chrome-style`, `editor-command-overlay-golden`,
   `editor-snap-popover-goldens`, `editor-accessibility-localization-goldens`.
   `viewer-chrome-style` is a code test (`viewer_chrome_test`) asserting the old
   options/status node shape and must be updated deliberately.
2. Focused tests: the four workspaces and their pane visibility, switching changes
   exactly the documented panes, cycling wraps both ways, persistence round-trip,
   unknown name falls back to `draw`, the status bar exposes hint/report/readout,
   and a workspace-switch sequence leaves the document bytes unchanged.
3. `./dev editor-smoke` keyboard coverage for the workspace switch.
4. Full frozen-command sweep, evidence, verifier round, ledger events, merge.
