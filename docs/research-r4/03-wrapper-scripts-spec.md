# Wrapper scripts specification (implementation target WO-BOOT-04)

The scripts are orchestration, not a security boundary. Use a dedicated macOS account or disposable VM for dangerous/untrusted code. `workspace-write` limits Codex writes to the active worktree but does not neutralize malicious build scripts, credentials readable by the account, or network exfiltration. Keep credentials out of the process environment, use a clean account, disable network at the VM/firewall layer where possible, and run dependency fetch as a separate reviewed step.

## CLI assumptions to pin and probe

Official Codex CLI reference currently documents `codex exec`, `codex exec resume [SESSION_ID]`, JSONL `--json`, `--output-schema`, `--output-last-message`, `--sandbox read-only|workspace-write|danger-full-access`, `--ask-for-approval never`, and `--image/-i` for exec and resume. Pin a tested CLI version in `.tool-versions`/bootstrap manifest. A preflight probe verifies `codex exec --help` and `codex exec resume --help` support the exact flags; refuse to run if not. Do not infer behavior from an old installed CLI. Resume requires a known session ID recorded from JSON events; never use ambiguous `--last` in automated worktrees.

## `wo-run.sh`

Interface:

```text
scripts/wo-run.sh new WO-M0-01 [--branch-base <sha>] [--image <png> ...]
scripts/wo-run.sh resume WO-M0-01 <session-id> [--image <png> ...]
```

Requirements:

- Validate WO ID and READY status, schema, hash, clean base, exact base SHA, no existing active lock.
- Create worktree `.worktrees/WO-ID` on `wo/WO-ID`; copy no secrets. Ensure allowlisted paths and separate `artifacts/runs/WO-ID/<run-id>`.
- New invokes `codex exec --model gpt-6-luna --config model_reasoning_effort=medium --sandbox workspace-write --ask-for-approval never --json --output-schema <executor.schema.json> --output-last-message <report.json> [--image path] - < prompt.md`; include only instruction to read WO/context and perform it. Capture JSONL events, stdout/stderr, start/end UTC, CLI version, `git status`, diff stats, usage if events expose it. `timeout`/Python supervisor sends TERM then KILL and marks timeout without claiming complete.
- Resume invokes `codex exec resume <session-id>` with the same required model/sandbox/approval/schema/output flags and a short continuation prompt. Pin the same worktree/session. Never resume after WO revision/spec hash changes.
- Maintain `threads.jsonl` as an operational cache keyed by `(wo_id, revision, spec_sha256, worktree_path, base_sha)`, mapping to Codex session ID and last run ID. Cache is only a lookup aid: ledger remains authoritative. Reject cache entries on any key mismatch; never select the newest session globally. Record the session ID as soon as a start event arrives, then fsync it so a wrapper crash can resume safely.
- After completion, reject nonzero CLI exit, missing/invalid schema report, base/head mismatch, paths outside allowlist, deleted/modified protected tests, uncommitted changes, or no evidence for each AC. Executor commits only after all required commands pass; wrapper verifies the commit is descendant of base and tree clean.
- No auto-commit by wrapper. No automatic merge. Session ID extracted from JSONL event type `thread.started` / `thread_id`; parser must tolerate unknown event types and fail closed if session ID missing when resume requested.
- Logging: one JSON metadata record per run; do not log credentials or full environment. Cost fields nullable and sourced from CLI/account usage, never fabricated.

Illustrative shell core (the production implementation should use Python for argument parsing, locking, JSON validation, timeout supervision, and JSONL parsing; shell only delegates):

```bash
#!/usr/bin/env bash
set -euo pipefail
umask 077
ROOT="$(git rev-parse --show-toplevel)"
WO_ID="${2:?usage: wo-run.sh new|resume WO-ID ...}"
case "$WO_ID" in WO-[A-Z0-9]* ) ;; *) exit 64;; esac
RUN_ID="$(python3 -c 'import uuid; print(uuid.uuid4())')"
RUN_DIR="$ROOT/artifacts/runs/$WO_ID/$RUN_ID"
mkdir -p "$RUN_DIR"
python3 scripts/wo_validate.py "$WO_ID" --ready --schemas
# The driver constructs argv as a list (never shell-evaluates WO content).
python3 scripts/wo_driver.py "$@" --run-dir "$RUN_DIR"
```

