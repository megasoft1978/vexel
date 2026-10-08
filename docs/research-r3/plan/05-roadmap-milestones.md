# 05 — Roadmap M0–M14

Estimates are planning estimates (±50%), exclude vendored lines and assume an experienced C/graphics lead. “LOC” means owned production C/ObjC/Metal source added. Gate budgets are provisional until M0 baseline. Work items are sized 0.5–3 working days; larger topics must be split before execution.

| Milestone | Goal / deliverable | Incremental LOC estimate | Calendar: 1 dev + LLM help | Team 2–3 |
|---|---|---:|---:|---:|
| M0 | Competition baseline, corpus, small headless parser/render spike, user interviews | 2–4k | 2–3 weeks | 1–2 weeks |
| M1 | macOS window, event loop, Metal texture blit, timing | 1.5–3k | 1–2 weeks | 1 week |
| M2 | Base library, arenas, test/CI, diagnostics | 4–7k | 2–3 weeks | 1–2 weeks |
| M3 | mmap XML/SVG indexer and progressive viewer | 8–14k | 6–9 weeks | 4–6 weeks |
| M4 | CPU tile renderer: fill/stroke/gradients/clips; backend choice | 10–18k | 8–12 weeks | 5–8 weeks |
| M5 | CSS subset, defs/use/style, static compatibility | 7–12k | 6–9 weeks | 4–6 weeks |
| M6 | UI core/widgets, CoreText, semantic tree/theme | 10–16k | 8–12 weeks | 5–8 weeks |
| M7 | Selection, transforms, hit-test, spatial index | 6–10k | 5–8 weeks | 3–5 weeks |
| M8 | Node editing, inspector, layers/XML panels | 8–14k | 7–10 weeks | 5–7 weeks |
| M9 | Undo/redo, minimal-diff save, recovery | 6–10k | 6–9 weeks | 4–6 weeks |
| M10 | User wedge report + go/no-go vs latest Inkscape/Donner | 1–3k | 3–5 weeks | 2–3 weeks |
| M11 | Linux X11/Wayland and system integrations | 8–14k | 8–12 weeks | 5–8 weeks |
| M12 | Windows Win32/D3D11/text/IME/a11y | 8–14k | 8–12 weeks | 5–8 weeks |
| M13 | SVG animation timeline v1 and SMIL/CSS save | 10–18k | 10–16 weeks | 7–11 weeks |
| M14 | Harden, package, sign, notarize, release | 4–8k | 5–8 weeks | 3–5 weeks |

Dependency: M0 → M1/M2 → M3 → M4/M5 → M6 → M7 → M8 → M9 → M10. M11/M12 depend on stable platform API after M6 and can parallelize. M13 only after M10 GO and static file stability. M14 follows platform support milestones. Total elapsed estimate: **1 dev 18–30 months** (rework/compatibility included); **2–3 devs 12–20 months** with a dedicated QA/design contribution. Animation adds roughly 3–5 months one person. These estimates are not commitments.

## M0 — benchmark harness + corpus

Tasks: `M0-T01` inventory real/synthetic files and lawful provenance (1d); `T02` generate object-count/path-node/image-size stress families (2d); `T03` script Inkscape stable/nightly and Donner builds (2d); `T04` capture open/first-pixel/select/pan/save/RSS traces (2d); `T05` define corpus manifest/hash/features (1d); `T06` interviews 12–20 target users, anonymized notes (3d); `T07` publish baseline report and decide parser/raster spike (1d). Dependencies: none. Budgets: harness ≤1k LOC, no product size target. Exit: at least 20 licensed/pinned fixtures; 1/10/100 MB plus 1 GB generated variants; reproduce metrics twice with <10% variance on dedicated machine; evaluate Donner direct. Demo: run one command, produce CSV/JSON + HTML report. Risks: competitor not buildable; user files legally unavailable. In that case report BLOCKED evidence and use generated files without claiming market validity.

## M1 — macOS shell + Metal blit

`M1-T01` ObjC shim creates NSWindow/CAMetalLayer (1d); `T02` normalized mouse/key/resize/scale event struct (1d); `T03` CPU RGBA buffer + Metal texture upload/blit (2d); `T04` frame timing/signpost and shutdown (1d); `T05` hello-rect pixel fixture (1d). Depends M0 scaffolding. LOC 1.5–3k; app package target <15 MB single-arch debug-stripped prototype **estimate**; idle CPU <1% proposed. Exit: 60/120 Hz pacing, correct resize/high-DPI, no busy loop, clean quit/reopen. Demo: native window showing rasterized SVG `<rect>` and checkerboard. Risks: Metal resource storage assumptions; upload copy unexpectedly expensive.

## M2 — base, test framework, CI

