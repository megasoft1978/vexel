---
id: WO-RUST-SVG-GRAMMAR-01
revision: 1
status: READY
milestone: SVG-FIDELITY
owner: project-owner
executor: gpt-6-luna
base_sha: d6c22a50c98bcddda664312b5eee93c3f468bf75
spec_sha256: f2fe0922993711f2d293f133a408da5255c901d3feaaec7acc11829bfde26ce1
parallel_safe: false
network: false
---
# Accept valid external-corpus SVGs that are rejected as `xml` or `viewport`

## Goal
Ten valid external-corpus SVGs (read-only corpus, never copied into the repo)
are rejected. Diagnosis found five causes (A-E). Fix each in its owning crate
with failing-first tests so the listed corpus IDs change verdict and the
in-repo set does not regress. Corpus verdicts must be measured, not assumed.

## Diagnosis (probes live in /private/tmp/diag/h; do not depend on them)
- (A) Path grammar gap, real Inkscape/Illustrator output. Implicit command
  repeat where the next number starts with `.` and has no separator fails:
  `M0 0l-.1.21.1.55z`, `L1 2.5.6 7`. Same input with spaces passes.
  Sites: `rust/crates/vexel-svg-path/src/parser.rs` ~478-480 (`begin_group`
  check does not accept `b'.'` as a number start) and ~459-462
  (`compact_dot_boundary` rejects `.` at group end).
- (B) Unknown `data-*` attributes. ~55+ elements carrying `data-region` make
  the decoder fail `InvalidArgument` at offset 0 (generated-map_t1/t2). `class`
  alone passes; 200 `<path data-region>` fails. Suspects, to be confirmed by
  the worker: `rust/crates/vexel-svg-decode/src/ignored.rs` ~236-258
  (`visited.get_mut(i)` returns `InvalidArgument` at the attribute offset when
  the bookkeeping vector is shorter than the attribute run) and the
  `stream.take().ok_or_else(|| invalid(0))` site just below it (offset 0).
  The count cliff suggests a fixed-capacity or index mismatch, not a policy.
- (C) Deliberate limits misclassified. `DepthLimit`, `NodeLimit`,
  `AttributeLimit`, `TextLimit`, `InputLimit` from `vexel-xml`
  (`rust/crates/vexel-xml/src/stream.rs` ~328, ~383, ~430;
  `tokenizer.rs:5`) are mapped to `ErrorKind::Xml` in
  `rust/crates/vexel-scene/src/lexical_document.rs` ~54-60, so the corpus tool
  reports `xml` for intentional limits (1000 nested `<g>` hits MAX_DEPTH 256;
  a 491k-element file hits the 250,002 node limit). `OutputExhausted` already
  maps to `ErrorKind::Limit(LimitKind::TokenBytes)` in the same match.
- (D) Root `width`/`height` above 4096 rejected in
  `rust/crates/vexel-svg-values/src/viewport.rs` ~229-233 (7409x2600 and
  4600x6056 real maps).
- (E) DOCTYPE internal-subset general entities (e.g. `&ns_extend;`) fail in
  `rust/crates/vexel-xml/src/stream.rs` ~493/509 (`entity_known`,
  `reference_step`). Expansion is deliberately disabled (tests.rs ~232).

## Assumptions
- Corpus tool (`vexel-corpus`) is NOT in this repo; its exact invocation is
  unknown here. The worker records the exact command and corpus root used in
  `result.md`. If it cannot be run, stop with that blocker; do not invent a
  substitute verdict.
- Parsing SVG path grammar: `1.2.3` is `1.2` then `.3`. Only valid input is
  newly accepted; malformed input stays rejected.
- (D) decision: the 4096 cap is a parse-time policy with no safety role, and
  output size is chosen by the caller. Remove the hard reject for finite
  positive root sizes. Before doing so, the worker must confirm the render
  path has its own raster size limit that returns a typed error (grep the
  render/raster crates). If it does not, stop and report instead of removing
  the cap.
- (E) decision: no expansion. Entities unresolved because of the disabled
  expansion become a typed `Unsupported` carrying the entity name. If
  `Diagnostic` has no field for the name, do not change its shape; report the
  gap as a blocker.

## Scope (in order; each item = failing test first, then fix, then verify)
1. (A) Parser: accept `.` as the start of an implicit number in both sites;
   keep every negative case in `number_separator.rs`. Add tests for the two
   sample strings, `1.2.3` -> `1.2`,`.3`, and malformed cases that must still
   fail.
2. (B) Decoder: `data-*` and any other non-namespaced attribute already
   ignored by policy are skipped with no count cliff. Tests with 200 and 2000
   `<path data-region>` elements, plus `class`-only control.
3. (C) Map the five `vexel-xml` limit kinds to `ErrorKind::Limit(...)` using
   existing `LimitKind` variants. Keep all limit values unchanged. If no
   existing variant fits a kind, map to the closest existing one and record it;
   do not add variants. Update only tests that pin the old `Xml` kind for these
   five, and list each updated test in `result.md`.
4. (D) Per the decision above; test that a 7409x2600 and a 4600x6056 root
   parse successfully, and that 0, negative, NaN and infinite sizes still fail.
5. (E) DOCTYPE general entity reference -> typed `Unsupported` with entity
   name; test with `&ns_extend;`; no expansion occurs (unchanged tests.rs
   behaviour).

