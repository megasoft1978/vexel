---
id: WO-UI-DOCS-01
revision: 1
status: READY
milestone: M7
owner: project-owner
executor: codex-gpt-6.1-sol
base_sha: c43fa826c0fbc8910e8dc0c09b4139c8aa615995
spec_sha256: 09553c1a90a1a3ce1d960b4077864688a90ac54d4278b4104c1cd0f863283287
parallel_safe: false
network: true
---
# Goal
Fix the animation editor UI issues found in Sol 6.1 review and publish accurate project and demo documentation on GitHub Pages.

# Authorization and ordering
- Owner authorizes this order to supersede the unfinished WO-M4-06 revision 4, defer its remaining renderer work, and prioritize this UI/documentation task. Preserve WO-M4-06's immutable specification and evidence; record its `blocked` then `superseded` transitions in `ledger.jsonl` without claiming acceptance.
- This order supersedes conflicting no-commit/no-network restrictions from WO-MOTION-03/04 only for the explicitly listed files and operations. Preserve those immutable specifications and prior evidence.
- Sol 6.1 is the requested reviewer/executor model for this order. No Claude Code use.
- Base is the clean remote `main` commit `c43fa826c0fbc8910e8dc0c09b4139c8aa615995`. Do not use the unrelated local `main` history or merge the unrelated MCP candidate. Confirm the base and clean state before execution.
- Owner has authorized a commit, push, and integration of the verified work. Merge only after required verification and independent review; do not force-push.

# Context pointers
- Read `AGENTS.md`, `OWNER-DIRECTIVES.md`, `STATE.md`, `docs/CODING_STANDARD.md`, `docs/testing.md`, `src/ui/README.md`, and `docs/research-r4/01-operating-model.md`, `docs/research-r4/02-work-order-template.md`.
- Review the Sol 6.1 findings retained in the task handoff: grid-area timeline scrolling, actual visible-row reveal, truncated layer IDs, compact icon sizing, missing timeline control icons/tooltips, and opaque animation-track labels.
- GitHub Pages is configured for `main:/docs` at `https://megasoft1978.github.io/vexel/`.

# Contracts and invariants
- Apple Silicon macOS 13+, C17, MPL-2.0, custom-drawn UI; no new dependencies, platforms, external SVG loading, or GPL source.
- Preserve SVG/XML source bytes and animation evaluator behavior. Keep touched source modules under 800 lines.
- Do not weaken assertions, tolerances, or frozen oracles. Do not alter unrelated screenshot baselines.
- Describe the demo honestly: it is a vector trace/reconstruction with transform articulation and squash/stretch. It does not establish exact original-drawing fidelity, bone skinning, or path morphing. Do not imply the software license covers the supplied artwork.
- External animation project is read-only. Import only the three assets listed below.

