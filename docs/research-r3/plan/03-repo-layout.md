# 03 — Repository layout

```text
AGENTS.md                         LLM/project rules and commands
CLAUDE.md                         Short pointer to AGENTS.md
README.md                         Product promise, build, support matrix
LICENSE                           Chosen license
NOTICE                            Third-party notices
CODING_STANDARD.md                Normative C17 rules
CMakeLists.txt                    Cross-platform source/build graph
CMakePresets.json                 macOS debug/release/size presets
build.sh                          Convenience configure/build wrapper
cmake/                            Compiler checks, options, toolchain helpers
  warnings.cmake                  Warning sets and reviewed suppressions
  sanitizers.cmake                Sanitizer profiles
  platform_macos.cmake            Framework linking
src/
  base/                           slices, status, arenas, checked math, UTF-8
  platform/                       normalized OS service interfaces
    mac/                          Cocoa/Metal/CoreText/IME/accessibility shim
    win/                          Win32/DirectWrite/TSF/D3D11 shim
    linux/                        X11/Wayland/xkbcommon/portal shim
  gfx/                            texture and presentation abstraction
  raster/                         CPU scan conversion, tile workers and cache
  text/                            shaping interface + CoreText first backend
  xml/                             safe tokenizer and source spans
  svg/                             SVG subset parser, ids, limits, diagnostics
  css/                             tokenizer, selector subset and cascade
  scene/                           immutable display list + transform resolution
  spatial/                         broad-phase BVH/cell index and hit testing
  document/                        source mapping, edit overlay, snapshots
  editor/                           commands, selection, tools, edit transactions
  ui/                               semantic tree, layout, paint, widgets
  app/                              composition, main, frame lifecycle
third_party/
  README.md                        allowlist, reasons, license and pinned hashes
  plutovg/                         optional raster spike only, MIT, pinned
  utf8proc/                        only if localization requires it, license reviewed
  test_fonts/                      license manifest and hashes
  test_corpus/                      manifest, hashes, provenance; no unclear assets
tests/
  unit/                              focused unit tests by module
  property/                          generated math/path/round-trip properties
  golden/                            SVG input, PNG golden, manifest
  fuzz/                              libFuzzer harnesses
  ui/                                scripted events/tree assertions
  corpus/                            small redistributable fixtures
bench/
  corpus/                            benchmark manifests and generators
  scripts/                           repeatable CLI and interaction scripts
  baselines/                         machine/build-tagged JSON; no hand-edits
  reports/                           generated reports
perf/                                 counters, trace events, signposts
shaders/                              Metal blit shader and generated headers
docs/
  adr/                                accepted/rejected decisions
  architecture/                       diagrams, ownership, threading
  compatibility/                      supported SVG matrix and loss policy
  security/                           limits, threat model, reporting
  release/                            packaging/notarization instructions
tools/
  svg_corpus_manifest.c              verify hashes/licenses
  render_batch.c                      deterministic render CLI
  svg_gen.c                           stress corpus generator
  size_report.py                     executable and bundle inventory
```

Purpose: no source module exceeds 3,000 C lines without an ADR; target files 200–800 lines. Keep one public header per module and `*_internal.h` private. Include guards `APP_BASE_SLICE_H`; exported symbols prefixed by module. Unity build is **off** initially; enable only for measured compile/link benefit and never use it to hide missing includes/static collisions. No PCH. CMake is default; generated shader/protocol headers are reproducible and checked for drift.

Vendor policy: upstream URL, exact commit/tag and SHA256 archive, license text, source count/size, rationale, supported feature gap, security history, patch list in `third_party/README.md`; no vendored package without maintainer and update date. Keep vendor behind adapter API. Corpus files need explicit redistribution rights; private user files stay out of Git/CI artifacts.
