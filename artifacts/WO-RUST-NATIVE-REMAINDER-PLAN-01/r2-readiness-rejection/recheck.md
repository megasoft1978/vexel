**BLOCKER — not ready for acceptance.**

The corrected canonical paths and source coverage are present: the inventory names all 53 `src/platform` files and records source definitions, Objective-C methods, and block callbacks. The revised reports also describe the current Apple crate boundary and its `NativeApp`, `Window`, `WindowId`, and private view seams.

Two readiness gaps remain:

- **No symbol-by-symbol API proof.** The reports list selected Cargo features and name binding-audit artifacts, but defer mapping planned API use to cached declarations and macOS minimum-availability evidence until each implementation order. This leaves calls such as CoreText shaping, ImageIO decoding, Metal presentation, AppKit panels, and accessibility without the required declaration, selected-feature, and macOS 13 evidence in this plan. Future workers cannot resolve a prerequisite the plan must gate explicitly.
- **Incomplete original test ownership.** The inventory gives individual case/assertion records for selected native suites. Other cited app, editor, UI, and Metal suites are assigned only by suite path or broad test category; their cases/assertions, source line/hash anchors, and sole future owners are not mapped. Numeric limits likewise appear in prose without consistent links to the exact original source/test anchors.

The canonical baseline correction and preserved initial rejection are good, but they do not close these gaps. This is a planning-only finding; no source, build, test, or runtime claim was made.