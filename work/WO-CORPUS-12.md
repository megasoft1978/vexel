---
id: WO-CORPUS-12
revision: 1
status: READY
milestone: SVG-FIDELITY
owner: product-owner
executor: codex-gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
parallel_safe: false
network: false
spec_sha256: d29d6b6fe9ebb07648e5e78bfb54b8ec00f4879223cfa1e83485b6fa72ff4e6f
---
# Goal
Compare candidate Vexel's first successful full render of manifested generated
T1 against the installed Inkscape renderer at the SVG's native 1600x1000 size.

# Context
- WO-CORPUS-11 renders T1 to a 128x128 PPM but does not prove visual parity.
- The exact external target is 979,796 bytes, SHA-256
  3578c89a2a55b487de739846aeac4fe84ba3bf0d71b2b03387afe3c822cf33b5.
- `/opt/homebrew/bin/inkscape` is installed as a command-line oracle. Execute the
  binary only; do not read, fetch, or copy GPL editor source.

# Files allowed to touch
- `work/WO-CORPUS-12.md`
- `artifacts/WO-CORPUS-12/**`
- `.cache/WO-CORPUS-12/**`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`

# Protected paths
- No production source, test/oracle source, corpus, docs outside this order,
  dependencies, platform behavior, or product limits may change.
- Keep generated renders and conversion files in ignored `.cache/WO-CORPUS-12/`.
- No UI/E2E, full corpus, fuzz, sanitizer, or broad benchmark.
- Do not infer broad corpus parity from one document.

# Required implementation
1. Verify T1 hash/size and capture the installed Inkscape version.
2. Build only Release `shape_render_cli` and render exact T1 at native dimensions
   to RGBA; use the installed Inkscape CLI to export the page at 1600x1000 PNG
   with transparent background.
3. Convert Vexel RGBA to PPM by compositing over white, matching the pixdiff
   helper's PNG alpha handling.
4. Compare full images with `scripts/pixdiff.py` at channel tolerance 0 and 2;
   retain the tolerance-2 difference highlight and record dimensions, differing
   pixels, bbox, max and mean delta.
5. Do not adjust renderer semantics or acceptance thresholds in this order.

# Verification
- Confirm both outputs are 1600x1000 and reference alpha handling is explicit.
- Record exact commands, app/reference versions, hashes, comparison output, and
  resource/time results if available.
- Validate frozen order and ledger; run `git diff --check`.

# Output
Write `artifacts/WO-CORPUS-12/verification.txt`; update STATE/LEDGER. Keep the
full product objective active and leave changes uncommitted.
