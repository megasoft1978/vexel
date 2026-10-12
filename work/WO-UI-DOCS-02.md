---
id: WO-UI-DOCS-02
revision: 1
status: READY
milestone: M7
owner: project-owner
executor: codex-gpt-6.1-sol
base_sha: 51f7198f3735c1f7a3dc2195ef4062e620767c56
spec_sha256: 79d6edde75d76ef86551825f23d0d0fb6b078eb43c5a518722a352702287e868
parallel_safe: false
network: false
---
# Goal
Correct the clipped Fit and Loop timeline glyphs identified by the independent review of WO-UI-DOCS-01 r3.

# Context pointers
- Read `AGENTS.md`, `OWNER-DIRECTIVES.md`, `STATE.md`, `docs/CODING_STANDARD.md`, `docs/testing.md`, `work/WO-UI-DOCS-01.md`, and `artifacts/WO-UI-DOCS-01/verification-summary.md`.
- Review the failed Sol 6.1 finding: `src/app/viewer_timeline_ui.c:447,449` calls the shared fixed 24-point atlas painter with 14/16-point glyph bounds. Do not call it at undersized bounds.
- Pinned starting commit: `51f7198f3735c1f7a3dc2195ef4062e620767c56` on `wo/WO-UI-DOCS-02`.

# Existing contracts and invariants
- C17, macOS 13+ Apple Silicon, no dependencies or platform expansion.
- WO-UI-DOCS-01 r3 is committed and immutable; its remaining overall review status is failed until this focused correction passes fresh review.
- Do not alter shared icon sources, unrelated baseline families, test thresholds, assertions, or protected native-shell baselines.
- Preserve the existing timeline control hit targets, colors, tooltip/accessibility labels, backing-scale behavior, and all previously passing timeline behavior.

# Files allowed to touch
- `work/WO-UI-DOCS-02.md` (freeze only)
- `src/app/viewer_timeline_ui.c`
- `tests/app/viewer_timeline_test.c`
- `tests/ui/viewer_timeline_render.c`
- `tests/ui/timeline-golden/**` (only directly affected timeline images and manifest entries)
- `artifacts/WO-UI-DOCS-02/**`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`

# Protected paths
- All paths outside the allowlist, including `src/ui/ui_icons.c`, shared widget/icon files, docs/README, artwork assets, prior WO-UI-DOCS-01 evidence, native-shell baselines, unrelated UI goldens, and all frozen tests/oracles.

# Required implementation
1. Draw complete Fit and Loop glyphs locally within the available 14/16 logical point geometry, or otherwise use a proven size-aware painter without changing protected shared sources. Ensure no clipping and at least six logical points of inset where button geometry permits.
2. Add a regression that verifies both glyphs render completely and distinctly at 1× and 2×. Preserve action semantics and accessible names.
3. Regenerate only directly affected timeline goldens. Preserve their prior committed images in `artifacts/WO-UI-DOCS-02/baseline-before/`, record hashes/bounds, and create a focused comparison sheet. Do not change thresholds or assertions to accept defects.
4. Include the reviewer finding/verdict in `artifacts/WO-UI-DOCS-02/` and update STATE/LEDGER honestly. Do not claim WO-UI-DOCS-01 accepted until a fresh independent review passes.

# Acceptance criteria
- AC-1: `./dev build` exits 0.
- AC-2: Focused timeline tests and `viewer-timeline-goldens` pass; retained output proves Fit/Loop complete at 1×/2× and other control labels/semantics remain correct.
- AC-3: Before/after hashes, changed bounds, and comparison sheet are present; no protected files/baselines or thresholds change.
- AC-4: `./dev t --all` is recorded exactly. Only the documented pre-existing `native-shell-goldens` mismatch may remain; candidate/pristine evidence in WO-UI-DOCS-01 must remain byte-identical and the protected fixture untouched. Do not report full-suite pass if this blocker remains.
- AC-5: `python3 scripts/ledger.py validate ledger.jsonl` and `git diff --check` exit 0; changed paths stay within scope.
- AC-6: Fresh read-only independent review passes the exact candidate with no unresolved high/critical findings before any push or merge.

# Verification sequence
1. Confirm clean worktree at pinned base and validate this frozen spec.
2. Establish the clipped-glyph regression against the base rendering.
3. Implement local glyph geometry; run focused tests/build and headless render comparisons.
4. Run `./dev t --all`, ledger validation, and diff check; retain exact evidence.
5. Obtain a fresh independent read-only review. Correct findings only within this scope and repeat relevant gates.
6. Commit and push the correction branch only after independent pass. Integration remains governed by repository merge checks and the explicit unresolved pre-existing native-shell gate.

# Non-goals
Shared icon atlas changes, compact icons elsewhere, layer-row rendering, MCP changes, additional docs or artwork, renderer changes, and fixing the pre-existing native-shell golden.

# Budget
One implementation round and one fresh independent review; 60-minute hard limit; no dependencies or network access.

# Failure and escalation
If complete glyphs cannot fit the existing controls without touching protected shared files or unrelated baseline families, preserve the failed regression and stop with the smallest required scope expansion. Never bypass the independent review or branch protections.

# Output
Commit convention: `fix(ui): render timeline fit and loop glyphs without clipping`. Report exact checks, changed paths, before/after evidence, fresh review verdict, commit/push state, and remaining known native-shell blocker.
