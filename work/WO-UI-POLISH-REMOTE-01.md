---
id: WO-UI-POLISH-REMOTE-01
revision: 1
spec_sha256: 34f117a3ec15d8e333c483ef2728b676dd8211d69cd69eca7877dd421f50bb83
status: READY
milestone: UI
owner: project-owner
executor: codex-gpt-6-luna
base_sha: c43fa826c0fbc8910e8dc0c09b4139c8aa615995
parallel_safe: false
network: false
---
# Goal
Apply the screenshot-reviewed UI polish to GitHub's `main`: reduce icon glyph visual weight without changing click targets and make the canvas status pill more compact.

# Scope
- `src/ui/ui_widget_paint.c`
- `src/app/viewer_chrome_editor.c`
- `artifacts/WO-UI-POLISH-REMOTE-01/**`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`

# Constraints
Keep 32-point icon-button targets and close glyph sizing. Preserve status content and interactions. macOS 13+ only, C17, no dependencies. Do not alter tests, goldens, or other work orders.

# Acceptance
- Common icon glyphs render at 15 points; close glyph remains 14 points; button targets are unchanged.
- Canvas status text remains readable in a shorter and narrower pill.
- Debug app bundle builds and the saved animated cartoon is visually reviewed in the updated app.
- Record evidence, validate the ledger, and keep changes within scope.
