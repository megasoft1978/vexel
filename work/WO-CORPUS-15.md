---
id: WO-CORPUS-15
revision: 1
status: READY
milestone: SVG-FIDELITY
owner: product-owner
executor: codex-gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: f1a8ce908adcf4bbad4f218688f4af77d643071db6fc135420c6a1898b2e94cf
parallel_safe: false
network: false
---
# Goal
Determine whether the residual T1 candidate mismatch components observed in WO-CORPUS-14 follow the filtered gradient path or the inherited gradient-stroke group by rerendering the two existing ablation inputs with the current renderer.

# Context
- WO-CORPUS-13 created two byte-derived SVG variants and rendered them with the previous renderer: filtered path only (A), and the 34,714-path gradient-stroke group only (B).
- WO-SVG-PAINT-02 changed per-shape antialias coverage compositing and improved full T1 from 12.099% to 0.783% differing pixels.
- WO-CORPUS-14 found a localized high-delta hotspot at x596..598/y100..102 and a distributed low-delta residual. This order tests whether those components separate across A/B.
- Use only `.cache/WO-CORPUS-13/{filtered-path-only.svg,gradient-stroke-group-only.svg,a-inkscape.png,b-inkscape.png}` and the repository renderer. Do not access external corpus sources or sibling directories.

# Files allowed to touch
- `work/WO-CORPUS-15.md`
- `artifacts/WO-CORPUS-15/**`
- `.cache/WO-CORPUS-15/**`
- `build/mac-release/**` (ignored generated output only)
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`

# Protected paths
- No product, test, corpus, golden, budget, or acceptance-threshold changes.
- Do not access external corpus files or sibling directories. Do not read/fetch/copy GPL editor source.
- Preserve all unrelated working-tree changes. Leave the repository uncommitted.
- No broad UI/E2E, full corpus sweep, sanitizer, fuzz, or performance suite.

# Required analysis
1. Verify the two local SVG variant hashes and the saved Inkscape reference hashes against WO-CORPUS-13 evidence.
2. Build only Release `shape_render_cli` from the current working-tree source.
3. Render variants A and B with the current Release renderer at their native 1600x1000 dimensions. Keep RGBA and white-composited PPM outputs under `.cache/WO-CORPUS-15/`.
4. Compare each current Vexel output to its saved Inkscape reference with `scripts/pixdiff.py --tolerance 2`; retain heatmaps and exact metrics.
5. Compare the mismatch localization against W14's full-T1 residual and W13's pre-fix variant results. State what the experiment establishes and what remains uncertain; do not tune renderer behavior or thresholds.

# Verification
- Validate frozen order/ledger, input hashes, output dimensions, and both exact pixel comparison summaries.
- Confirm generated output is limited to ignored `.cache/WO-CORPUS-15/` and `build/mac-release/**`; run `git diff --check`.
- Do not run tests or broad suites.

# Output
Write `artifacts/WO-CORPUS-15/verification.txt`, update `STATE.md` and both ledgers, and keep the full product goal active.
