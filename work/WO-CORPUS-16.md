---
id: WO-CORPUS-16
revision: 1
status: READY
milestone: SVG-FIDELITY
owner: product-owner
executor: codex-gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: bf3e1a42b1549178c09fb1ae24af61519f0837bb066f8a3a2ab29b21438de4bc
parallel_safe: false
network: false
---
# Goal
Isolate the high-error endpoint mismatch in the T1 filtered-path fixture among marker rendering, Gaussian blur, and gradient fill semantics using controlled single-feature variants and independent Vexel/Inkscape raster comparisons.

# Context
- WO-CORPUS-15 shows all 69 full-T1 pixels with delta >10 overlap the filtered-path-only mismatch; its 83 high-delta pixels cluster at the source path endpoint near x=600,y=100.
- The local fixture `.cache/WO-CORPUS-13/filtered-path-only.svg` contains one path with a gradient fill, `feGaussianBlur`, and `marker-end`.
- The shape's location suggests the marker may be involved, but does not prove it. No renderer code change is authorized by this diagnostic order.

# Files allowed to touch
- `work/WO-CORPUS-16.md`
- `artifacts/WO-CORPUS-16/**`
- `.cache/WO-CORPUS-16/**`
- `build/mac-release/**` (ignored generated output only)
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`

# Protected paths
- No product, test, corpus, golden, budget, or acceptance-threshold changes.
- Use only the local filtered-path fixture from W13 and derived variants. Do not access external corpus files or sibling directories. Do not read/fetch/copy GPL editor source; use installed Inkscape only as a renderer binary.
- Preserve all unrelated working-tree changes. Leave the repository uncommitted.
- No broad UI/E2E, full corpus sweep, sanitizer, fuzz, or performance suite.

# Required analysis
1. Verify the W13 filtered-path fixture SHA-256 and retain its original bytes unchanged.
2. Create byte-spliced variants that independently remove `marker-end`, remove the filter reference, or replace the gradient fill with the endpoint stop color `#3289a7`; change no other document content. Record hashes and prove each splice changed exactly one target attribute.
3. Build only Release `shape_render_cli` from current source; render all three variants at 1600x1000 with Vexel and installed Inkscape binaries.
4. White-composite outputs using `pixdiff.py`'s floor integer formula; compare each Vexel render to its Inkscape render at tolerance 2. Record full-page metrics and endpoint ROI metrics for x=580..619, y=80..119.
5. Compare changes in endpoint-region high-delta pixels (>10) to baseline filtered-path-only; state whether a feature removal reduces/relocates the hotspot. Avoid claiming causation if interactions remain confounded.
6. Recommend a narrowly scoped renderer implementation order only if a variant isolates the cause; otherwise identify the smallest next experiment.

# Verification
- Validate frozen order/ledger, all input and variant hashes, dimensions, and exact full-page/ROI metrics.
- Confirm generated output stays in ignored `.cache/WO-CORPUS-16/` and `build/mac-release/**`; run `git diff --check`.
- Do not run tests or broad suites.

# Output
Write `artifacts/WO-CORPUS-16/verification.txt` and `variant-analysis.json`, update `STATE.md` and both ledgers, and keep the full product goal active.
