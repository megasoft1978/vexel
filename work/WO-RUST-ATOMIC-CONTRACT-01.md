---
id: WO-RUST-ATOMIC-CONTRACT-01
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: codex-coordinator
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: ec3f2969c216e99a1ad2b37e0f7d34a8f73c9c229d3e9cc757f32dccfb691a03
parallel_safe: false
network: false
---
# Goal
Resolve the verified std-only atomic-close contract blocker without weakening precommit error semantics: accept a tiny consuming Darwin checked-close boundary and supersede atomic-files revision1 with frozen revision2.

# Files allowed to touch
- `work/WO-RUST-ATOMIC-CONTRACT-01.md`, `work/WO-RUST-ATOMIC-FILES-01.md`
- `docs/adr/0025-rust-checked-close.md`
- `artifacts/WO-RUST-ATOMIC-CONTRACT-01/**`, `artifacts/WO-RUST-ATOMIC-FILES-01/**`
- `.worktrees/rust3-atomic-files/work/WO-RUST-ATOMIC-FILES-01.md`
- `.worktrees/rust3-atomic-files/docs/adr/0025-rust-checked-close.md`
- `.worktrees/rust3-atomic-files/ledger.jsonl`, `.worktrees/rust3-atomic-files/artifacts/WO-RUST-ATOMIC-FILES-01/**`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`, `docs/product/rust-port-goal-and-plan.md`

# Protected paths
All application/Rust source, manifests/locks/config, legacy source/fixtures/oracles, other workers, previous immutable evidence. Preserve atomic r1 spec and blocked worker results before revision. This order changes metadata/contracts only; actual implementation remains isolated atomic r2. No downloads, dependencies, installs, unsupported OS, source deletion/staging/commit, application tests or UI campaign.

# Requirements
Verified C checks close errors before publication; std File drop hides them. New ADR authorizes only sys.rs private Darwin close(int)->int, consuming OwnedFd into_raw_fd once, single native close, immediate errno, no blind retry/double-close and no public raw ptr/fd. Safe close_checked(OwnedFd)->io::Result wrapper; File converts via owned From. Preserve existing entropy/identity/peer APIs/8tests. Exact SDK declaration and primary docs facts checked2026-10-14; no algorithm/source copying. Archive frozen r1/evidence, record terminal r1 lifecycle, revise r2 SHA, record explicit coordinator worktree metadata delta and refresh local ledger so validators remain accurate. No false r1 implementation/testing claim. Existing owner full-port/small audited FFI authorization suffices; no external dependency or new product scope.

# Verification
python3 scripts/wo_validate.py WO-RUST-ATOMIC-CONTRACT-01 --ready
python3 scripts/wo_validate.py WO-RUST-ATOMIC-FILES-01 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check
Validate archived r1 digest, r2/local work-order equality, exact allowed metadata deltas and original603/index protection. No application tests for contract-only order.

# Output
Accepted checked-close contract/ADR, faithfully revised atomic order and auditable coordinator overlay delta. Resume original subscription LunaHigh writer against r2 without scope weakening or new billing.
