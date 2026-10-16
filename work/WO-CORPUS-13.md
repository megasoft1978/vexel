---
id: WO-CORPUS-13
revision: 1
status: READY
milestone: SVG-FIDELITY
owner: product-owner
executor: codex-gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
parallel_safe: false
network: false
spec_sha256: d7b43b491438ecc5370653e74c567103954b0c87699d6141c9eec75e11238365
---
# Goal
Use byte-preserving ablation copies of generated T1 to identify whether its
large pixel mismatch is dominated by the filtered gradient shape or the
34,714-path gradient-stroke group.

# Context
- WO-CORPUS-12 measured 12.099% pixels above channel tolerance 2 versus
  installed Inkscape 1.4.4 at 1600x1000.
- The source has one leading path using gradient fill, Gaussian blur filter, and
  end marker, followed by a large group of simple paths using gradient stroke.
- These are diagnostic derived inputs only; original corpus bytes remain fixed.

# Files allowed to touch
- `work/WO-CORPUS-13.md`
- `artifacts/WO-CORPUS-13/**`
- `.cache/WO-CORPUS-13/**`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`

# Protected paths
- No product/test/oracle source changes; no edits to original corpus inputs.
- All derived SVGs, images and output remain in ignored `.cache/WO-CORPUS-13/`.
- Execute installed Inkscape only as a binary; no GPL source access.
- No broad UI/E2E, corpus sweep, fuzz, sanitizer or benchmark suite.

# Required implementation
1. Verify original T1 identity against WO-CORPUS-12.
2. Create two byte-derived, temporary documents: (A) retain the leading filtered
   gradient path but remove the large path group; (B) remove the leading path
   but retain the group. Keep definitions and document dimensions unchanged.
3. Render both variants with Vexel `shape_render_cli` and installed Inkscape at
   native 1600x1000 with transparent output.
4. Composite over white and compare with `scripts/pixdiff.py` at tolerance 2.
   Record variant hashes and exact difference metrics.
5. Change no code or acceptance thresholds; recommend the next focused feature.

# Verification
- Confirm the original input hash, both derived variant identities and all four
  render dimensions. Record Inkscape version and any warnings.
- Validate frozen order/ledger and run `git diff --check`.

# Output
Write `artifacts/WO-CORPUS-13/verification.txt`, update state and ledgers, and
keep the full product goal active.
