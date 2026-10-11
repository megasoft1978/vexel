---
id: WO-MOTION-04
revision: 1
spec_sha256: 04e49cfa822c12a9eb01ae345fd035b741b0e19561e3a45694088ead0beca5a4
status: READY
milestone: M7
owner: project-owner
executor: codex-gpt-6
base_sha: 3739b0b2dcce1dae5c4959452565d1a4dfdb2188
parallel_safe: false
network: false
---
# Goal
Add a contextual, keyboard-operable SMIL easing editor for the selected key's outgoing interval.

# Context pointers
- Read: `AGENTS.md`, `OWNER-DIRECTIVES.md`, `STATE.md`, `docs/CODING_STANDARD.md`, `docs/testing.md`, `src/ui/README.md`, and the WO-MOTION-04 section of the supplied `r4-motion.md` research report.
- Reuse the existing animation evaluator, widget system, theme tokens, and accessibility snapshot model.
- Do not fetch or copy SVG editor source.

# Existing contracts and invariants
- Apple Silicon macOS 13+, C17, custom-drawn controls, no new dependencies.
- Key-spline coordinates are finite and within the current parser range [0,1]. Update one tuple and `calcMode`; preserve all unrelated tuples and source bytes.
- Spline editing is disabled for paced, unsupported, or otherwise non-editable source combinations. Failed edits leave source and render state unchanged.
- Headless tests cover both themes and increased contrast. Do not automate or capture the shared native window.

# Files allowed to touch
- `work/WO-MOTION-04.md`
- `src/app/viewer_timeline.{h,c}`
- `src/app/viewer_timeline_ui.c`
- `src/app/viewer_timeline_easing.c`
- `src/app/viewer_anim_edit*.{h,c}`
- `src/ui/ui_widgets.{h,c}` and `src/ui/ui_widget_paint*.c` only if required by the implementation
- `tests/app/viewer_timeline_test.c`
- `tests/anim/animation_authoring_test.c`
- `tests/anim/test_animation.py`
- `tests/ui/animation-easing-golden/**` (new files only)
- `tests/test_map.json`
- `CMakeLists.txt` (a `# lane: w14-keyframes` block only)
- `docs/lanes/w14-keyframes.md`
- `artifacts/WO-MOTION-04/**`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`

# Protected paths
- `OWNER-DIRECTIVES.md`, `AGENTS.md`, all other `work/**`, `tests/oracle/**`, `tests/golden/**`, existing `tests/ui/**` baselines and manifests, `tests/ui/mixed-commands.jsonl`, `tests/ui/native-lifecycle.jsonl`, all prior artifacts and benchmark results, `scripts/**`, `dev`, build presets, and all paths outside the allowlist.
- Do not touch external corpus files or drive the native window.

# Required implementation
1. Show the selected key's outgoing interval mode. Provide clear linear/discrete states and a custom-drawn cubic graph with four named editable controls and numeric fields.
2. Provide linear, ease-in, ease-out, and ease-in-out presets; expose custom values in the inspector and accessibility snapshot.
3. Pointer and keyboard handle/field edits remain in [0,1], round-trip exact numeric values, and update only the selected interval's `keySplines` tuple plus `calcMode`.
4. Disable spline editing for paced/unsupported tracks. Keep UI event hooks small and headless-test graph geometry, edits, accessibility labels/values, and deterministic light/dark/high-contrast pixels.
5. Verify linear, discrete, and spline samples against the evaluator and prove middle-interval edits preserve neighboring spline tuples.

# Acceptance criteria
- AC-1: model tests prove valid pointer/keyboard edits, finite [0,1] bounds, exact numeric round trips, and spline tuple isolation.
- AC-2: accessibility exposes mode and four named values; unsupported modes disable spline controls.
- AC-3: headless graph goldens pass for light, dark, and increased contrast; linear/discrete/spline render samples match evaluator output.
- AC-4: `./dev t --all`, `python3 scripts/ledger.py validate ledger.jsonl`, and `git diff --check` pass; report exact results.

# Verification sequence
1. Add headless model/paint regressions and establish pre-fix failures.
2. Run focused timeline/animation tests and compare the three graph goldens.
3. Run `./dev t --all` and ledger/diff validation.
4. Record exact output under `artifacts/WO-MOTION-04/` and update `STATE.md`, `LEDGER.md`, and the lane note.

# Non-goals
Graph workspace, arbitrary SMIL timing editing, spline modes for unsupported source, native-window automation, new dependencies, non-macOS support, and commits.

# Budget
One executor run; no network; no commit.

# Failure and escalation
Never weaken frozen tests or edit existing baselines. If parser constraints cannot safely express an operation, disable it and report the limitation. Keep verification headless.

# Output
Leave changes uncommitted. Report changed paths, exact checks, and maintainer real-window verification needs.
