# 05 — C code architecture and contributor rules (Q7)

## Module boundaries and dependency direction

```text
app_shell -> platform_{mac,win,linux} -> OS APIs
    │                │
    ├── ui_core -> ui_layout -> draw_list -> render_{cpu,metal,d3d,vulkan}
    ├── editor -> document -> xml_source / style / svg_scene / geometry
    ├── file_io -> document (source spans + patch writer)
    └── workers -> immutable parse/render jobs

Allowed core dependencies: base -> alloc/string/math -> xml -> css/svg -> geometry -> scene -> editor.
Forbidden: XML/parser calling UI; SVG core importing OS APIs; renderer mutating document; platform shim knowing SVG nodes.
```

Suggested modules: `base/{alloc,arena,str,slice,hash,log,assert}`, `platform`, `ui/{layout,widgets,text,accessibility}`, `xml/{tokenize,index,namespace}`, `css/{parse,cascade}`, `svg/{path,transform,units,resource,scene}`, `geom/{flatten,hit,bounds}`, `render/{cpu,metal,tiles,composite}`, `document/{model,oplog,serialize,autosave}`, `editor/{selection,tools,layers}`, `bench`, `tests/fuzz`.

## C conventions

- C17 baseline; `snake_case` functions and fields, `PascalCase` typedef names only if project convention chooses it consistently; public prefixes (`svg_`, `ui_`, `plat_`). Opaque handles at module boundaries; private structs in `.c` when practical.
- Header declares contract/types; `.c` holds implementation. Keep module API small. Use unity build only as optional CI/build speed optimization after ordinary translation-unit boundaries work; unity builds hide missing includes/static symbol collisions and can worsen incremental rebuilds. LTO gives cross-TU optimization without collapsing source boundaries.
- Strings/slices are `(const char *ptr, size_t len)`; never assume input is NUL terminated. `StrSpan` refers into immutable source; ownership explicit in type/name (`*_view`, `*_owned`, `*_copy`). UTF-8 validation rules documented.
- Arena API has lifetime label: document, frame, job, undo, scratch. Reset frame/scratch arenas only at boundaries; no pointers into reset arenas. Document IDs are stable handles with generation; avoid raw pointer identity across realloc/async boundaries.
- Error returns: `bool`/enum status and caller-owned `Error { code, source_span, message_slice }`; no hidden global last error, no `longjmp` across ownership boundaries. Use assertions for programmer invariants, not malformed input checks.
- No malloc/free in hot paint/layout/hit-test loop. Preallocate frame command arenas, job scratch and tile buffers; allocation counters in debug. Document growth is chunked and observable.
- Worker jobs consume immutable snapshots or generation-checked handles; UI thread owns mutable document commits. Cancellation token and stale-result check required.
- Compile flags baseline: `-std=c17 -Wall -Wextra -Wpedantic -Werror` (compiler-specific allowlist), debug `-O0 -g`, release `-O2` or `-O3` after measurement. Separate `-Os`/`-Oz` binary profile from speed profile.
- CI: ASan + UBSan on tests/fuzz corpus; MSan where supported; fuzz XML tokenization/path parser/CSS/path geometry; static analyzer `clang --analyze`/scan-build, `clang-tidy` selectively, cppcheck if useful. Fuzz malformed UTF-8, deep nesting, entities, float edge cases, arc flags and huge counts. Warnings-as-errors applies to owned code; vendor code has isolated warning suppressions.
- Tests: small deterministic C unit tests with golden input/output and explicit allocator; property tests for transforms/path parse/serialize; fuzz tests; image fixtures with exact hash for deterministic CPU renderer and tolerance/perceptual diff for GPU. Don't add benchmarks to unit suite; benchmark separately.
- SIMD: scalar reference first; compile-time or runtime dispatch, no SIMD types in public ABI; align/unaligned behavior explicit; force scalar test target.

## Build and portability

Use CMake or Meson for multi-platform configuration, plus a thin `build.sh` convenience wrapper. A single Makefile is fine for local macOS MVP but becomes awkward for Windows project generation and platform libraries. `zig cc` can be a useful cross-compiler experiment for C targets, but Apple SDK/signing and Windows SDK/DirectWrite import libs still need appropriate SDKs; do not promise one host can emit release-grade signed artifacts for every platform without verification.

