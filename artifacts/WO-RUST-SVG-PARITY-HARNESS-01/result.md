# WO-RUST-SVG-PARITY-HARNESS-01 result

Integrated a standalone, offline Rust harness at `rust/tools/vexel-svg-parity/`. Its nested workspace has its own lockfile and uses local path crates only. No root Cargo workspace, manifest, or lockfile changed. The harness invokes the accepted `DecodeSource`, cached paint resolver, and `DefinitionPlanner` APIs directly.

## Frozen parity predicates

- Cached gradient scan: 4,096 XML comments, 512 rectangles, exactly two gradient stops; all 512 computed fill spans pass through `resolve_cached_fill`. The production `PaintCache` records 8,211 token visits (allowed 4,096–37,376), publishes one two-stop template, and leaves source bytes unchanged.
- Single clip boundary: an exact 65,536-byte child span succeeds. At 65,537 bytes, `DefinitionPlanner` returns `Limit(References)` at source offset 33. Source bytes are checked after both results.
- Aggregate clip boundary: eight distinct owner declarations each plan a 65,536-byte wrapped job (51-byte wrapper + 65,485-byte child), totaling exactly 512 KiB and succeeding. Nine distinct owners cause the ninth production planner call to return `Limit(References)` at offset 33. Source bytes are checked after success and failure.

## Review and verification

The initial Sol Medium review found three defects; all were repaired. Its follow-up confirmed all three repairs and no remaining code blocker. It then requested refreshed hashes/line counts and a post-repair verification log. The coordinator reran the exact offline package check/test, rustfmt, order validator, 41-file seed audit, root ledger validator, and `git diff --check` after that review. All passed. Evidence is in `verification.json`, `repaired-candidate-verification.log`, `sol-review-r2.log`, `main-seed-audit.log`, and `coordinator-scope-audit.json`.

Rust source is 343 lines (README 27; manifest 22). All 41 source/oracle seed hashes still match. The integrated package hashes are recorded in the scope audit.

## Scope boundary

This closes only a cross-crate parity prerequisite for the incomplete SVG decoder. It does not complete full decoder three-input allocation/output parity, one-versus-sixteen group clip resolution, renderer-produced 256-byte RGBA parity, tile-cache rollover/LRU, app/UI, MCP runtime, performance, release, or demo. The full Vexel goal remains active.
