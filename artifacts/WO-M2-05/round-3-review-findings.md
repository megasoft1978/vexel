# Fresh verifier round 3 findings and fixes

The exact-commit read-only review of `5e1041e33522d1f83d73c048c78d2b9e0657a071` found:

- The 65,536 path-command cap was local to each path. A document-level builder counter now enforces the total cap, with a C test containing two paths whose individual counts fit but combined count exceeds the limit.
- `viewBox` mapping stretched content when viewport and viewBox aspect ratios differed. Raster mapping now uses the SVG default `xMidYMid meet` scale and centered letterboxing, with a mismatched-aspect-ratio pixel test.
- The differential test allowed large edge channel errors without an explicit bound. It now requires exact interior pixels, at most 120 differing pixels per fixture within one pixel of either renderer's coverage boundary, edge alpha delta no greater than 160, and covered RGB delta no greater than 1. All seven fixtures pass these limits.
- `docs/PERF.md` and the benchmark JSON disagreed on binary/library sizes and the latest timing. The table and narrative now match the raw Release measurements.

All local acceptance commands pass on the corrected candidate. Fresh verifier round 4 is pending.
