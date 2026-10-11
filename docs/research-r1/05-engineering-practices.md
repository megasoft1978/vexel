# Engineering practices, safety and licensing (Q10–Q11)

## Repository and module boundaries

```text
/app/                 platform entry points, lifecycle, menus, files
/ui/                  toolkit widgets, commands, accessibility, shortcuts
/document/xml/        loss-preserving XML parser, writer, namespace model
/document/svg/        SVG semantics, CSS cascade, resource references
/document/model/      editing commands, selection, undo, document IDs
/geometry/            paths, transforms, bounds, booleans, snapping
/render/display-list/ immutable render graph and cache keys
/render/backend/      CPU/GPU adapters; renderer contract only
/text/                font discovery, HarfBuzz shaping, glyph layout
/io/                  PNG/PDF/SVGZ import/export and asset security
/animation/           time model, tracks, easing, SMIL/CSS compilers
/extensions/          narrow stable C ABI/plugin host
/platform/macos/      AppKit integration, Metal, signing scripts
/platform/windows/    Windows input, fonts, packaging
/platform/linux/      GTK/desktop integration, packaging
/tests/fixtures/      SVG, Inkscape SVG, browser, hostile and huge files
/docs/                architecture decision records, module guides, formats
```

Dependency direction: UI → document commands → model/XML/geometry; document → renderer interface but never toolkit; renderer backends never mutate source XML; animation compiler emits source nodes through document commands. Keep module public APIs small and opaque. Each module has an owner and a “what is canonical / who frees it / thread rules” page.

## Coding and ownership standard

- Formatting: clang-format pinned in repo; one naming convention (`snake_case` C functions, `*_t` types, `k`/`enum` constants consistently), explicit prefixes by module, include guards, SPDX headers.
- Header discipline: declarations only in headers; include only required headers; no private struct layout across modules; no cyclic includes; generated API docs from public headers.
- Error handling: return a `status_t`; optional structured `error_t` carries code, source URI, line/column and context; no silent fallback, no exceptions crossing C ABI, no `abort()` on malformed documents.
- Ownership: document owns source tree; node IDs stable for a document generation; weak references are IDs, not raw pointers; transient render snapshot owns its cache; workers take immutable snapshots and deliver generation-tagged results; UI thread owns toolkit objects.
- Memory: checked arithmetic for all untrusted sizes; explicit resource budgets for XML depth, nodes, text bytes, path segments, image pixels, filter surfaces, external resources and time. Never fetch external XML/image/CSS by default without user consent and a sandbox/policy.
- Async: cancellation tokens and document revision IDs; stale background renders cannot replace a newer frame. Undo commands are atomic and tested for redo symmetry.
- C-specific: no borrowed pointer without lifetime in API docs; use a single documented allocator per owning context; expose destroy functions; avoid macro metaprogramming; encapsulate vectors/strings and bounds-check all indexes.

## CI and verification

| Area | Required pipeline |
|---|---|
| Platforms | macOS arm64 + x86_64/universal packaging; Windows x64; Linux x86_64 + arm64 build where practical. Test actual GUI on at least one hosted/self-hosted machine per OS. |
| Build | CMake or Meson with pinned compiler/toolchain presets, reproducible dependency versions, offline-capable cache, debug/release and shared/static library configurations. CMake is safer ecosystem default; Meson is concise and strong in C/GTK ecosystems. Do not maintain parallel build systems without need. |
| Memory/concurrency | ASan + UBSan on parser/document/geometry; TSan on render workers and undo; MSVC ASan or equivalent where supported. Run fuzzers with sanitizer builds. |
| Fuzzing | libFuzzer for XML/SVG/CSS/path/filter/image decoders; AFL++ as optional corpus exploration. Seed with WPT/resvg/Inkscape files; save minimized regressions. Parser must handle external entities, recursion, huge dimensions, cyclic references and malformed UTF-8 safely. |
| Static analysis | clang static analyzer, clang-tidy with a short rule set, cppcheck; compiler warnings as errors for owned code. Coverity/Sonar optional if budget permits. |
| Format/render regressions | Render fixed SVG corpus at canonical sizes; compare pixel diffs with tolerances and perceptual metrics; compare metadata/XML round trip separately. Store browser raster references and renderer version. |
| Performance | Benchmark load/first paint, pan/zoom p50/p95, selection/hit-test, style change, undo, export and peak RSS on fixed hardware/corpus. CI alerts on regression, not hard timing on noisy shared runners. |
| Release budgets | Track signed package compressed/uncompressed bytes, executable text/data, cold start, steady-state RSS, peak RSS per benchmark file, and dependency SBOM. Fail CI if budgets exceed agreed thresholds without accepted ADR. |
| Accessibility | Automated tree checks plus keyboard-only and screen-reader manual test on macOS VoiceOver, Windows Narrator, Linux Orca; test IME composition, RTL text, zoom and contrast. |

