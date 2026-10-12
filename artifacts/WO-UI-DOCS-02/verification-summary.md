# WO-UI-DOCS-02 revision 1 candidate

Base: `51f7198f3735c1f7a3dc2195ef4062e620767c56`. Frozen spec SHA-256: `79d6edde75d76ef86551825f23d0d0fb6b078eb43c5a518722a352702287e868`.

## Correction

Fit and Loop now use local glyph geometry drawn with `ui_widget_paint_rect`, which scales logical rectangles to backing pixels. The previous raw-pixel drawing helpers caused glyphs to shift/shrink at 2×. No shared icon source or painter changed. The test `test_fit_and_loop_glyphs_at_backing_scales` renders both controls at 1× and 2×, samples their in-control foreground masks, requires each to contain at least 24 logical pixels, and requires the two masks to differ. The renderer supports a deterministic loop-mode image for visual review.

## Verification

- `./dev build`: pass; see `build.txt`.
- `./dev t -k viewer-timeline`: pass, 2/2 CTests; see `focused-timeline.txt`.
- `python3 tests/ui/verify_timeline_goldens.py`: pass at dark/light, 1×/2×; see `timeline-goldens.txt`.
- `./dev t --all`: pass, 82/82 CTests and 70/70 Python; exit 0 in `all-tests.txt` / `all-tests.exit`.
- Loop-mode headless renders: `headless/timeline-loop-dark-1x.png` and `headless/timeline-loop-dark-2x.png`. Fit appears in the four regenerated timeline goldens.
- Before/after screenshot hashes, changed bounds and pixel counts: `baseline-comparison.json`; sheet: `comparison-sheet.png`; original images: `baseline-before/timeline-golden/`.
- `python3 scripts/wo_validate.py WO-UI-DOCS-02 --ready`, `python3 scripts/ledger.py validate ledger.jsonl`, and `git diff --check`: pass.

Previous review found both the 2× coordinate error and the absence of a glyph completeness assertion. These have been addressed in this candidate. Fresh independent review: PASS, with no unresolved code or baseline findings; see `review-round-01.md`. No commit/push/merge/publication is yet claimed for WO-UI-DOCS-02. WO-UI-DOCS-01 remains committed and pushed at `51f7198f3735c1f7a3dc2195ef4062e620767c56`, not merged.
