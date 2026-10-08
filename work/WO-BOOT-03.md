---
id: WO-BOOT-03
revision: 1
status: READY
milestone: M0
owner: project-owner
executor: codex-gpt-6-luna
base_sha: 4cc8bfbab2ecdda7c4cd72354f99cb28c08ab477
parallel_safe: false
spec_sha256: 871c2fa6be203ff1d53941848629c76feef98af9ad3385c59619a59991a2b4f0
---
# Goal
Provide a repeatable macOS-only local build/test/verify path and one pinned, macOS Apple Silicon CI job.

# Context pointers
- Read: `AGENTS.md`, `STATE.md`, `OWNER-DIRECTIVES.md`, `work/WO-BOOT-03.md`, `docs/research-r4/01-operating-model.md`, `docs/research-r4/02-work-order-template.md`, `docs/research-r4/03-wrapper-scripts-spec.md`, `docs/research-r4/05-revised-roadmap.md`.
- Relevant ADRs: `docs/adr/0001-product-and-platform-locks.md`, `docs/adr/0002-source-preservation-and-provenance.md`.
- Tests: `tests/test_ledger.py`; no frozen oracles.
- External docs: GitHub Actions runner docs and official `actions/checkout` release, pinned in workflow comments. Checkout SHA verified as `3d3c42e5aac5ba805825da76410c181273ba90b1` (v7.0.1).

# Existing contracts and invariants
macOS 13+, Apple Silicon only; no other-OS CI, runners, shims, or portability. No system package installation. Keep local fast checks under 30 seconds. No workflow secrets.

# Files allowed to touch
- `dev`
- `.github/workflows/**`
- `scripts/check_ci.py`
- `tests/test_dev.py`
- `docs/ci.md`
- `AGENTS.md`, `STATE.md`, `LEDGER.md`, `ledger.jsonl`
- `work/WO-BOOT-03.md`
- `artifacts/WO-BOOT-03/**`

# Protected paths
- `OWNER-DIRECTIVES.md` (read-only)
- `docs/research-*/**`, `tests/oracle/**`, `tests/golden/**`, `bench/corpus/**`
- `CMakeLists.txt`, CMake presets, app source, and BOOT-01/02 evidence

# Required implementation
1. Extend `./dev` with `build`, `test-fast`/`test`, `verify`, and `help`, with clear errors for unknown commands and macOS-only execution.
2. `test-fast` runs the repository Python tests and validates `ledger.jsonl`; `verify` runs build then test-fast.
3. Add a minimal GitHub Actions workflow on `macos-15` (Apple Silicon) only, triggered by push, pull request, and manual dispatch. Use least permissions, timeout, no secrets/cache, and pin every action to a full commit SHA.
4. Add a dependency-free CI policy checker and tests that reject non-macOS runners and unpinned actions.
5. Document local commands, CI behavior, runner assumptions, and whether hosted CI was actually observed.

# Acceptance criteria (machine-checkable)
- AC-1: `./dev build`, `./dev test-fast`, and `./dev verify` exit 0; test-fast finishes under 30 seconds; evidence `artifacts/WO-BOOT-03/local-checks.txt`.
- AC-2: `actionlint .github/workflows/ci.yml` exits 0; evidence `artifacts/WO-BOOT-03/actionlint.txt`.
- AC-3: `python3 scripts/check_ci.py .github/workflows/ci.yml` exits 0 and confirms only `macos-15`, full-SHA action pins, read-only permissions, and required triggers; evidence `artifacts/WO-BOOT-03/ci-policy.txt`.
- AC-4: `python3 -m unittest tests.test_dev tests.test_ledger` passes positive and negative policy/dispatch tests; evidence `artifacts/WO-BOOT-03/tests.txt`.
- AC-5: Workflow contains no secrets, package installation, cache, or non-macOS job; remote run is explicitly marked pending if no configured GitHub remote is available.
- AC-6: `git diff --check` passes and all changed paths are allowed.

# Verification sequence
1. Run dev commands and time the fast suite.
2. Run actionlint and CI policy checker plus negative tests.
3. Inspect workflow triggers, permissions, runner, action pin, and no secrets.
4. Fresh read-only codex verifier; update state and ledger after pass.

# Non-goals
No Linux/Windows matrix, external cache, package installation, benchmark runner, formatter, sanitizer/fuzzer workflow, or deployment.

# Budget
Executor max runs: 2; verifier: 1 per round, at most 3 rounds; elapsed hard limit: 120 min; no runtime dependencies.

# Failure and escalation
Do not relax macOS-only policy. If the pinned macOS runner cannot be represented in the selected CI provider, document the exact limitation and stop before adding a different platform.

# Output
Commit `ci: add macOS local verification`; update `STATE.md`, `LEDGER.md`, `ledger.jsonl`, and executor/verifier JSON artifacts.
