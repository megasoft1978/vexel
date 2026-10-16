# WO-RUST-V1-APP-SHELL-01 repair R3

## Changes

- Established the shell's explicit non-editing text-input state after window creation and on focus gain. Native focus routing also marks the route as having no text target; a focused key plus the ensuing IME query/insert callback regression confirms the route does not queue `TextInputStateMissing`.
- CoreText attributed strings now set `kCTForegroundColorFromContextAttributeName` to `kCFBooleanTrue`, so drawing uses the caller's sRGB RGBA context fill color, including alpha. Bounded tests cover attribute availability and RGBA channel/alpha construction.
- Semibold now requests normalized weight 0.3 in CoreText font descriptor traits. If descriptor construction is unavailable, the regular system font is selected. Focused tests cover the requested weight and fallback selection.
- Inspector is omitted unless its fixed rows fit; rail rows adapt to compact content height and omit secondary labels when short; timeline rows appear only when their complete bounds fit. Deterministic geometry tests cover 900x520 and 440x300 in Draw and Animate modes.
- Mode pointer-down activation accepts only primary button 0.

## Verification

`repair-r3-command-results.json` records all five locked/offline commands, exit codes, environment, and output-log SHA-256 values. The four package commands and validator each exited 0. `repair-r3-source-fingerprint.json`, `repair-r3-scope-audit.json`, and `repair-r3-source-inventory.txt` record source hashes, the candidate fingerprint, source-seed comparison, and line counts. All inventoried source files are below 800 lines. The frozen `source-seed.json` and prior scope/hash manifest were not changed.

## Runtime limits and scope note

No native UI was launched. Package checks and tests are code-level verification only; they do not establish live CoreText rendering, window lifecycle behavior, clipping/presentation visuals, or visual acceptance. The full native UI and product campaign remains unverified.

The frozen source-seed comparison shows a pre-existing candidate change to `rust/crates/vexel-platform-apple/README.md`, which is outside this repair turn's narrower allowlist. It was not touched. Cargo manifests and lockfiles match their source-seed hashes.
