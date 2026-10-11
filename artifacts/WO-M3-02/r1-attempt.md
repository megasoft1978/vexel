# WO-M3-02 revision 1 attempt

Revision 1 is retained in Git history and was superseded before any implementation
commit. Its isolated worktree remains available for continued implementation.

The spatial query/cache implementation, 2,000-query differential oracle, 10,000
cache insertions, focused ASan/UBSan CTest, 100,000-record benchmark, and
1,000,000-record build smoke passed. The benchmark reported 0.002 ms p95 query
time on the named Mac.

The frozen verifier could not start because the runner requires a `# Verification
commands` section; revision 1 used `# Verification sequence`. Its focused Python
command also referred to `tests.test_spatial_cache` without allowing that test
file. The full fast suite reported 165 passed, 8 failed, and 2 errors. Debug,
ASan, and UBSan full CTest each failed only `index-replay`, which reported that
an offscreen Metal device or queue was unavailable; `spatial-cache` passed in
all three configurations. The attempt output is in
`artifacts/WO-M3-02/` and `artifacts/runs/WO-M3-02/8e569860-60d5-4bb3-9e34-88902179424d/`.

Revision 2 changes the runner heading and allows the focused Python wrapper.
The existing candidate changes remain uncommitted in `wo/WO-M3-02` for repair
and completion against the corrected work order.
