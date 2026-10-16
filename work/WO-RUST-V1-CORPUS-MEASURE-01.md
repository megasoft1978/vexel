---
id: WO-RUST-V1-CORPUS-MEASURE-01
revision: 1
status: READY
milestone: V1-SVG-COMPOSITION
owner: project-owner
executor: gpt-6-luna
base_sha: 1d077a5539d44b64586b5416ccf7dab99f9f58f9
spec_sha256: 9c52d3ef470df4e3198f159347bd03f734d6b782e4dab6805a78a786e3592841
parallel_safe: true
network: false
---
# Measure the Rust decode+render pipeline over the external corpus

## Goal
Add `rust/tools/vexel-corpus`, a developer tool that runs the Rust pipeline
(`vexel_app::compose::compose_document`) over the read-only external corpus and
writes a per-file verdict report with wall time and peak RSS, and compare the
verdict histogram with the legacy C baseline. This is measurement only: no
fidelity, conformance or performance claim, and no renderer change. Closes the
HANDOFF gap "no corpus measurement for Rust".

## Dependencies
Read first: `HANDOFF.md`, `bench/corpus/README.md`, `BLOCKERS.md` (items on the
3/39 and 5/39 sweeps), `rust/crates/vexel-app/src/compose/{mod,error}.rs`,
`rust/tools/vexel-dev/src/main.rs` (tool style). Legacy baseline, read with
`git show legacy-c-final:<path>`: `artifacts/WO-M4-10/render-diagnostics.{json,md}`
(3/39: ok 3, limit 17, path 2, unsupported 11, viewport 2, xml 4, skipped 19;
render size 128x128; size threshold 33554432 bytes; manifest sha256
`9fbf9a2f384d70a41da72e99ac259d2c5d137ffd3c8a01ce4be4741c1c29bc30`),
`artifacts/WO-CORPUS-01/render-diagnostics-before.json` and
`artifacts/WO-CORPUS-04/measure.json` (the later 5/39 candidate, never accepted).
The coordinator creates `rust/tools/vexel-corpus/Cargo.toml` (dependencies
`vexel-app`, `vexel-json`, `vexel-base` by path only) and registers the member;
workers never edit manifests. If it is absent, stop.

## Assumptions
- Two inputs exist and both are read-only: the in-repo manifest
  `bench/corpus/manifest.json` (24 small project-authored fixtures, digest in
  `bench/corpus/manifest.sha256`) and the external corpus manifest
  `~/Desktop/Freelance/vexel-corpus-real/manifest.json` (58 entries: 25 real, 33
  generated; fields `id`, `tier`, `path`, `bytes`, `sha256`, `kind`). The
  external one is the one the legacy 3/39 and 5/39 numbers refer to; the
  in-repo one is run as a small control group. Report them separately.
- Hash guard: the tool refuses to run unless the manifest file's SHA-256 equals
  a pin stored in `rust/tools/vexel-corpus/pins.json` (the external pin is the
  value above) and each file's SHA-256 equals its manifest entry; a mismatch is
  verdict `guard_failed` for that file and a nonzero exit overall. Hashing uses
  `/usr/bin/shasum -a 256` through `std::process::Command` (no new dependency).
- Isolation and RSS: each file runs in a child process (the tool re-invokes
  itself with a hidden `--one <path>` mode) started through `/usr/bin/time -l`,
  whose "maximum resident set size" (bytes on macOS) is parsed. The parent
  enforces a wall-clock timeout by polling `try_wait` and killing the child.
- The corpus is never copied, written to or moved; the tool opens files
  read-only and writes only under `artifacts/WO-RUST-V1-CORPUS-MEASURE-01/`.

## Scope
1. CLI: `vexel-corpus run --manifest <path> --pins <path> --out <dir>
   [--size 128x128] [--max-bytes 33554432] [--timeout-secs 60] [--rss-limit-mib
   2048] [--only <id>...]`, plus `compare --baseline <json> --current <json>`.
   Defaults match the legacy sweep (128x128, 33554432-byte skip threshold).
2. Verdicts per file: `ok`, `unsupported` (`ComposeError::Unsupported`,
   `Hidden`), `limit` (`LimitExceeded`), `xml` (`Malformed`, `InvalidUtf8`),
   `timeout`, `rss_exceeded` (the post-hoc `/usr/bin/time` peak is above
   `--rss-limit-mib`), `skipped` (over `--max-bytes`, not read), `crash` (non-zero exit or
   signal, with the exit status recorded), `guard_failed`. The mapping from
   `ComposeError` variants is a documented table in `result.md`.
3. Report `artifacts/WO-RUST-V1-CORPUS-MEASURE-01/report.json` and `report.md`:
   tool git sha, manifest sha, size, thresholds, host (`sw_vers`, `uname -m`),
   per-file `{id, tier, kind, bytes, verdict, detail, wall_ms, peak_rss_bytes}`,
   verdict histogram, totals, slowest and largest-RSS five files. Deterministic
   file order (manifest order); no timestamps inside the hashed body.
