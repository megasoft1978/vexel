# W7 UIKit

This lane adds the reusable UI kit for the token/surface/icon portion of
WO-UI-01 and the widget primitives from WO-UI-03. The w8-chrome integration
uses these APIs for the application toolbar, searchable layer tree, inspector,
canvas HUD, and status bar; the original scalar painter remains for deterministic
legacy fixtures.

## Delivered

- Light, dark, and increased-contrast token profiles, including interaction
  surfaces, selection/focus colors, semantic colors, spacing/radius scales,
  type scale, and editor metrics.
- 45 project-authored 24×24 icon paths. The bounded path parser accepts
  absolute M/L/C/Q/A/Z commands; raster masks are cached at 1×/2×/3× and tinted
  by state. The legacy icon painter remains available to preserve existing
  editor goldens. The paths follow the supplied UI review; the arc conversion
  follows the SVG elliptical-arc specification. No editor source or third-party
  icon code was copied.
- Custom-painted buttons, segmented controls, switches, checkboxes, tabs,
  tooltips, HUD pills, headers, tree rows, empty states, inline errors, toasts,
  number/color fields, scrub labels, steppers, sliders, dropdowns, and text
  fields.
- Bounded pointer, UTF-8 edit, number, hex color, slider, and dropdown state
  machines with explicit commit/cancel paths.
- Deterministic component and icon galleries with light/dark and 1×/2× PNG
  goldens, plus human-review exports in `docs/ui/`.
- Computed contrast evidence in [uikit-contrast-report.json](../ui/uikit-contrast-report.json).

## Verification

- `./dev build` — passed (Debug, AppleClang 21.0.0).
- `cmake --preset mac-release && cmake --build --preset mac-release` — passed.
- `build/mac-release/bin/ui_kit_test && build/mac-release/bin/ui_widget_fuzz 200000 20261010`
  — passed; fuzz output was 200,000 cases, seed 20261010.
- `ctest --test-dir build/mac-debug --output-on-failure` — 43 passed, 2
  Metal-dependent failures, 1 fixture-guarded skip. The failures were
  `native-viewer-smoke` and `index-replay`; both report that a Metal device or
  command queue is unavailable. The new `ui-widget-kit`,
  `ui-widget-parser-fuzz`, and `ui-kit-gallery` tests passed, as did
  `editor-style-goldens`.
- `./dev asan` — completed with CTest exit 8 because of the same two
  Metal-dependent failures and one skip; every other test passed. The UIKit
  contract, 200,000-case fuzz, and gallery tests passed under ASan+UBSan.
- `./dev ubsan` — completed with CTest exit 8 because of the same two
  Metal-dependent failures and one skip; every other test passed. The UIKit
  contract, 200,000-case fuzz, and gallery tests passed under UBSan.
- Final focused CTest: the UIKit contract, 200,000-case fuzz, and gallery
  passed in Debug, ASan+UBSan, and UBSan; `editor-style-goldens` also passed in
  Debug after the final changes.
- `python3 -m unittest discover -s tests -t .` — 258 passed, 8 failed, 2
  errored, 1 skipped. Seven benchmark process tests receive exit 71 from
  `sandbox-exec` under the managed shell; native lifecycle/Metal tests also
  fail without the required UI/Metal services. No failure was in the UIKit
  tests.
- `./dev test-fast` — failed with the same Python test results; its subsequent
  ledger step did not run. The ledger was validated separately below.
- `python3 scripts/ledger.py validate ledger.jsonl` — passed (406 entries).
- `git diff --check` — passed.

## Limits

- The component gallery uses the deterministic scalar label callback. The
  application chrome injects the CoreText callback and draws SF Pro Text or SF
  Mono through the modern widget painter.
- Text edits are bounded to 127 UTF-8 bytes and semantic labels to 63 bytes.
  Number and hex fields validate and commit values but do not own app document
  commands.
- The icon cache is static and intended for UI-thread drawing. Static paths
  only are accepted; this is not a general SVG path parser.
- Native Metal smoke and full benchmark process-isolation tests need services
  unavailable in the managed test environment.
