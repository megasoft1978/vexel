---
id: WO-BOOT-02
revision: 1
status: READY
milestone: M0
owner: project-owner
executor: codex-gpt-6-luna
base_sha: 2d8e1ea60356d418d7a69cb6c926eacad7f16d08
parallel_safe: false
spec_sha256: 0dcc6bb47b027f55c1d78968b8b5e3316e297ee9b146ff3655271cce563eadfa
---
# Goal
Establish concise project instructions, locked-decision ADRs, current state, and a validated append-only work ledger.

# Context pointers
- Read: `AGENTS.md`, `STATE.md`, `LEDGER.md`, `OWNER-DIRECTIVES.md`, `docs/research-r4/01-operating-model.md`, `docs/research-r4/02-work-order-template.md`, `docs/research-r4/05-revised-roadmap.md`, `docs/research-r3/plan/04-coding-standard.md`.
- Relevant ADRs: none yet.
- Frozen tests/oracles: none.
- External docs: official Mozilla MPL-2.0 text at `https://www.mozilla.org/MPL/2.0/index.txt`; use Python standard library only otherwise.

# Existing contracts and invariants
macOS 13+, Apple Silicon only; C17; MPL-2.0; no GPL SVG-editor source; no other-OS stubs, CI, or portability work. Preserve user files and keep all generated content repo-local.

# Files allowed to touch
- `AGENTS.md`, `CLAUDE.md`, `STATE.md`, `LEDGER.md`, `ledger.jsonl`, `LICENSE`, `NOTICE`, `README.md`
- `docs/adr/**`, `docs/CODING_STANDARD.md`, `docs/schemas/**`
- `scripts/ledger.py`, `scripts/README.md`
- `tests/__init__.py`, `tests/test_ledger.py`, `.gitignore`
- `work/WO-BOOT-02.md`
- `artifacts/WO-BOOT-02/**`

# Protected paths
- `OWNER-DIRECTIVES.md` (read-only, owner-provided)
- `docs/research-*/**`
- All BOOT-01 source/build files and `artifacts/WO-BOOT-01/**`
- `tests/oracle/**`, `tests/golden/**`, `bench/corpus/**`

# Required implementation
1. Keep `AGENTS.md` under 200 lines, link to owner directives and authoritative docs, and describe repo map, commands, safety, and macOS-only scope. Add root `CLAUDE.md` as a short pointer to `AGENTS.md`.
2. Record locked product, platform, license, dependency, UI, parser/rasterizer, and source-preservation decisions in dated ADRs without reopening them.
3. Include the official MPL-2.0 license text and add normative human-readable C17 coding standards consistent with the project decisions.
4. Define strict JSONL ledger entry schema and a Python standard-library validator/atomic append command. Reject malformed JSON, unknown keys, invalid UTC timestamps, invalid transitions, duplicate run IDs, and invalid required fields.
5. Seed `ledger.jsonl` with BOOT-01 lifecycle entries from the existing run evidence. Keep `LEDGER.md` as a concise human-readable index.
6. Add automated tests for valid lifecycle, malformed JSON/entries, duplicate run IDs, invalid transitions, and concurrent append integrity.

# Acceptance criteria (machine-checkable)
- AC-1: `AGENTS.md` is at most 200 lines, mentions `OWNER-DIRECTIVES.md`, and states the macOS-only policy; `CLAUDE.md` points to it.
- AC-2: ADRs and coding standard record the locked choices and MPL-2.0 without conflicting with owner directives.
- AC-3: `python3 scripts/ledger.py validate ledger.jsonl` exits 0 for the seeded BOOT-01 history.
- AC-4: `python3 -m unittest tests.test_ledger` passes valid, malformed, duplicate-ID/run, transition, and concurrent-append cases.
- AC-5: `python3 scripts/ledger.py append --help` and invalid append examples behave deterministically; append uses a repository-local lock and rejects schema violations without changing the log.
- AC-6: `git diff --check` passes and all changed paths are allowed.

# Verification sequence
1. Run line-count and directive checks.
2. Validate seeded JSONL and run ledger unit tests.
3. Exercise invalid append against a temporary repo-local fixture; compare bytes before/after.
4. Run concurrent append test and validate resulting event sequence.
5. Fresh read-only codex verifier, then record report and commit.

# Non-goals
No workflow runner, general WO engine, package installation, database, network access, or edits to owner directives.

# Budget
Executor max runs: 2; verifier: 1 per round, at most 3 rounds; elapsed hard limit: 100 min; no network/dependencies.

# Failure and escalation
Stop if atomic append cannot be made safe on macOS with the standard library. Do not weaken validation. Preserve evidence and write a blocker if unresolved.

# Output
Commit `docs: establish agent operating state`; update `STATE.md`, `LEDGER.md`, machine ledger, and executor/verifier JSON artifacts.
