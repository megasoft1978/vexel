---
id: WO-RUST-SCENE-LIMITS-01
revision: 1
status: READY
milestone: V1-SVG-COMPOSITION
owner: project-owner
executor: gpt-6-luna
base_sha: d0ae379c12411e7f3e9965e637f00bc2d1acacef
spec_sha256: 6311f598ecc63e60b526686ebbea73cbf235e8cbd26fc5200fc17709a6295cf6
parallel_safe: false
network: false
---
# Scene, reference and text limits: measured, bounded raises

## Goal
Name each limit that still refuses a corpus file, compare it with the legacy
budgets (BLOCKERS.md defect 1: edges 4M, elements 65,536, text 64 MiB with 64 KiB
per token, glyphs 1M per document / 65,536 per run, transform ops 1M), and raise
only what a measured file needs, with a memory bound proven by tests. Measured
2026-10-17 (release harness, macOS text provider not linked, so text sizes are
estimates until AC4):

| refused file | verdict site (file:line) | current value | measured need |
|---|---|---|---|
| `generated-use_t1` (17,536 `<use>` of a 91-byte symbol) | `vexel-svg-decode/src/use_expansion.rs:99-103` cumulative reference bytes | 1,048,576 | 1.60 MB; trips at byte 648,183 (about 11,500 uses) |
| `generated-use_t2` (178,589 `<use>`) | same file, `:71` instances; then shapes | 65,536 instances; `Limits.shapes` 65,536 | 178,589 instances/shapes |
| `commons-chuaattractormodified` | `vexel-svg-reference/src/lib.rs:12` `MAX_REFERENCE_IDS`, `build.rs:230` | 65,536 ids (slots 131,072) | 100,088 `id` attributes (it has 48 `use`, none large) |
| `commons-ideogram-human-chromosome-20` | same ids limit | 65,536 | 144,217 ids (0 `use`) |
| `generated-text_t1/t2` | `vexel-scene/src/request.rs:229` `DOCUMENT_TEXT_COMMAND_LIMIT`, admitted at `vexel-svg-engine/src/text.rs:163` and `vexel-svg-media/src/text_geometry.rs:469` | 4,194,304 commands | t1: 575,208 text chars, t2: 5,857,812 chars; commands per glyph unmeasured |

So chua and ideogram are id-table limits, not `use` limits (all four report
`Limit(References)`). `TextCommand` is 56 bytes (verb + 6 f64): 4,194,304
commands is at most 224 MiB. Glyph cap 1,048,576 (`text.rs:67`) means t2
(5.86M chars) stays out of reach; do not raise glyph or text-byte caps here.

## Dependencies
WO-RUST-PIPELINE-PERF-01 must be integrated first: with the base pipeline a
17,536-shape file takes minutes, so a raised limit cannot be shown useful.
Read first: BLOCKERS.md defect 1, `artifacts/WO-RUST-LARGE-DOC-01/result.md`,
the files in the table, `vexel-app/src/compose/decode.rs` (`Limits`).

## Assumptions
- Raises are constants only; no algorithm change. Each new value is justified by
  a measured need plus a worst-case byte bound, not by round numbers.
- The reference index is a power-of-two open-addressed `u32` table
  (`slot & (cap - 1)`): `REFERENCE_SLOT_CAPACITY` must stay a power of two and
  at least 2x `MAX_REFERENCE_IDS`.
- Proposed (the worker confirms or lowers by measurement):
  `MAX_REFERENCE_IDS` 262,144 and `REFERENCE_SLOT_CAPACITY` 524,288 (2 MiB);
  cumulative `use` reference bytes 16 MiB, per-reference 64 KiB and nesting 16
  unchanged; instances stay 65,536 unless `Limits.shapes` is raised (app-owned,
  in `compose/decode.rs`) to 262,144 and `size_of::<Shape>() x 262,144` is under
  256 MiB; `DOCUMENT_TEXT_COMMAND_LIMIT` raised only to what text_t1 measures,
  capped at 8,388,608 (448 MiB at 56 bytes).
- If a raised limit only moves the file to the next named limit or a timeout,
  that is a valid outcome: record it, stop raising.

## Scope
1. A measurement test or script (no repo writes outside the allowed paths) that
   prints, per file in the table, id count, `use` count, summed reference bytes,
   and for text the commands, glyphs and text bytes admitted at refusal.
2. Constant changes at the sites above and the matching `Limits` value in
   `compose/decode.rs`.
3. Boundary tests: N accepted, N+1 refused with the same `ErrorKind`, for ids,
   cumulative reference bytes, instances and text commands.
4. Compile-time assertions: `REFERENCE_SLOT_CAPACITY.is_power_of_two()`,
   `>= 2 x MAX_REFERENCE_IDS`, and `DOCUMENT_TEXT_COMMAND_LIMIT x
   size_of::<TextCommand>() <= 512 MiB`.

## Non-goals
Pipeline speed (WO-RUST-PIPELINE-PERF-01), glyph/text-byte/edge/transform
limits, streaming decode, `use_t3/t4` (over 32 MiB), `<use>` of non-shape
targets, new SVG features.

## Files allowed to touch
- `rust/crates/vexel-app/src/compose/decode.rs`, `rust/crates/vexel-app/tests/**`,
  `artifacts/WO-RUST-SCENE-LIMITS-01/**`
