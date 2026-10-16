---
id: WO-RUST-MCP-ACCESS-EVIDENCE-01
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: codex-gpt-6-luna
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 2905b9f0d6e0bed25bbc50f45c73cd89a1c46f505bf2f6a3872afeb98aa75bd0
parallel_safe: true
network: false
---
# Goal and frozen scope
Close three evidence gaps in the accepted MCP access port without changing its production behavior: (1) prove `from_environment()` takes a stable one-time snapshot of both environment values, using a child test process so environment mutation cannot race other tests; (2) prove the exact maximum accepted 8191-byte `VEXEL_MCP_ALLOWED_DIRS` value grants a valid file while 8192 bytes are denied; (3) prove the exact maximum config pathname (1023 bytes excluding NUL) remains eligible, while the next longer `HOME` path is rejected. Compare boundary outcomes with `src/mcp/mcp_access.c` and record exact formula/adaptation. Do not revisit the withdrawn config line-count claim.

# Reference and allowed files
Reference only: `src/mcp/mcp_access.c`, `src/mcp/mcp_broker_internal.h` and accepted `rust/crates/vexel-mcp-access/src/lib.rs`.
Allowed: this work order, `rust/crates/vexel-mcp-access/tests/access.rs`, and `artifacts/WO-RUST-MCP-ACCESS-EVIDENCE-01/**`. Production sources, APIs, manifests, ledgers, STATE/LEDGER, legacy files and all unrelated paths are protected.

# Test constraints
Use only repo-local `.cache/access-*` fixtures. The snapshot test must run its mutation/assertions in a dedicated child invocation of its own integration-test executable. Never mutate the parent test process environment. The environment-boundary fixture must remain under `ALLOWED_DIRS_CAP`, including exact 8191 and reject 8192 values. The config boundary fixture must construct a real nested path under 1024-byte POSIX pathname limits without creating any outside-repository files. If the host cannot create or access required paths, record evidence and stop without weakening assertions. No dependencies, unsafe production code, behavior changes, broad UI/E2E, performance or fuzz runs.

# Verification
Run with repo-local outputs and offline mode:
`CARGO_TARGET_DIR=.cache/target-mcp-access-evidence cargo +stable check -p vexel-mcp-access --locked --offline`
`CARGO_TARGET_DIR=.cache/target-mcp-access-evidence cargo +stable test -p vexel-mcp-access --locked --offline`
Also run `python3 scripts/wo_validate.py WO-RUST-MCP-ACCESS-EVIDENCE-01 --ready` and `git diff --check`. Do not edit or validate shared ledger/state in this worker.

# Output
Candidate only. Record exact commands/outcomes, host and toolchain, source hashes, test scope, and any remaining limits in `artifacts/WO-RUST-MCP-ACCESS-EVIDENCE-01/{result.md,verification.txt,parity-map.json}`. Coordinator review and integrated-main verification are still required before acceptance. Existing subscription allowance only; no purchased credits/API billing.
