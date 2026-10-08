# 09 — First 30 days

Assumption: one experienced C/graphics developer, working five days/week; LLM assistance drafts isolated code/docs, while the developer owns integration and correctness. User interviews and competitor builds run in parallel. Do not start the full editor before M0 evidence.

## Week 1 — Evidence and harness

- **Day 1 / M0-T01:** list candidate corpora, legal source, hash and feature tags; ask 5 contacts for anonymized files.
- **Day 2 / M0-T03:** install/build current Inkscape stable and current 1.5 development snapshot; capture exact versions.
- **Day 3:** build current Donner release and main if feasible; record compiler/build deps, license, run its editor on same sample.
- **Day 4 / M0-T02:** generate SVG fixtures with controlled nodes, elements, base64 image bytes, deep groups, filters.
- **Day 5 / M0-T05:** add manifest schema and one-run benchmark skeleton; publish what is and is not measurable.

## Week 2 — Compare workflows

- **Day 6–7 / M0-T04:** script open, first visible pixel, pan, zoom, select/deselect, save, RSS; repeat runs.
- **Day 8:** build 1 MB/10 MB/100 MB/500 MB/1 GB families; separate bytes from object count and path nodes.
- **Day 9–10 / M0-T06:** interview 5–8 GIS, scientific and fabrication users; obtain real wait-time tasks and pilot commitment.

## Week 3 — C parser/render prototype

- **Day 11:** initialize CMake C17 app, strict warnings, one test executable, CI compile.
- **Day 12:** `hello_rect.svg` tokenizer reads root/rect and source offsets; no tree allocations per attribute.
- **Day 13:** scalar bounds/transform for rect; CPU 256×256 RGBA buffer and deterministic test PNG.
- **Day 14:** macOS NSWindow + CAMetalLayer blits CPU buffer, with event loop and resize.
- **Day 15:** add visible first-pixel progress and cancel button; intentionally do not implement editing.

## Week 4 — Hard gate

- **Day 16–17:** add safe parse limits/fuzz harness and source-span no-op output test.
- **Day 18:** compare SVG `<rect>` and representative path render against resvg/Inkscape; mark gaps.
- **Day 19:** rerun exact benchmark and profile CPU/RSS/allocs. Evaluate PlutoVG as a replaceable backend, no commitment yet.
- **Day 20:** interview remaining users; write M0 report, decide whether to spend M1–M3 time.

## Prototype definition: “hello rect”

Input `tests/corpus/hello_rect.svg`:

```xml
<svg xmlns="http://www.w3.org/2000/svg" width="320" height="200" viewBox="0 0 320 200">
  <rect x="24" y="18" width="272" height="164" rx="12" fill="#2b77c4"/>
</svg>
```

Acceptance: on macOS arm64, open creates a resizable native window; C17 tokenizer validates the root and rect without loading external resources; scalar CPU rasterizer draws a deterministic rounded blue rect; Metal presents it via a plain texture blit; resize redraws; keyboard shortcut toggles checkerboard; quit joins all work cleanly. `hello_rect` test verifies dimensions, pixel at `(30,30)`, transparent/outside pixel, and exact output hash in scalar build. Prototype proves platform path and API shape only, not speed or product viability.

## M0 viewer acceptance script

Save as `bench/scripts/m0_acceptance.sh` once CLI commands are implemented; shell is illustrative until then.

```sh
set -eu
./build/bin/svgbench --manifest bench/corpus/m0.json --competitors inkscape,donner \
  --cold-runs 5 --warm-runs 10 --json out3/m0-results.json
./build/bin/svg_open_probe --input bench/corpus/fixtures/10mb-simple.svg \
  --first-pixel --cancel-after-ms 200 --json out3/open-simple.json
./build/bin/svg_open_probe --input bench/corpus/fixtures/100mb-map.svg \
  --first-pixel --interactive-pan --cancel-parse --json out3/open-map.json
./build/bin/svg_roundtrip --input bench/corpus/fixtures/unknown-namespaces.svg \
  --no-edit --assert-byte-identical
./build/bin/svg_open_probe --input bench/corpus/fixtures/1gb-stress.svg \
  --max-rss-mib 2048 --timeout-sec 180 --expect-progress-or-limit --json out3/open-1gb.json
```

M0 passes only if (1) source files and hashes are legal and pinned; (2) at least 20 fixtures plus size/object/path/image stress families; (3) same Mac and corpus used for Inkscape stable/dev and Donner where buildable; (4) run variance <10% or reported as noisy; (5) no-op source bytes preserved; (6) large-file run shows progress/cancel or safe explicit limit, never hang/crash; (7) first-pixel/latency/RSS are captured, with exact software/hardware; (8) at least 5/15 target users agree to pilot/provide real tasks. A fail is useful: no feature coding commitment.

## First 10 commit messages

1. `chore: scaffold C17 project and strict compiler warnings`
2. `bench: add corpus manifest and fixture hash verifier`
3. `bench: add deterministic SVG stress fixture generator`
4. `bench: record Inkscape and Donner workflow timings`
5. `base: add checked size arithmetic and byte slices`
6. `xml: add bounded scalar tokenizer with source spans`
7. `test: add sanitizer-ready parser corpus runner`
8. `raster: draw deterministic CPU rounded rectangle fixture`
9. `platform: add macOS window and Metal texture blit shim`
10. `docs: publish M0 results and product continuation gate`

No commit message should claim speed or compatibility before the measured artifact exists.
