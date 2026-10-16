---
id: WO-RUST-SVG-VALUES-01
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: codex-gpt-6-luna
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 732417f983e5b8be4c9661df29b35b9e05888e31b02c25258a808b8baf0c3456
parallel_safe: true
network: false
---
# Goal and frozen scope
Port affine inverse and all transform/viewport/viewBox/preserveAspect/root/nested/symbol mapping plus required numeric/length helpers. Use accepted f64 geometry and Span. Adapt C XML token-array attribute lookup into a documented bounded Attribute list or trait with validated source name/value spans; preserve original byte-offset diagnostics and defaults/units/clipping/output math. This is a real standalone input adapter, not C backend or full SVG decode claim. Numeric lexical grammar and comma/space/sign/exponent/units rules match C; no parse::<f64> accepting non-SVG literals. Test matrixorder/inverse singularity/limits/aspect meet-slice/percentunits/nested/symbol/root mappings, nonfinite/overflow/diagnosticpositions. Other shape.c/path/scene behavior not in scope.

# Reference paths
src/svg/shape/transform.c, viewport.c; number/length reader helpers in shape.c and declarations in shape_internal.h; matching tests/svg/shape

# Files allowed to touch
- `work/WO-RUST-SVG-VALUES-01.md`
- `rust/crates/vexel-svg-values/src/**`, `rust/crates/vexel-svg-values/tests/**`
- `artifacts/WO-RUST-SVG-VALUES-01/**`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`, `docs/product/rust-port-goal-and-plan.md` (coordinator only)

# Protected paths
All old C/ObjC/include/Python/shell, original tests/fixtures/oracles, other Rust crates, all Cargo manifests/lock/config/toolchain, coordinator docs/records. Preserve accepted same-crate behavior/assertions while extending it. No main edits, deps/downloads/system tools/formatters, unsupported OS code, C runtime shim, staging/commits/reset/clean, recursive agents/network, broad UI/E2E/C tests/corpus/perf campaign. Domain unsafe forbidden; only native-system order permits documented sys.rs boundary under ADR0024.

# Evidence and quality
Read OWNER-DIRECTIVES/AGENTS/ADR0022-0024/Rust goal then this order. Safe owned or lifetime-bound data, checked math and fallible bounded allocation. Map named Csymbols to Rustsymbols/focused tests/API adaptations in parity-map.json. No TODO/unimplemented/stub for namedscope; no whole-subsystem completion claim. Source<800lines, cohesive modules. If API changes outside scope needed stop with concrete dependency conflict, no manifest edit. Preserve original assertions and port meaningful cases, not merely is_err for diagnostic positions. Root reviews source and integrated checks before acceptance.

# Execution and verification
Only .worktrees/rust2-svg-values; Luna high, one implementation plus one narrowed review repair, thin context. Root serializes integration/shared metadata.
cargo +stable check -p vexel-svg-values --locked --offline
cargo +stable test -p vexel-svg-values --locked --offline
python3 scripts/wo_validate.py WO-RUST-SVG-VALUES-01 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check
Exact compiler1.99.0 via installed stable, repo-local CARGO_HOME/TMPDIR/target, MACOSX_DEPLOYMENT_TARGET13. No rustfmt/clippy installed; do not claim/install them. Workers only own src/tests/artifact; never root records.

# Output
Leave uncommitted. verification.txt/parity-map.json with exact behavior/checks/errors/adaptations/remaining scopes. Return<=450words. No additional billing/credit purchase; stop/checkpoint on rate/allowance errors instead of retryloop.
