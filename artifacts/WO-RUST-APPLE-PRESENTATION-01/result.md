# WO-RUST-APPLE-PRESENTATION-01 result

Implemented safe RGBA frame presentation inside `vexel-platform-apple`. Presentation prepares a configured `CALayer` off-hierarchy from the current view-layer bounds, requested contents scale, resize gravity, and a prepared `CGImage`, then publishes through one guarded hierarchy operation: `addSublayer` for the first frame or `replaceSublayer_with` for replacement. `WindowRecord` retains the active layer only after that operation returns successfully; `Window` remains a weak handle. Existing visible layers are not mutated, and direct contents are not followed by `setNeedsDisplay`.

The pure `failed_publish_keeps_prior_presenter_state` test injects a failed publish callback and verifies only the pure Rust ownership transaction. It does not prove AppKit/Core Animation runtime behavior or native exception atomicity. The public API documentation states that same limitation.

## Verification

The isolated candidate passed focused checks and an independent Sol Medium review. After integration, the main tree was checked again:

- `cargo +stable check -p vexel-platform-apple --locked --offline` — exit 0.
- `cargo +stable test -p vexel-platform-apple --locked --offline` — exit 0: 36 unit, 5 native-boundary, 1 native-window, and 6 doc tests passed (48 total).
- `python3 scripts/wo_validate.py WO-RUST-APPLE-PRESENTATION-01 --ready` — exit 0.
- `python3 scripts/ledger.py validate ledger.jsonl` — exit 0; full ledger validation output is in `ledger-validation.txt`.

Exact main-tree logs are `main-check.log`, `main-test.log`, and `main-validator.log`. The independent final review is recorded in `sol-final-corrected.log`; source-seed and scope evidence are in `source-seed.json` and `scope-audit.md`. Affected file hashes are in `verification.json`.

Still unverified in a live AppKit window: displayed channel order, top-left orientation, sRGB/alpha compositing, image retention after return, replacement/release behavior, resize/backing-scale replacement, teardown, and native exception behavior including hierarchy-operation atomicity. Package tests do not establish these properties. This is a frame-presentation seam; it does not complete the V1 app shell or editor workflow.
