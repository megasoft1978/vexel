# WO-CORPUS-04 probe (executor, base 19234a0)

Method: scratch worktree `.cache/WO-CORPUS-04/probe` (detached HEAD, removed afterwards), every
`SVG_SHAPE_ERROR_LIMIT` site in `src/raster/**` wrapped to print `__FILE__:__LINE__`, `__func__` and
`backtrace()` with `dladdr` names; Release build; `--render-frame <file> --size 128x128`.
Instrumentation was never committed. No lldb.

## Result at the base tree (both files identical)

Five LIMIT sites print. Four are only the default-initialised diagnostic at function entry
(`raster_fill_scanline_cache_create` shape_scanline_fill.c:188, `raster_shape_bounds_build` shape_bounds.c:146,
`raster_shape_bounds_mark_boundaries` shape_bounds.c:338, `raster_scanline_index_create` shape_scanline.c:199),
overwritten with NONE on success. The refusal is the fifth:

    shape_render.c:321 raster_shape_render_flat   (raster_shape_render_work_preflight returned false)

Inside the preflight a `work_add` hit the budget, the only failure that fired:

| file | function (backtrace) | budget | value reached |
|---|---|---|---|
| science_t1 | work_add <- raster_shape_render_work_preflight | RASTER_PREFLIGHT_MAX_WORK 200,000,000 (shape_render_preflight.c) | total 199,999,625, next amount 477 |
| embedded_t2 | work_add <- count_fast_fill_intervals_for_runs <- raster_shape_render_work_preflight | same | total 200,000,000, next amount 1 |

The site is a pure work budget (counts units of sampling work, allocates nothing), not a memory ceiling or
allocation-size check. `RASTER_BOUNDARY_WORK_LIMIT` (shape_bounds.c) does not fire for either file.

## Work actually needed (budget raised to 4e11 in the scratch tree, instrumented total at preflight exit)

| file | total work units | wall (128x128 Release) | max RSS |
|---|---|---|---|
| embedded_t2 | 447,653,246 (pixels 1,600,000; samples 86,868,792) | 4.96 s | 381,911,040 |
| science_t1 | 10,466,491,930 (pixels 1,600,000; samples 15,510,949) | 13.24 s | 76,759,040 |

science_t1 is dominated by many full-canvas shapes with about 665 active edges per sample (665 units per sample).
Both render to completion with exit 0 once the budget is lifted.

## Remaining blockers

None. After the change both target files are `ok` in the sweep, so no next site exists to record
(final-tree re-probe not needed: `generated-science_t1` and `generated-embedded_t2` exit 0).
