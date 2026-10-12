# WO-MOTION-04 revision 3 closure evidence (2026-10-12)

Implementation: `src/app/viewer_timeline_easing.c` and the timeline UI. Per-command output is retained in `../r3-verify/` (exact runner records).

The acceptance tests live in `tests/app/viewer_timeline_keyframe_test.c` (CTest `viewer-timeline-keyframes`, 5 test functions), not in `viewer_timeline_test.c`.

| AC | Test in `tests/app/viewer_timeline_keyframe_test.c` |
|---|---|
| AC-1 pointer and keyboard edits, [0,1] bounds, exact numeric round trips, tuple isolation | `test_easing_pointer_and_keyboard_edits`: pointer down/move/up on the graph and arrow-key handle edits. `test_easing_numeric_field_edits`: focus a field, type a value, Return; 0.37, 0 and 1 are committed exactly in the selected interval's tuple with other tuples byte-identical and a fresh decode equal; 1.5, -0.1, empty and `abc` are rejected with the source unchanged. `test_easing_rejected_field_does_not_leak`: rejected or uncommitted drafts never leak into another field's commit. CTest `animation-authoring` covers spline tuple isolation |
| AC-2 accessibility mode and four named values; disabled for unsupported modes | `test_easing_accessibility_and_raster_goldens`: `Interpolation mode` = `Spline`, slider labels `Spline x1`..`y2`, `Spline y2` = `0.52000000000000002`; a paced track disables easing and the mode and `Spline x1` controls. `test_easing_numeric_field_edits` asserts the slider value after a commit |
| AC-3 light/dark/increased-contrast graph renders; evaluator samples | `test_easing_accessibility_and_raster_goldens` renders the graph in three themes and compares RGBA FNV-64 hashes (pairwise distinct). The PNGs in `tests/ui/animation-easing-golden/` are written from the same renders when `VEXEL_EASING_GOLDEN_DIR` is set; the test does not read them back. CTest `ui-timeline-goldens` and `svg-animation` cover the timeline goldens and evaluator |
| AC-4 gates | the runner records in `../r3-verify/` |

Defects found and fixed while closing this revision, in `viewer_timeline_easing.c`: (1) Backspace, Delete, Left and Right were not mapped to the number field, so a typed value could only be appended, never replaced; (2) a rejected or uncommitted draft left a stale value in its field and a later commit of another field wrote that value into the source. Both are covered by the tests above.

Revision-1 executor evidence in the parent directory is history. Maintainer real-window review is owner-gated and is not claimed.
