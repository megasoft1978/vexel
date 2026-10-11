# WO-M1-03 handoff

Status: accepted and merged at `734c23ba42d405354d2408dcf8de69ef19a29b39`.
The fresh read-only verifier passed with no findings. The native replay uses headless Metal blit
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
- Fresh verifier: `2c6ae867-72ae-4f61-9b04-580079e1d580`, pass with no findings.
- Release binary: 94,752 bytes; recorded placeholder and Metal replay
  latencies: 100.976 ms and 179.164 ms. RSS and SVG throughput are not claimed.
- ASan and UBSan CTest each pass 1/1. Evidence files are `focused-tests.txt`,
  `full-suite.txt`, `asan.txt`, `ubsan.txt`, `native-replay.txt`, `replay.txt`,
  and the runner outputs under `artifacts/runs/WO-M1-03/`.

## Resume steps

1. Continue with WO-M1-04 from the current `main` commit.
