---
id: WO-RETRO-01
revision: 1
status: READY
milestone: M5
owner: project-owner
executor: deepseek-flash
base_sha: e31084f396d4d6ddabcfffb99f346d741f3a8349
spec_sha256: 38513a9f87c105b2f949ad9a1f6f144826c28e6906c870ac10a7694541c39049
parallel_safe: false
network: false
---
# Goal
Describe, after the fact, the work shipped in commits `071e0ec`, `f68de49`, `77e808b`, `3f8ad16`, `2f75c19`, `b61e6e9` and `bcdc3fa` that no earlier work order governed, so the ledger matches what exists. This order is descriptive: it records what is on `main`, not a plan.

# Context pointers
- `git log e31084f..bcdc3fa`, `HANDOFF.md`, `docs/ui/blender-alignment-review.md`, `.cache/final-gate-2026-10-12.log`.

# Existing contracts and invariants
- macOS-only, C17, custom-drawn UI, no new dependency, source files under 800 lines; no frozen oracle or acceptance test weakened.

# Files allowed to touch
- Read-only: this order describes already-merged changes. Only `work/WO-RETRO-01.md`, `ledger.jsonl`, `artifacts/retro-verification-2026-10-12/**`.

# Required implementation
Already shipped, to be verified as present:
1. Blender-alignment items: focus mode, density, tool options strip, Adjust Last Operation popover, numeric scrub, label-collision and Escape-ordering fixes.
2. Shape budgets: edges 4M, per-path commands 4M, elements 65,536, document text 64 MiB (64 KiB per token), glyphs 1M, transform operations 1M; 1 GiB source cap with streamed tokens.
3. Release evidence: `scripts/make_sbom.py`, `scripts/install_smoke.py` and their CTest registrations.
4. Worktree hygiene recorded in `artifacts/hygiene-worktrees-{before,after}.json`.
5. Gate repairs in `bcdc3fa`: label length over-reads fixed, `tests/test_literal_lengths.py` lint, ubsan link options for `viewer_workspace_test`, fuzz probe bounds raised to 32 MiB.

# Acceptance criteria
- AC-1 `./dev final` exits 0 at the candidate commit (log `.cache/final-gate-2026-10-12.log`).
- AC-2 Each numbered item above is present in the source at the candidate commit.
- AC-3 `tests/test_literal_lengths.py` fails on the pre-`bcdc3fa` label code and passes after.
- AC-4 `python3 scripts/ledger.py validate ledger.jsonl` and `git diff --check` exit 0.
- AC-5 A fresh read-only verifier reports pass.

# Verification commands
```text
./dev final
python3 scripts/ledger.py validate ledger.jsonl
git diff --check
```

# Non-goals
No new behaviour. Gaps against earlier orders (UI-01..03, BUG-01, M4-10, HYGIENE-01) are recorded in their own verdicts, not here.

# Budget
Verifier max rounds: 1; dependency/network: none.

# Failure and escalation
If a numbered item is absent, record it as a gap and block this order.

# Output
Ledger events and the verifier verdict under `artifacts/retro-verification-2026-10-12/`.
