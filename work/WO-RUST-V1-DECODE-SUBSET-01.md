---
id: WO-RUST-V1-DECODE-SUBSET-01
revision: 1
status: READY
milestone: V1-SVG-COMPOSITION
owner: project-owner
executor: gpt-6-luna
base_sha: 2898e150bc65d81f48d02341ba77e611fd1a6172
spec_sha256: 0b70f202b314bdb6b913817aee7e4e877a1178f43cd6829da68e2935027c61f4
parallel_safe: false
network: false
---
# Accept a production SVG decoder for the ADR-0029 V1 subset

## Goal
Replace the 3-line registration stub in `rust/crates/vexel-svg-decode/` with a
production decoder accepted for the ADR-0029 V1 subset only, and add the
`vexel-svg-engine` composition adapter as far as that subset needs. This closes
the V1 decoder/composition gate (docs/product/vexel-v1-vertical-slice.md). It
does not accept the full DECODE partition of `WO-RUST-SVG-DECODE-01`, which stays
open and owns all remaining parity. No full SVG, Inkscape, renderer, UI, project
or product claim.

## Candidate inputs (immutable, byte-audited before any edit)
- Decoder: `.cache/WO-RUST-SVG-DECODE-01/current-continuation-11/rust/crates/vexel-svg-decode/`
  (72 files; 93 passing package tests; locked/offline check passes; byte-identical
  to continuation 10; evidence in that cache's `artifacts/.../current-continuation-11/`).
- Engine: `.cache/WO-RUST-SVG-ENGINE-ADAPTER-01/reseed-r4/rust/crates/vexel-svg-engine/`
  (`src/lib.rs` 278 lines, `tests/decoder.rs` 317 lines; 6 passing engine tests
  against 90 decoder tests at R9).
- Definition views: main already integrates the `traversal_scene` planner fix
  (`WO-RUST-SVG-DEFINITION-VIEWS-01`, `artifacts/WO-RUST-SVG-DEFINITION-VIEWS-01/`).
  The engine clip test must succeed as in that report, not expect `InvalidArgument`.
Coordinator creates `.cache/WO-RUST-V1-DECODE-SUBSET-01/worker/` from current main
Rust sources plus these candidates and writes a SHA-256 seed to
`artifacts/WO-RUST-V1-DECODE-SUBSET-01/source-seed.json`. The worker never edits
the original cache directories or main.

## Assumptions
- R11 decoder source is the baseline; its remaining named gaps (production
  paint/definitions composition in `DecoderProductionOperations`, integrated
  clip history, whole-render output/memory gates, full original-suite parity) are
  outside this order unless the V1 subset needs them.
- Existing decoder tests keep passing; new tests are additive. No test, oracle or
  assertion is weakened, deleted or loosened to pass.
- The workspace can register `vexel-svg-engine` without new dependencies (its
  candidate manifest uses only existing path crates).
- Accepted peers (xml, values, path, css, paint, definitions, reference,
  animation, scene) are sufficient; a missing shared contract is reported, not
  substituted.

## Subset scope
In: `svg` root; `viewBox`/`width`/`height`/`preserveAspectRatio` viewport;
`g`; `transform` (all forms); `path`; `rect` (incl. rx/ry); `circle`; `ellipse`;
`line`; `polyline`; `polygon`; solid `fill`/`stroke` (color, none, opacity,
stroke-width/linecap/linejoin/miterlimit/dasharray) via presentation attributes
and inline `style`; unknown elements/attributes/namespaces/editor metadata
preserved as source (SourceOwner, byte-for-byte, offsets stable).
Deferred, must NOT panic and must either report typed source-offset
`Unsupported`/diagnostic (fail closed) or be preserved untouched with the result
disclosing it: `filter`, `mask`, `pattern`, `text`/`tspan`, `marker`, `image`,
gradients beyond the existing cached-paint path, `use`/`switch`/`symbol` beyond
what the shape subset needs, CSS `<style>` selectors, SMIL animation, external
references. Silent drop and silent success on unhandled content are rejected.

## Non-goals
Whole-document SVG/CSS/effects fidelity; rendering/raster; app UI, MCP, native
project, GIF; performance targets; corpus campaigns; completing the other 18
assigned original suites; changing frozen oracles; recovery; remote anything.

## Files allowed to touch (worker, candidate workspace only)
- `rust/crates/vexel-svg-decode/src/**`, `tests/**` (new/additive), `README.md`
- `rust/crates/vexel-svg-engine/**`
- `artifacts/WO-RUST-V1-DECODE-SUBSET-01/**` (except `source-seed.json`)
- ignored repo-local cache/target outputs for the named offline checks

## Protected paths
Root `Cargo.toml`/`Cargo.lock`, `rust/Cargo.*`, every other crate (including
`vexel-svg-definitions`, `vexel-svg-paint`, `vexel-scene`), fixtures, oracles,
`tests/`, prior evidence, `STATE.md`, `LEDGER*.md`, `ledger.jsonl`, `docs/**`,
`work/**`, git HEAD/index, user app state. Coordinator alone registers
`vexel-svg-engine` in the candidate workspace manifest/lock, and integrates the
accepted exact patch into main. One worker, no nested workers. No unsafe code,
new dependencies, network, installs, GPL source, GUI/app launch, E2E, commit,
stage, reset, or deletion. Each source/test file under 800 lines.

## Acceptance criteria (each observable and achievable from the seed)
1. Candidate `vexel-svg-decode` compiles and its 93 R11 tests still pass
   (count may only increase). `vexel-svg-engine` compiles and its 6 tests pass.
2. Subset decode: a real-Decoder test through `vexel-svg-engine` decodes one
   SVG covering path, rect, circle, ellipse, line, polyline, polygon, a nested
   `g` with `transform`, solid fill and stroke, and `viewBox`. Asserts exact
   shape count/kinds, resolved transforms, paint colors/widths and root viewport.
3. Source preservation: for input with unknown elements, an unknown namespace
   attribute and an editor-metadata element, the published scene's source
   bytes equal the input bytes exactly and element source offsets match.
4. Deferred features: one new test per `filter`, `mask`, `pattern`, `text`,
   `marker` (via the engine, real Decoder) asserts no panic and either a typed
   `Unsupported`/diagnostic with a source offset pointing at the element, or
   preserved-untouched with the subset shapes still published. Test names state
   which outcome each feature has; no feature silently succeeds without it.
5. Malformed/limit: new tests for truncated XML, non-numeric path data, huge
   coordinates, NaN/inf lengths, and a `Stop`/cancel mid-decode each return a
   typed error or rollback with zero leaked scene state (no panic, no partial
   success).
6. Engine session: `finish` succeeds only after `Complete`; `step` bounded;
   pending pattern work is never reported as success; clip test completes and
   publishes the owner clip (DEFINITION-VIEWS parity).
7. Source audit: decoder diffs versus seed are listed file-by-file with reason;
   any production source change is justified by a failing subset test.
8. Disclosure: `result.md` (<=400 words) lists the subset, deferred behavior per
   feature, remaining gaps (production paint/definitions composition coverage,
   full original parity, renderer) and what remains unexecuted. No full-parity
   claim.
9. Limits: new crate has no dependencies beyond existing workspace path crates;
   `cargo tree` shows no new external packages.

If an AC cannot be met from the seed, stop, record the exact blocker and
evidence, and report; do not weaken the AC or fake a result.

## Verification (frozen; candidate workspace, locked and offline)
- `cargo +stable check -p vexel-svg-decode --locked --offline`
- `cargo +stable test -p vexel-svg-decode --locked --offline`
- `cargo +stable check -p vexel-svg-engine --locked --offline`
- `cargo +stable test -p vexel-svg-engine --locked --offline`
- `cargo +stable fmt --manifest-path Cargo.toml --package vexel-svg-decode --package vexel-svg-engine -- --check`
- `python3 scripts/wo_validate.py WO-RUST-V1-DECODE-SUBSET-01 --ready`
- `python3 scripts/ledger.py validate ledger.jsonl`
The repo does not run clippy; do not invent a clippy gate. Use
`CARGO_HOME=.cache/cargo` and a candidate-local `CARGO_TARGET_DIR` as in
prior candidates. Record exact commands, exit codes and test counts.

## Evidence artifacts
`artifacts/WO-RUST-V1-DECODE-SUBSET-01/`: `source-seed.json` (coordinator),
`result.md`, `verification.json`, `subset-coverage.json` (each AC to test name
and outcome), `source-diff.json` (changed files with SHA-256), `deferred-map.json`
(feature to outcome), `changed-path-audit.json`.

## Worker and review
Worker: gpt-6-luna at High effort, Standard speed, existing subscription only; stop on quota/rate errors, no
restart loop. A successful exit is a candidate only. Acceptance requires an
independent fresh read-only reviewer (Luna or Sol Medium, not the worker) who
reruns the frozen commands, audits changed paths against the seed and checks
each AC against the tests. Then the coordinator integrates the exact patch and
reruns fresh main-tree checks: the commands above plus
`cargo +stable test -p vexel-svg-definitions --locked --offline`. Only then
update STATE.md, LEDGER.md and ledger.jsonl.
