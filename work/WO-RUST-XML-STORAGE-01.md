---
id: WO-RUST-XML-STORAGE-01
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: codex-gpt-6-luna
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 7aaeeb7233aa07ee7d8b89e91fc845426eadb2f7d293d14115d3250674b8855d
parallel_safe: false
network: false
---
# Goal and frozen scope
Repair accepted XML stream allocation contract needed by SVG index/document: explicit checked tokenizer allocated token capacity budget, exact OutputExhausted error/offset and observable allocated storage. Existing new(source,max_storage_bytes) must bound actual retained Vec capacity * size_of(XmlToken), not just len; bounded significant fallible growth requests and release/rollback oversized rejected reservation, no retained excess. Preserve full accepted29 XML tests/assertions/API/parse diagnostics/name-read-budget/cancel/visitor behavior. Count-limited private with_limits used by cancelable batch preserves count semantics and overflow-safe representable allocation limits. Expose token_storage_bytes() and open_name_storage_bytes() (or equivalent safe exact-capacity accessors) so index can charge actual shared stack capacity; visitor storage remains exact allocated scratch capacity. Check initial open-name/scratch capacities against fixed max capacities, no subsequent reserve can exceed their caps; do not claim allocator RSS beyond Vec exposed capacity. Memory cap concerns token buffer separately from bounded stack/scratch, matching named C component contract. Handle zero/sub-token/exact/nonmultiple storage boundaries and collecting take/failure/cancel states accurately. No new parse subset, smaller source limits, scan-then-parse, fake allocator/OOM or unsafe. Port focused regressions across small odd token capacities where Vec geometric growth exceeds bound (1/3/5/etc), successful progress at exact cap, exhausted exact diagnostic/offset/no partialtokens, take ownership/storage counts, visitor actual stack/scratch, source/name work/cancel unchanged. Cohesive source<800lines; use private helper module if needed, no protected test edits.

# Files allowed to touch
- `work/WO-RUST-XML-STORAGE-01.md` (coordinator freeze only)
- `rust/crates/vexel-xml/src/**`, `rust/crates/vexel-xml/tests/**`
- `artifacts/WO-RUST-XML-STORAGE-01/**`
- `artifacts/WO-RUST-SVG-INDEX-01/coordinator-overlay-delta.json`, `artifacts/WO-RUST-DOCUMENT-MODEL-01/coordinator-overlay-delta.json` (coordinator only after acceptance)
- `.worktrees/rust5-index/rust/crates/vexel-xml/**`, `.worktrees/rust5-document-model/rust/crates/vexel-xml/**` (coordinator approved exact accepted dependency copy only)
- `.worktrees/rust5-index/work/WO-RUST-XML-STORAGE-01.md`, `.worktrees/rust5-document-model/work/WO-RUST-XML-STORAGE-01.md`, corresponding local ledger.jsonl (coordinator only approved input snapshot)
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`, `docs/product/rust-port-goal-and-plan.md` (coordinator only)

# Protected paths and references
All legacy603sources/oracles/tests, prior29XMLassertions, other accepted Rust code, Cargo/config/lock/toolchain, prior frozen orders/artifacts and candidate code. Read OWNER-DIRECTIVES/AGENTS first. C references src/svg/xml/xml_stream_internal.inc, xml_tokenizer_api.c and their bounded reserve helpers only, accepted Rust stream/tokenizer/storage tests. New docs/evidence not performance/app certification. Std-only safe code, no dependencies/network/download/install/unsafe/C shim/unsupported OS/stage/commit/delete/reset/agents/broadUIE2E. One isolated writer at .worktrees/rust-xml-storage; Standard subscription LunaHigh, no credits/APIbilling. Coordinator owns shared dependency propagation and audits against approved delta; original candidate baselines stay immutable.

# Verification
cargo +stable check -p vexel-xml --locked --offline
cargo +stable test -p vexel-xml --locked --offline
python3 scripts/wo_validate.py WO-RUST-XML-STORAGE-01 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check
Exact installed Rust1.99/mac13/localcache/tmp/target. Original29assertions/source provenance retained; only focused tests.

# Output
Uncommitted full allocation-contract correction, result<=400words/parity-map.json/verification.txt with exact success/failure/capacity/adaptations; independent review then main checks before dependency propagation. No acceptance from writer exit. Stop/checkpoint allowance/rate error, no retryloop.
