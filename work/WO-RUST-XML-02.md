---
id: WO-RUST-XML-02
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: codex-gpt-6-luna
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 61a568a2fb89b374083b2a52e506b7d73b740a10fa6e4e0f804814a67bb133ab
parallel_safe: true
network: false
---
# Goal and frozen scope
Port full bounded XML tokenizer/API/resumable streaming and lexical DOCTYPE subset; source spans/parent-child/attribute matching/diagnostics/cancel/limits/progress and C malformed behavior. Preserve accepted reader10 assertions/functions. No namespace-resolution/entity-expansion/external-resource behavior beyond current code. No unbounded recursion; fallible tokens/state, stable original-source spans. All named public tokenizer/API/stream functions mapped, no stubs. Own or lifetime-bound source, no self-referential pointers. Exact diagnostic offsets and token/raw spans tested; staged work/cancellation/limit boundaries and failure cleanup. Full SVG decoder/index is outside this order.

# Reference paths
src/svg/xml/xml_tokenizer.c, xml_tokenizer_api.c, xml_stream_internal.inc, xml_doctype_parse.inc, headers and tests/svg/xml; accepted Rust reader/character helpers

# Files allowed to touch
- `work/WO-RUST-XML-02.md`
- `rust/crates/vexel-xml/src/**`, `rust/crates/vexel-xml/tests/**`
- `artifacts/WO-RUST-XML-02/**`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`, `docs/product/rust-port-goal-and-plan.md` (coordinator only)

# Protected paths
All old C/ObjC/include/Python/shell, original tests/fixtures/oracles, other Rust crates, all Cargo manifests/lock/config/toolchain, coordinator docs/records. Preserve accepted same-crate behavior/assertions while extending it. No main edits, deps/downloads/system tools/formatters, unsupported OS code, C runtime shim, staging/commits/reset/clean, recursive agents/network, broad UI/E2E/C tests/corpus/perf campaign. Domain unsafe forbidden; only native-system order permits documented sys.rs boundary under ADR0024.

# Evidence and quality
Read OWNER-DIRECTIVES/AGENTS/ADR0022-0024/Rust goal then this order. Safe owned or lifetime-bound data, checked math and fallible bounded allocation. Map named Csymbols to Rustsymbols/focused tests/API adaptations in parity-map.json. No TODO/unimplemented/stub for namedscope; no whole-subsystem completion claim. Source<800lines, cohesive modules. If API changes outside scope needed stop with concrete dependency conflict, no manifest edit. Preserve original assertions and port meaningful cases, not merely is_err for diagnostic positions. Root reviews source and integrated checks before acceptance.

# Execution and verification
Only .worktrees/rust2-xml-tokenizer; Luna high, one implementation plus one narrowed review repair, thin context. Root serializes integration/shared metadata.
cargo +stable check -p vexel-xml --locked --offline
cargo +stable test -p vexel-xml --locked --offline
python3 scripts/wo_validate.py WO-RUST-XML-02 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check
Exact compiler1.99.0 via installed stable, repo-local CARGO_HOME/TMPDIR/target, MACOSX_DEPLOYMENT_TARGET13. No rustfmt/clippy installed; do not claim/install them. Workers only own src/tests/artifact; never root records.

# Output
Leave uncommitted. verification.txt/parity-map.json with exact behavior/checks/errors/adaptations/remaining scopes. Return<=450words. No additional billing/credit purchase; stop/checkpoint on rate/allowance errors instead of retryloop.
