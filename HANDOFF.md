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
