R2 fixes implemented: `evaluate_frame` now requires an explicit caller budget; the separate original 64 MiB clip cap remains. Every owned variable scan polls control, including empty shape suffixes, while preserving legacy local work accounting.

All 31 tests pass, including policy overlap above 64 MiB, atomic small-budget rejection, and mid-loop cancellation. Offline check/test, rustfmt, and R2 validation pass. All 31 symbols and 52 cases remain recorded; frozen inputs are unchanged and files remain under 800 lines.

[Result and evidence](/Users/megasoft78/Desktop/Freelance/vexel/.cache/WO-RUST-SVG-ANIMATION-FRAME-01/worker/artifacts/WO-RUST-SVG-ANIMATION-FRAME-01/result.md). Candidate awaits independent review.