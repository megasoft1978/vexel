---
id: WO-BOOT-04
revision: 2
status: READY
milestone: M0
owner: project-owner
executor: codex-gpt-6-luna
base_sha: 837e88acdc247c790b0b9785efed98f544584fcb
parallel_safe: false
spec_sha256: 42422db000a6c118a693273085d93981868732448b6395b1258a2de53cc41766
---
# Goal
Implement macOS-only work-order execution and verification wrappers that enforce frozen scope, exact session identity, structured reports, and the owner-approved Codex CLI contract recorded in ADR-0003.

# Context pointers
- Read: `AGENTS.md`, `STATE.md`, `OWNER-DIRECTIVES.md`, this file, `docs/research-r4/01-operating-model.md`, `docs/research-r4/02-work-order-template.md`, `docs/research-r4/03-wrapper-scripts-spec.md`, `docs/research-r4/05-revised-roadmap.md`, `docs/CODING_STANDARD.md`, `scripts/README.md`.
- Relevant ADRs: `docs/adr/0001-product-and-platform-locks.md`, `docs/adr/0002-source-preservation-and-provenance.md`, `docs/adr/0003-codex-cli-compatibility-amendment.md`.
- Tests: new `tests/test_wo_runner.py`; no frozen oracles.
- External docs: OpenAI Codex CLI reference https://learn.chatgpt.com/docs/developer-commands?surface=cli; verify installed CLI flags from local help, not recollection.

# Existing contracts and invariants
- macOS 13+, Apple Silicon only; no non-macOS code or runners.
- Shell wrappers delegate all parsing, validation, locking, process management, JSONL parsing, and git checks to Python standard library code.
- Never evaluate work-order content as shell. Never use `resume --last`. No auto-commit or auto-merge.
- The ledger JSONL shape in `docs/schemas/ledger-entry.schema.json` remains authoritative; every event must refer to an existing frozen WO whose declared spec hash matches its content.
- `.worktrees/` and `.cache/` are generated local state; artifact outputs stay beneath `artifacts/`.

# Files allowed to touch
- `scripts/wo-run.sh`, `scripts/wo-verify.sh`, `scripts/wo_driver.py`, `scripts/wo_validate.py`
- `scripts/ledger.py`, `scripts/README.md`, `docs/schemas/**`, `docs/adr/0003-codex-cli-compatibility-amendment.md`, `tests/test_wo_runner.py`, `tests/test_ledger.py`
- `.gitignore`, `dev`, `STATE.md`, `LEDGER.md`, `ledger.jsonl`, `work/WO-BOOT-04.md`
- `artifacts/WO-BOOT-04/**`

# Protected paths
- `OWNER-DIRECTIVES.md`
- `tests/oracle/**`, `tests/golden/manifest.lock`, `bench/corpus/manifest.lock`
- `CMakeLists.txt`, `app/**`, `src/**`, `.github/**`

# Required implementation
1. Freeze and validate the work-order SHA-256 over the complete UTF-8 contents with the `spec_sha256` line removed. Require READY status, revision, IDs, exact declared base commit, clean main for `new`, a known branch/session for `resume`, and no existing per-WO lock. The declared base is an ancestor of the clean main commit so the frozen WO itself is available in the task worktree.
2. Create task worktrees as `.worktrees/WO-ID` on `wo/WO-ID` from the clean main commit; scope changes from that start commit and require resulting commits to descend from the WO's declared base. Never copy environment secrets or use shell evaluation. Reject path escapes, protected path changes, dirty/uncommitted executor state, and commits not descending from base.
3. Preflight installed `codex exec --help` and `codex exec resume --help` for capabilities in the owner-approved CLI contract (ADR-0003). Record version and captured help; do not claim approval-gating or resume sandbox flags absent from the installed CLI.
4. Invoke executor argv using `-m gpt-6-luna -c model_reasoning_effort=high --dangerously-bypass-approvals-and-sandbox --skip-git-repo-check --json --output-schema <executor.schema.json> --output-last-message <report.json> [--image path] - <prompt.md>`, in the task worktree only. Apply timeout, path guard, protected-path check, and clean-tree check before accepting a candidate. A fresh verifier uses `codex exec -s read-only --output-schema ...` at the exact candidate commit. Parse unknown JSONL event types and fsync a known `thread.started` session ID to `threads.jsonl` immediately. Resume only an explicitly supplied matching UUID/session and frozen cache key; reject a changed WO revision/hash/base.
5. Validate executor report schema, exit status, base/head, all acceptance command evidence, changed paths against explicit allowlists/protected paths, clean commit and a valid descendant relation. Artifact metadata must be JSON, omit environment values, and use nullable usage only when unavailable.
6. Implement a fresh read-only verifier wrapper at exact candidate commit. Run the WO's deterministic verification commands before model review, capture results, and reject failed AC. The verifier gets the frozen WO and bounded diff/change content; it cannot alter the candidate or ledger.
7. Ensure ledger events validate the referenced frozen WO and its declared spec hash. Implement structured `show` and `summary` without changing the established entry schema or existing historical entries.
8. Fake-Codex tests cover: success; malformed JSONL; timeout; missing report/schema; unsupported CLI flag; missing thread ID; invalid/changed resume identity; dirty base/tree; protected/disallowed path; forged AC pass; verifier failure; concurrent lock; invalid ledger transition; interrupted atomic ledger append. Tests must use temporary repos and never access the network or real Codex.
9. Add thin executable shell delegates, macOS guard, usage output, concise wrapper documentation, and developer command `./dev wo-help` only.