**Tests to include:** W3C Web Platform Tests SVG reftests (adapt because browser-specific HTML test harness may not apply to editors), resvg regression fixtures (subject to license and attribution review), Inkscape’s own file/test fixtures, real user documents with permission, and our own generated malformed/huge fixtures. WPT explicitly notes editors may need bespoke test steps. [WPT SVG testing](https://github.com/w3c/svgwg/wiki/Testing) · [resvg tests](https://github.com/linebender/resvg) · [Inkscape tests](https://gitlab.com/inkscape/inkscape/-/tree/master/testfiles)

## LLM-friendliness and maintainability

- Keep modules small enough to explain; write `README.md` per module with invariant, ownership, errors, data flow, and one minimal example.
- Add root `AGENTS.md` and `CONTRIBUTING.md`; document commands, style, test boundaries, file ownership, generated-code rules and “do not edit” artifacts. Keep instructions specific and link to module guides.
- Use architecture decision records for language, XML model, renderer, UI, license and animation representation. Every generated API must say source-of-truth and regeneration command.
- Keep fixtures and expected outputs reviewable; avoid opaque binary test-only assets. Use deterministic IDs/serialization in golden output.
- Prefer one idiom per subsystem, explicit names, small functions and clear ownership over maximal abstraction. Require reviewer-visible rationale for `unsafe`, raw pointer ownership, parser relaxation and external resource loading.
- LLMs can help implement/triage but do not create a clean-room legal process by themselves; maintain human review and provenance records.

## macOS release/signing

Build arm64 and x86_64 (universal only if dependencies and performance support it); produce signed `.app` and DMG/PKG, Developer ID sign, submit for notarization, staple ticket and verify Gatekeeper on a clean machine. Apple documents Developer ID signing, `notarytool` and `stapler`; notarization status is a release gate, not a code-quality claim. [Apple notarization](https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution) · [workflow](https://developer.apple.com/documentation/Security/customizing-the-notarization-workflow)

## Licensing and legal risk

- Inkscape’s public license page offers GPL version 2 text and describes the software as GPL; the repository says most source is GPL-2.0-or-later, with exceptions, and notes packaged binaries can have different effective terms because of dependencies. **Do not reduce this to “all of Inkscape is GPL-2+.”** [license page](https://inkscape.org/en/about/license/) · [COPYING](https://github.com/inkscape/inkscape/blob/master/COPYING)
- A rewrite that copies protected source code, expressive structure or nonfunctional text may be derivative; merely implementing file formats/ideas does not automatically answer the legal question. “Clean room” is a process to reduce copying risk, not a legal guarantee. Get counsel’s review before commercial distribution. Keep contributor provenance; do not feed Inkscape source to implementation agents if clean-room separation is required. [GNU GPL FAQ](https://www.gnu.org/licenses/gpl-faq.en.html)
- **License recommendation:** if the product is genuinely independent, use Apache-2.0 OR MIT for the application/core (Apache adds explicit patent grant/notice framework; MIT is simpler), with third-party notices and SPDX/SBOM. If reusing Inkscape implementation, use a GPL-compatible open-source license and comply with source/offering obligations. LGPL components have conditions; GPL-only Qt modules require compatible application licensing. [Qt licensing](https://doc.qt.io/qt-6/licensing.html)
- Patent risk: W3C standards policy provides royalty-free licensing for essential claims under its policy, but only within policy scope and conditions; it is not a blanket patent clearance for every implementation, codec, hardware API or non-W3C feature. [W3C patent policy](https://www.w3.org/policies/patent-policy/)
- Trademark: avoid “Inkscape” in product name, iconography or implying endorsement. Use descriptive compatibility claims only after trademark counsel checks policy/jurisdiction. [Inkscape branding](https://inkscapesoftware.com/about/branding/)
- Create a dependency/license allowlist at M0. Record exact version, license, optional flags, static/dynamic linking, bundled assets/fonts and notices. Pay special attention to Qt modules, codec patent status by jurisdiction/use, and whether app distribution bundles runtime libraries.
- Binary size is a product KPI, not license evidence. A minimal C app linking Qt, font stack, PDF, codecs and GPU libraries can exceed a Rust binary. Establish a size budget after the smallest useful feature slice is measured.
