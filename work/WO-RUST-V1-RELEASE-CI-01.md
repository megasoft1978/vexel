---
id: WO-RUST-V1-RELEASE-CI-01
revision: 1
status: READY
milestone: V1-SVG-COMPOSITION
owner: project-owner
executor: gpt-6-luna
base_sha: 1d077a5539d44b64586b5416ccf7dab99f9f58f9
spec_sha256: c932878cba8e166dea57d31a5454ec036a31dd0f97568691edaadcc4588d24c0
parallel_safe: true
network: false
---
# Rust CI gates, release tooling, fuzz and perf gates

## Goal
Rebuild, in Rust-only form, the gates the legacy C removal dropped (HANDOFF.md
"Still to rebuild in Rust"): a CI that checks format/check/test, `vexel-dev`
subcommands that produce `Vexel.app`, a DMG, an SBOM and an install smoke, and
deterministic fuzz/robustness and perf-gate tests. Signing with real
credentials and notarization are owner-gated and out of scope.

## Dependencies
Read first: `HANDOFF.md`, `AGENTS.md` (macOS-only CI, no secrets, no package
installs, pinned actions), `.github/workflows/ci.yml`, `scripts/check_ci.py`,
`scripts/make_sbom.py`, `rust/tools/vexel-dev/src/main.rs`, `resources/`,
`vexel-app/src/main.rs` (flags), `vexel-xml`, `vexel-svg-engine`,
`vexel-protocol`, `vexel-mcp-stdio` public APIs. Legacy reference only:
`git show legacy-c-final:app/Info.plist.in`, `packaging/vexel_packaging.py`,
`docs/release.md`.

## Assumptions
- Measured at base: `rustfmt --check` reports drift in 29 workspace members
  (not only `vexel-protocol`; for example `vexel-render` 212 hunks,
  `vexel-xml` 116, `vexel-protocol` 31, `vexel-dev` 4), while `vexel-app`,
  `vexel-svg-decode`, `vexel-svg-engine`, `vexel-mcp-stdio` are clean. Reformatting
  foreign crates would collide with the parallel orders PNG, RENDER-DASH and
  POLISH, so this order does NOT mass-format. It formats only `vexel-dev`
  (allowed) and gates the rest by list.
- `cargo clippy` is not usable on the base host (`cargo clippy --version`
  fails: component absent). The CI step therefore runs clippy only when
  `cargo clippy --version` succeeds and prints a notice otherwise; the order
  states which happened. No `-D warnings` gate is introduced.
- The CI policy in `scripts/check_ci.py` stays satisfied: exactly one
  `macos-15` job, full-SHA pinned actions, `contents: read`, no secrets, no
  package installs or caches. `vexel-app` tests are excluded from CI today;
  include them (`cargo test -p vexel-app --locked`) only if they pass on the
  host with no display attached, recording the evidence either way.
- `vexel-app --mcp-preview` opens a window, so a live launch cannot run in a
  headless smoke. The smoke does everything that does not need a session and
  discloses the live launch as a coordinator check.
- The bundle identifier and icon are owner assets. Use the constants
  `BUNDLE_ID = "app.vexel.Vexel"` (placeholder, flagged) and an optional
  `--icon <file.icns>`; if none exists the bundle has no icon and `result.md`
  says so. Do not invent artwork.

## Scope
1. CI (`.github/workflows/ci.yml`, `scripts/check_ci.py` only if its checks need
   widening): steps `cargo fmt --check` through `scripts/check_rust_fmt.py`
   (reads `rust/ci/fmt-clean.txt`, runs `cargo fmt -p <crate> -- --check` for
   each, and fails if any workspace member is missing from `fmt-clean.txt` and
   `fmt-debt.txt`, so new crates must be classified), `cargo check --workspace
   --locked`, `cargo test --workspace --locked` (exclusions per Assumptions),
   conditional clippy, and a `workflow_dispatch`-only step running the bundle
   smoke. `fmt-debt.txt` lists the 29 drifting members as known debt for a later
   mechanical order.