## Non-goals
Entity expansion, new SVG features, raster limit changes, changing any limit
value, new external dependencies, Diagnostic shape changes (unless a blocker is
reported first), any corpus file copied or committed, parity oracle edits.

## Files allowed to touch
- `rust/crates/vexel-svg-path/src/**`, `rust/crates/vexel-svg-path/tests/**`
- `rust/crates/vexel-svg-decode/src/**`, `rust/crates/vexel-svg-decode/tests/**`
- `rust/crates/vexel-xml/src/**`, `rust/crates/vexel-xml/tests/**`
- `rust/crates/vexel-svg-values/src/viewport.rs`, `rust/crates/vexel-svg-values/tests/**`
- `rust/crates/vexel-scene/src/lexical_document.rs` (minimal mapping edit only)
- `rust/crates/vexel-scene/tests/**` (only tests pinning the old `Xml` kind for
  the five limits; each recorded)
- `artifacts/WO-RUST-SVG-GRAMMAR-01/**`

## Protected paths
Workspace and lock manifests, `Cargo.toml`/`Cargo.lock`, all other crates,
`rust/fixtures/**` existing files, goldens, oracles, `parity.rs` expected
bytes (test file may gain tests; existing assertions untouched), `tests/`
top-level, `docs/**`, `STATE.md`, `LEDGER*.md`, `ledger.jsonl`, other work
orders, git HEAD/index. No `unsafe`, new deps, network, installs, GUI launch,
commit, stage, reset, deletion. Files under 800 lines.

## Acceptance criteria
1. Failing-first: each of items 1-5 has at least one test that fails before the
   fix (recorded in `result.md` with the failing output line).
2. (A) `M0 0l-.1.21.1.55z` and `L1 2.5.6 7` parse to the same segments as their
   space-separated forms; `number_separator.rs` existing cases pass unchanged.
3. (B) 200 and 2000 `data-region` elements decode with no error; output
   identical to the same file without `data-*`.
4. (C) 1000 nested `<g>` yields `Limit` (not `Xml`); a 250,002-node input yields
   `Limit`; limit constants diff is empty (`git diff -- '*limit*' '*tokenizer*'`
   shows no value change).
5. (D) 7409x2600 and 4600x6056 roots decode; invalid sizes still fail; render
   limit behaviour unchanged (cited in `result.md`).
6. (E) `&ns_extend;` yields `Unsupported` naming the entity; no expansion.
7. Corpus, measured by the corpus tool with `--only <id>`, verdict after fix:
   `commons-germany-districts-municipalities-location-map-2013-svg`,
   `commons-france-osm-mapnik-svg`, `generated-map_t1`, `generated-nested_t1`,
   `commons-1-42-polytope-7-cube-svg`,
   `commons-aegean-sea-anatolia-and-armenian-highlands-regions-large-topographic-basemap-svg`,
   `commons-columbiarivermap-svg`. Each `ok` where the decoder path is the
   only blocker; the nested and 491k cases must read `limit`, and the DOCTYPE
   case `unsupported`. Before and after verdicts recorded per ID.
8. No regression: in-repo corpus stays 24/24 ok; all existing fixture render
   hashes identical (`pipeline_hashes` output diff empty).
9. Parity: `rust/crates/vexel-svg-path/tests/parity.rs` and legacy oracle
   fixtures byte-identical (`git diff --stat` shows no change to them).
10. Workspace tests pass; `git diff --check` clean.

If an AC cannot be met, stop with the exact blocker; no test weakening, no
oracle or threshold edits.

## Verification (frozen; repo-local CARGO_HOME/CARGO_TARGET_DIR, locked, offline)
- `cargo +stable test -p vexel-svg-path --locked --offline`
- `cargo +stable test -p vexel-xml --locked --offline`
- `cargo +stable test -p vexel-svg-decode --locked --offline`
- `cargo +stable test -p vexel-svg-values --locked --offline`
- `cargo +stable test -p vexel-scene --locked --offline`
- `cargo +stable test -p vexel-app --test pipeline_hashes --locked --offline`
- `cargo +stable test --workspace --locked --offline`
- `cargo +stable fmt --manifest-path rust/Cargo.toml --package vexel-svg-path --package vexel-xml --package vexel-svg-decode --package vexel-svg-values --package vexel-scene -- --check`
- corpus: `vexel-corpus run <external corpus root> --only <id>` for each ID in AC 7 (exact invocation recorded)
- `python3 scripts/wo_validate.py WO-RUST-SVG-GRAMMAR-01 --ready`
- `git diff --check`

## Evidence
`artifacts/WO-RUST-SVG-GRAMMAR-01/`: `result.md` (<=400 words: failing-first
outputs, per-ID before/after verdicts, updated test list, decisions D/E
rationale, blockers), `verification.json`, `ac-map.json`,
`changed-path-audit.json`.

## Worker and review
Worker: gpt-6-luna High, Standard speed, one writer, existing allowance only;
stop on quota errors. Candidate only until a fresh read-only reviewer (separate
session, Luna or Sol Medium, diff + order only) reruns the frozen commands and
checks each AC. The coordinator integrates the exact patch, reruns main-tree
checks, then updates STATE.md, LEDGER.md and ledger.jsonl.
