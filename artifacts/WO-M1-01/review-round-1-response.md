# Fresh review round 1 response

All three findings were correctness/evidence blockers and were addressed.

1. The native driver now translates each validated JSONL open, resize, key,
   pointer, and quit command into argv consumed by the C application. Key and
   pointer injection execute one action per scenario command. The focused
   `test_100_cycles` now launches the native Release application for 100 actual
   create/resize/input/close/destroy cycles.
2. Resize event matching now returns failure when a resize notification does
   not match the requested pixel size at the reported backing scale. Key and
   pointer payloads are checked against scenario values.
3. `artifacts/WO-M1-01/native-smoke.txt` and `bench/results/WO-M1-01.json` now
   distinguish the AC smoke observation from a separate five-run sample. The
   sample records every elapsed time, median, and nearest-rank p95; Release
   executable size is measured after the final build.

The corrected host verification transcript and focused/native evidence are
retained in `full-checks.txt`, `focused-tests.txt`, `native-smoke.txt`, and
`native-smoke-benchmark.txt`.