2. `vexel-dev` subcommands (std only, no new dependency): `bundle [--out DIR]
   [--icon FILE]` builds `vexel-app` in release (`--locked --offline`, repo-local
   `CARGO_HOME`/`CARGO_TARGET_DIR` as the existing `build` does) and assembles
   `Vexel.app` (`Contents/MacOS/vexel`, `Info.plist` with
   `CFBundleExecutable`, `CFBundleIdentifier`, `CFBundleShortVersionString` from
   the workspace version, `LSMinimumSystemVersion` 13.0, `NSHighResolutionCapable`,
   `Contents/Resources/en.lproj/Localizable.strings` copied from `resources/`,
   optional icon), then ad-hoc signs (`codesign --force --sign - --timestamp=none`)
   and verifies (`codesign --verify --strict`); `dmg` runs `hdiutil create
   -format UDZO -ov` and `hdiutil verify`; `sbom` runs `scripts/make_sbom.py`
   (fix it minimally if the C removal broke it, listing the change);
   `install-smoke` mounts the DMG read-only at a temp mount point, copies the app
   to a temp "Applications" directory, runs `plutil -lint`, `codesign --verify
   --strict`, `lipo -archs` (must contain `arm64`), checks the executable bit,
   detaches in all paths (including failure) and never launches the GUI.
   Pure helpers (plist rendering, path planning, argument parsing) are unit
   tested; shelling out is behind a small trait so tests use a recording fake.
3. Fuzz/robustness (deterministic, no external crate): a shared xorshift64*
   generator and mutator (bit flip, byte replace, delete range, duplicate range,
   truncate, token-dictionary insert) copied into each test file (no shared crate;
   keep each under 200 lines). Targets: `vexel-xml` tokenizer,
   `vexel-svg-engine` decode (seeds from `bench/corpus/fixtures/*.svg` read at
   test time), `vexel-protocol` `handle` with a stub `ProtocolBackend`, and
   `vexel-mcp-stdio::serve` over mutated line streams. Defaults: seed
   `0x5EED_0001`, 2000 cases per target; `VEXEL_FUZZ_SEED` and `VEXEL_FUZZ_CASES`
   override for reproduction. A failure prints seed, case index and the input
   (hex, first 256 bytes).
4. Perf gate: `rust/crates/vexel-app/tests/perf_gate.rs` (new file) times three
   deterministic synthetic workloads (XML tokenize ~1 MiB, `compose_svg` of a
   generated 2000-rect document at 512x512, 1000 `tools/list` round trips)
   against `rust/ci/perf-baseline.json` (host string, per case `baseline_ms`
   measured once by the worker and `max_ms` = at least 8x baseline and never
   under 250 ms). The test fails only above `max_ms`; `VEXEL_PERF_PRINT=1` prints
   measured values; it never rewrites the baseline.

## Non-goals
Developer ID signing, entitlements with real identities, notarization, stapling,
update channels, a release workflow, GitHub Release publishing, Linux/Windows
anything, performance targets as claims, mass formatting, clippy fixes, any
change to crate source outside the listed paths, running a GUI session.

## Files allowed to touch
- `.github/workflows/ci.yml`, `scripts/check_ci.py`, `scripts/check_rust_fmt.py`
  (new), `scripts/make_sbom.py`
- `rust/ci/**` (new: `fmt-clean.txt`, `fmt-debt.txt`, `perf-baseline.json`)
- `rust/tools/vexel-dev/src/**`, `rust/tools/vexel-dev/tests/**` (formatting of
  this crate is allowed; `Cargo.toml` is coordinator-only)
- new test files only: `rust/crates/vexel-xml/tests/mutation_fuzz.rs`,
  `rust/crates/vexel-svg-engine/tests/mutation_fuzz.rs`,
  `rust/crates/vexel-protocol/tests/fuzz_jsonrpc.rs`,
  `rust/crates/vexel-mcp-stdio/tests/fuzz_stdio.rs`,
  `rust/crates/vexel-app/tests/perf_gate.rs`
- `artifacts/WO-RUST-V1-RELEASE-CI-01/**`

