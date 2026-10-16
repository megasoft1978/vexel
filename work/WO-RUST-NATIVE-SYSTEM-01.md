---
id: WO-RUST-NATIVE-SYSTEM-01
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: codex-gpt-6-luna
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 165760987e1c09776660f665ed1bdc2d7c44daf9ca73cdd4d36f0f2bd9f05893
parallel_safe: true
network: false
---
# Goal and frozen scope
Implement safe Rust macOS13arm system wrappers fill_random(&mut[u8]) with<=256byte cap/zero-on-native-failure, current_uid()->u32, peer_credentials(BorrowedFd)->Result(uid:u32,gid:u32). Use Security SecRandomCopyBytes(NULL documented default), System getuid/getpeereid with exact C ABI types checked against SDKheaders. No external crate/C shim. Crate deniesunsafe, explicitly allow only tiny private sys.rs, denyunsafe_op_in_unsafe_fn, document each SAFETY borrow/length/FD/no retention/thread invariants. Safe signatures never expose raw pointers; captureerrno immediately and all native errors failclosed. Guard macOSarm only, no otherOSstubs. Test native empty/small/boundary entropy and safe cap/failure classification via narrow test helper, socketpair same-UID and non-socket error, no credentialvalues/secrets printed. No AppKit/window/UI/cryptography certification claim.

# Reference paths
Existing C peer-UID checks and task entropy behavior; ADR0024; local Apple SDK Security/SecRandom.h + sys/unistd/socket headers and primary docs cited in ADR

# Files allowed to touch
- `work/WO-RUST-NATIVE-SYSTEM-01.md`
- `rust/crates/vexel-platform-macos/src/**`, `rust/crates/vexel-platform-macos/tests/**`
- `artifacts/WO-RUST-NATIVE-SYSTEM-01/**`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`, `docs/product/rust-port-goal-and-plan.md` (coordinator only)

# Protected paths
All old C/ObjC/include/Python/shell, original tests/fixtures/oracles, other Rust crates, all Cargo manifests/lock/config/toolchain, coordinator docs/records. Preserve accepted same-crate behavior/assertions while extending it. No main edits, deps/downloads/system tools/formatters, unsupported OS code, C runtime shim, staging/commits/reset/clean, recursive agents/network, broad UI/E2E/C tests/corpus/perf campaign. Domain unsafe forbidden; only native-system order permits documented sys.rs boundary under ADR0024.

# Evidence and quality
Read OWNER-DIRECTIVES/AGENTS/ADR0022-0024/Rust goal then this order. Safe owned or lifetime-bound data, checked math and fallible bounded allocation. Map named Csymbols to Rustsymbols/focused tests/API adaptations in parity-map.json. No TODO/unimplemented/stub for namedscope; no whole-subsystem completion claim. Source<800lines, cohesive modules. If API changes outside scope needed stop with concrete dependency conflict, no manifest edit. Preserve original assertions and port meaningful cases, not merely is_err for diagnostic positions. Root reviews source and integrated checks before acceptance.

# Execution and verification
Only .worktrees/rust2-platform; Luna high, one implementation plus one narrowed review repair, thin context. Root serializes integration/shared metadata.
cargo +stable check -p vexel-platform-macos --locked --offline
cargo +stable test -p vexel-platform-macos --locked --offline
python3 scripts/wo_validate.py WO-RUST-NATIVE-SYSTEM-01 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check
Exact compiler1.99.0 via installed stable, repo-local CARGO_HOME/TMPDIR/target, MACOSX_DEPLOYMENT_TARGET13. No rustfmt/clippy installed; do not claim/install them. Workers only own src/tests/artifact; never root records.

# Output
Leave uncommitted. verification.txt/parity-map.json with exact behavior/checks/errors/adaptations/remaining scopes. Return<=450words. No additional billing/credit purchase; stop/checkpoint on rate/allowance errors instead of retryloop.