# Acceptance criteria (machine-checkable)
- AC-1 `python3 scripts/wo_validate.py --self-test` exits 0 and validates exact WO hash/status/allowlist boundaries; evidence `artifacts/WO-BOOT-04/validation.txt`.
- AC-2 `python3 -m unittest tests.test_ledger tests.test_wo_runner` passes all cases in under 30 seconds; evidence `artifacts/WO-BOOT-04/tests.txt`.
- AC-3 `scripts/wo-run.sh --help` and `scripts/wo-verify.sh --help` print usage; both reject non-Darwin invocation before side effects; evidence `artifacts/WO-BOOT-04/help.txt` and `platform-guard.txt`.
- AC-4 installed Codex CLI preflight records version and detects missing required capabilities before model invocation; evidence `artifacts/WO-BOOT-04/codex-preflight.txt`.
- AC-5 `python3 scripts/ledger.py validate ledger.jsonl`, `./dev test-fast`, `./dev verify`, `git diff --check`, and `actionlint .github/workflows/ci.yml` pass; evidence `artifacts/WO-BOOT-04/local-checks.txt`.
- AC-6 Fresh read-only `codex exec` review verdict schema passes with `verdict=pass`; save exact command/output under artifacts.

# Verification commands

Run these commands in order at the candidate commit, with no shell evaluation:

```text
python3 scripts/wo_validate.py --self-test
python3 -m unittest tests.test_ledger tests.test_wo_runner
python3 scripts/ledger.py validate ledger.jsonl
./dev test-fast
./dev verify
git diff --check 837e88acdc247c790b0b9785efed98f544584fcb HEAD
actionlint .github/workflows/ci.yml
```

# Verification sequence
1. `./dev verify` and `python3 -m unittest tests.test_ledger tests.test_wo_runner`.
2. Run fake executable acceptance suite; preserve each case name and status.
3. Probe installed Codex help; only invoke the owner-approved executor pattern inside the guarded task worktree after all local fake-Codex cases pass.
4. Test wrapper help and Darwin guard; no worktree, model session, or network side effect.
5. Save bounded command outputs beneath `artifacts/WO-BOOT-04/`; report exact CLI version and unavailable live-run reason.

# Non-goals
No app code, non-macOS support, CLI installation/update, dependency installation, automatic merge, credentials or environment dump, model fallback, or changes to frozen oracles. Executor may commit candidate code after acceptance checks; wrapper never commits or merges. The owner-approved CLI bypass is constrained by task-worktree cwd and path guards in ADR-0003.

# Budget
Executor max runs: 2; verifier: 1; elapsed hard limit: 120 min; output-token soft cap: 15000; dependency/network: none.

# Failure and escalation
If a capability required by the amended CLI contract is unavailable, wrappers must refuse model execution and record the incompatibility. Do not silently substitute another model or omit path guards. Retry only for implementation/test defects.

# Output
Commit convention: `build: add work-order runner and verifier`. Expected changed files are within the allowlist. Use the existing executor/verifier JSON schemas in `docs/schemas/`; final response summarizes evidence and any local CLI limitation.