4. Comparison section in `report.md`: Rust histogram next to the legacy 3/39
   (WO-M4-10), the 5/39 candidate (WO-CORPUS-04) and per-file verdict deltas by
   id (new ok, regressions, changed category). Legacy category `path` and
   `viewport` have no Rust equivalent; the table maps them to the Rust category
   they correspond to or lists them as unmapped, never silently merged.
5. Measured run committed as evidence: one run of both manifests. Wall times and
   RSS are single-run observations on one host.

## Non-goals
Pixel comparison, goldens, Inkscape comparison, fidelity or conformance claims,
performance targets or gates (those belong to WO-RUST-V1-RELEASE-CI-01), fixing
any failing file, changing decoder or renderer limits, network access, any
corpus fetch or regeneration, CI integration.

## Files allowed to touch
- `rust/tools/vexel-corpus/src/**`, `rust/tools/vexel-corpus/tests/**`,
  `rust/tools/vexel-corpus/pins.json` (`Cargo.toml` is coordinator-written)
- `artifacts/WO-RUST-V1-CORPUS-MEASURE-01/**`

## Protected paths
Workspace/lock manifests and every `Cargo.toml`, `bench/**`, the external corpus
directory (read-only: no write, move, delete or copy), all crates, C/ObjC,
`tests/`, goldens, oracles, `docs/**`, `BLOCKERS.md`, `STATE.md`, `LEDGER*.md`,
`ledger.jsonl`, other work orders, git HEAD/index. No unsafe, external
dependencies, network, installs, GUI launch, commit, stage, reset or deletion.
Files under 800 lines.

## Acceptance criteria
1. Guard tests: a manifest copy with one altered byte (in a temp dir) is
   refused; a file whose hash differs yields `guard_failed` and exit code != 0;
   neither case runs the pipeline. The tool never writes outside `--out`
   (asserted by a test listing a temp tree before and after).
2. Verdict mapping unit tests cover every `ComposeError` variant and each
   non-compose verdict, using small synthetic inputs created in a temp dir.
3. Timeout test: a child mode that sleeps is killed within timeout + 2 s and
   reported `timeout`; a limit test with `--rss-limit-mib 1` reports
   `rss_exceeded` for any real render.
4. Control run over the 24 in-repo fixtures completes with a histogram that
   sums to 24 and no `crash`/`guard_failed`; the report records it.
5. External run over the 58 entries completes with a histogram that sums to 58
   (the 19 over-threshold files are `skipped`), every file has a verdict,
   `wall_ms` and (for non-skipped) `peak_rss_bytes`, and the report contains the
   legacy comparison table with the 3/39 and 5/39 figures cited to their files.
   A `crash` or `timeout` verdict is a valid result, not a failure of this order.
6. `report.md` states, in its first paragraph, that these are verdict counts and
   timings only, not fidelity or performance claims, and that the corpus bytes
   are not in the repository (`git status` shows nothing outside the allowed
   paths; the audit proves it).
7. `cargo tree -p vexel-corpus` adds no external package; no `unsafe`.
8. `result.md` (<=300 words): histograms (Rust vs legacy), notable regressions
   or new passes by id, host, and what is unexecuted (other hosts, repeat runs).

If an AC cannot be met, stop with the exact blocker; no test weakening.

## Verification (frozen; repo-local CARGO_HOME/CARGO_TARGET_DIR, locked, offline)
- `cargo +stable check -p vexel-corpus --locked --offline`
- `cargo +stable test -p vexel-corpus --locked --offline`
- `cargo +stable fmt --manifest-path Cargo.toml --package vexel-corpus -- --check`
- `cargo +stable run -p vexel-corpus --release --locked --offline -- run
  --manifest bench/corpus/manifest.json --pins rust/tools/vexel-corpus/pins.json
  --out artifacts/WO-RUST-V1-CORPUS-MEASURE-01/in-repo`
- the same `run` with `--manifest ~/Desktop/Freelance/vexel-corpus-real/
  manifest.json --out artifacts/WO-RUST-V1-CORPUS-MEASURE-01/external`
- `python3 scripts/wo_validate.py WO-RUST-V1-CORPUS-MEASURE-01 --ready`
- `python3 scripts/ledger.py validate ledger.jsonl`
- `git diff --check`

## Evidence
`artifacts/WO-RUST-V1-CORPUS-MEASURE-01/`: `result.md`, `verification.json`,
`ac-map.json`, `changed-path-audit.json`, `in-repo/report.{json,md}`,
`external/report.{json,md}`.

## Worker and review
Worker: gpt-6-luna Medium, one writer, existing allowance only; stop on quota
errors. Candidate only until a fresh read-only reviewer reruns the frozen
commands (the external run may differ in timing, not in verdicts) and checks each
AC. The coordinator registers the member, integrates the patch, reruns main-tree
checks, then updates STATE.md, LEDGER.md, ledger.jsonl and, if the numbers
warrant it, BLOCKERS.md.
