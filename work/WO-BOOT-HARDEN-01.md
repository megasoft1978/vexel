---
id: WO-BOOT-HARDEN-01
revision: 1
status: READY
milestone: M0
owner: project-owner
executor: codex-gpt-6-luna
base_sha: abab44bed8c52ed0db6be6e7aed85bf2e945b612
spec_sha256: 65eab3a0cc65d52059dd6508a190957fa4d083964dc1b84e85d189e33eaf1d27
parallel_safe: false
network: false
---
# Goal
Harden the work-order runner so normal executor commands stay inside the workspace-write sandbox and candidate/root scope checks protect frozen project inputs.

# Context pointers
- Read: `AGENTS.md`, `OWNER-DIRECTIVES.md`, `STATE.md`, this file, `SECURITY-FINDINGS.md`, `scripts/README.md`, `scripts/wo_driver.py`, `scripts/wo_validate.py`, `tests/test_wo_runner.py`, `docs/research-r4/01-operating-model.md`, `docs/research-r4/02-work-order-template.md`.
- Frozen source: `SECURITY-FINDINGS.md` SHA-256 `a156b5187b6741385a1fcfe31a8330e0ac257120c8f963ddc342f3fb732dfc92`.
- Relevant ADRs: `docs/adr/0003-codex-cli-compatibility-amendment.md`.
- Protected tests/inputs: `tests/oracle/**`, `tests/golden/manifest.lock`, `bench/corpus/manifest.lock`, and every `work/**` file.
- External docs: none; use the installed `codex exec --help` only to confirm supported sandbox flags.

# Existing contracts and invariants
- Candidate checks use the frozen READY order from the repository root, whose SHA-256 is validated by `wo_validate.load_wo`.
- Executor work is limited to an isolated task worktree and its active order allowlist. Normal orders have no network access.
- The verifier remains `-s read-only`.
- Scope protection is a hard deny: an allowlist entry cannot permit edits to any frozen order or oracle/manifest input.
- macOS Apple Silicon only; Python standard library only; no network for this order.

# Files allowed to touch
- `scripts/wo_driver.py`
- `scripts/README.md`
- `tests/test_wo_runner.py`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`
- `artifacts/WO-BOOT-HARDEN-01/**`

# Protected paths
- `OWNER-DIRECTIVES.md`
- `work/**`
- `tests/oracle/**`, `tests/golden/manifest.lock`, `bench/corpus/manifest.lock`
- `.github/**`, `app/**`, `src/**`, `platform/**`

# Required implementation
1. Inspect the active order only from the root repository copy after `load_wo` confirms its frozen hash. Recheck the active order bytes/hash before accepting executor output; fail closed if the root order changed during execution.
2. Build executor argv with `-s workspace-write` by default. Only an explicit front-matter `network: true` may select the existing owner-approved unsandboxed invocation; absent, malformed, or false values must select workspace-write. Do not weaken or remove the read-only verifier sandbox.
3. Add unconditional frozen-path rules for `work/**`, `tests/oracle/**`, `tests/golden/manifest.lock`, `bench/corpus/manifest.lock`, and `OWNER-DIRECTIVES.md`. Check these before order allowlists in both candidate-diff and root-working-tree scope validation.
4. Permit the active frozen order in a candidate only when its bytes exactly match the already validated root copy; reject modified or newly added work orders, even when a test order allowlists `work/**`.
5. Add regression tests for default and explicit-network sandbox argv, verifier read-only argv, malformed network metadata, mutation of the active order, mutation/addition of another work order under a broad allowlist, and each frozen manifest/oracle path under an overlapping broad allowlist.
6. Document the sandbox selection, frozen root-copy contract, and hard-deny scope rules in `scripts/README.md`.

# Acceptance criteria (machine-checkable)
- AC-1 `python3 -m unittest tests.test_wo_runner` exits 0; tests assert default executor `-s workspace-write`, explicit `network: true` policy, and verifier `-s read-only`.
- AC-2 The same suite proves malformed/absent network metadata defaults safely and cannot broaden scope.
- AC-3 The same suite proves any changed `work/**`, oracle, manifest lock, or owner-directive path is rejected even if the work-order allowlist overlaps it; an unchanged active frozen order remains acceptable.
- AC-4 `./dev verify` exits 0 and reports zero failed or skipped tests.
- AC-5 `python3 scripts/ledger.py validate ledger.jsonl` and `git diff --check` exit 0; evidence is retained beneath `artifacts/WO-BOOT-HARDEN-01/`.
- AC-6 `python3 scripts/wo_validate.py WO-BOOT-HARDEN-01 --ready` confirms the root order SHA-256 before execution and verification.

# Verification commands

```text
python3 -m unittest tests.test_wo_runner
./dev verify
python3 scripts/ledger.py validate ledger.jsonl
python3 scripts/wo_validate.py WO-BOOT-HARDEN-01 --ready
git diff --check
```

# Non-goals
No changes to product code, network permissions for this order, CI, other operating systems, or the frozen work-order format beyond optional `network: true` metadata.

# Budget
Executor max runs: 2; verifier: 1; elapsed hard limit: 60 min; output-token soft cap: 8000; dependency/network: none.

# Failure and escalation
Stop if the installed CLI cannot provide workspace-write and read-only sandbox modes or if runner isolation cannot be established. Do not restore the broad bypass as a default.

# Output
Commit convention: `fix: harden work order execution scope`. Retain command evidence under `artifacts/WO-BOOT-HARDEN-01/`; record observed results in `STATE.md`, both ledgers, and the executor report.
