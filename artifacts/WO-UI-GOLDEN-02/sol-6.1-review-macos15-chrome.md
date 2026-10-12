# Fresh Sol 6.1 review — macOS 15 editor-chrome profile

**Verdict: PASS** for the pending capture update.

- Both profile images exactly match the hosted run 38208517342 actuals and report hashes: dark `b9501f49c08918ca0ca27e0bd0dd1ff0a0ddf424aee667909f2ef2b7ac861720`, light `ad51fdfb4d35dd3d9b1ebe41f4b0863a3492730a7488408f1fb06fd6664619b9`. Both are 1280 × 800.
- The hosted capture report has 50 exact matches, zero mismatches, and two expected unbaselined images. Those two images are the Mac 15 editor-chrome captures added here.
- All 63 pending paths pass the frozen work-order allowlist. `git diff --check`, ledger validation, and frozen work-order validation pass.
- Local verification passes 82/82 CTests and 303/303 Python tests; focused screenshot tests pass 6/6; the macOS 27 audit passes 52/52 exact.

Hosted Mac 15 CI and nightly sanitizer checks on the final pushed candidate remain required before merge.