- PROTECTED crates, constants only, each edit proven by a failing boundary or
  measurement test added first and listed in the audit:
  - `rust/crates/vexel-scene/src/request.rs` (the single
    `DOCUMENT_TEXT_COMMAND_LIMIT` constant and one const assertion) and
    `rust/crates/vexel-scene/tests/**`
  - `rust/crates/vexel-svg-decode/src/use_expansion.rs` (the 1_048_576 byte
    budget, and `65_536` only if shapes are raised) and its `tests/**`
  - `rust/crates/vexel-svg-reference/src/lib.rs` (two constants) and
    `build.rs`/`find.rs` only if a test proves a hardcoded capacity assumption
  - existing tests that hardcode an old value, updated to the new boundary
    (never loosened without a new boundary test of the same strength)

## Protected paths
`vexel-render`, `vexel-svg-engine`, `vexel-svg-media` (glyph and text-byte caps
stay), `vexel-raster`, all other crates, workspace/lock manifests,
`rust/tools/**`, goldens, oracles, `docs/**`, `BLOCKERS.md`, `STATE.md`,
`LEDGER*.md`, `ledger.jsonl`, other work orders, git HEAD/index, the external
corpus. No unsafe, new deps, network, commit, stage, reset or deletion. Files
under 800 lines. Not parallel with WO-RUST-PIPELINE-PERF-01.

## Acceptance criteria
1. `artifacts/WO-RUST-SCENE-LIMITS-01/limits.md` has one row per limit above:
   file:line, current value, unit, legacy budget, measured need per file, and
   whether the value changed. Unchanged legacy budgets (edges, per-path
   commands, elements, text bytes, glyphs, transform ops) are listed as unchanged.
2. Reference ids: `MAX_REFERENCE_IDS`/`REFERENCE_SLOT_CAPACITY` per Assumptions
   (or measured lower values); boundary tests at N and N+1 on `ReferenceIndex`
   and through `compose_svg`; chua and ideogram no longer end with
   `Limit(References)` at their id attributes. Their new verdict is recorded
   (`ok`, `timeout`, `unsupported` or another named limit with byte offset).
3. `use` budget: cumulative reference bytes raised to the measured need (16 MiB
   proposed); `generated-use_t1` passes the reference-bytes check (refusal offset
   moves past 648,183 or the file composes). Its verdict, time and peak RSS are
   recorded. If `use_t2` still stops at instances/shapes, state which and the
   value that would be needed; raise shapes only under the Assumptions bound.
4. Text: on macOS compose `text_t1` with the real provider and record glyphs,
   commands, text bytes at completion or refusal. If commands needed <= 8,388,608,
   set the limit to the next power of two at or above 1.25 x needed; otherwise
   keep 4,194,304 and state the number. `text_t2` is expected to stop at the
   glyph cap: record it, do not raise that cap.
5. Memory proof: `limits.md` gives the worst-case bytes per raised limit
   (ids 2 MiB; reference bytes budget is a counter, show it allocates nothing
   per byte; text commands x 56 B; shapes x `size_of::<Shape>()`), the three
   compile-time assertions pass, and an adversarial test (ids at N+1, commands at
   limit+1) refuses without growing allocation past the proven bound.
6. Sweep (release, 128x128, 60 s, 2048 MiB, external manifest): no file that was
   `ok` regresses; no `rss_exceeded` or `crash`; every previously `limit_decode`
   file among the four has a recorded new verdict; peak RSS of each newly decoded
   file is below 1.5 GiB. In-repo sweep verdicts unchanged.
7. All existing tests pass; counts only increase; byte-identical output for every
   file that already composed (hash compare on the in-repo fixtures before/after).
8. `cargo tree -p vexel-app` shows no new package; no `unsafe`.
9. `result.md` (<=300 words): before/after verdict table for the six corpus
   files, each constant changed with its proving test, remaining ceiling per file.

If an AC cannot be met, stop with the exact blocker; no test weakening.

## Verification (frozen; repo-local CARGO_HOME/CARGO_TARGET_DIR, locked, offline)
- `cargo +stable check -p vexel-app -p vexel-scene -p vexel-svg-decode -p vexel-svg-reference --locked --offline`
- `cargo +stable test -p vexel-app -p vexel-scene -p vexel-svg-decode -p vexel-svg-reference -p vexel-svg-engine -p vexel-svg-media --locked --offline`
- `cargo +stable fmt --manifest-path Cargo.toml --package vexel-app --package vexel-scene --package vexel-svg-decode --package vexel-svg-reference -- --check`
- `cargo +stable run -p vexel-corpus --release --locked --offline -- run --manifest bench/corpus/manifest.json --pins rust/tools/vexel-corpus/pins.json --out artifacts/WO-RUST-SCENE-LIMITS-01/in-repo`
- the same `run` with `--manifest ~/Desktop/Freelance/vexel-corpus-real/manifest.json --out artifacts/WO-RUST-SCENE-LIMITS-01/external` (owner-local; if absent, stop and report)
- `python3 scripts/wo_validate.py WO-RUST-SCENE-LIMITS-01 --ready`
- `python3 scripts/ledger.py validate ledger.jsonl`
- `git diff --check`

## Evidence
`artifacts/WO-RUST-SCENE-LIMITS-01/`: `result.md`, `limits.md`, `verification.json`,
`ac-map.json`, `changed-path-audit.json`.

## Worker and review
Worker: gpt-6-luna High, Standard speed, one writer, existing allowance only; stop
on quota errors. Candidate only until a fresh read-only reviewer (separate
session, Luna or Sol Medium, diff + order only) reruns the frozen commands and
checks each AC. The coordinator integrates the exact patch after
WO-RUST-PIPELINE-PERF-01, reruns main-tree checks and the live text check, then
updates STATE.md, LEDGER.md and ledger.jsonl.
