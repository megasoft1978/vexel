# Fresh independent review — WO-UI-DOCS-02 r1

**Verdict: PASS.** Reviewed the updated candidate read-only on 2026-10-12.

- The prior Fit/Loop clipping and 2× coordinate findings are resolved. Both icons use scale-aware `ui_widget_paint_rect` geometry; shared icon painters remain unchanged.
- The new regression renders both controls at 1×/2×, requires at least 24 foreground logical cells per mask, and requires distinct masks. It would reject missing/off-control glyphs.
- Independent rerender of four timeline goldens and both loop-mode evidence images matches retained output. The changed bounds scale from `(359,590,369,600)` at 1× to `(718,1180,738,1200)` at 2×.
- Preserved before images and hashes match the comparison record. Only timeline baseline family is affected.
- `./dev build`, focused timeline checks, timeline goldens, and the latest full suite pass; full suite: 82/82 CTests and 70/70 Python, exit 0.
- Frozen spec hash matches `79d6edde75d76ef86551825f23d0d0fb6b078eb43c5a518722a352702287e868`. All 29 changed/new paths are allowed; no protected code, baseline, test assertion, or threshold changed. `viewer_timeline_ui.c` remains 663 lines.
- Ledger validation and `git diff --check` pass.

Evidence: `comparison-sheet.png`, `headless/timeline-loop-dark-{1,2}x.png`, `baseline-comparison.json`, `focused-timeline.txt`, `timeline-goldens.txt`, and `all-tests.txt`.
