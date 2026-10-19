---
id: WO-RUST-WO-RUNNER-REVIEW-DIFF-01
revision: 1
status: READY
milestone: MIGRATION-TOOLING
owner: project-coordinator
executor: codex-gpt-6-luna
base_sha: 0a96a7110c9a9acf326dbf54dfc4d5de9a6e64b0
spec_sha256: b00be623254b7b7b872908b587d639faa4599ce7b21d76a30c8984d55db08919
parallel_safe: true
network: false
---
# Goal
Make the fresh work-order verifier review exactly the candidate change range that the runner scope-checks, while preserving the frozen work-order base as provenance.

# Context pointers
- Read: `AGENTS.md`, `OWNER-DIRECTIVES.md`, `scripts/wo_driver.py`, `tests/test_wo_runner.py`
- Current defect: the runner validates `task-start -> candidate`, but builds `candidate.diff` from `frozen-base -> candidate`. Coordinator edits made after task start then appear as unauthorized candidate edits to the independent reviewer.

# Invariants
- Preserve the frozen `base_sha` in work-order metadata and reports.
- Validate ancestry `base_sha -> task-start -> candidate`, and preserve protected-path checks, exclusions, and diff size bounds.
- Without cached task-start metadata, retain the current base-as-start fallback.
- No changes to schemas, frozen work orders, ledgers, or candidate acceptance policy.

# Files allowed to touch
- `scripts/wo_driver.py`
- `tests/test_wo_runner.py`

# Protected paths
- All other paths, including work orders, artifacts, ledgers, manifests, lockfiles, and C/Objective-C references.
- No dependencies, network, installs, GUI actions, resets, or deletions. Candidate commit on the isolated task branch is permitted; do not commit to `main`.

# Required implementation
1. Resolve the validated task-start SHA once through the existing scope check.
2. Use that same `task-start -> candidate` range for changed-path scope and independent reviewer diff; identify this range accurately in the review prompt.
3. Preserve the frozen base as provenance and enforce the existing ancestry, protected-path, exclusion, and size-limit checks.
4. Add an offline public-runner regression where coordinator files change between base and task start, then allowed candidate code changes. Require coordinator changes absent and candidate changes present in saved diff/reviewer input.
5. Preserve no-metadata fallback behavior.

# Acceptance criteria
- AC-1: `base -> task-start -> candidate` ancestry is validated; invalid ancestry fails before reviewer execution.
- AC-2: reviewer diff equals the scope-checked candidate range; coordinator changes before task start are omitted.
- AC-3: allowed candidate changes remain visible and candidate protected-path edits remain rejected.
- AC-4: no-metadata metadata uses `base_sha` as start and retains prior behavior.
- AC-5: `python3 -m unittest tests.test_wo_runner` exits 0.
- AC-6: `git diff --check` exits 0 and changed paths remain in the allowlist.

# Verification commands
```text
python3 -m unittest tests.test_wo_runner
git diff --check
```

# Budget
Executor max runs: 2; verifier: 1; elapsed hard limit: 60 min; output-token soft cap: 10000; dependency/network: none.

# Output
Commit the candidate on its isolated task branch after checks. Coordinator handles shared ledger transitions and integration after independent review.
