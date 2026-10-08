# ADR 0005: One-time oracle baseline seeding

- Status: accepted
- Date: 2026-10-08
- Decision owner: project owner

## Context

Before M0-02, the repository had no independent raster goldens. The first
baseline must be committed while preserving the normal freeze on oracle and
lock paths for every later work order.

## Decision

Only the first accepted implementation of `WO-M0-02` with frozen metadata
`baseline: true` may seed the baseline, and only when the WO base contains no
oracle files or golden lock. The runner checks the exact work-order ID, frozen
baseline flag, empty base, and Git's added status for each exception path.
Edits, deletions, additional files after the first seed, later work orders,
root-scope changes, corpus locks, and unrelated frozen paths remain denied.

The baseline uses project-authored MPL-2.0 fixtures and the locally installed
librsvg CLI as an independent test renderer. It is test data only, not a
conformance claim or a Vexel performance result.

## Consequences

The exception is implemented in the scope driver and covered by positive and
negative runner tests. Offline fast tests validate all source and PNG hashes
without requiring the optional renderer. Any future baseline change requires a
new explicit work order and owner decision.
