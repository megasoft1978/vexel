---
id: WO-RUST-CI-CHROME-TRACE-01
revision: 1
status: READY
milestone: FULL-RUST-PROJECT
owner: project-coordinator
executor: codex-gpt-6-luna
base_sha: 9e030caf17dca33ceeb7ba7a0e560e0c3e013feb
spec_sha256: 1fcd8c507d8f3f5c4398fb545faeb5b0833299aeb8e29997562b107b4d70b473
parallel_safe: true
network: false
---
# Goal
Enable the already-reviewed, bounded chrome-golden failure diagnostics in hosted macOS CI so the next exact failure records spatial mismatch summaries.

# Context
- `WO-RUST-CI-CHROME-01` adds opt-in `VEXEL_CHROME_GOLDEN_DIAGNOSTICS=1` failure-only diagnostics to the golden test; focused local test passes with diagnostics unset.
- The hosted failure `39297932450` on macOS 15.7.9 does not contain mismatch locations. CoreText variation remains a hypothesis, not a confirmed cause.
- `scripts/check_ci.py` governs pinned macOS workflow policy. Read `AGENTS.md` and inspect the exact existing test step before editing.

# Allowed paths
- `.github/workflows/ci.yml` (only add the diagnostic environment variable to the existing `Run tests` step; preserve command, runner, ordering and every other step)
- `artifacts/WO-RUST-CI-CHROME-TRACE-01/**`
- `work/WO-RUST-CI-CHROME-TRACE-01.md` and `ledger.jsonl` by coordinator only

# Protected paths
All application/test sources, fixtures/goldens/oracles, comparison limits, all other workflow content, manifests/locks, dependencies, other work orders/evidence, preserved legacy source, and external paths. No tests/builds, action additions, network access, runner change, or CI test change.

# Acceptance
- AC-1: only `VEXEL_CHROME_GOLDEN_DIAGNOSTICS: "1"` is added to the existing Run tests step.
- AC-2: the variable has no effect on successful golden comparisons and causes bounded summaries only on golden mismatch.
- AC-3: `python3 scripts/check_ci.py .github/workflows/ci.yml` and `git diff --check` pass.
- AC-4: a hosted run on the committed change captures the summaries; interpreting that output is a separate coordinator decision. No root-cause claim or fix is made by this order.

# Verification
```text
python3 scripts/check_ci.py .github/workflows/ci.yml
python3 scripts/wo_validate.py --ready WO-RUST-CI-CHROME-TRACE-01
git diff --check
```
