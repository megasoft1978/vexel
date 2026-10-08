# Repository instructions

Read [OWNER-DIRECTIVES.md](OWNER-DIRECTIVES.md) first; it sets the platform
boundary and overrides conflicting roadmap text. vexel is macOS-only initially
(macOS 13+, Apple Silicon). Keep platform-specific code behind `platform/`
interfaces; do not add other-OS code, CI jobs, runners, shims, or portability
work.

## Map and commands

- `app/`: executable composition; current entry point is `app/main.c`.
- `docs/research-r4/`: authoritative operating model and ordered roadmap.
- `docs/research-r3/plan/`: architecture and coding guidance where r4 is silent.
- `work/`: active work orders; `OWNER-DIRECTIVES.md` takes precedence.
- `./dev build`: configure and build the macOS debug preset.

Use C17, keep dependencies minimal, and do not read, fetch, or copy GPL SVG
editor source. Preserve all user-provided files and keep generated output under
ignored `build/` or `.cache/`. For each work order, inspect its allowed paths,
acceptance criteria, and protected paths before editing. Do not claim tests,
sanitizers, or performance results that were not run. Record external algorithm
provenance in `docs/provenance.md` before introducing one.
