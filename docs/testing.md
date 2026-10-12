# Testing: quick loop, precise failures, final gate

Day to day, use the fast tiers. The expensive checks (e2e, soak, fuzz, native window smokes, sanitizer
suites) belong to the final gate only.

| Tier | Command | Typical time | What it runs |
| --- | --- | --- | --- |
| Inner loop | `./dev t` | 0.3–3 s | Incremental Debug build, then only the CTest and Python tests mapped to files changed vs `main` (plus uncommitted). Slow Python (librsvg comparisons, corpus, e2e) is skipped and listed. |
| Targeted | `./dev t -k PATTERN` | 0.2–14 s | CTest `-R PATTERN` and Python `-k PATTERN`. |
| Wider | `./dev t --full` / `--all` | ~15 s / ~25 s | Adds the slow Python modules / runs every CTest in parallel. |
| Pre-merge | `./dev verify` | ~1 min | Debug + Release build, all CTest, all Python. |
| Final gate | `./dev final` | several minutes | verify + ASan + UBSan + shape/MCP fuzz + e2e + soak + real-window editor and viewer smokes. Run once before merging a milestone. |

The mapping from paths to tests lives in `tests/test_map.json`. A changed source file with no mapping is
reported as `unmapped sources` so the map cannot silently rot. Add a prefix when you add a module.

## Precise failures

* `./dev t` prints the first failing test's full output, not a wall of logs.
* `./dev shot file.svg [--size WxH] [--zoom N] [--out f.png]` renders exactly what the viewer draws and prints
  the PNG path, for looking at results immediately.
* `./dev pixdiff A B [--out diff.png] [--tolerance N]` compares PPM/PNG images: differing pixel count, bounding
  box, max/mean channel delta, the first differing pixels with both colours, and a red-highlight image.
  Exit status 0 means identical within tolerance. Use it for golden failures and before/after UI work.

## Real-corpus render diagnostics

`scripts/corpus_render_report.py` measures a **full-document render** for every file in the pinned
real corpus and records the error category and source offset when the render fails. It exists because
`reports/M4/head-to-head.md` measures a synthetic single-element preview built by
`svg_index_make_preview_source`, which is not file coverage; see
`docs/adr/0016-product-target-and-gates.md`.

```sh
python3 scripts/corpus_render_report.py \
  --corpus /Users/megasoft78/Desktop/Freelance/vexel-corpus-real \
  --output-json reports/M4/render-diagnostics.json \
  --output-markdown reports/M4/render-diagnostics.md
```

* The corpus lives outside the repository and is read-only; nothing is copied in.
* The manifest SHA-256 is pinned in the script. A mismatch aborts with a non-zero exit and writes no
  report, so the tool can never silently describe different inputs.
* Files larger than `--max-bytes` (default 32 MiB) are recorded as `skipped` with a reason, so the
  report always accounts for every manifest entry.
* The JSON output is byte-reproducible: wall-clock timing is excluded by design and only the Markdown
  summary reports elapsed milliseconds.
* `tests/test_corpus_render_report.py` covers the digest guard, diagnostic parsing and verdict
  mapping, size/skip accounting, the missing-source path, and JSON determinism, using an injected
  fake runner so it needs neither the corpus nor a built binary.

## Rules of thumb

* Write the regression test first and confirm it fails before the fix (`./dev t -k name`).
* Do not run e2e, soak, or the native smokes in the inner loop.
* After touching the decoder or rasterizer, run `./dev t --full` once before committing.
