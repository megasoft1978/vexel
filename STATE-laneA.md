# Lane A state: SVG renderer

- Current branch: `wo/WO-M4-06`; renderer source `e53c760436c0d16afb9b15f7e8adaf6967dc9a2a`, with README-only follow-up `3396d8109223de9018956104b066d8d0a5255ace`.
- WO-M4-06 revision 4 fixes omitted `stop-color` to SVG's initial black value and documents 64 KiB per-reference / 1 MiB combined subtree-copy bounds.
- Verified: Debug CTest 12/12; `./dev verify` CTest 12/12, Python 199/199 and native smoke/replay; ASan and UBSan CTest 12/12 each; Release configure/build; 16 focused tests and 35 librsvg fixtures; 1,000,000 ASan+UBSan fuzz cases in 93.495 s; corpus run/check 58 files / 116 trials.
- Real corpus remains 2 previewable, 56 unsupported, 0 limits, 0 crashes. The current harness is open/index only; it does not prove renderer parity.
- Current renderer benchmarks use 21 trials/workload; details are in `bench/results/WO-M4-06.json` and `docs/PERF.md`.
- Next: fresh read-only review, merge under `.git/vexel-merge.lock`, then WO-M4-06b to connect renderer output to the viewer and corpus bench. Continue to M4-07 and M4-08.
