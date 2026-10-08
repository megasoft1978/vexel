# WO-M1-03 handoff

Status: candidate machine checks pass; implementation commit and fresh
read-only verification are pending. The native replay uses headless Metal blit
readback after LLDB and ASan localized repeated visible-window teardown crashes
to `platform_window_destroy`'s autorelease-pool drain. Windowed Metal remains
covered by the separate M1-02 smoke.

## Evidence

- Focused suite: 19 passed, zero failures/errors/skips.
- Full `./dev verify`: 142 passed, zero failures/errors/skips.
- Placeholder replay: 20/20 matching digests; digest
  `c46e612bbfa618d18a27693c33a952b9fce16bb941e3d9b6f5048b7bfd280c59`.
- Headless Metal replay: 20/20 exact readbacks; SHA-256
  `3d636badd712ea35b36c861871517af1a2a0809d0a0a418efd5419b4f4c1196c`.
- Release binary: 94,752 bytes; recorded placeholder and Metal replay
  latencies: 100.976 ms and 179.164 ms. RSS and SVG throughput are not claimed.
- ASan and UBSan CTest each pass 1/1. Evidence files are `focused-tests.txt`,
  `full-suite.txt`, `asan.txt`, `ubsan.txt`, `native-replay.txt`, `replay.txt`,
  and the runner outputs under `artifacts/runs/WO-M1-03/`.

## Resume steps

1. Complete the exact frozen acceptance commands and commit the implementation.
2. Run the read-only verifier; its clean worktree now builds Debug and Release
   before the frozen tests execute.
3. On verifier pass, merge and continue to WO-M1-04.
