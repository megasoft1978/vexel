# WO-MOTION-04 revision 2 closure evidence (2026-10-12)

Implementation: `src/app/viewer_timeline_easing.c` (676 lines) and the timeline UI, already on `main`.
Per-command output is retained in `../r2-verify/` (exact runner records).

The acceptance tests live in `tests/app/viewer_timeline_keyframe_test.c` (CTest `viewer-timeline-keyframes`), not in `viewer_timeline_test.c`; the revision-1 executor added them there.

| AC | Test in `tests/app/viewer_timeline_keyframe_test.c` |
|---|---|
| AC-1 pointer and keyboard edits, [0,1] bounds, exact numeric round trips, tuple isolation | `test_easing_pointer_and_keyboard_edits` (line 220): pointer down/move/up on the cubic graph, keyboard and field edits, edited source checked for the selected `keySplines` tuple; neighbouring tuples asserted unchanged. `animation-authoring` (spline tuple isolation, out-of-range rejection) |
| AC-2 accessibility mode and four named values; disabled for unsupported modes | `test_easing_accessibility_and_raster_goldens` (line 153): `Interpolation mode` = `Spline`, slider labels `Spline x1`..`y2`, `Spline y2` value `0.52000000000000002`; a paced track makes `viewer_timeline_easing_enabled` false and the mode and `Spline x1` controls disabled |
| AC-3 light/dark/increased-contrast goldens; evaluator samples | same test: three PNG hashes compared against `tests/ui/animation-easing-golden/` (light, dark, high-contrast), pairwise distinct; CTest `ui-timeline-goldens`, `svg-animation` |
| AC-4 gates | the runner records in `../r2-verify/` |

Revision-1 executor evidence in the parent directory is history (its `dev-t-all.txt` recorded a sandbox failure and is superseded). Maintainer real-window review is owner-gated and is not claimed.
