---
id: WO-RUST-CORPUS-UNSUPPORTED-01
revision: 1
status: READY
milestone: V1-SVG-COMPOSITION
owner: project-owner
executor: gpt-6-luna
base_sha: 6e072ef8a00bee2526acf20ce858f4f8f354b631
spec_sha256: 3937d131b8deca194d7d236d27003ff274da32c47ea5d8e452df1a2eb84b279a
parallel_safe: true
network: false
---
# Re-measure and close the top unsupported causes

## Goal
Clip-path is NOT a top cause: in the external report it accounts for 1 of the 7
`unsupported` verdicts (`natural-earth-world-map-blank`). Causes at that HEAD:
`text` 4 (`generated-cad_t1`, `generated-science_t1`, `generated-text_t1`,
`natural-earth-world-map-countries`), `image` 1 (`generated-embedded_t1`),
`filter` 1 (`generated-inkscape_t1`), `clip path` 1. Text and filter have since
merged (WO-RUST-SVG-TEXT-01, WO-RUST-SVG-FILTER-01) and the clip decode exists,
so first re-measure, then close the real remaining gaps for those 7 files plus
the in-repo 7 unsupported.

## Dependencies
Read first: `compose/error.rs` (`UnsupportedKind`), `vexel-svg-decode` and
`vexel-svg-engine` sources where the 7 files stop, the report at
`git show 974e9501:artifacts/WO-RUST-V1-CORPUS-MEASURE-01/external/report.md`.
Independent of WO-RUST-LARGE-DOC-01 and WO-RUST-RENDER-PERF-01 (disjoint paths).

## Assumptions
- The compose wiring (`vexel-app`) may still return `Unsupported` for kinds the
  engine now supports; the first fix may be wiring only, and is reported as such.
- Only constructs that block a measured file are in scope; each fix needs a
  minimal fixture and a test, and parity with the legacy C golden where a golden
  exists, otherwise a disclosed hand-derived oracle.
- Embedded `<image>` is in scope only for data-URI PNG/JPEG already supported by
  `vexel-svg-media`; external hrefs stay unsupported.

## Scope
1. Re-run the two sweeps on current HEAD; write the per-file unsupported
   cause table (element, byte offset, kind) to the artifacts.
2. For each of the 7 external files: fix the first blocking cause, re-run, and
   continue until the file renders or hits a cause outside the allowed paths
   (then record it as the exact residual blocker, do not widen scope).
3. Fixes limited to listed crates; no new feature without a fixture.

## Non-goals
Size caps (LARGE-DOC), renderer speed (RENDER-PERF), SMIL/animation, new text
shaping, external resource loading, fidelity claims beyond measured verdicts.

## Files allowed to touch
- `rust/crates/vexel-app/src/compose/**`, `rust/crates/vexel-app/tests/compose*`
  (new test files only)
- `rust/crates/vexel-svg-decode/src/**`, `tests/**`
- `rust/crates/vexel-svg-engine/src/**`, `tests/**`
- `rust/fixtures/unsupported/**` (new fixtures only)
- `artifacts/WO-RUST-CORPUS-UNSUPPORTED-01/**`

## Protected paths
Workspace/lock manifests, `vexel-corpus`, `vexel-raster`, `vexel-render`,
`vexel-svg-values`, other crates, `rust/ci/**`, C/ObjC, `tests/`, existing
goldens, oracles, `docs/**`, `STATE.md`, `LEDGER*.md`, `ledger.jsonl`, other work
orders, git HEAD/index. No unsafe, new external deps, network, installs, GUI
launch, commit, stage, reset, deletion. Files under 800 lines.

## Acceptance criteria
1. `artifacts/.../causes.md` lists, for the in-repo 7 and external 7 unsupported
   files, the cause at current HEAD, with element and byte offset.
2. Each of the 14 files is either `ok` after the order or has one named residual
   blocker; the external `unsupported` count drops below 7 or every file's
   residual is outside the allowed paths and named.
3. No regression: previously `ok` files in both sweeps stay `ok` (per-file diff
   against the pre-change run recorded); in-repo ok count >= 17.
4. Every new supported construct has a fixture and test; existing goldens
   unchanged (git diff shows none); engine/decode/app tests pass, counts may
   only increase.
5. `cargo tree -p vexel-app` shows no new external package; no `unsafe`.
6. `result.md` (<=400 words): before/after histograms, constructs fixed,
   residuals, and what remains unmeasured.

If an AC cannot be met, stop with the exact blocker; no test weakening.

## Verification (frozen; repo-local CARGO_HOME/CARGO_TARGET_DIR, locked, offline)
- `cargo +stable check -p vexel-app -p vexel-svg-decode -p vexel-svg-engine --locked --offline`
- `cargo +stable test -p vexel-app -p vexel-svg-decode -p vexel-svg-engine --locked --offline`
- `cargo +stable fmt --manifest-path Cargo.toml --package vexel-app --package vexel-svg-decode --package vexel-svg-engine -- --check`
- `cargo +stable run -p vexel-corpus --release --locked --offline -- run --manifest bench/corpus/manifest.json --pins rust/tools/vexel-corpus/pins.json --out artifacts/WO-RUST-CORPUS-UNSUPPORTED-01/in-repo`
- the same `run` with `--manifest ~/Desktop/Freelance/vexel-corpus-real/manifest.json --out artifacts/WO-RUST-CORPUS-UNSUPPORTED-01/external` (owner-local corpus; if absent, stop and report)
- `python3 scripts/wo_validate.py WO-RUST-CORPUS-UNSUPPORTED-01 --ready`
- `python3 scripts/ledger.py validate ledger.jsonl`
- `git diff --check`

## Evidence
`artifacts/WO-RUST-CORPUS-UNSUPPORTED-01/`: `result.md`, `verification.json`, `ac-map.json`,
`changed-path-audit.json`.

## Worker and review
Worker: gpt-6-luna High, Standard speed, one writer, existing allowance only; stop
on quota errors. Candidate only until a fresh read-only reviewer (separate
session, Luna or Sol Medium, diff + order only) reruns the frozen commands and
checks each AC. The coordinator integrates the exact patch, reruns main-tree
checks, then updates STATE.md, LEDGER.md and ledger.jsonl.