# Files allowed to touch
- `work/WO-UI-DOCS-01.md` (freeze only)
- `src/app/viewer_timeline.c`, `src/app/viewer_timeline.h`, `src/app/viewer_timeline_layout.c`, `src/app/viewer_timeline_ui.c`, `src/app/viewer_timeline_accessibility.c`
- `src/app/viewer_chrome_structure.c`, `src/app/viewer_editor_widget_paint.c`
- `src/ui/ui_widgets.h`, `src/ui/ui_widget_paint.c`, `src/ui/ui_widget_chrome_paint.c`, `src/ui/ui_icons.h`, `src/ui/ui_icon_paths.c`, `src/ui/ui_icon_paths.h`
- `tests/app/viewer_timeline_test.c`, `tests/app/viewer_timeline_keyframe_test.c`
- `tests/ui/ui_kit_test.c`, `tests/ui/ui_interaction_test.c`, `tests/ui/viewer_timeline_render.c`
- `tests/ui/style-golden/**`, `tests/ui/chrome-golden/**`, `tests/ui/timeline-golden/**`, `tests/ui/uikit-golden/**` (only directly affected images/manifests)
- `tests/test_map.json`
- `CMakeLists.txt` (test registration only)
- `README.md`, `docs/index.html`, `docs/examples/README.md`
- `docs/examples/un-cuore-per-due-vector-rig-v2.svg`
- `docs/examples/un-cuore-per-due-vector-rig-preview-v3.gif`
- `docs/examples/v3-contact-sheet-8fps.png`
- `artifacts/WO-UI-DOCS-01/**`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`

# Protected paths
- `OWNER-DIRECTIVES.md`, `AGENTS.md`, every other `work/**` file, and all paths outside the allowlist.
- `tests/oracle/**`, `tests/golden/**`, frozen manifests, and all unrelated existing UI baselines/manifests.
- `scripts/**`, `dev`, build presets, dependency/release configuration, prior artifacts, and benchmark results.
- `src/mcp/**`, SVG parser/rasterizer/document/evaluator code, and all external source artwork.
- The external directory `/Users/megasoft78/Desktop/Freelance/animazioni/animazione_output/vexel-deep-test` is read-only.

# Required implementation
1. Vertical wheel scrolling over either the timeline gutter or keyframe grid scrolls tracks. Preserve Command-wheel zoom and horizontal pan; clamp scroll range after resizing and document changes.
2. Revealing a selected track uses actual visible row count and works for compact docks and the final track. Expose an accessible indication of additional offscreen tracks.
3. Prefer meaningful layer IDs over generic `Group`/`Path` labels within available row width. Bound indentation without losing hierarchy cues; retain eye/lock controls, complete accessible names, and full-name hover/focus labels for truncated rows.
4. Use approximately 16-point compact glyphs with at least six-point inset when geometry permits. Keep hit targets, selected/disabled states, stroke style, and backing-scale behavior intact.
5. Give Play/Pause, playback mode, and Fit appropriate icons, localized tooltips, and accessible action names. Keep speed and elapsed/duration values as readable text; bound tooltip placement to the viewport.
6. Give animation tracks distinguishable labels using stable animation or target IDs plus attribute/type context. Keep anonymous fallbacks and expose full labels to hover/accessibility.
7. Update the root README and `docs/index.html` with current app capabilities, build/run instructions, live/headless MCP behavior as present on remote main, supported animation behavior, requirements, license, limitations, website/demo links, and the demo preview/downloads. Do not document candidate-only features as shipped.
8. Copy exactly the following outputs byte-for-byte from the read-only source directory, and document hashes, dimensions, animation metadata, and provenance in `docs/examples/README.md`:
   - `un-cuore-per-due-vector-rig-v2.svg`
   - `un-cuore-per-due-vector-rig-preview-v3.gif`
   - `screenshots/v3-contact-sheet-8fps.png`
   Do not copy original scans, voice recordings, or individual frame captures. Record the distinction between the 11.01-second SVG scene and the GIF's 88 frames × 120 ms = 10.56-second encoded loop.
9. Document evidenced MCP workflow issues: element handles refresh after mutations, coordinated timing attributes need atomic updates, and built-in full-page animation export failed despite successful MCP frame rendering. Do not imply this order changes MCP source.
10. For every directly affected UI screenshot baseline, preserve the original under `artifacts/WO-UI-DOCS-01/baseline-before/`, record before/after hashes and rationale, and create a comparison sheet. Update only directly affected manifest entries. Do not change thresholds/assertions; a fresh independent reviewer must approve the differences before integration.
11. Retain deterministic headless UI evidence. The owner previously authorized native screenshots during this task; any native capture must use supported computer-use tooling, avoid unrelated apps, and not replace unsaved user documents.

# Acceptance criteria
- AC-1: Focused timeline tests cover twelve tracks, one track, compact dock, last-track reveal, clamping, scrolling over grid and gutter, and preserved zoom/pan modifiers.
- AC-2: Headless evidence covers deep nesting/long layer IDs, compact glyph geometry, tooltip bounds, and complete accessible names in light, dark, and increased contrast at 1× and 2×.
- AC-3: Timeline controls expose correct action names, modes, values, and disabled states through accessibility snapshots.
- AC-4: Imported file hashes match source; SVG has 12 named spline-eased tracks; GIF metadata and duration are recorded; exactly the three allowed artwork assets are imported.
- AC-5: README and Pages links/downloads resolve locally; page is usable at desktop and narrow widths; statements match code and evidence.
- AC-6: `./dev build`, `./dev t --all`, `python3 scripts/ledger.py validate ledger.jsonl`, and `git diff --check` pass. Preserve exact output in `artifacts/WO-UI-DOCS-01/`.
- AC-7: Fresh read-only verification passes changed-path scope, preserved tests/oracles, relevant baselines and comparison sheet, UI evidence, documentation claims, and asset provenance with no high/critical findings.
- AC-8: Push/merge outcome is recorded. After integration, check `https://megasoft1978.github.io/vexel/`, deployment status, and asset links; report actual published/pending/failed status.

# Verification sequence
1. Confirm pinned clean base, validate frozen spec and the M4-06 supersede transitions; inventory affected screenshots and test targets.
2. Capture focused pre-fix failures; implement fixes and run focused regressions/headless render evidence.
3. Preserve and compare affected baselines; run all required checks and asset/documentation checks.
4. Obtain fresh read-only review before integrating. Correct genuine findings and rerun affected gates.
5. Commit the verified changes, push the task branch, merge only if all gates pass and the pinned base remains valid (otherwise rebase and reverify). Verify GitHub Pages deployment after merge.

# Non-goals
MCP source/protocol fixes, animation exporter implementation, background extraction, true bone skinning, path morphing, renderer features, additional artwork imports, new dependencies, and non-macOS support.

# Budget
Two implementation/review rounds; 120-minute hard limit per run; no dependencies or package downloads. Network is limited to GitHub branch/PR and Pages status operations. Do not switch models or services if Sol 6.1 is unavailable.

# Failure and escalation
- Never report unsupported, unverified, committed, pushed, merged, or published outcomes as complete.
- If a UI change requires protected source or an unlisted baseline family, stop and propose a bounded revision before editing it.
- If an unrelated required CI gate fails, preserve its output and do not weaken it. Report exact blockers; do not bypass required checks.
- If GitHub Pages needs configuration outside the allowlist, push the allowed `/docs` changes only if the repository's merge gate permits and report the exact remaining configuration.

# Output
Commit: `fix(ui): improve animation workflow and publish current project docs`. Report exact checks, independent review, before/after evidence, asset provenance/metadata, state and ledger updates, commit/PR/merge/push identifiers, Pages status, and remaining limitations.
