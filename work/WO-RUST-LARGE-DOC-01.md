---
id: WO-RUST-LARGE-DOC-01
revision: 1
status: READY
milestone: V1-SVG-COMPOSITION
owner: project-owner
executor: gpt-6-luna
base_sha: 6e072ef8a00bee2526acf20ce858f4f8f354b631
spec_sha256: eb7c7d4f1009c38db76082956f02ef86b27e1d42a0dd78e52d10634edb1ffcda
parallel_safe: true
network: false
---
# Lift the 8 MiB compose cap and split corpus `limit` verdicts

## Goal
The measured external sweep (artifacts/WO-RUST-V1-CORPUS-MEASURE-01, in git
history) renders 1 of 39 attempted files; 26 of its 28 `limit` verdicts are
sources over `MAX_SVG_BYTES` (8 MiB, `vexel-app/src/compose/decode.rs`). Replace
that single cap with documented bounded budgets up to the sweep's 32 MiB
`--max-bytes`, keep cancellation, and make the corpus tool report decode-cap and
renderer-limit refusals as distinct verdicts. Product gate: large-file
responsiveness in docs/product/vexel-goal-and-delivery-plan.md; no performance
claim is made here beyond the measured sweep.

## Dependencies
Read first: `compose/decode.rs`, `compose/open.rs`, `compose/error.rs`,
`vexel-corpus/src/verdict.rs`, BLOCKERS.md defect 1 (budgets: edges 4M, elements
65,536, text 64 MiB, glyphs 1M, transform ops 1M). Known hidden 16 MiB sites in
the path: `LexicalDocument::parse(.., 1 << 24, ..)` and
`ChargeLedger::new(.., 1 << 24, ..)` in `decode.rs`, and `MAX_SOURCE_BYTES`
(16 MiB) in `vexel-svg-values`. Find every other site with a failing test, not
by guessing.

## Assumptions
- Target cap is 32 MiB for compose (open and decode share one constant); larger
  files keep the typed `TooLarge`/`LimitExceeded` refusal. True streaming of
  >32 MiB sources is out of scope and reported as the next step.
- Element/edge budgets are NOT raised unless a measured file needs it and the
  RSS bound below still holds; if so state each new value and why.
- Source bytes are held once (no second full copy beyond the existing `Arc`);
  if the current path copies (`bytes.to_vec()`), reduce to at most one copy and
  report peak RSS.

## Scope
1. One `pub const` source cap (32 MiB) used by `open.rs` and `decode.rs`; the
   internal `1 << 24` limits derive from it. `vexel-svg-values` constant is
   changed only if a failing test shows it blocks a <=32 MiB source; the change
   is a single constant plus its tests, listed in the audit.
2. Split `ComposeError::LimitExceeded` into a decode-cap kind (source or
   structural budget at decode) and a renderer-limit kind (raster work/size
   budget); `vexel-corpus` maps them to new verdicts `limit_decode` and
   `limit_render`, keeps the histogram/compare tables and JSON schema
   consistent, and `compare` treats legacy `limit` as either.
3. Cancellation: a cancel request during decode of a >8 MiB source stops within
   one checkpoint (test with a synthetic >8 MiB source and an immediate cancel).
4. Re-run both corpus sweeps and record the histogram.

## Non-goals
Streaming decode of >32 MiB, renderer speed (WO-RUST-RENDER-PERF-01), new SVG
features (WO-RUST-CORPUS-UNSUPPORTED-01), UI changes, raising the sweep
`--max-bytes`, editing goldens, oracles or legacy C.

## Files allowed to touch
- `rust/crates/vexel-app/src/compose/**`, `rust/crates/vexel-app/tests/compose*`
  (new test files only; do not edit `perf_gate.rs`)
- `rust/tools/vexel-corpus/**`
- `rust/crates/vexel-svg-values/src/lib.rs` and its tests only per Scope 1
- `rust/fixtures/large-doc/**` (new, generated-at-test-time sources preferred;
  no committed file over 100 KiB)
