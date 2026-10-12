# WO-M4-09 evidence — corpus full-render diagnostics

Frozen spec revision 2, spec SHA-256
`3817a27f1dc7fd1a1784de56d651cb547d6f0783761572062d937e5c342409b9`.

All **ten** frozen verification commands exited 0 on the candidate tree, in the
order the spec lists them.

| # | frozen command | exit | record |
|---|---|---:|---|
| 1 | `python3 -m unittest tests.test_corpus_render_report` | 0 | `check-01.json` (25 tests) |
| 2 | `cmake --preset mac-release` | 0 | `check-02.json` |
| 3 | `cmake --build --preset mac-release` | 0 | `check-03.json` |
| 4 | `python3 scripts/corpus_render_report.py --corpus ... --output-json reports/M4/render-diagnostics.json --output-markdown reports/M4/render-diagnostics.md` | 0 | `check-04.json` |
| 5 | `./dev verify` | 0 | `check-05.json` |
| 6 | `./dev asan` | 0 | `check-06.json` |
| 7 | `./dev ubsan` | 0 | `check-07.json` |
| 8 | `python3 scripts/ledger.py validate ledger.jsonl` | 0 | `check-08.json` |
| 9 | `python3 scripts/wo_validate.py WO-M4-09 --ready` | 0 | `check-09.json` |
| 10 | `git diff --check` | 0 | `check-10.json` |

## Generated report

`reports/M4/render-diagnostics.json` and `reports/M4/render-diagnostics.md`,
produced at `--size 128x128` with the default 32 MiB size threshold. JSON
SHA-256 `8c209813acaca2622434466be3a26871ecba5388d45dd49d1bf08dc77206c5fd`.

| Verdict | Files |
|---|---:|
| `ok` | 1 |
| `limit` | 26 |
| `unsupported` | 8 |
| `xml` | 4 |
| `skipped` (> 32 MiB) | 19 |
| **total** | **58** |

Coverage is explicit in the JSON and the Markdown: **39 files were
render-tested** and 19 were not, because their sources exceed the size
threshold. Every non-`ok` record carries either a diagnostic category plus
offset or an explicit `diagnostic_note`; no record is left unexplained. The one
file that renders among the 39 render-tested files is
`files/generated/embedded_t1.svg`; the 19 untested files are larger than 32 MiB
and were not attempted.

AC-3 determinism is recorded in `determinism.json`: two independent runs over
the identical corpus produced JSON with the same SHA-256
(`8c209813…c5fd`), byte-identical to the committed report.

## Findings usable for sequencing

Counts below are read from the committed JSON. The JSON records the raw category and offset; the
semantic labels ("glyph cap", "path-command cap", "use expansion cap") are attributed from the
renderer constants cited in `src/svg/shape/media.h` and `src/svg/shape/traverse_internal.h` and are
an interpretation of the offsets, not a field in the JSON.

- **The full-render path is hard-capped at 16 MiB of source.**
  `SHAPE_MAX_BYTES` (`src/svg/shape/traverse_internal.h:6`) rejects any document
  larger than 16 MiB in `svg_shape_decode` (`src/svg/shape/decode.c:63`).
  **11 files** fail at exactly offset `16777216` (all in category `limit`). The
  index path allows 1 GiB (`SVG_INDEX_MAX_SOURCE_BYTES`), which is why the
  preview benchmark appears to work on large files while a render cannot. The
  cap exists because `svg_shape_decode` allocates a token array sized
  `source.len / 2 + 2` of `svg_xml_token` (96 bytes), so a 94 MiB document would
  need about 4.4 GiB of tokens; lifting it is a decode-architecture change, not
  a constant bump.
- **2 files** report `limit` with a diagnostic offset of 0, which carries no
  usable source location (`generated-embedded_t2`, `generated-science_t1`).
- **2 files** are rejected for a `<!DOCTYPE ...>` declaration
  (`commons-columbiarivermap-svg`, `commons-gaussian-2d-svg`);
  `SVG_XML_ERROR_UNSUPPORTED_DTD` exists by design.
- **2 files** use a `<clipPath>` element defined outside `<defs>` and fail
  closed (`natural-earth-world-map-blank`, `natural-earth-world-map-countries`).
- **2 generated files** fail on `stroke="url(#...)"` — a paint server used as a
  stroke.
- **2 generated files** fail on an unknown non-namespaced attribute
  (`data-region`).
- **2 files** hit the document-wide glyph cap, **2** the path-command cap,
  **2** the `<use>` expansion cap, and **2** the XML nesting cap reported as an
  `xml` error.
- **1 file** fails on the CSS property `writing-mode`.

## Record sequencing

`STATE.md` is updated in the implementation commit. `LEDGER.md` and `ledger.jsonl`
carry the `verified`, `accepted` and `merged` events, so they are written after the
verifier verdict and name the accepted candidate commit; the transition table in
`scripts/ledger.py` requires that order. The schema-valid executor report is
retained under `artifacts/WO-M4-09/` and names the verified candidate commit.

## Known stale text in history

The commit message of `85e7d17` says "13 real files fail at exactly 16777216".
That count was wrong; the measured value is 11, as recorded in the committed JSON
and corrected in the README and in `454c09d`. The message is immutable history
and is recorded here rather than silently left to contradict the evidence.

No renderer, parser, raster, policy, or platform code was changed by this work
order; the cap and policy fixes are separate orders.
