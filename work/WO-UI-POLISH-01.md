---
id: WO-UI-POLISH-01
revision: 1
spec_sha256: 45511e8a8ce20180e0b57385e16a0e6d3122bdabdf419edc30baa8fc2adc5a3c
status: READY
milestone: UI
owner: project-owner
executor: codex-gpt-6-luna
base_sha: f3e28cdc2f465cbab3ec0ecc55490a44cd7800c0
parallel_safe: false
network: false
---
# Goal
Polish the editor controls visible in the approved MCP cartoon screenshots: reduce icon glyph visual weight without changing hit targets, and make the canvas status pill more compact and less intrusive. Verify the result in the built macOS app with the saved animated cartoon open.

# Scope
- `src/ui/ui_widget_paint.c`
- `src/app/viewer_chrome_editor.c`
- `artifacts/WO-UI-POLISH-01/**`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`

# Constraints
Keep the 32-point icon-button hit areas and close glyph sizing. Preserve status information and interaction behavior. macOS only, C17, no new dependencies. Do not modify any frozen golden, test, or other work order.

# Acceptance
- Common icon glyphs are visibly smaller and balanced inside existing targets; close glyph stays unchanged.
- Canvas status uses a compact pill with readable text and the existing information.
- Debug app bundle builds successfully.
- Capture and inspect screenshots with the saved animation open, then record evidence and this work in `STATE.md` and ledgers.
- `git diff --check` is clean and all modifications stay in scope.