## `wo-verify.sh`

`wo-verify.sh WO-ID <commit>` creates a detached verification worktree at exact candidate SHA and launches a fresh `codex exec` session in `--sandbox read-only` with verifier schema. Deterministic AC scripts are executed by the wrapper before the model review, in a clean build/cache directory where practical. Input is the frozen WO, base and head diff (`git diff --no-ext-diff --no-renames`), changed file contents, test IDs, machine result JSON, and bounded logs. Exclude agent transcripts and executor final prose to reduce anchoring. It checks scope, implementation against contract, adversarial edge cases, mutation of tests, unexplained fixtures/special cases, error-path/resource-limit behavior, and discrepancy between claims and artifacts. It returns `pass` only if all AC scripts pass and no high/critical finding. It never edits or commits.

## `ledger`

Python CLI: `ledger init|ready|start|finish|verify|accept|reject|merge|block|show|summary`. Append JSONL under advisory lock using temp write + `fsync` + atomic replace or locked append; validate schema/transitions and spec hash. `show` supports small per-WO structured summary; no transcript import. `summary` reports per milestone pass rates, attempts, minutes, observed tokens/cost when available, failure class, rework rate, and worktree state. Fail closed on malformed ledger.

## Worktree lifecycle

- `new`: require clean main; `git worktree add -b wo/ID .worktrees/ID BASE_SHA`; write `.git`-ignored runtime metadata.
- `verify`: separate detached worktree `.worktrees/verify-ID-RUN`; no write permission in Codex sandbox; scripts may write only build/artifact temp locations on separate verifier filesystem. Better use a disposable APFS snapshot/VM for full protection.
- `merge`: only `ledger accept`; run `git merge --ff-only` if main equals base, else rebase task branch on new main, rerun all AC and verifier. Cleanup worktrees only after artifact retention and acceptance recorded.
- Parallel: lock shared build/cache paths or give each worktree its own build. `parallel_safe` must be true, and overlap checker must confirm touched paths disjoint.

## Timeouts, sandboxing, network, usage

- Default hard timeout 120 min, termination grace 30 sec; no wrapper automatic retries. Retry is a new run ID, same WO revision, explicit prompt with prior failure artifacts. Maximum 2 executor attempts total (one retry), then Sonnet escalation; bootstrap WOs may set a larger total in the spec.
- `workspace-write` + `--ask-for-approval never` is unattended convenience. It is not OS isolation. For scope-critical runs use separate macOS standard account with only repository/build access; for hostile repos, unknown dependencies, or fuzzing use ephemeral VM. Provide no SSH, cloud, signing, browser, package-registry or personal credentials. Network default deny; corpus/dependency downloads fetched by pinned hash in a separate reviewed process.
- `danger-full-access` prohibited for routine tasks. Shell command allowlisting is supplementary and brittle; use VM/network restrictions.
- Record model, CLI version, config, run duration, usage/credits if supplied; query account usage separately. Per-WO spending ceiling is implemented from known available billing telemetry only; if token metering is unavailable on ChatGPT plan, apply run count/time caps and owner-approved weekly allowance. Pause on usage-limit errors; do not silently switch to API billing or another model.
- Redact secrets before artifacts leave machine. Keep raw logs local with restricted permissions and retention; structured summaries are the handoff.

## Must-have tests for wrapper implementation

Fake `codex` executable tests: success; timeout; malformed JSON; missing report; unsupported CLI flag; missing thread ID; invalid resume ID; changed spec hash; dirty worktree; protected path modified; disallowed path created; forged AC pass; verifier fail; concurrent lock; ledger invalid transition; interrupted atomic append. Tests use temp repos and no network. This infrastructure itself is a bootstrap prerequisite before app implementation scales.
