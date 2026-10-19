---
id: WO-RUST-MIGRATION-INVENTORY-01
revision: 2
status: READY
milestone: FULL-RUST-MIGRATION
owner: project-coordinator
executor: codex-gpt-6-luna
base_sha: c61b192e4c87d15264aae3efe809612d1441da54
spec_sha256: f64ed82c476925d754854720ebdcdd8aff0b7eb853eecd29e5672c320118cf42
parallel_safe: true
network: false
---
# Goal
Produce a complete, evidence-linked accounting of the 603-file legacy code baseline and current first-party additions without changing source files.

# Context pointers
- Read: `OWNER-DIRECTIVES.md`, `docs/product/rust-port-goal-and-plan.md`, `docs/adr/0022-full-rust-port.md`
- Baseline reference: git tag `legacy-c-final`; canonical baseline count is 603 code files / 137,570 lines (five `.inc` files are included).
- Acceptance sources: current work-order ledger and accepted per-module parity maps/evidence. Work-order READY headers alone are not acceptance evidence.
- Do not fetch, copy or inspect GPL SVG-editor source. Do not modify legacy sources or the tag.

# Existing contracts and invariants
- Preserve all existing source, migration references, evidence and frozen acceptance data.
- Every baseline file and current first-party addition must receive exactly one provisional disposition: accepted Rust replacement (with accepted WO and evidence), pending port, historical data (with justification), or unresolved.
- Do not claim migration closure from filename similarity, crate counts, tests alone, or old C implementation removal. Record unknown mappings honestly.
- Treat Rust and Python first-party tooling as in-scope; historical tooling is not a completed Rust replacement.

# Files allowed to touch
- `artifacts/WO-RUST-MIGRATION-INVENTORY-01/**`

# Protected paths
- All source, tests, fixtures, docs, work orders, manifests/lockfiles, shared ledger/status, git HEAD/index, tags, and C/Objective-C migration references.
- No dependencies, network, installs, GUI, commits, staging, resets or deletions.

# Required implementation
1. Enumerate the exact 603 baseline code paths from `legacy-c-final`, including the five `.inc` files; record each baseline blob id and one disposition.
2. Map each claimed accepted Rust replacement to concrete Rust path(s), accepted work-order ID(s), and immutable evidence path(s). If any link is missing or ambiguous, use `unresolved` or `pending_port`.
3. Enumerate current first-party application, platform, UI, parser, raster, document, animation, MCP, development, test, benchmark and release tooling additions not covered by the baseline. Identify retained Python scripts and test tooling explicitly.
4. Record shader/assets and any retained native/FFI source separately so none disappear from the accounting.
5. Emit `baseline.jsonl`, `current-additions.jsonl`, `summary.json`, and a concise `result.md` under the allowed artifact directory. Include methods, commands, commit/tag hashes, counts and limitations.

# Acceptance criteria (machine-checkable)
- AC-1: Baseline output contains exactly 603 unique paths, exactly one disposition per row, and includes all five `.inc` files.
- AC-2: `summary.json` counts equal the actual row counts and do not mark unresolved/pending entries as complete.
- AC-3: Every `accepted_rust_replacement` row names at least one existing accepted/merged ledger work order, existing Rust path, and existing evidence artifact; all references are validated. No claim is inferred from READY headers.
- AC-4: Current additions include all current first-party code paths outside the baseline that fall within the canonical scope, including Python tooling and native FFI/shaders, or explain each excluded path.
- AC-5: No source, test, document, manifest, ledger, tag, or git index content is changed; only the allowed artifact directory is added.
- AC-6: `python3 scripts/ledger.py validate ledger.jsonl` exits 0 and `git diff --check` exits 0.

# Verification commands
```text
python3 scripts/ledger.py validate ledger.jsonl
git diff --check
```

Also use `git ls-tree` against `legacy-c-final` to verify baseline count/blob IDs. Recompute row counts, uniqueness, disposition values and referenced path existence; record exact outputs in `result.md`. Audit that only artifact paths changed.

# Non-goals
Porting code, deleting or rewriting legacy references, making completion claims, changing acceptance gates, or resolving every unknown mapping by guesswork.

# Budget
Executor max runs: 1; verifier: 1; elapsed hard limit: 90 min; output-token soft cap: 12000; dependency/network: none.

# Failure and escalation
If the legacy tag is absent, its exact baseline cannot be enumerated, or evidence cannot support per-file mappings, stop and report that exact gap. Do not substitute another baseline or mark uncertain files complete.

# Output
Evidence-only candidate; independent reviewer must validate all counts and citations. Do not update shared status/ledgers or commit to `main` from the task worktree. Executor returns the required JSON report and `result.md`.
