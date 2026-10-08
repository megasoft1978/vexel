# ADR-0006: M1-04 queue owner quiescence

- Status: accepted by planner directive, 2026-10-08
- Scope: `WO-M1-04` revision 2

## Context

The first M1-04 candidate passed its frozen machine checks, but review found
that a raw queue pointer cannot protect a caller paused before its first
instruction from a concurrent destroy. Extending the API to use externally
retained handles would add a separate lifetime mechanism to this small queue.

## Decision

The queue owner must quiesce every API caller before destroy: no concurrent
entry may start and no call may remain in flight. Destroy during concurrent
entry is undefined behavior, matching standard C resource-destruction
contracts. In debug/assert-enabled builds, an active-call counter must make
destroy with an admitted API call a deterministic abort. A test hook must pause
a caller after admission and verify the abort in a death-test subprocess.

The worker cancellation and join path remains race-free and bounded. It still
requests cancellation, wakes workers, joins each worker, and releases storage
only after all worker callbacks have returned. Owner quiescence does not relax
the worker join requirement.

## Consequences

Callers have a clear lifetime rule and misuse is caught deterministically in
debug builds. Release callers remain responsible for quiescence. No external
reference-counting API is introduced. The M1-04 acceptance criteria are
narrowed only for concurrent API entry during destroy; queued/running
cancellation, generation suppression, allocation-failure cleanup, 1,000
lifecycle cycles, and bounded joins remain required.
