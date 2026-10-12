# WO-M4-09 evidence — corpus full-render diagnostics

Frozen spec revision 2, spec SHA-256
`3817a27f1dc7fd1a1784de56d651cb547d6f0783761572062d937e5c342409b9`.

All nine frozen verification commands exited 0 on the candidate tree.

| # | frozen command | exit | record |
|---|---|---:|---|
| 1 | `python3 -m unittest tests.test_corpus_render_report` | 0 | `check-01.json` (17 tests) |
| 2 | `cmake --preset mac-release` | 0 | `check-02.json` |
| 3 | `cmake --build --preset mac-release` | 0 | `check-03.json` |
| 4 | `./dev verify` | 0 | `check-04.json` (CTest 80/80, Python 319/319) |
| 5 | `./dev asan` | 0 | `check-05.json` (CTest 80/80) |
| 6 | `./dev ubsan` | 0 | `check-06.json` (CTest 80/80) |
| 7 | `python3 scripts/ledger.py validate ledger.jsonl` | 0 | `check-07.json` (467 entries) |
| 8 | `python3 scripts/wo_validate.py WO-M4-09 --ready` | 0 | `check-08.json` |
| 9 | `git diff --check` | 0 | `check-09.json` |

## Generated report

`reports/M4/render-diagnostics.json` and `reports/M4/render-diagnostics.md`, produced at
`--size 128x128` with the default 32 MiB size threshold:

| Verdict | Files |
|---|---:|
| `ok` | 1 |
| `limit` | 26 |
| `unsupported` | 8 |
| `xml` | 4 |
| `skipped` (> 32 MiB) | 19 |
| **total** | **58** |

Every non-`ok`, non-`skipped` file carries a diagnostic category and a source
offset; no failure was recorded without a location. The only file that renders
is `files/generated/embedded_t1.svg`.

AC-3 determinism was checked directly: two runs over the identical corpus
produced byte-identical JSON.

## Findings usable for sequencing

- **The full-render path is hard-capped at 16 MiB of source.**
  `SHAPE_MAX_BYTES` (`src/svg/shape/traverse_internal.h:6`) rejects any document
  larger than 16 MiB in `svg_shape_decode` (`src/svg/shape/decode.c:63`). Thirteen
  real Commons files fail at exactly offset `16777216`. The index path allows
  1 GiB (`SVG_INDEX_MAX_SOURCE_BYTES`), which is why the preview benchmark
  appears to work on large files while a render cannot.
- Two files are rejected for a `<!DOCTYPE ...>` declaration
  (`commons-columbiarivermap-svg`, `commons-gaussian-2d-svg`).
- Two files use a `<clipPath>` element defined outside `<defs>` and fail closed
  (`natural-earth-world-map-blank`, `natural-earth-world-map-countries`).
- Two generated files fail on `stroke="url(#...)"` (a paint server used as a
  stroke), two on an unknown non-namespaced attribute (`data-region`), two on
  the document-wide glyph cap, two on the path-command cap, two on the `<use>`
  expansion cap, two on the XML nesting cap reported as an `xml` error, one on
  `writing-mode`, and three report `limit` at offset 0 with no usable location.

No renderer, parser, raster, policy, or platform code was changed by this work
order; the cap and policy fixes are separate orders.
