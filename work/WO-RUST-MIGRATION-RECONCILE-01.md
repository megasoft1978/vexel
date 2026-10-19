---
id: WO-RUST-MIGRATION-RECONCILE-01
revision: 1
status: READY
milestone: FULL-RUST-PROJECT
owner: project-coordinator
executor: codex-gpt-6-luna
base_sha: 849371835fa4cb8c3e48ff154778a3ec5d7de196
spec_sha256: 47f3db906c6370350bf996e7b55fd4fa94b22beb3dced02e2212e8c865c013d0
parallel_safe: true
network: false
---
# Goal
Produce a source-backed reconciliation candidate for the frozen Rust migration inventory. For each legacy baseline path and current code/tooling addition, identify accepted Rust replacement evidence, confirmed remaining port work, justified historical/reference data, or unresolved status. Do not treat an inventory row or similar filename as proof of parity.

# Context and locks
- Follow `OWNER-DIRECTIVES.md`: preserve legacy C/Objective-C, unfinished candidates and evidence as migration references; no delete/reset or false completion claims.
- Read `artifacts/WO-RUST-MIGRATION-INVENTORY-01/result.md`, `baseline.jsonl`, `current-additions.jsonl`, and `summary.json` as immutable inputs.
- Read the canonical Rust goal and current accepted work-order/result evidence. Cite exact paths and work-order IDs for every proposed replacement.
- Do not modify the source tree, baseline inventory, ledger, manifests, dependencies, or current implementation work.
- Do not run tests, builds, benchmarks, GUI checks, or install analysis tools. This is evidence mapping only.

# Allowed paths
- `work/WO-RUST-MIGRATION-RECONCILE-01.md`
- `ledger.jsonl` only by coordinator
- `artifacts/WO-RUST-MIGRATION-RECONCILE-01/**`

# Protected paths
All source trees (including the preserved C/Objective-C reference), original inventory artifacts, tests/oracles, manifests/locks, workflows, ADRs, unrelated work orders/evidence, `OWNER-DIRECTIVES.md`, external apps, and non-macOS code.

# Acceptance
- AC-1: every input row remains accounted for in the candidate report; no path is silently dropped.
- AC-2: proposed Rust replacements cite an accepted work order/result and corresponding Rust path(s); uncertain cases remain unresolved or pending.
- AC-3: reference/history dispositions explain why the path remains preserved and what evidence would allow closure.
- AC-4: all legacy source and tooling remain untouched; no migration-complete claim is made by this candidate.
- AC-5: verifier/implementation owners receive specific, disjoint gap groups suitable for future frozen work orders.

# Deferred verification
No product tests run for this evidence-only slice. Coordinator will validate report schema/path coverage and reconcile it against owner directives before any inventory disposition is accepted.
