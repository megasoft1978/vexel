# Wave 2 bundle lane

## Built

- macOS app and compressed DMG packaging with a generated icon, SVG document
  registration, hardened-runtime code-signature verification, and optional
  Developer ID/notarization parameters.
- Seeded headless rendering coverage for generated SVGs and corrupted inputs.
- Bounded soak command with ASan+UBSan XML/shape campaigns, TSan worker
  coverage, and a 10,000-render RSS check.
- Toolchain doctor and macOS nightly/release workflow drafts; nightly CI runs
  the seeded e2e matrix.

## Limits

- The default ad-hoc signature is for local testing and cannot be distributed
  as a Developer ID signed release.
- The tag workflow attaches the DMG to its Actions run. It does not publish a
  GitHub Release asset.
- The e2e input generator covers the currently supported renderer subset; it
  does not claim SVG specification coverage.

## Verification record

- `./dev build`: passed; Debug CMake build includes the `render_soak_driver`
  and its bounded CTest smoke.
- `python3 -m unittest tests.e2e.test_doctor tests.e2e.test_e2e_svg tests.e2e.test_packaging tests.e2e.test_soak tests.e2e.test_workflows`:
  25 passed.
- `./dev e2e`: passed with seed `20261010`; 256 valid SVGs, 162 mutated
  inputs, 674 render invocations, 256 deterministic repeat checks, 674 valid
  PPMs, no valid-input stderr, crashes, or timeouts. Slowest render was
  0.012843 seconds. Report: `.cache/e2e/latest.json`.
- `./dev soak --minutes 1 --seed 20261010`: passed. XML fuzz ran 30 seconds
  and completed 2,042,122 cases; shape fuzz ran 30 seconds and completed
  986,112 cases; worker TSan passed. The 10,000-render Release loop measured
  475,136 bytes of RSS growth and 2,490,368 bytes peak RSS. Report:
  `.cache/soak/latest.json`.
- `./dev doctor`: passed for macOS 27 arm64, CMake 3.31.6, Ninja 1.13,
  Apple Clang 21, rsvg-convert 2.63.2, Python 3.12.8, codesign, and hdiutil.
- `./dev bundle`: passed. `Info.plist` and the generated 1024×1024 ICNS
  validate; the packaged CLI reports `svgworkbench 0.0.1`, renders a valid
  PPM, and passes strict hardened-runtime signature verification.
- `./dev dmg`: attempted; local `hdiutil` cannot start `hdiejectd` in the
  managed sandbox and returns `Device not configured`. No DMG was left behind.
  Unit tests cover the staging layout, `/Applications` symlink, UDZO options,
  notarization command sequence, and partial-output cleanup.
- Debug, ASan+UBSan, and UBSan CTest each build successfully and pass 19/21.
  The two failures are environment constraints: `native-viewer-smoke` cannot
  receive LaunchServices events, and `index-replay` has no offscreen Metal
  device or queue. The new render-driver CTest passes in all three profiles.
- `python3 -m unittest discover -s tests -t .`: 238 tests ran; 228 passed,
  with 8 failures and 2 errors from the managed host's process policy, missing
  Metal device, and unavailable LaunchServices event service. The focused
  25-test lane suite passes.
- `python3 scripts/check_ci.py` passes for both new workflows;
  `python3 scripts/ledger.py validate ledger.jsonl` validates all 406 existing
  entries; `git diff --check` passes.

## Remaining verification limits

The managed sandbox blocks real disk-image creation, so `build/Vexel.dmg` was
not produced locally. The workflow uploads it as an Actions artifact on tag
runs; publishing a GitHub Release asset and running Developer ID notarization
remain unverified until the owner supplies the signing setup. The complete
Python and CTest suites retain only the host-specific failures listed above.
