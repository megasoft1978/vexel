# Agent rules

Read [OWNER-DIRECTIVES.md](OWNER-DIRECTIVES.md) first. It overrides conflicting
roadmap and tooling text. Follow the active `work/WO-*.md`; inspect its allowed
and protected paths before editing. Keep each work order buildable, tested,
verified, and recorded in `STATE.md` and the ledgers.

## Platform and product locks

- Initial support is Apple Silicon macOS 13+. Do not add Linux/Windows code,
  shims, CI jobs, runners, packaging, or portability work.
- Keep platform implementations behind `platform/` interfaces; do not add
  stubs for unsupported operating systems.
- Rust for all first-party code is the active owner-approved migration target.
  MPL-2.0, minimal pinned dependencies, custom-drawn UI, and an owned SVG
  parser/rasterizer remain locked. The legacy C17 code was removed after V1; it remains at tag `legacy-c-final` as a migration
  reference; do not resume old C implementation orders. See ADR-0022.
- Never read, fetch, or copy GPL SVG-editor source. Do not fetch external SVG
  resources from the application. Record provenance for external algorithms.

## Repository map

- `app/`: executable composition.
- `src/`: base, platform, XML/SVG, scene, raster, document, UI modules as added.
- `docs/research-r4/`: historical operating model and roadmap (superseded by the Rust
  plan in `docs/product/`); older material is in `docs/archive/`.
- `rust/`: Cargo workspace (`crates/`, `tools/`, `fixtures/`); the active implementation.
- `docs/research-r3/plan/`: architecture and coding guidance where r4 is silent.
- `docs/adr/`: accepted decisions; `work/`: immutable ready work orders.
- `tests/`: unit, property, fuzz, UI, and oracle tests as introduced.
- `bench/`: corpus metadata, generators, runners, and results.
- `artifacts/`: retained harness evidence (see `artifacts/README.md`); older evidence is in
  git history at tag `legacy-c-final`. `build/` and `.cache/` are generated.

## Commands

Rust is the only implementation. From the repository root (macOS, pinned toolchain):

- `cargo check --workspace --locked` and `cargo test --workspace --locked`
- `cargo run -p vexel-app` runs the editor; `--mcp-preview` serves MCP over stdio;
  `--grant-dir <dir>` authorizes a directory for MCP file access.
- `python3 scripts/ledger.py validate ledger.jsonl` validates work history;
  `python3 scripts/wo_validate.py --ready WO-ID` validates a frozen work order;
  `python3 -m unittest tests.test_ledger tests.test_wo_runner` runs the harness tests.
- The legacy C workflow (`./dev`, CMake) was removed; the last tree containing it is
  tag `legacy-c-final`.

## Engineering and safety

Use the contracts in `docs/CODING_STANDARD.md`: explicit ownership, pointer+length
slices, checked size arithmetic, bounded user input, no VLAs, and no unsafe
string functions. Preserve unknown SVG/XML source. Keep generated files in
ignored repo-local directories. Do not install system packages or touch sibling
directories. Add dependencies only through an ADR and pinned provenance record.

Never weaken frozen acceptance tests or oracles. Run commands named by the
active work order, then report exact evidence. Do not invent test or performance
results. Keep source files focused and under 800 lines; split cohesive modules
before they grow further.

## Rust migration execution

Follow `docs/product/rust-port-goal-and-plan.md` and ready Rust work orders.
Multiple Luna codex exec processes are authorized, with isolated current-source
worktrees and disjoint modules; coordinator alone changes shared manifests,
locks, interfaces and ledgers. Use Low for inventory/docs, Medium for routine
ports, High for parser/renderer/concurrency/unsafe FFI. Escalate only on a named
problem; use the existing subscription allowance and no purchased credits.
Preserve strict validation, original fixtures/assertions and byte-preserving
round trips. Rust ownership replaces manual C ownership conventions; bounded
inputs, checked arithmetic, safe cleanup and source limits still apply. Keep
unsafe operations behind documented platform/FFI boundaries.
