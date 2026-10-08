# 04 — Coding standard and contributor instructions

This is normative. Revisions require review and a dated ADR when they affect ABI, ownership, threading or source preservation.

## Style and names

- C17, four-space indentation, UTF-8 source, LF, 100-column soft limit; clang-format checked in.
- `snake_case` for functions/variables, `lower_snake_case` structs/enums, `UPPER_SNAKE_CASE` constants/macros. Module prefix all exported symbols (`svg_`, `ui_`, `raster_`). Avoid abbreviations except `id`, `utf8`, `rgba`, `xml`, `svg`.
- One function does one operation. Early returns for errors; no deeply nested control flow. Boolean names read as predicates.
- Public APIs use opaque handles and value structs; no compiler vector, OS, or vendor types in portable headers.

## Ownership and memory

- Every pointer comment or type name states borrowed/owned and lifetime. `*_create` returns owned handle; `*_destroy` consumes it. Borrowed view is valid only for documented owner lifetime.
- No unowned pointer crosses thread boundaries. Use immutable snapshot refs plus generation handles. Never retain pointers into growable vectors.
- Allocation only via module allocator/arena. No `malloc` in hot loops. Every size calculation uses checked add/multiply. Explicit per-document and per-cache limits. OOM returns error without partial commit.
- No VLAs, `alloca`, `strcpy`, `strcat`, `sprintf`, unsafe format construction, or implicit null-termination assumptions. Strings carry lengths; validate UTF-8 at boundaries.
- Avoid recursive descent over user-controlled depth; if recursion is essential, check/decrement explicit depth budget.

## Errors, assertions, and parsing

- Return `app_status`; populate optional bounded `app_error`. Error text is for diagnostics, not logic.
- Assertions are for impossible internal invariants only. Malformed SVG, limits, unsupported features and OOM are normal runtime cases.
- Parser never loads network resources or executes script. Preserve unknown source spans. Every expansion path has a byte/count/depth/pixel limit.
- Integer overflow, NaN/Inf, zero divisors, invalid matrix, path arc flags and malformed UTF-8 have explicit policy and tests.

## Concurrency and rendering

- UI/document mutation only on UI thread. Workers read snapshots and return owned results tagged with generation.
- Jobs support cancellation and bounded queues; shutdown order: cancel → drain/join → release snapshots → unmap source.
- Renderer code is deterministic in scalar mode; no allocation inside pixel loops. Scalar implementation is reference for every SIMD path.
- SIMD uses private wrappers and aligned/unaligned explicit loads. Test scalar and NEON on identical golden input. No SIMD ABI leakage.
- Record trace counters for parse, index, style, flatten, bin, tile, upload, present, cache hits, bytes and allocations.

## Comments and file limits

Comments explain invariants, units, lifetime, coordinate spaces and non-obvious algorithms—not restate code. Public functions have brief contract comments. Every math API states coordinate and color space. Target source files ≤800 lines; split at cohesive ownership boundaries by 1,500; exceeding 3,000 requires ADR. No generated/vendor changes without their source/update record.

## Build and verification commands

```sh
cmake --preset mac-debug && cmake --build --preset mac-debug
ctest --preset mac-debug
cmake --preset mac-asan && cmake --build --preset mac-asan && ctest --preset mac-asan
cmake --preset mac-release && cmake --build --preset mac-release
./build/bin/svgbench --manifest bench/corpus/m0.json --json out.json
./build/bin/svgfuzz-smoke --corpus tests/corpus
```

Commands become executable when presets/tools land; never imply a command works before it exists. Benchmarks are separate from unit tests. For each PR: report commands run and skipped; no test/benchmark is run implicitly by an agent unless requested in that task or CI policy.

## Review checklist

- Does data remain source-preserving, including unknown namespace data?
- Are ownership/lifetime/thread boundaries stated and checked?
- Are all sizes, depth and expansion bounded with overflow checks?
- Does failure leave old document usable?
- Is scalar behavior the reference and output deterministic?
- Does UI state expose role/name/value/focus and keyboard route?
- Is the feature in the support matrix and corpus?
- Any third-party code/assets? Record exact provenance/license/hash.
- Does a hot-path optimization have a before/after artifact on pinned workloads?
- Is there any hidden platform dependency in portable headers?

## `AGENTS.md` draft

```md
# Agent rules
Read README.md, CODING_STANDARD.md, relevant docs/adr and module README before edits.
Product lock: C17, Mac first, one custom-drawn UI, Metal texture presentation, SVG source preservation, basic editor before animation. Do not reopen a locked choice without evidence of a hard blocker.
Map: src/base is dependency root; parser/style/scene/raster are portable; platform/* are OS adapters; app composes; tests and bench are separate.
Do not add a UI toolkit. Do not copy Inkscape code. Do not drop unknown XML or fetch external SVG resources. Do not use raw pointers across worker boundaries.
When adding a module: add public header, internal implementation, README with ownership/API, unit tests, corpus case if relevant, CMake entry, and dependency diagram update.
Before claiming verification, run only commands requested by the task or project CI. Report exact commands and results; do not invent benchmark figures.
When adding vendor code, pin URL/tag/hash and license in third_party/README.md and keep it behind an adapter.
Use `cmake --preset mac-debug`, `ctest --preset mac-debug`, and bench scripts once available. Keep changes cohesive, readable, and below file-size limits.
```

## `CLAUDE.md` draft

```md
# Claude Code
Project instructions live in `AGENTS.md` and `CODING_STANDARD.md`; follow them exactly. Read those files and the relevant ADR/module README before editing. C17, one custom UI, Mac first, preserve SVG source. Do not add a widget framework or claim performance without pinned benchmark evidence. Use the project's explicit build/test/bench commands only when requested by the task or required by CI. Report changed modules, risks, and commands actually run.
```
