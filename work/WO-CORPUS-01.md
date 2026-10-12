---
id: WO-CORPUS-01
revision: 1
status: READY
milestone: M4
owner: project-owner
executor: deepseek-flash
base_sha: 27e7781207c746a9012dd4b15d98df0478cd82ef
spec_sha256: 43acd9011fb3c91c6d9d2c9459e97a1b7fa421d4070e8d72c2d7f65c620fa64b
parallel_safe: false
network: false
---
# Goal
Replace fail-closed handling with ignore-and-diagnose (ADR-0016 decision 5) for four constructs the SVG specification treats as non-fatal: unknown non-namespaced attributes, `<meta>`, `<!DOCTYPE>` with or without an internal DTD subset, and `writing-mode`. Measure the effect by regenerating `artifacts/WO-M4-10/render-diagnostics.json` against the same pinned manifest.

# Context pointers
- `docs/adr/0016-product-target-and-gates.md`, `BLOCKERS.md` defect 2, `artifacts/WO-M4-10/render-diagnostics.{json,md}` (baseline at `27e7781`: ok 3, limit 13, unsupported 13, xml 8, viewport 2, skipped 19).
- `src/svg/xml/xml_tokenizer.c` (DOCTYPE/ENTITY rejected at lines ~552 and ~618), `src/svg/shape/traverse.c`, `src/svg/shape/shape_style.c`, `src/svg/shape/text.c`, `src/svg/css/`, `src/app/render_frame_cli.c` (stderr diagnostics), `scripts/corpus_render_report.py` (unchanged; parses `render-frame: <path>: <category> at byte <n>`).
- Corpus (external, read-only): `/Users/megasoft78/Desktop/Freelance/vexel-corpus-real`, manifest SHA-256 `9fbf9a2f384d70a41da72e99ac259d2c5d137ffd3c8a01ce4be4741c1c29bc30`.

# Existing contracts and invariants
- Unknown source is preserved byte-for-byte for editing and save; ignoring affects rendering only.
- Entities are never expanded: an internal subset is skipped unprocessed, and a reference to any non-predefined entity still fails as an XML error. Existing XXE/entity-expansion tests stay unchanged and passing.
- Namespaced foreign attributes, all other limits, and every unsupported-feature category other than these four keep their current behaviour.
- No frozen test, golden or oracle is weakened. An existing test that pins the old fail-closed behaviour for one of the four constructs is changed to assert the new ignore-and-diagnose behaviour, never deleted.
- macOS-only, C17, no new dependency, files under 800 lines; no corpus byte enters the repository.

# Files allowed to touch
- `src/svg/xml/**`, `src/svg/shape/**`, `src/svg/css/**`, `src/app/render_frame_cli.c`, `tests/svg/**`, `CMakeLists.txt`, `docs/**` except `docs/adr/**`, `artifacts/WO-M4-10/render-diagnostics.json`, `artifacts/WO-M4-10/render-diagnostics.md`, `artifacts/WO-CORPUS-01/**`, `BLOCKERS.md`, `STATE.md`, `LEDGER.md`, `ledger.jsonl`.

# Protected paths
- `work/**`, `OWNER-DIRECTIVES.md`, `AGENTS.md`, `docs/adr/**`, `scripts/**`, `tests/oracle/**`, `tests/golden/**`, every golden fixture, `src/raster/**`, `bench/**`, `reports/**`, other prior artifacts, every path outside the allowlist.

# Required implementation
1. Ignore each construct for rendering and record a bounded per-document summary: per kind, a count and the first byte offset.
2. On a successful render, `render-frame` prints one line per ignored kind: `render-frame: <path>: note: ignored <kind> x<count>, first at byte <offset>`.
3. Focused tests per construct: the document decodes, shapes render as without the construct, the summary has the right count and offset; plus a DOCTYPE with an internal subset declaring an entity that is referenced still fails as XML without expansion.
4. List every changed existing test assertion with old and new form in `artifacts/WO-CORPUS-01/test-changes.md`.
5. Save `git show 27e7781:artifacts/WO-M4-10/render-diagnostics.json` as `artifacts/WO-CORPUS-01/render-diagnostics-before.json`, regenerate with the corpus command, and write `artifacts/WO-CORPUS-01/delta.json` (histogram before/after, per-file verdict changes).
6. `artifacts/WO-CORPUS-01/residual-causes.md`: for every file still `unsupported`, `xml` or `viewport`, the construct at its offset, quoting only the report's own `excerpt`.
7. Record each verification command, its exit code and the last 40 output lines in `artifacts/WO-CORPUS-01/check-NN.txt`.

# Acceptance criteria
- AC-1 `./dev verify`, `./dev asan`, `./dev ubsan` and `./dev t --all` exit 0, with output recorded.
- AC-2 `./dev fuzz-xml --duration 300 --seed 1` and `./dev shape-fuzz` exit 0.
- AC-3 The corpus command exits 0; the JSON keeps `manifest_sha256` `9fbf9a2f…bc30`, `max_bytes` 33554432 and `render_tested` 39; `histogram.ok` is greater than 3; no file that was `ok` before is not `ok` after.
- AC-4 No residual `unsupported` or `xml` cause in `residual-causes.md` is one of the four constructs.
- AC-5 The tests of item 3 run under CTest and pass; `test-changes.md` exists; `git show --name-only --format= HEAD` (this order's single commit) lists only allowlisted paths.
- AC-6 `python3 scripts/ledger.py validate ledger.jsonl`, `python3 scripts/wo_validate.py WO-CORPUS-01 --ready` and `git diff --check` exit 0.
- AC-7 A fresh read-only verifier reports pass in `artifacts/WO-CORPUS-01/verdict.txt`.

# Verification commands
```text
cmake --preset mac-debug
cmake --build --preset mac-debug
ctest --test-dir build/mac-debug --output-on-failure
./dev verify
./dev asan
./dev ubsan
./dev t --all
./dev fuzz-xml --duration 300 --seed 1
./dev shape-fuzz
python3 scripts/corpus_render_report.py --corpus /Users/megasoft78/Desktop/Freelance/vexel-corpus-real --binary build/mac-debug/bin/svgworkbench --output-json artifacts/WO-M4-10/render-diagnostics.json --output-markdown artifacts/WO-M4-10/render-diagnostics.md
git show --name-only --format= HEAD
python3 scripts/ledger.py validate ledger.jsonl
python3 scripts/wo_validate.py WO-CORPUS-01 --ready
git diff --check
```

# Non-goals
No `<clipPath>` outside `<defs>`, `stroke="url(#…)"`, `<use>`, budget, edge-storage or viewport change; no vertical text layout (`writing-mode` falls back to horizontal); no change to the report script.

# Budget
Executor max runs: 3; verifier max rounds: 2; elapsed hard limit: 300 min; dependency/network: none.

# Failure and escalation
If after all four constructs are handled `histogram.ok` is still 3, do not widen scope: record each formerly blocked file's next blocking construct in `residual-causes.md` and stop for planner review.

# Output
One commit `feat(svg): ignore and diagnose non-fatal attributes, meta, doctype and writing-mode`, evidence under `artifacts/WO-CORPUS-01/`, regenerated diagnostics, updated `BLOCKERS.md`, `STATE.md` and both ledgers, and the schema-valid executor report.
