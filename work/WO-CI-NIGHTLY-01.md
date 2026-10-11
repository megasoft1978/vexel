---
id: WO-CI-NIGHTLY-01
revision: 1
status: READY
milestone: CI
owner: project-owner
executor: codex-gpt-6-luna
base_sha: c43fa826c0fbc8910e8dc0c09b4139c8aa615995
parallel_safe: true
network: false
spec_sha256: bc9febc4769dc0edc1902d51131e364daa82c12498a663eb07a7c4e1fd365119
---
# Goal
Make the macOS nightly sanitizer job provision librsvg and Debug subprocess binaries and use the hosted-runner owner-step limit already used by main CI.

# Context pointers
- Read: `AGENTS.md`, `OWNER-DIRECTIVES.md`, `STATE.md`, `docs/ci.md`, `.github/workflows/ci.yml`, `.github/workflows/nightly.yml`, `dev`, `scripts/check_ci.py`, `work/WO-CI-NIGHTLY-01.md`.
- CI evidence: GitHub Actions runs `38147398332` and `38147398376` at base commit `c43fa826c0fbc8910e8dc0c09b4139c8aa615995`.
- Current behavior: `./dev asan` builds and tests `build/mac-asan`, while Python subprocess tests resolve executables from `build/mac-debug`; the nightly workflow currently installs no librsvg. CI sets `VEXEL_OWNER_STEP_LIMIT_MS=500` in its build/verification step; the replay test defaults to 100 ms.

# Existing contracts and invariants
- Apple Silicon macOS 13+ only; keep the single `macos-15` runner and current pinned actions.
- CI may install only librsvg for reference-renderer tests, as permitted by `scripts/check_ci.py`.
- Keep all CTest cases, Python tests, goldens, and assertions unchanged. Do not hide or update known golden differences in this order.
- `./dev build` creates the Debug binaries consumed by Python tests; sanitizer builds/tests remain `./dev asan` and `./dev ubsan`.

# Files allowed to touch
- `.github/workflows/nightly.yml`
- `work/WO-CI-NIGHTLY-01.md`
- `artifacts/WO-CI-NIGHTLY-01/**`
- `STATE.md`
- `LEDGER.md`
- `ledger.jsonl`

# Protected paths
- `.github/workflows/ci.yml`, all other workflows, `OWNER-DIRECTIVES.md`, `AGENTS.md`, all application/source/test files, all golden/oracle inputs, all existing artifacts, and all paths outside the exact allowlist above.
- All other work orders are protected; only this order is writable under `work/`.

# Required implementation
1. Add the approved `brew install librsvg` setup step to the nightly job before tests that use the reference renderer.
2. Build Debug binaries with `./dev build` before `./dev e2e`, `./dev asan`, and `./dev ubsan`, so Python subprocess tests that use `build/mac-debug/bin` have their required executables.
3. Set the nightly job's `VEXEL_OWNER_STEP_LIMIT_MS` to `500`, matching `.github/workflows/ci.yml` to tolerate hosted-runner scheduling jitter without changing the local default.
4. Preserve all existing nightly steps, seeded parameters, sanitizer/fuzz coverage, timeout, runner, permissions, and artifact upload behavior.
5. Record the frozen order and implementation status in `STATE.md`, `LEDGER.md`, and `ledger.jsonl`; keep the order revision/hash and lifecycle events valid.

# Acceptance criteria (machine-checkable)
- AC-1: The focused structural command in the Verification commands block confirms the nightly step order, approved formula, 500 ms job setting, and preservation of the original campaign/artifact commands; evidence `artifacts/WO-CI-NIGHTLY-01/workflow-check.txt`.
- AC-2: `actionlint .github/workflows/nightly.yml` and `python3 scripts/check_ci.py .github/workflows/ci.yml` exit 0; evidence `artifacts/WO-CI-NIGHTLY-01/ci-static-checks.txt`.
- AC-3: `python3 scripts/wo_validate.py WO-CI-NIGHTLY-01 --ready`, `python3 scripts/ledger.py validate ledger.jsonl`, and `git diff --check` exit 0; the implementation diff changes only allowed paths; evidence `artifacts/WO-CI-NIGHTLY-01/scope-check.txt`.
- AC-4: Review confirms no tests, goldens, source, sanitizer flags, or test-selection rules changed. The four observed golden mismatches (`native-shell-goldens`, `editor-style-goldens`, `editor-snap-overlay-goldens`, `editor-accessibility-localization-goldens`) remain explicitly out of scope and are not reported as fixed.

# Verification commands
```text
python3 -c 'from pathlib import Path; s=Path(".github/workflows/nightly.yml").read_text(); needles=["VEXEL_OWNER_STEP_LIMIT_MS: \"500\"", "run: brew install librsvg", "run: ./dev build", "run: ./dev e2e", "run: ./dev asan", "run: ./dev ubsan", "run: ./dev soak --minutes 8 --seed 20261010", "name: vexel-nightly-reports-${{ github.run_id }}"]; positions=[s.index(x) for x in needles]; assert positions == sorted(positions), positions; assert "runs-on: macos-15" in s and "timeout-minutes: 60" in s and "if-no-files-found: warn" in s; print("nightly prerequisites/order and existing campaign contract: pass")'
actionlint .github/workflows/nightly.yml
python3 scripts/check_ci.py .github/workflows/ci.yml
python3 -m unittest tests.test_dev
python3 scripts/wo_validate.py WO-CI-NIGHTLY-01 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check
```

# Non-goals
Do not change or exclude tests, regenerate/relax goldens, repair renderer output, modify scripts or build presets, add CI runners, or claim the four golden mismatches are resolved. Do not run or trigger GitHub Actions from this order.

# Budget
Executor max runs: 2; verifier: 1; elapsed hard limit: 45 min; output-token soft cap: 10000; dependency/network: none for the executor. The CI job's existing approved Homebrew installation is a workflow operation.

# Failure and escalation
Stop if the CI policy rejects the approved librsvg setup, if the existing workflow cannot be amended without changing protected files, or if the frozen-order/path guard rejects the scope. A local full sanitizer run may still report the known golden mismatches; preserve and report their exact status without changing tests or baselines.

# Output
Commit the frozen READY order and lifecycle entry first, then the implementation with `ci: provision nightly sanitizer prerequisites`. Expected implementation paths are `.github/workflows/nightly.yml`, this order unchanged, `STATE.md`, `LEDGER.md`, `ledger.jsonl`, and `artifacts/WO-CI-NIGHTLY-01/**`. Report exact commands, status, evidence paths, commit SHA, and remaining golden failures.