- `artifacts/WO-RUST-LARGE-DOC-01/**`

## Protected paths
Workspace/lock manifests, all other crates (incl. `vexel-svg-decode`,
`vexel-svg-engine`, `vexel-raster`, `vexel-render`), `rust/ci/**`, C/ObjC,
`tests/`, goldens, oracles, `docs/**`, `STATE.md`, `LEDGER*.md`, `ledger.jsonl`,
other work orders, git HEAD/index. No unsafe, new external deps, network,
installs, GUI launch, commit, stage, reset, deletion. Files under 800 lines.

## Acceptance criteria
1. A synthetic valid 9 MiB SVG (many small rects, generated in the test) and a
   30 MiB one are no longer refused with the size cap; a 33 MiB source still
   returns the typed too-large error. Both limits asserted at the constant.
2. External sweep (`--max-bytes 33554432`, 60 s timeout, 2048 MiB RSS): the
   histogram has `limit_decode` for sources decoded past budgets and no file
   whose only reason was the 8 MiB cap; record per-file verdict, wall ms and
   peak RSS. Success measure: `ok` count strictly greater than the baseline 1,
   or each remaining file named with its exact new blocker.
3. No file in the external baseline `ok` set regresses; the in-repo sweep
   (`bench/corpus/manifest.json`) still reports 17 ok / 7 unsupported / 0 other
   and the same per-file verdicts.
4. No file exceeds 2048 MiB peak RSS (`rss_exceeded` = 0) or the 60 s timeout
   newly; any that do are listed as findings, not hidden.
5. Cancellation test of Scope 3 passes without panic.
6. Corpus tool tests cover both new verdicts, the JSON/markdown output and
   compare mapping; old reports still parse.
7. Existing `vexel-app` and `vexel-corpus` tests pass (counts may only
   increase); `cargo tree -p vexel-app -p vexel-corpus` shows no new external
   package; no `unsafe`.
8. `result.md` (<=400 words) lists new constants, every limit site found, the
   before/after histogram and what remains unmeasured (live app open of a big
   file).

If an AC cannot be met, stop with the exact blocker; no test weakening.

## Verification (frozen; repo-local CARGO_HOME/CARGO_TARGET_DIR, locked, offline)
- `cargo +stable check -p vexel-app -p vexel-corpus --locked --offline`
- `cargo +stable test -p vexel-app --locked --offline`
- `cargo +stable test -p vexel-corpus --locked --offline`
- `cargo +stable test -p vexel-svg-values --locked --offline`
- `cargo +stable fmt --manifest-path Cargo.toml --package vexel-app --package vexel-corpus -- --check`
- `cargo +stable run -p vexel-corpus --release --locked --offline -- run --manifest bench/corpus/manifest.json --pins rust/tools/vexel-corpus/pins.json --out artifacts/WO-RUST-LARGE-DOC-01/in-repo`
- the same `run` with `--manifest ~/Desktop/Freelance/vexel-corpus-real/manifest.json --out artifacts/WO-RUST-LARGE-DOC-01/external` (owner-local corpus; if absent, stop and report)
- `python3 scripts/wo_validate.py WO-RUST-LARGE-DOC-01 --ready`
- `python3 scripts/ledger.py validate ledger.jsonl`
- `git diff --check`

## Evidence
`artifacts/WO-RUST-LARGE-DOC-01/`: `result.md`, `verification.json`, `ac-map.json`,
`changed-path-audit.json`.

## Worker and review
Worker: gpt-6-luna High, Standard speed, one writer, existing allowance only; stop
on quota errors. Candidate only until a fresh read-only reviewer (separate
session, Luna or Sol Medium, diff + order only) reruns the frozen commands and
checks each AC. The coordinator integrates the exact patch, reruns main-tree
checks, then updates STATE.md, LEDGER.md and ledger.jsonl.
