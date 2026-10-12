---
id: WO-HYGIENE-01
revision: 1
status: READY
milestone: M0
owner: project-owner
executor: deepseek-flash
base_sha: 5894cb40acfaf0aac10127d82c30fb8be9d38cb4
spec_sha256: 53bfaf036bd67e12b817fd5a915d07e8aea004667464cfae7a466955499a327b
parallel_safe: false
network: false
---
# Goal
Reclaim the 21 GB of stale Git worktrees and make the disconnected second-root branch lineage durable as archive tags, without deleting any commit and without touching any branch that is unmerged on the main lineage.

# Context pointers
- Read: `AGENTS.md`, `OWNER-DIRECTIVES.md`, `docs/adr/0016-product-target-and-gates.md`, `reports/PROJECT-REVIEW-2026-10-12.md` (finding F7).
- The repository has one root, `b90dfbc` (main lineage), and a second root `d97da2f` ("Initial public snapshot"). Eight branch tips share no merge base with `main`: `integrate/final-ui-docs`, `integrate/vexel-main-migration`, `worktree-agent-ac9b3f7832e7ddfc2`, `plan/ci-nightly`, `plan/ui-golden-audit`, `wo/WO-UI-DOCS-01`, `wo/WO-UI-DOCS-02`, `fix/ui-polish-main`.
- `.worktrees/` is gitignored and holds about 97 linked worktrees (21 GB). Other linked worktrees live under `/private/tmp/vexel-baseline-native-shell` and nested inside other worktrees.

# Existing contracts and invariants
- No commit may become unreachable. Every branch ref is preserved unless it is one of the eight second-root tips, and those are preserved by annotated tags before anything else happens.
- Branches that are unmerged on the main lineage (`fix/ui-polish`, `integrate/mcp-next`, `lane/m2-view-anim`, `lane/m3-agent`, `wo/WO-M1-02`, `wo/WO-M1-03-r1`, `wo/WO-M1-04-r1`, and any other `git branch --no-merged main` entry that is not a second-root tip) are **not** deleted and their worktrees are **not** removed.
- A worktree is removed only when it is not the main working tree, its `git status --porcelain` is empty, and its `HEAD` is reachable from `main`, from a preserved branch, or from a newly created archive tag. Nested worktrees are unregistered before their parents.
- Repository history is never rewritten. No `git gc --prune` beyond default behaviour, no `filter-branch`, no rebase.
- Do not touch any directory outside the repository, including sibling project directories.

# Files allowed to touch
- `artifacts/WO-HYGIENE-01/**`
- `STATE.md`
- `LEDGER.md`
- `ledger.jsonl`
- `work/WO-HYGIENE-01.md` (evidence references only; the spec hash must not change)

# Protected paths
- Every tracked file, `src/**`, `app/**`, `tests/**`, `docs/**`, `work/**` other than this order, `OWNER-DIRECTIVES.md`, `AGENTS.md`, and all prior artifacts. Git refs under `refs/heads/main`, `refs/remotes/**` and all tags that already exist are immutable to this order except for the new `archive/**` tags it creates.

# Required implementation
1. Before removing anything, record `git worktree list --porcelain`, `git branch -vv`, `git branch --no-merged main` and, for the eight second-root tips, their full commit SHAs into `artifacts/WO-HYGIENE-01/inventory-before.json`.
2. Create an annotated tag `archive/second-lineage` at `integrate/final-ui-docs` and one annotated tag per second-root tip under `archive/second-lineage/<short-name>`; record every tag and its target SHA in `artifacts/WO-HYGIENE-01/archive-tags.json`. Verify with `git merge-base --is-ancestor <tip> archive/second-lineage/<name>` that each tag resolves to the intended commit.
3. Remove linked worktrees under the rule in "Existing contracts and invariants". For each candidate, record path, branch/detached state, `HEAD` SHA, `git status --porcelain` result, and the reachability justification, then remove it with `git worktree remove` (add `--force` only when the worktree is already missing or the reason is documented). Run `git worktree prune` afterwards.
4. If any worktree cannot be removed because its HEAD is not reachable or its tree is dirty, keep it, leave it registered, and list it in the artifact with the reason. Never force-remove a dirty worktree.
5. Produce `artifacts/WO-HYGIENE-01/inventory-after.json` with the final `git worktree list --porcelain`, the remaining linked worktree count, and the reclaimed size measured with `du -sh .worktrees` before and after.
6. Confirm the repository still builds and tests after the removals: `./dev build`, `./dev t --all`, `./dev verify`.

# Acceptance criteria (machine-checkable)
- AC-1 Every commit that was reachable before the change is still reachable after it (`git rev-list --all --count` before/after recorded; no decrease; all eight second-root tips reachable from the new archive tags).
- AC-2 The number of linked worktrees is reduced substantially and the reduction and reclaimed bytes are recorded in `inventory-after.json`.
- AC-3 `git branch --no-merged main` after the change still lists every non-second-root unmerged branch that it listed before, and every branch that existed before still exists except the eight second-root tips (which remain reachable through tags).
- AC-4 `./dev build`, `./dev t --all` and `./dev verify` exit 0 after the removals.
- AC-5 `git status --porcelain` is empty at the end of the work (apart from this order's own committed evidence), and `git worktree list` reports no broken or missing entries.
- AC-6 `python3 scripts/ledger.py validate ledger.jsonl`, `python3 scripts/wo_validate.py WO-HYGIENE-01 --ready` and `git diff --check` exit 0.
- AC-7 A fresh read-only verifier reports pass against the exact candidate commit and cannot reach a commit that the archive tags fail to preserve.

# Verification commands
Run sequentially from the repository root unless the command changes directory itself.

```text
git worktree list --porcelain
git branch -vv
git branch --no-merged main
git rev-list --all --count
git tag --list 'archive/*'
./dev build
./dev t --all
./dev verify
python3 scripts/ledger.py validate ledger.jsonl
python3 scripts/wo_validate.py WO-HYGIENE-01 --ready
git diff --check
```

# Non-goals
No history rewrite, no repository re-pack beyond default `git worktree prune`, no deletion of unmerged main-lineage branches, no change to `.gitignore`, no change to any source, test, doc or configuration file. This order is filesystem and ref hygiene only.

# Budget
Executor max runs: 2; verifier max rounds: 2; elapsed hard limit: 120 min; dependency/network: none.

# Failure and escalation
If a worktree cannot be classified as safe to remove, keep it and record why. If any archive tag cannot be created, stop before removing any worktree and record the failure; do not proceed with removals that would leave a commit unreachable.

# Output
One implementation commit using `chore(repo): archive the second lineage and prune stale worktrees`. Include the before/after inventories and the reclaimed-size measurement under `artifacts/WO-HYGIENE-01/`, update `STATE.md` and both ledgers, and provide the schema-valid executor report.
