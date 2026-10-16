# WO-CORPUS-04 design

Probed site: `RASTER_PREFLIGHT_MAX_WORK` (200,000,000) in `src/raster/shape_render_preflight.c`, a work budget
(see `probe.md`). Not a memory or allocation check, so it is changed.

## New budget

    budget(w, h) = min(CAP, BASE + max(0, w*h - RAMP_START) * PER_PIXEL)
    BASE        = 200,000,000       (the old budget; unchanged for every canvas up to RAMP_START)
    RAMP_START  = 1,048,576 px      (1024x1024)
    PER_PIXEL   = 24,576 work units per canvas pixel above RAMP_START
    CAP         = 16,000,000,000    (reached at 1,691,479 px)

Why a ramp and not `pixels * K`: the frozen test `shape-render` (tests/svg/shape/test_shape.c,
`clip_work_budget_check`) renders a 1024x1024 document with a 1000-segment clip and requires `LIMIT`. Its
measured work is 1,183,128,292 units (1,128 per pixel). science_t1 needs 10,466,491,930 at 1,600,000 px
(6,541 per pixel). No budget of the form `pixels * K` (or a floor plus that) satisfies both, so the budget
stays exactly 200M up to 1024x1024 and only grows above it. Every canvas of 1,048,576 px or fewer keeps the old
behaviour bit for bit; that test is unchanged and passes.

Named constants `RASTER_PREFLIGHT_BASE_WORK`, `RASTER_PREFLIGHT_RAMP_START_PIXELS`,
`RASTER_PREFLIGHT_WORK_PER_PIXEL`, `RASTER_PREFLIGHT_WORK_CAP`; accessor `raster_preflight_work_budget(width,
height)` in `shape_render_preflight.h`. 64-bit unsigned arithmetic with an overflow guard. The limit is set once
per preflight call into a thread-local that `work_add` reads.

Corpus targets (1600x1000 = 1,600,000 px): budget 13,751,796,224.
- science_t1 needs 10,466,491,930 (76% of budget), embedded_t2 needs 447,653,246 (3%).

## Worst-case arithmetic

- Work: a document that just fits the cap performs 1.6e10 units. Measured rate on this host (Release):
  science_t1 10.47e9 units in 13.2 s = 1.26 s per 1e9, so the cap is about 20 s of render at 128x128. The
  60 s ceiling is met with margin; the preflight stops at the first `work_add` over budget.
- Bytes: no allocation scales with work units. Maximum RSS measured 76,775,424 B (science_t1) and
  381,911,040 B (embedded_t2); the cap does not change per-shape tables or the canvas, so the 2 GiB ceiling
  holds.
- Pixels: only the accept/refuse decision changes. Over-budget documents still fail with
  `SVG_SHAPE_ERROR_LIMIT` at offset 0.

## Measured in the scratch probe tree (budget raised, no other change)

| file | wall | max RSS |
|---|---|---|
| science_t1 | 13.24 s | 76,759,040 B |
| embedded_t2 | 4.96 s | 381,911,040 B |

Final tree (committed change, `measure.json`): science_t1 13.69 s / 76,775,424 B, embedded_t2 5.77 s /
381,911,040 B.

## Tests

`tests/svg/shape/raster_budget_test.c` (CTest `shape-render-scaled-budget`): budget values (200M at 1x1 and
1024x1024, 382,452,224 at 1056x1000, 13,751,796,224 at 1600x1000, cap 16e9), a 1056x1000 gradient zigzag with
1,000 teeth (work 365,760,771: above the old 200M, within the new budget) renders, and 1,100 teeth (over the
budget) fails with `LIMIT` at offset 0.
