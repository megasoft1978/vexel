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
- C17, MPL-2.0, minimal dependencies, custom-drawn UI, and an owned SVG
  parser/rasterizer are locked decisions. See `docs/adr/`.
- Never read, fetch, or copy GPL SVG-editor source. Do not fetch external SVG
  resources from the application. Record provenance for external algorithms.

## Repository map

- `app/`: executable composition.
- `src/`: base, platform, XML/SVG, scene, raster, document, UI modules as added.
- `docs/research-r4/`: authoritative operating model and roadmap.
- `docs/research-r3/plan/`: architecture and coding guidance where r4 is silent.
- `docs/adr/`: accepted decisions; `work/`: immutable ready work orders.
- `tests/`: unit, property, fuzz, UI, and oracle tests as introduced.
- `bench/`: corpus metadata, generators, runners, and results.
- `artifacts/`: work-order evidence; `build/` and `.cache/` are generated.

## Commands

- `./dev build`: configure and build Debug.
- `cmake --preset mac-release && cmake --build --preset mac-release`: Release.
- `python3 scripts/ledger.py validate ledger.jsonl`: validate work history.
- `python3 -m unittest`: run Python harness tests when present.
- `./dev test`, `bench`, `fmt`, `lint`, `asan`, and `fuzz` become authoritative
  only when their work orders implement them; do not claim missing commands run.

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
