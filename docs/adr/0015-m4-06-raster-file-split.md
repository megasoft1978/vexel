# ADR-0015: M4-06 raster file split after merging raster culling

- Status: accepted by planner
- Date: 2026-10-09
- Scope: WO-M4-06 revision 4 only

## Context

Merging `main` (raster culling, `lane/raster-perf`) into the M4-06 branch makes
`src/raster/shape_render.c` 917 lines, above the 800-line hard limit in the work
order and `AGENTS.md`. The frozen allowlist names only `shape_render.c/.h` in
`src/raster/`, so a split cannot be done inside the list.

## Decision

New files under `src/raster/` are permitted for WO-M4-06 solely to split
`shape_render.c` into cohesive modules (for example per-sample paint-server and
clip sampling and compositing). No behavior, limit, threshold, golden, or oracle
change is permitted; output must stay byte-identical, proven by the existing
reference-oracle differential tests.

## Consequences

The path guard treats `src/raster/*.c|h|README.md` as allowed for this work
order. The 800-line limit remains hard.
