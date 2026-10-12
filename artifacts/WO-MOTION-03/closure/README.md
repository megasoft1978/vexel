# WO-MOTION-03 revision 2 closure evidence (2026-10-12)

Implementation: `src/svg/animation/authoring.{h,c}`, `src/app/viewer_anim_edit.{h,c}`, already on `main` at `5c2d0ed`.
Per-command output is retained in `../r2-verify/` (exact runner records, all exit 0).

| AC | Evidence |
|---|---|
| AC-1 authoring bounds | CTest `animation-authoring` (#76) |
| AC-2 byte-exact undo/redo and preservation | CTest `animation-authoring` (#76), `viewer-timeline-keyframes` (#77) |
| AC-3 fresh decode and render match edited source | CTest `animation-authoring` (#76), `svg-animation` (#12), `svg-animation-python-reference` (#13), `svg-animation-rsvg-reference` (#14) |
| AC-4 gates | `ctest.txt`, `dev-t-all.txt`, `ledger.txt`, `diff-check.txt` |

Revision-1 executor evidence in the parent directory is history. Maintainer real-window review of the timeline is owner-gated and is not claimed.
