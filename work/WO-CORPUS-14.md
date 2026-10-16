---
id: WO-CORPUS-14
revision: 1
status: READY
milestone: SVG-FIDELITY
owner: product-owner
executor: codex-gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 617e04924c209fe9786dac20a47f3e3e843a4ae19b5def70d684b91fa6cddcd7
parallel_safe: false
network: false
---
# Goal
Localize and characterize the residual tolerance-2 mismatch in the already rendered native-size T1 candidate after WO-SVG-PAINT-02, so the next renderer change targets an evidenced region or sample class.

# Context
- The current candidate reports 12,526 / 1,600,000 differing pixels (0.783%), max channel delta 149; it is not accepted parity.
- The required Vexel and Inkscape PNG outputs are already in ignored `.cache/WO-SVG-PAINT-02/` and `.cache/WO-CORPUS-12/` respectively, with hashes recorded in their accepted verification artifacts.
- Analyze only these existing outputs. Do not read or modify external corpus sources or sibling directories.

# Files allowed to touch
- `work/WO-CORPUS-14.md`
- `artifacts/WO-CORPUS-14/**`
- `.cache/WO-CORPUS-14/**`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`

# Protected paths
- No product, test, corpus, golden, build, budget, or acceptance-threshold changes.
- Do not access external corpus files or sibling directories. Do not read/fetch/copy GPL editor source.
- Preserve all unrelated working-tree changes. Leave the repository uncommitted.
- No broad UI/E2E, full corpus sweep, sanitizer, fuzz, or performance suite.

# Required analysis
1. Verify the two local PNG dimensions and SHA-256 values against WO-SVG-PAINT-02 and WO-CORPUS-12 evidence.
2. Re-run `scripts/pixdiff.py` at tolerance 2 to reproduce the residual mismatch and retain a derived heatmap in `.cache/WO-CORPUS-14/`.
3. Produce deterministic per-row, per-column, and 100x100 tile counts for differing pixels, plus counts and max/mean channel delta grouped by Vexel alpha bands (0, 1..254, 255). Use the same white-composite integer rounding as `pixdiff.py`.
4. Record the highest-density tiles and representative pixel pairs. Interpret only spatial/alpha evidence; do not assert an SVG feature cause without source evidence.
5. Recommend the smallest next diagnostic or fix order, naming what evidence is still needed.

# Verification
- Validate this frozen work order and ledger; confirm source PNG hashes/dimensions and exact reproduction of the 12,526-pixel result.
- Confirm derived files remain under ignored `.cache/WO-CORPUS-14/`; run `git diff --check`.
- Do not run tests or builds.

# Output
Write `artifacts/WO-CORPUS-14/verification.txt` and `residual-analysis.json`, update `STATE.md` and both ledgers, and keep the full product goal active.
