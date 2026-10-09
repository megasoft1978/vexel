# Handoff

WO-M4-04 revision 1 is complete and fast-forward merged to `main` at
`dac63d6e8f8cd08787f3ee7e71099d0655ee258e`. Final candidate
`f48c19f017ddf1cbc756e4ff79a9c26d674335fd` passed fresh read-only review
`verify-3c3ca13c-0382-4d62-8823-d11408b97fa4` with no findings in 89.518 s;
all 18 frozen commands pass. Debug/ASan/UBSan CTest 11/11, Python 192/192,
native smoke/replay, 9 focused tests, 26 strict librsvg fixtures, and 1M
ASan+UBSan fuzz pass. The final raster benchmark records 2.138/4.301/5.331 ms
simple/curved/multi-shape fill and 2.501/10.735/78.994 ms line/curved/dashed
strokes. The corpus has 2/58 previews and 56 unsupported; the 100 MiB preview
gate fails, while max-step (0.199 ms) and RSS gates pass. This is open/index and
selected-shape evidence, not full SVG parity. Detailed results:
`artifacts/WO-M4-04/`, `reports/M4/head-to-head.md`, and `docs/PERF.md`.

Next: prepare and freeze WO-M4-05 for the CSS/style subset, following the M4
feature priority inventory at `reports/M4/corpus-feature-frequency.md`.

## WO-M4-05 candidate in progress

The first `wo-run.sh new` executor stalled before writing its report; I interrupted only that WO process after it had been idle for over 15 minutes. The candidate worktree is preserved. Its initial sandbox runs reported Metal/process-policy failures, but direct native reruns pass. Current evidence under `artifacts/WO-M4-05/` includes Debug CTest 12/12, 199/199 fast tests, 199 Python tests plus native smoke/replay from `./dev verify`, ASan and UBSan CTest 12/12, 16 focused Python reference tests, 27 librsvg fixtures, 1,000,000 sanitized fuzz cases, Release build, and 58-file/116-trial corpus checks.

The current corpus still has 2 previewable T1 files and 56 unsupported outcomes, with no limit/crash rows. CSS-specific Release workloads have 21 fresh-process trials, median/p95 of 26.529/33.697 ms for stylesheet classes and 26.223/27.232 ms for inline style, and peak RSS of 2,179,072/2,146,304 bytes. Both outputs share SHA-256 `827e21fe25c8fd57db0185583ded7a6b9d988f54846b914cf12c536ba393fd60`. The pre-WO base has no comparable CSS workload.

Next: commit the reviewed candidate paths, refresh post-commit benchmark identity, run `python3 scripts/wo_validate.py WO-M4-05 --ready`, ledger/diff/scope checks, then invoke `./scripts/wo-verify.sh WO-M4-05 <candidate-sha>`. Repair any genuine verifier findings and merge only after the fresh review passes.