## Protected paths
Workspace/lock manifests and every `Cargo.toml` (a needed dev-dependency is a
stop-and-report), all existing sources and tests of every crate, `tests/`,
other workflows, C/ObjC, goldens, oracles, `docs/**`, `STATE.md`, `LEDGER*.md`,
`ledger.jsonl`, other work orders, git HEAD/index. No unsafe, external
dependencies, network, installs (`brew`, `rustup component add`), secrets,
credentials, notarization, GUI launch, commit, stage, reset or deletion.
Build outputs go under the repo-local ignored `build/` directory.

## Acceptance criteria
1. `python3 scripts/check_ci.py .github/workflows/ci.yml` passes; `rust/ci/
   fmt-clean.txt` plus `fmt-debt.txt` exactly partition the workspace members
   (a unit test or the script's self-check proves it) and every crate in
   `fmt-clean.txt` passes `cargo fmt -p <crate> -- --check`.
2. `scripts/check_rust_fmt.py` fails with a clear message on an unclassified
   member and on a drifting clean-listed crate (tested with temp fixtures).
3. `cargo run -p vexel-dev -- bundle`, `dmg`, `sbom` and `install-smoke` succeed
   on the worker host; evidence logs list the produced `Vexel.app`, `codesign
   --verify` output, `hdiutil verify` result and the SBOM path. The smoke
   leaves no mounted volume (`hdiutil info` before/after).
4. Unit tests for plist rendering (key set, XML-escaping, version), argument
   errors and the fake-runner sequence of `install-smoke` including detach on a
   forced failure.
5. Each fuzz target passes with default seed and 2000 cases in under 60 s in
   debug; a deliberately failing injected property (test-only, `#[ignore]`d
   `reproducer`) shows the seed/index/hex report format. Output of every target is
   either a typed error or valid output: xml offsets monotonic and in range;
   engine returns `Ok`/typed `Err`; protocol replies parse with `vexel-json`
   or are empty for notifications; stdio never panics and always terminates.
6. Perf gate passes on the worker host; `perf-baseline.json` records host and
   three cases with `max_ms` per the rule above; the test never writes it.
7. Existing tests of every touched crate pass unmodified; test counts only
   increase; `cargo fmt -p vexel-dev -- --check` is clean.
8. `cargo tree` shows no new external package; no `unsafe`; files < 800 lines.
9. `result.md` (<=350 words) states: clippy available or not, whether
   `vexel-app` tests were added to CI and why, that signing/notarization and a
   live launch are unexecuted, the placeholder bundle id and missing icon, and
   the measured baseline values.

If an AC cannot be met, stop with the exact blocker; no test weakening and no
threshold weakening beyond the stated rule.

## Verification (frozen; repo-local CARGO_HOME/CARGO_TARGET_DIR, locked, offline)
- `python3 scripts/check_ci.py .github/workflows/ci.yml`
- `python3 scripts/check_rust_fmt.py`
- `cargo +stable check --workspace --locked --offline`
- `cargo +stable test -p vexel-dev -p vexel-xml -p vexel-svg-engine -p vexel-protocol -p vexel-mcp-stdio -p vexel-app --locked --offline`
- `cargo +stable run -p vexel-dev --locked --offline -- bundle`, then `dmg`,
  `sbom`, `install-smoke`
- `python3 scripts/wo_validate.py WO-RUST-V1-RELEASE-CI-01 --ready`
- `python3 scripts/ledger.py validate ledger.jsonl`
- `git diff --check`

## Evidence
`artifacts/WO-RUST-V1-RELEASE-CI-01/`: `result.md`, `verification.json`,
`ac-map.json`, `changed-path-audit.json`, `bundle.log`, `sbom.json`.

## Worker and review
Worker: gpt-6-luna High, one writer, existing allowance only; stop on quota
errors. Candidate only until a fresh read-only reviewer reruns the frozen
commands and checks each AC. The coordinator registers nothing here, integrates
the patch, reruns main-tree checks, performs the live launch of the bundled app
and updates STATE.md, LEDGER.md and ledger.jsonl. Signing identity,
notarization and the final bundle id/icon are owner decisions.
