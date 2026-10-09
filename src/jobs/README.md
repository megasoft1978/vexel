# Immutable worker jobs

`jobs_source_snapshot` copies one caller-owned byte slice (0–256 MiB) before it
publishes the opaque snapshot. The caller retains its original reference until
it releases it. Scheduling adds one atomic reference, and the worker sees only
const bytes. Callers must already own a reference when they pass a snapshot to
the scheduler; retain cannot make an unowned pointer safe.

The scheduler is created and controlled on its owner thread. Submit, cancel,
generation advance, and poll reject calls from other threads (poll returns
false). Worker callbacks only scan immutable bytes and inspect the atomic queue
cancellation token. The configured queue capacity is 1–64 tasks, workers are
1–4, and aggregate scheduled snapshot-reference bytes are capped at a nonzero
configured limit no greater than 256 MiB. Completed work releases its snapshot
reference and byte accounting after the callback; its small result remains
pollable until consumed. Failed submission does not retain the snapshot or
reserve bytes.

Snapshot errors distinguish invalid pointer/arguments, limit/overflow, and
allocation failure. Scheduler errors distinguish wrong-thread use, stale
generation, duplicate ID, full queue, byte backpressure, stopped queue, and
allocation failure. IDs are unique while their queue slot remains active.
Generation advance and cancellation suppress results before owner polling.
Results carry the task ID, generation, success flag, and FNV-1a 32-bit digest
of the exact snapshot bytes; the digest is a deterministic scan check, not a
cryptographic hash.

## Queue context cleanup

The platform queue keeps the old borrowed-context API unchanged. Its owned
submission API takes context ownership only on success. It calls the optional
cleanup once after callback return or after a queued task is discarded by
cancellation, generation advance, or shutdown. Cleanup always runs outside
the queue mutex. Failed submissions leave ownership with the caller. Shutdown
cancels and joins workers; the scheduler and its accounting mutex stay alive
until this join finishes. As required by the platform queue contract, the owner
must quiesce API callers before destroy.

## Tests

`job_snapshot_test` checks invalid and empty slices, over-limit rejection,
source mutation, concurrent retain/release, all owned cleanup terminal paths,
failed owned submission, reference digest, byte backpressure, generation
filtering, fixed-seed four-worker schedules, and duplicate/stale suppression.
`python3 -m unittest tests.test_jobs` retains the machine-readable focused
result. `./dev tsan` builds and runs only this worker/snapshot test with Xcode
ThreadSanitizer. The Release `--benchmark` mode runs exactly 1,000
create/submit/cancel/join cycles and records host/compiler/build metadata.
