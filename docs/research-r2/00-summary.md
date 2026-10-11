# Round 2 — decision brief

Research checked 8 October 2026. Confidence: **high** on platform/text/API constraints and parser memory risks; **medium** on architecture recommendations; **low** on predicted LOC, final bundle/RSS, and speedups until a prototype measures them. Proposed numbers are estimates or gates, never published facts.

## Verdict

The chosen direction is feasible as a focused C17 editor, but “no UI library” moves substantial product work into this project. It does not block an MVP. It does block parity-level accessibility, IME, menus, text fidelity, and cross-platform polish from being cheap. Use native OS services for windowing, text shaping, file dialogs, menus, accessibility hooks, clipboard and IME; draw the application content in a custom renderer. Keep editor UI retained for persistent state and accessibility identity, with immediate command generation for painting and transient overlays. Start with a CPU tiled renderer behind a backend interface, then add Metal presentation/acceleration only after CPU baselines.

## Recommended architecture

```text
SVG source (mmap + spans) ──> parser/style/scene index ──> spatial index + display list
                                     │                            │
                              editor document + op log      CPU tiled rasterizer
                                     │                            │
Custom UI (retained state / immediate paint commands) ───> Metal surface/blit
                                     │
               C platform API: windows/events/clipboard/IME/text/menus/dialogs
                 macOS: small Objective-C .m bridge; Windows: Win32 C; Linux: Wayland
```

### Top decisions

1. Use **C17** and a small, explicit C API; compile one Objective-C `.m` bridge on macOS. Do not force Cocoa calls through hand-written `objc_msgSend` wrappers just to claim “pure C.”
2. M0 first: corpus + benchmark harness + mmap parser + progressive viewer. Do not begin with the polished UI or custom path rasterizer.
3. Keep the SVG byte source authoritative. Index parsed structures and preserve untouched source spans; serialize changed elements with minimal patches. Preserve unknown XML/CSS and Inkscape metadata.
4. Use CoreText on Mac and DirectWrite on Windows for shaping; on Linux offer fontconfig + FreeType/HarfBuzz as a system or optional dependency. Build a small owned fallback path only for simple Latin UI text.
5. Build a CPU reference rasterizer and tile cache, use Metal for upload/compositing first. Test GPU path rendering only if the profile says path rasterization is the limit.
6. Design toward 1 GB input support with bounded memory and progressive first paint, not a 1 GB fully materialized DOM.

## Hard blockers and honest risks

- **No blocker to viewer/edit MVP.** **Hard product blocker for professional desktop quality:** OS integration cannot be skipped. IME, accessibility, clipboard/DnD, dialogs, menus, focus, font fallback and high-DPI behavior need native bridges; Linux Wayland multiplies this work.
- **No acceptable “custom UI only” answer for a11y.** Mac AX, Windows UIA, Linux AT-SPI require semantic widget trees and actions; defer broad polish only with an accessibility architecture from M0.
- **No text parity without shaping engines.** Reimplementing OpenType shaping is a major project; system shapers are the honest minimum for editor text.
- A 500 MB or 1 GB SVG may contain huge embedded images, one enormous path, millions of elements, or malicious constructs; bytes alone do not determine RAM/render time.

## First three actions

1. Implement a repeatable benchmark harness before editor features. Lock machines, versions, fixtures, user actions, and artifact capture.
2. Benchmark existing readers/renderers and collect a legal, tiered corpus: W3C/resvg tests, real map/CAD/plot SVGs, generated stress fixtures, plus owner-provided files.
3. Prototype the Mac `.m` platform seam and a C17 mmap/token-index viewer; emit time-to-first-pixel, peak RSS, binary size and unchanged-file round-trip result.