Reproducible build: pin compiler version, SDK baseline, dependency commits, generated shaders/protocol headers, locale, timestamps, and build flags; emit SBOM/license manifest and build provenance. Linux builds should use minimum supported glibc/sysroot or provide static libraries only when legally/technically suitable.

macOS: universal arm64+x86_64 optional; compare increased executable/resources size and test both. Link system frameworks, use `-dead_strip`, LTO, `-Os` only for release-size target, strip symbols, keep dSYM separately, inspect with `size`, `otool -L`, `codesign --verify`. Sign Developer ID and notarize/staple for distribution. No-libc static macOS is not a realistic GUI route. UPX compression is not recommended for signed/notarized macOS apps; it modifies Mach-O and can interfere with signing/Gatekeeper, and runtime decompression gives no ordinary installed-size benefit once packaged/compressed.

Windows: MSVC is the product support compiler; clang-cl can be secondary. `/GL` + `/LTCG`, `/OPT:REF /OPT:ICF`, `/Brepro`, `/DEBUG:FULL` side symbols. Do not assume MinGW equivalence for shell COM, TSF and UIA.

Linux: clang or GCC; `-ffunction-sections -fdata-sections -Wl,--gc-sections`, LTO, strip separate debug; avoid static glibc. Build on oldest supported distro baseline and test Wayland/X11 independently.

## Proposed repository tree

```text
AGENTS.md
CLAUDE.md                  # short pointer to AGENTS.md, no duplicate policy
CODING_STANDARD.md
README.md  docs/adr/       docs/architecture/
build.sh CMakeLists.txt cmake/ toolchain/
src/base/ src/platform/{mac,win,linux}/ src/ui/ src/render/{cpu,metal,d3d11,vulkan}/
src/xml/ src/css/ src/svg/ src/geom/ src/document/ src/editor/ src/app/
vendor/                    # only allowlisted, versioned, license-noted deps
tests/unit/ tests/corpus/ tests/fuzz/ tests/golden/
bench/{corpus,generators,scripts,baselines,reports}/
```

## One-page CODING_STANDARD outline

1. **Purpose/scope:** C17, minimum deps, compatibility first, measured optimization.
2. **Naming/API:** prefixes, slices, opaque handles, `const`, no public ownership ambiguity.
3. **Ownership/lifetimes:** arena categories, who frees, async generation checks, mapped source lifetime.
4. **Error/contract:** status codes and spans; assertions vs hostile input; no hidden global error.
5. **Hot paths:** no allocation in render/layout/hit-test; cache/invalidation and counters.
6. **Concurrency:** immutable snapshots, UI-thread commits, cancellation and shutdown ordering.
7. **Portability:** OS code only under platform module; no OS headers in portable core; Unicode/dpi/time rules.
8. **Testing/security:** unit/golden/fuzz/sanitizer/analyzer commands, external resource policy, limits.
9. **Formatting/build:** formatter, compiler warnings, debug/release/small-size profiles, generated files.
10. **Review:** complexity/benchmark evidence for optimizations; license/provenance for added code and assets.

## AGENTS.md / CLAUDE.md outline

- State product decisions (C17, basic editor, no widget dependency) and non-goals; don't reopen fixed choices absent hard evidence.
- “Before coding”: read module README/ADR; check ownership boundaries and source-of-truth; run only requested verification (benchmark/test commands described, never silently expand scope).
- “Architecture rules”: dependency direction above; portable core cannot import platform headers; all UI events normalized; renderer consumes immutable display lists.
- “C safety”: no implicit ownership; no parser trust; no allocation hot paths; no raw pointer retained by worker; error path preserves source.
- “Generated/vendor”: never edit generated files by hand; vendor requires license/version/size and reason in `vendor/README.md`.
- “Output”: include changed modules, risk, build command and whether tests/benchmarks were run; don't claim performance without baseline artifact.
- Keep `CLAUDE.md` as a short compatibility pointer to `AGENTS.md` to prevent drift.
