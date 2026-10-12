---
id: WO-MOTION-03
revision: 2
status: READY
milestone: M7
owner: project-owner
executor: codex-gpt-6
base_sha: 5c2d0ed69d5f1c45d0faf0211949fff9222f92e4
spec_sha256: 325a504610098ddf1ac7d180254bcdc4f712267ae63d297becfc41bbdcd52f40
parallel_safe: false
network: false
---
# Goal
Add source-preserving, transactional editing commands for supported SMIL keyframes and tracks.

# Closure note
The executor run for the prior revision finished without a verified ledger outcome: its output stayed uncommitted, and the implementation reached `main` later through lane integration. This closure revision is based on current `main`, which already contains the implementation. The candidate may add only retained evidence and records under the allowlist. The full frozen verification commands must pass on the candidate tree, and a fresh verifier must review the acceptance evidence. No test, golden or oracle may change.

# Context pointers
- Read: `AGENTS.md`, `OWNER-DIRECTIVES.md`, `STATE.md`, `docs/CODING_STANDARD.md`, `docs/testing.md`, `src/svg/animation/README.md`, and the WO-MOTION-03 section of the supplied `r4-motion.md` research report.
- Preserve the existing SMIL parser/evaluator and document edit transaction/undo contracts.
- Do not fetch or copy SVG editor source.

# Existing contracts and invariants
- Apple Silicon macOS 13+, C17, no new dependencies; all mutation is on the editor owner thread.
- Source bytes outside the edited animation start tag or inserted/removed track stay byte-identical. Undo restores the exact previous source.
- Reject unsupported track kinds, timing/composition, malformed values, and operations exceeding 256 keys or 64 KiB per edited value. Failed edits leave the document and preview state unchanged.
- Static property edits never create animation implicitly. A track is created only by an explicit command and contains an explicit time-zero seed.

# Files allowed to touch
- `work/WO-MOTION-03.md`
- `src/svg/animation/authoring.{h,c}`
- `src/svg/animation/README.md`
- `src/app/viewer_anim_edit*.{h,c}`
- `src/app/viewer_timeline.{h,c}`
- `src/app/viewer_timeline_ui.c`
- `src/app/viewer_events.c` (localized timeline mutation handoff only)
- `tests/anim/animation_authoring_test.c`
- `tests/anim/test_animation.py`
- `tests/app/viewer_timeline_test.c`
- `tests/test_map.json`
- `CMakeLists.txt` (a `# lane: w14-keyframes` block only)
- `docs/lanes/w14-keyframes.md`
- `artifacts/WO-MOTION-03/**`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`

# Protected paths
- `OWNER-DIRECTIVES.md`, `AGENTS.md`, all other `work/**`, `tests/oracle/**`, `tests/golden/**`, existing `tests/ui/**` goldens, `tests/ui/mixed-commands.jsonl`, `tests/ui/native-lifecycle.jsonl`, all prior artifacts and benchmark results, `scripts/**`, `dev`, build presets, and all paths outside the allowlist.
- Do not touch external corpus files or drive the native window.

# Required implementation
1. Add bounded authoring operations for supported numeric, color, and transform tracks: add/edit/move/delete keys, create/remove tracks, and edit duration/timing using the existing source edit undo path.
2. For an unanimated property, explicitly seed the property value at time zero when creating a track. Keep unsupported tracks preview-only. Fail before mutation if any input, source span, value count, arithmetic, or timing is unsupported or over limit.
3. Preserve all unrelated attributes, namespace declarations, comments, and source bytes. No-op commands create no history entry. Undo/redo restores byte-exact source.
4. Add editor command hooks in new focused `viewer_anim_edit*.c` files; keep shared UI and event integration small.
5. Add headless regression tests for numeric/color/transform edits, track/key lifecycle, timing, undo/redo source identity, preservation, rejected edits, and fresh decode/render state.

# Acceptance criteria
- AC-1: authoring C tests cover supported edit operations and fail safely at all input/count bounds.
- AC-2: regression fixtures prove byte-exact undo/redo and unchanged comments, namespaces, unknown attributes, and duplicate-track order.
- AC-3: fresh decode and deterministic rendering at edited times match the edited editor source.
- AC-4: `./dev t --all`, `python3 scripts/ledger.py validate ledger.jsonl`, and `git diff --check` pass; report exact results.

# Verification sequence
1. Add regression tests and run the focused authoring test.
2. Run focused timeline/animation Python and C tests.
3. Run `./dev t --all` and ledger/diff validation.
4. Record exact output under `artifacts/WO-MOTION-03/` and update `STATE.md`, `LEDGER.md`, and the lane note.

# Verification commands
Run sequentially from the repository root.

```text
cmake --preset mac-debug
cmake --build --preset mac-debug
ctest --test-dir build/mac-debug --output-on-failure
./dev t --all
python3 scripts/ledger.py validate ledger.jsonl
python3 scripts/wo_validate.py WO-MOTION-03 --ready
git diff --check
```

# Non-goals
Path (`d`) editing, motion-path editing, CSS animation, implicit track creation from static-field edits, native-window automation, new dependencies, non-macOS support, and commits.

# Budget
One executor run; no network; no commit.

# Failure and escalation
Never weaken frozen tests. If parser semantics cannot safely express an edit, return an unsupported error without changing source. Keep all verification headless.

# Output
Leave changes uncommitted. Report changed paths, exact checks, and maintainer real-window verification needs.
