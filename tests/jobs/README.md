# Worker and snapshot tests

The native test executable covers snapshot validation and ownership, concurrent
atomic reference use, borrowed and owned task submission, cleanup on success,
queued cancellation, stale generation, and shutdown, failed submission
ownership, deterministic digest delivery, generation filtering, seeded
multiworker scheduling, and repeated scheduler teardown. Cleanup callbacks use
atomic counters, and the single-worker terminal-path cases hold one callback so
queued discard is deterministic.

Run the executable with `./dev build` followed by
`./build/mac-debug/bin/job_snapshot_test`. The Python adapter
`python3 -m unittest tests.test_jobs` stores its JSON in
`artifacts/WO-M3-03/scheduler-tests.json` and the snapshot ownership transcript
in `artifacts/WO-M3-03/snapshot-tests.txt`. The Release benchmark writes JSON
to the path passed to `--output`.