`M2-T01` status/error and checked arithmetic (1d); `T02` slices/UTF8/string builder (1.5d); `T03` bounded arena + allocation counters (2d); `T04` test runner/assertions/temp fixtures (1.5d); `T05` CMake presets/warnings/ASan+UBSan CI (2d); `T06` analyzer and dependency/license report (1d); `T07` docs and ownership examples (1d). Depends M1 build. LOC 4–7k. Budget: test harness <1 MB linked into test binary; allocation failure injection test support. Exit: CI macOS arm64 passes, analyzer baseline empty or triaged, arena limit/overflow tests pass. Demo: run deterministic base suite under sanitizers. Risk: warnings-as-errors portability friction.

## M3 — mapped XML index + progressive viewer

`M3-T01` file mapping abstraction and buffered fallback (2d); `T02` scalar XML tokenizer spans with quote/comment/CDATA states (3d); `T03` limits/depth/entity policy + fuzz harness (2d); `T04` compact element table and parent/sibling navigation (2d); `T05` rectangle/circle/path-bounds subset parse (3d); `T06` viewport culling and first basic scene commands (3d); `T07` progressive scheduler/cancel/stale generation (3d); `T08` 100 MB stress report (2d). Depends M2. LOC 8–14k. Proposed budget: first pixel <2s 100MB; parser pause <100ms; parser/index memory ≤3× source size for simple objects or publish measured exceptions; no UI stall. Exit: malformed corpus no crash, unknown spans retained, 1GB input either progresses or cleanly reports explicit limit; no eager base64 decode. Demo: open 100MB file while progress advances; pan visible region during parse. Risks: CSS/use dependencies prevent early complete paint; HTML-like XML corner cases.

## M4 — raster backend and tiles

`M4-T01` evaluate PlutoVG against scalar reference corpus (2d); `T02` tile data model/LRU/dirty regions (2d); `T03` fill-rule and antialias tests (3d); `T04` stroke caps/joins/dashes accuracy spike (3d); `T05` gradient stop cache and spread modes (2d); `T06` clip/mask bounded offscreen policy (3d); `T07` worker pool/cancellation (3d); `T08` NEON blend/gradient spans (2d); `T09` Metal upload dirty tiles (2d); `T10` benchmark report and choose vendor/own (1d). Depends M3; PlutoVG spike before commit. LOC 10–18k. Budgets: p95 visible tile <33ms on M0 medium corpus; tile cache cap 256 MiB default; 4 workers; no worker writes shared tile. Exit: deterministic scalar goldens and zero mismatches on supported reference subset. Demo: pan/zoom stress SVG at target refresh with cache overlay. Risks: stroke correctness, mask memory, library license/coverage.

## M5 — styles, CSS, defs/use

`M5-T01` style/presentation precedence (2d); `T02` CSS tokenizer with bounded selectors (3d); `T03` inherited style computation + invalidation (3d); `T04` id index and duplicate diagnostic (2d); `T05` defs and gradient inheritance (2d); `T06` use reference graph/cycle/expansion caps (2d); `T07` style/url reference corpus (2d); `T08` resvg/W3C test runner and support matrix (2d). Depends M3/M4. LOC 7–12k. Proposed exit ≥85% supported selected resvg fixtures and ≥70% total declared baseline; skip unsupported with reason, never count skip as pass. Memory cap remains visible. Demo: render CSS + cloned symbols, report unsupported features. Risks: CSS complexity, cascade correctness and invalidation fan-out.

## M6 — UI core + text + accessibility

`M6-T01` semantic node IDs/tree and native accessibility adapter design (2d); `T02` layout measure/arrange primitives (3d); `T03` paint list + hit-test map (2d); `T04` focus, keyboard routing, pointer capture (3d); `T05` scroll/split/tab/button/field widgets (3d); `T06` virtual list with 1M logical rows and bounded allocation test (2d); `T07` CoreText shaping and IME client (3d); `T08` command registry + palette + shortcut map (2d); `T09` theme/tokens/localization-ready layout (2d); `T10` headless UI snapshots (2d). Depends M1/M2/M3. LOC 10–16k. Budget: 16.7ms UI-only frame p95 on reference Mac; list allocations <2,000 row/widget instances regardless of count (proposed). Exit: keyboard-only workflow, VoiceOver reads core controls, IME composition and clipboard pass manual QA. Demo: editor shell showing canvas, layers, inspector, command palette. Risks: accessibility APIs wider than estimate.

## M7 — spatial selection and transforms

`M7-T01` immutable scene snapshot IDs (1.5d); `T02` BVH/packed index prototype and benchmark (2d); `T03` incremental invalidation design (2d); `T04` exact hit test fill/stroke (3d); `T05` selection overlay and multi-select (2d); `T06` transform matrix editing/preserve attributes (3d); `T07` snapping basic grid/guides (2d); `T08` 100k/1M selection benchmark (1.5d). Depends M5/M6. LOC 6–10k. Exit: 100k simple objects hit/select/move p95 <50ms; stale snapshots rejected. Demo: select far/near objects while parse index completes. Risk: worst-case overlapping bboxes degrade query.

## M8 — path node edit and panels

