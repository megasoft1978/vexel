# Environment and run evidence

## Observed facts

- Hosted failure: run `39297932450`, commit `9e030caf17dca33ceeb7ba7a0e560e0c3e013feb`, runner label `macos-15-arm64`, macOS `15.7.9` (`24G830`), image `20260907.0337.1`. The recorded failure is limited to `vexel-app/tests/ui_chrome_goldens.rs::chrome_frames_match_first_party_goldens`; the work order reports all 32 draw/animate, dark/light, empty/compose-v1, size, and scale variants failing the maximum-channel-delta limit while differing-pixel rates stay below 0.5%. These hosted details are supplied by the active work order; no raw hosted log or hosted toolchain version is present in the repository evidence inspected for this report.
- Local host observations: `sw_vers -productVersion` returned `27.0.1`; `uname -m` returned `arm64`. `rustc -Vv` reports `1.99.0 (b940084d7 2026-09-28)`, host `aarch64-apple-darwin`, LLVM `23.1.1`; `cargo -V` reports `1.99.0 (5f94df478 2026-08-27)`. The local toolchain does not establish the hosted toolchain.
- At the hosted commit, `rust/fixtures/ui/chrome/` contains 32 tracked `.rle` fixtures. The current worktree has the same 32 files, and every file's SHA-256 matches the corresponding blob at `9e030caf17dca33ceeb7ba7a0e560e0c3e013feb`. A sorted manifest of relative paths and per-file SHA-256 values hashes to `c598167082b67c8b6704a89b18c25cf0fae8609370f3e7e3b68abb292273193f` at both the base commit and current worktree. `git diff --name-status <base> -- rust/fixtures/ui/chrome` is empty.
- The chrome test documents the CoreText/OS-variation caveat and fixed limits (0.5% differing pixels; maximum channel delta 24) in `rust/crates/vexel-app/tests/ui_chrome_goldens.rs`. No policy or threshold inference is made here.

## Relevant prior run history

- Local artifact `artifacts/WO-RUST-UI-CHROME-01/verification.json` records `cargo +stable test -p vexel-app --locked --offline` passing all 3 tests in `ui_chrome_goldens` (the frame golden test and two supporting tests).
- `artifacts/WO-RUST-UI-WIRING-01/logs/test-app.log` and `artifacts/WO-RUST-UI-TIMELINE-01/logs/test-app.log` also show the frame golden test passing in local aarch64-apple-darwin builds.
- Later records are not clean same-state comparisons: `artifacts/WO-RUST-UI-POLISH-01/verification.json` and `artifacts/WO-RUST-UI-POLISH-01/result.md` report this golden test failing after visual changes, with candidate frames awaiting acceptance; `artifacts/WO-RUST-UI-TIMELINE-02/verification.json` reports it failing alongside the timeline golden after order-mandated visual changes. These records establish that the test has passed and failed in different local source states, not that either reproduces the hosted mismatch.
- No prior matching hosted run evidence (same commit and hosted runner/OS/image profile) was found in the inspected repository history/artifacts. No network lookup was performed; the work order marks network unavailable.

## Interpretation (hypotheses only)

The local host and hosted runner differ in macOS version. Given the CoreText text in the rendered frames and the documented OS variation caveat, OS-level text rasterization is a plausible hypothesis, but the available evidence does not establish it. The identical fixture hashes rule out fixture drift between the base commit and this worktree. Prior local passes and later failures occurred across different source states, so they do not isolate OS, toolchain, or renderer behavior. The run-wide failures alone do not identify mismatch locations or prove that glyph rasterization caused the channel deltas. Keep the hosted cause undetermined pending diagnostic evidence.
