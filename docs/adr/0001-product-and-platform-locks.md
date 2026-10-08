# ADR 0001: Product and platform locks

- Status: accepted
- Date: 2026-10-08
- Decision owner: project owner

## Context

The first product proof targets large static SVG work on Apple Silicon. Keeping
the initial surface narrow reduces platform and dependency risk while the
parser, renderer, and editing model are still being proven.

## Decisions

- Project name: `vexel`; executable: `svgworkbench`.
- License: Mozilla Public License 2.0 (MPL-2.0), text in root `LICENSE`.
- Initial platform: Apple Silicon, macOS 13 or later only. Do not add other-OS
  code, stubs, CI jobs, runners, packaging, or portability work.
- Language/build: ISO C17, CMake presets, Ninja, Apple Clang; minimal
  dependencies and no configure-time downloads.
- UI: custom drawn UI, with native platform services behind `platform/`
  interfaces.
- SVG: implement the first parser and rasterizer in this project. Preserve
  source, report unsupported features, and do not fetch external resources.
- Animation and other platforms remain outside the initial product proof.

## Consequences

The build rejects non-macOS and non-Apple-Silicon targets. Any later expansion
requires explicit owner direction and a superseding ADR. Dependencies require a
separate justification, license review, pinned source hash, and adapter boundary.

## Sources

- Owner-provided `OWNER-DIRECTIVES.md` (platform boundary).
- `docs/research-r4/05-revised-roadmap.md` (ordered work and gates).
- Mozilla, [MPL 2.0 license text](https://www.mozilla.org/MPL/2.0/index.txt).
