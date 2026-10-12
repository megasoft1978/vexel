---
id: WO-GAP-HYGIENE-01
revision: 1
status: READY
milestone: M0
owner: project-owner
executor: deepseek-flash
base_sha: 27e7781207c746a9012dd4b15d98df0478cd82ef
spec_sha256: 1bda3a742b2919041c127a3a4f2f30d548b17511d319c65aa788a3842ddde52d
parallel_safe: false
network: false
---
# Goal
Close the failed and unverifiable acceptance criteria of `work/WO-HYGIENE-01.md` with evidence: create `artifacts/WO-HYGIENE-01/`, record the inventories, verify (never re-create) the archive refs that already exist, and clear the two prunable worktree entries.

# Context pointers
- Parent spec `work/WO-HYGIENE-01.md`, verdict `artifacts/retro-verification-2026-10-12/WO-HYGIENE-01.verdict.txt`.
- Pruning already happened in `071e0ec`; its records are `artifacts/hygiene-worktrees-before.json` (98 worktrees with `head` SHAs) and `artifacts/hygiene-worktrees-after.json` (23 kept, 7 dirty kept, 21G to 3.3G, unmerged-branch list).
- Archive refs as of `27e7781` (lightweight; present locally and, per `git ls-remote origin` on 2026-10-12, on origin): tag `archive/second-lineage` = `d97da2f4e12e02d482af99c837e12b3234ce9606`; tag `archive/second-lineage-final` = `09c8663fecac15ba5c803e6d2fa1d169bd961aaf`; tag `archive/origin-main-9896eb5` = `9896eb54b8440658dcb3ff88e51da8db3f7da015`; branch `archive/second-lineage-tip` = `09c8663…`, also `refs/remotes/origin/archive/second-lineage-tip`. All eight second-root tips of the parent are ancestors of `09c8663`.
- Parent item 2's per-tip tags `archive/second-lineage/<name>` cannot coexist with the existing tag `archive/second-lineage` (Git ref directory/file conflict); preservation is through `archive/second-lineage-final` and `archive/second-lineage-tip`.
- Prunable entries at `27e7781`: `/private/tmp/vexel-baseline-native-shell` (HEAD `f3e28cd`) and `.worktrees/WO-M4-06/.cache/M4-06/frozen-base` (HEAD `81f2008`).

# Existing contracts and invariants
- No commit becomes unreachable; no ref is created, moved, deleted or pushed; no history rewrite; no `gc --prune`. Dirty worktrees are never removed. No directory outside the repository is touched (`git worktree prune` edits only `.git/worktrees/`).

# Files allowed to touch
- `artifacts/WO-HYGIENE-01/**`, `artifacts/WO-GAP-HYGIENE-01/**`, `STATE.md`, `LEDGER.md`, `ledger.jsonl`; Git admin data only through `git worktree prune`.

# Protected paths
- Every source, test, script, doc and `work/**` file, `OWNER-DIRECTIVES.md`, `AGENTS.md`, all other prior artifacts, all refs.

# Required implementation
1. `artifacts/WO-HYGIENE-01/inventory-before.json`: derived from `artifacts/hygiene-worktrees-before.json` plus the eight tip SHAs, marked `"reconstructed": true`, stating that the pre-pruning `git branch -vv` and `git rev-list --all --count` were not recorded and cannot be recovered.
2. `artifacts/WO-HYGIENE-01/archive-tags.json`: for each of the four archive refs: name, kind (tag/branch), object type (lightweight or annotated, as found), target SHA; per tip: name, SHA, and the `git merge-base --is-ancestor` result against `archive/second-lineage-final` and `archive/second-lineage-tip`; plus the deviation note on per-tip tags from Context.
3. Verify each prunable entry's HEAD is reachable (`git for-each-ref --contains <sha>` nonempty), record it, then run `git worktree prune`.
4. `artifacts/WO-HYGIENE-01/inventory-after.json`: `git worktree list --porcelain`, linked count, `du -sh .worktrees`, `git rev-list --all --count`, `git branch --no-merged main`, the reduction from 98, and the reachability result for every `head` in the before record.
5. Record each verification command, its exit code and the last 40 output lines in `artifacts/WO-GAP-HYGIENE-01/check-NN.txt`.

# Acceptance criteria
- AC-1 Every `head` in `hygiene-worktrees-before.json` and every one of the eight tips is reachable from a current ref, recorded in `inventory-after.json`.
- AC-2 `inventory-after.json` records the linked-worktree count, the reduction from 98 and the reclaimed size.
- AC-3 Every branch in the `unmerged_branches` list of `hygiene-worktrees-after.json` still exists.
- AC-4 `./dev build`, `./dev t --all` and `./dev verify` exit 0, with output recorded.
- AC-5 `git status --porcelain` is empty after the commit and `git worktree list --porcelain` contains no `prunable` line.
- AC-6 The four archive refs resolve to the SHAs listed in Context and all eight tips are ancestors of both `09c8663` refs, as recorded in `archive-tags.json`.
- AC-7 `python3 scripts/ledger.py validate ledger.jsonl`, `python3 scripts/wo_validate.py WO-HYGIENE-01 --ready`, `python3 scripts/wo_validate.py WO-GAP-HYGIENE-01 --ready` and `git diff --check` exit 0.
- AC-8 A fresh read-only verifier re-judges AC-1..AC-7 of `work/WO-HYGIENE-01.md` (AC-1 and AC-7 in the forms of AC-1 and AC-6 here) and AC-1..AC-7 here, and reports pass in `artifacts/WO-GAP-HYGIENE-01/verdict.txt`.

# Verification commands
```text
git worktree list --porcelain
git for-each-ref --contains f3e28cdc2f465cbab3ec0ecc55490a44cd7800c0 --format='%(refname)'
git for-each-ref --contains 81f2008311e886d3b8dc8da262fe3f40f2f9b14b --format='%(refname)'
git worktree prune
git tag --list 'archive/*' --format='%(refname) %(objecttype) %(*objectname) %(objectname)'
git for-each-ref refs/heads/archive refs/remotes/origin/archive
git branch --no-merged main
git rev-list --all --count
du -sh .worktrees
./dev build
./dev t --all
./dev verify
python3 scripts/ledger.py validate ledger.jsonl
python3 scripts/wo_validate.py WO-HYGIENE-01 --ready
python3 scripts/wo_validate.py WO-GAP-HYGIENE-01 --ready
git diff --check
git status --porcelain
```

# Non-goals
No new or converted archive tags, no push, no worktree removal beyond `git worktree prune`, no branch deletion. Origin re-verification (`git ls-remote origin 'refs/tags/archive/*' 'refs/heads/archive/*'`) needs network and is owner-run.

# Budget
Executor max runs: 2; verifier max rounds: 2; elapsed hard limit: 90 min; dependency/network: none.

# Failure and escalation
If a recorded `head` or tip is unreachable, or a prunable entry's HEAD has no containing ref, stop before pruning and report the SHA; do not create a ref.

# Output
One commit `chore(evidence): record WO-HYGIENE-01 inventories and archive refs`, evidence under `artifacts/WO-HYGIENE-01/` and `artifacts/WO-GAP-HYGIENE-01/`, updated `STATE.md` and both ledgers, and the schema-valid executor report.
