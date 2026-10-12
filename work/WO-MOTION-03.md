---
id: WO-MOTION-03
revision: 4
status: READY
milestone: M7
owner: project-owner
executor: codex-gpt-6
base_sha: 4ea7404dc3441f0e3e3546faf2935d1e628be6c1
spec_sha256: b749985fb7acbc37c253588d96a5674d43cda1320d5148fc66585bf288d879e0
parallel_safe: false
network: false
---
# Goal
Add source-preserving, transactional editing commands for supported SMIL keyframes and tracks.

# Repair note
The closure verifier for revision 2 rejected the implementation already on `main`, and revision 3 could not be verified because its allowlist used brace patterns that the runner path guard does not expand. Revision 4 is the repair revision, based on the commit before the repair. Besides evidence and records, the candidate may edit `src/svg/animation/authoring.h`, `authoring.c`, the new `authoring_create.c` and `authoring_internal.h` beside them, `src/svg/animation/README.md`, `tests/anim/animation_authoring_test.c`, the new `tests/anim/animation_authoring_edge_test.c`, and the `# lane: w14-keyframes` block of `CMakeLists.txt` to fix exactly these findings. No other behavior, golden or oracle may change, and no existing assertion may be weakened.

1. Spline tracks must support key edits. In `authoring.c`, `set_key_sequence`, `move_key` and `delete_key` stop rejecting spline tracks. `set_key_sequence` takes the keySplines bytes to write. A spline track keeps exactly key_count-1 keySplines tuples: value edit and move pass the original bytes through unchanged; add duplicates the tuple of the interval it splits; delete drops one tuple (key 0 drops tuple 0, the last key drops tuple N-2, any other key k drops tuple k). Deleting down to one key removes keySplines and sets `calcMode="linear"`, because the parser rejects a one-key spline track. A keySplines value over 64 KiB returns the existing limit error before the source is modified. Document this in the animation README.
2. Bound and edge tests. Add tests for: NULL output patch; track index at track_count; corrupt source spans (offset at source length, length past the end, offset not on `<`); open tags over 64 KiB or over 128 attributes; key index at key_count; empty, `;`-containing or over-64 KiB values; add-key times at begin, at begin+dur, NaN, Inf, and equal to an existing key; move-key on the first, last or only key, outside the neighbouring keys, and to the same time; deleting the only key; timing with NaN, Inf, non-positive duration and begin+dur over 3600; spline NaN, negative and out-of-range interval; unsupported create-track kinds, attributes and seed values. Each rejection also asserts that the patch has no replacement.
3. Byte-exact undo and redo. The shared undo helper compares the undone source with the original, and the redone source with the edited source, byte for byte. Use it for add, edit, move, delete, timing, color, transform, spline and calc-mode edits. Track create and remove assert byte-exact undo and redo too.
4. Tests for spline key edits: after each of add, edit, move and delete on a spline track, the source re-parses with `calcMode` spline and key_count-1 tuples, and untouched tuples are byte-identical.

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
- `tests/anim/animation_authoring_edge_test.c`
- `src/svg/animation/authoring_internal.h`
- `src/svg/animation/authoring_create.c`
- `work/WO-MOTION-03.md`
- `src/svg/animation/authoring.h, src/svg/animation/authoring.c`
- `src/svg/animation/README.md`
- `src/app/viewer_anim_edit*.h, src/app/viewer_anim_edit*.c`
- `src/app/viewer_timeline.h, src/app/viewer_timeline.c`
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
