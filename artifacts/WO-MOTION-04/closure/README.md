# WO-MOTION-04 revision 2 closure evidence (2026-10-12)

Implementation: `src/app/viewer_timeline_easing.c` and the timeline UI, already on `main`. Per-command output is retained in `../r2-verify/` (exact runner records).

| AC | Evidence |
|---|---|
| AC-1 pointer/keyboard edits, [0,1] bounds, tuple isolation | CTest `viewer-timeline`, `viewer-timeline-keyframes`, `animation-authoring` |
| AC-2 accessibility values, disabled modes | CTest `viewer-timeline`, `ui-timeline-goldens` |
| AC-3 light/dark/increased-contrast graph goldens, evaluator samples | CTest `ui-timeline-goldens`, `svg-animation` |
| AC-4 gates | the runner records in `../r2-verify/` |

Revision-1 executor evidence in the parent directory is history. Maintainer real-window review is owner-gated and is not claimed.
