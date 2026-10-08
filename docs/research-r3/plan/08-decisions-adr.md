# 08 — Architecture decision records

Status `LOCKED` means owner direction; it is not reopened absent evidence-backed hard blocker. `PROPOSED` requires owner decision or a spike.

## ADR-001: Language — LOCKED

Use C17. Reason: owner decision; minimal dependency and readable core. Guardrails: opaque APIs, checked slices, sanitizers/fuzzing, avoid pointer-heavy long-lived structures. Revisit only if the chosen compiler/platform requirement makes a required service infeasible; do not claim C is inherently faster.

## ADR-002: Product shape — LOCKED

Basic editor first; measure speed/size/memory/opening vs Inkscape; animation later; same SVG format. M10 decides continuation.

## ADR-003: UI — LOCKED

One custom-drawn UI on macOS first, then Linux/Windows; no widget library. Retained semantic tree + immediate paint commands. Thin native shims only for OS services. Preserve accessibility from first UI milestone.

## ADR-004: Presentation/render — LOCKED

CPU tiled rasterizer presented as plain texture blit; Metal first. Internal raster backend remains replaceable. GPU rendering of document content is not required by this lock.

## ADR-005: License — PROPOSED default MPL-2.0

Options: MIT, Apache-2.0, MPL-2.0, GPL-2.0-or-later. Recommend MPL-2.0 for visible source with file-level copyleft and easy embedding; Apache-2.0 if broad permissive ecosystem/patent grant is priority. Have counsel review dependency set and contribution policy. Clean-room implementation means no inherited Inkscape GPL license automatically.

## ADR-006: Product name — OPEN

Default: invented short name, no “Inkscape” in name or domain; use “opens/saves SVG and preserves supported Inkscape metadata” as factual compatibility copy after tests. Trademark clearance before public launch.

## ADR-007: Minimum macOS — PROPOSED

Start at macOS 13 (Ventura) as a conservative modern Metal/CoreText baseline, then check actual user cohort and deployment APIs. If earliest versions require significant fallback or reduce support, raise to 14; owner to approve before M1 packaging. Current Apple support window changes and must be verified at release.

## ADR-008: GPU API — LOCKED FIRST / OPEN OTHER OS

Metal for initial macOS texture presentation. Windows candidate D3D11; Linux candidate OpenGL/EGL or Vulkan. Recommendation for Linux: OpenGL/EGL fallback to maximize desktop availability unless M11 hardware tests justify Vulkan-only. Keep `gfx` abstraction to permit change.

## ADR-009: Text stack — PROPOSED

macOS CoreText in system shim, DirectWrite on Windows. Linux shaping default HarfBuzz+FreeType system libraries (or pinned dynamic deps) because platform shaping service has no universal app-level equivalent. Fallback text layout is explicitly limited. SVG text cross-platform visual variation must be documented; do not silently outline all text.

## ADR-010: Build tool + Apple Silicon CI — PROPOSED

CMake with presets, no generated code until necessary. Dedicated self-hosted Mac mini for stable benchmark; hosted runner procurement/cost unknown, revisit with provider quotes.

## ADR-011: Unity build — PROPOSED no

Keep separate translation units initially for explicit dependencies and easier agent/human review. Consider unity build only if compile time proves a problem and can preserve private symbol hygiene.

## ADR-012: Rasterizer — OPEN, M4 spike required

Options: own scalar tile fill; vendor PlutoVG; hybrid. Default hybrid: abstract `raster_backend`, benchmark PlutoVG MIT against scalar reference and selected fixtures; do not vendor before target-license/feature audit. Blend2D is C++ and not compatible with C17 core without a C wrapper/build cost. Vello/resvg are Rust and renderer-oracle/competitor references, not native dependencies by default.

## ADR-013: XML strategy — PROPOSED

Memory-mapped immutable source + structural spans, compact index, lazy decode; buffered fallback. No external entity/network access. Preserve unsupported data. Full DOM is rejected for primary model due estimated memory amplification; can build bounded transient semantic projections.

## ADR-014: Text shaping and system services — PROPOSED

System shaping is allowed as an OS-owned service per owner lock. Keep shaped glyph output behind C ABI. Linux may need HarfBuzz/FreeType (permitted system libraries, but not “zero dependencies”).

## ADR-015: Accessibility schedule — PROPOSED

Semantic tree and UIA/NSAccessibility/AT-SPI architecture begins M6; macOS core VoiceOver validation required for M6; Linux/Windows bridges in M11/M12. Hard blocker only if no feasible service mapping for custom UI, to be evidenced with a prototype.

## ADR-016: Animation format — LOCKED outcome, OPEN metadata

Remain SVG. Prefer interoperable SMIL/CSS for supported tracks; private namespace only for authoring data with static fallback. Timeline after M10. Decide exact namespace after browser/Inkscape round-trip prototype.

## ADR-017: Boolean geometry — PROPOSED defer

No boolean ops in MVP. If demand requires, spike Clipper2 (Boost license) and robust transform/quantization; do not write general cubic boolean engine as a side task.