`M8-T01` path command decode/lazy cache (2d); `T02` node/handle topology and hit testing (3d); `T03` cusp/smooth/symmetric ops (2d); `T04` drag preview transaction and cancel (2d); `T05` fill/stroke inspector with common values (3d); `T06` layer tree virtualized/filter/visibility (3d); `T07` XML read-only tree/source span pane (2d); `T08` keyboard, snapping, focus and undo interaction tests (2d). Depends M7. LOC 8–14k. Budget: no node-panel allocation proportional to hidden million rows; per-frame edit preview p95 <33ms on selected path budget (up to 10k segments). Exit: path editing and style controls preserve unselected source. Demo: edit a real path-heavy map/plot. Risk: path command output fidelity, large path edit data.

## M9 — minimal-diff save, undo, recovery

`M9-T01` edit op log/inverse ops (3d); `T02` merge drag into one transaction (2d); `T03` source-span serializer and parent rewrite rules (3d); `T04` unique IDs for inserted objects (1d); `T05` atomic temp/rename + permissions (2d); `T06` autosave/recovery journal (2d); `T07` unknown namespace/metadata round-trip suite (3d); `T08` crash/restart harness (2d). Depends M8. LOC 6–10k. Exit: unchanged save exact byte equality; changed document preserves every unedited subtree; undo/redo returns semantic and rendered state; injected crash recovers last autosave. Demo: edit Inkscape file, save, reopen in Inkscape and compare. Risk: serializer/cross-tool normalization, filesystem atomicity.

## M10 — wedge evaluation / go-no-go

`M10-T01` rerun pinned corpus on current Inkscape stable + development and Donner release/head (2d); `T02` pilot with 5 users and workflow observation (3d); `T03` fidelity and loss audit (2d); `T04` binary/RSS/latency package report (1d); `T05` contribution/upstream option comparison (1d); `T06` investor-style go/no-go memo (1d). Depends M0–M9. LOC 1–3k. Exit: publish metrics and limitations; meet gates in `01-product-scope.md` or stop/pivot. Demo: user task recordings, reports, actual round-trip samples. Risk: biased sample and competitor update.

## M11 — Linux

`M11-T01` xdg-shell Wayland and X11 window event backend (3d); `T02` xkbcommon keyboard normalization (2d); `T03` text-input-v3 IME and fallback (3d); `T04` libdecor/client-side decoration (2d); `T05` clipboard and DnD (2d); `T06` portal file dialogs (2d); `T07` AT-SPI bridge (3d); `T08` package Flatpak/AppImage/distro tarball (3d); `T09` test GNOME/KDE compositors and fractional scaling (3d). Depends stable platform interface. LOC 8–14k. Exit: declare supported compositor matrix; keyboard/IME/accessibility manual QA. Budget: app package 15–60MB estimate. Demo: Wayland and X11 build opens/edits/saves. Risks: compositor protocol variance, portal async lifecycle.

## M12 — Windows

`M12-T01` Win32 window/input and DPI (3d); `T02` D3D11 texture upload/blit (3d); `T03` DirectWrite text service (2d); `T04` TSF IME (3d); `T05` clipboard/OLE DnD/dialogs (3d); `T06` UI Automation semantic bridge (3d); `T07` portable ZIP/MSIX/signing (3d); `T08` test Windows 10/11 supported baseline (2d). Depends platform interface and M6. LOC 8–14k. Exit: a11y/IME/manual verification; packaged app preserves file association optional. Demo: portable build full MVP flow. Risk: TSF/UIA complexity, signing pipeline.

## M13 — SVG animation timeline v1

`M13-T01` time model/clock determinism (2d); `T02` SMIL animate/set/animateTransform subset parse (3d); `T03` tracks/keyframes and timeline UI (4d); `T04` easing/preview/scrub (3d); `T05` CSS keyframe subset (3d); `T06` source-preserving save and static fallback (3d); `T07` browser round-trip tests Chrome/Firefox/Safari (3d); `T08` optional Lottie export feasibility (1d). Depends M10 GO + M9. LOC 10–18k. Exit: animate common transform/opacity/fill across engines with documented subset; original unanimated data preserved; deterministic static fallback. Demo: authored animated SVG opens in stock browsers and imported/reopened in Inkscape. Risk: browser timing differences, timeline UI scope.

## M14 — package and release

`M14-T01` reproducible versioned build (2d); `T02` SBOM/licenses/notices (2d); `T03` macOS universal/arm64 signed app notarization (3d); `T04` Windows installer/signing and Linux package feeds (3d); `T05` update/crash/diagnostic opt-in design (2d); `T06` release security and recovery checklist (2d); `T07` install/uninstall/upgrade clean-machine test (2d); `T08` published support matrix/known gaps (1d). Depends M11/M12. LOC 4–8k. Exit: signature verification, notarization ticket, package size budgets measured, reproducible source/build hashes, license notices. Demo: install fresh on clean macOS/Windows/Linux and open sample. Risk: signing accounts and external build runner availability.
