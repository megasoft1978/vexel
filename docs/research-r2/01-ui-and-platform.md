# 01 — Custom UI and platform layer (Q1, Q2, Q11)

## What precedents actually demonstrate

| Precedent | Useful lesson | Size/limitations |
|---|---|---|
| **Sokol** | Standalone C headers split app/window/input from graphics; app owns event loop/context and gfx is backend-neutral. Supports macOS Metal, Windows D3D11, Linux X11/OpenGL. | Repo does not publish one canonical LOC/KB number. Its docs require compiling macOS app/Metal implementation as Obj-C/Obj-C++ and linking Apple frameworks; this is a good model for tiny `.m` seam, not proof Cocoa is pure C. [Sokol](https://github.com/floooh/sokol) |
| **SDL3** | A large, battle-tested OS abstraction surface; event model and platform code can be reused, but is a dependency/library rather than “no UI library.” | System library with zlib license; comparatively broad scope (window/input/audio/etc.). Published official size/LOC figure not located. [SDL3](https://wiki.libsdl.org/SDL3/FrontPage) |
| **GLFW** | Narrower window/context/input abstraction than SDL; suitable if custom UI/rendering owns everything else. | No menus, native dialogs, text widget system or SVG-specific support; exact LOC depends on checkout/version. [GLFW](https://www.glfw.org/download) |
| **raylib** | Demonstrates a C API over per-platform backends and bundled simple drawing APIs. | Game-oriented, not desktop-native accessibility/IME/dialogs; no reliable cross-version size number used here. [raylib](https://github.com/raysan5/raylib) |
| **Clay** | C layout can be a compact, renderer-independent, arena-based module. Project reports a 4.8k LOC single header, no dependencies, ~3.5 MB for 8192 elements and microsecond layout. | Layout only: no window, widget semantics, font shaping, accessibility or platform integration. Numbers are project-reported. [Clay](https://github.com/nicbarker/clay) |
| **microui / Nuklear** | Compact immediate-mode C widgets are excellent for tools/prototypes and establish a widget API without a retained object framework. | Small feature surface means app owns accessibility, text editing, IME, native menus/dialogs and usually window/render backends. Nuklear is single-header; do not interpret that as a complete desktop UI. [Nuklear](https://github.com/Immediate-Mode-UI/Nuklear) |
| **Dear ImGui** | “Immediate” describes API/state flow, not per-widget driver calls; it builds draw lists and supports custom renderer/backend integration. | C++ and tool-centric interaction model; docking, accessibility and native feel require extra work. [Dear ImGui](https://github.com/ocornut/imgui) |
| **RAD Debugger** | Strong relevant paradigm: rebuild box tree per frame, hash IDs for persistence, layout/event/draw passes; clear separation between UI, font cache, draw list and D3D/OpenGL renderers. | Debugger-specific; its implementation is not an accessibility-ready general consumer desktop toolkit. [UI overview](https://deepwiki.com/EpicGamesExt/raddebugger/3-user-interface) |
| **Casey Muratori / Handmade Hero** | Single-path immediate API can simplify UI control flow; Handmade Hero’s platform layer teaches isolate OS APIs behind a compact event/file/thread interface. | Design essays/videos, not a ready toolkit or proof IMGUI wins for large document UI. Casey’s article is a useful primary source. [IMGUI article](https://caseymuratori.com/blog_0001) |
| **Blender GHOST** | Long-lived editor needs an explicit platform abstraction for windows, input, timers, clipboard and OS behavior. | Blender’s mature layer is not small; it has accumulated platform-specific behavior. A 2025 Blender proposal notes its old C API became confusing after Blender moved to C++, warning against over-opaque handles. [GHOST background](https://projects.blender.org/blender/blender/pulls/151792) |
| **Flutter / Impeller** | Flutter uses a retained widget/render pipeline with a dedicated renderer and platform embedding; Impeller precompiles shaders and owns predictable GPU pipelines. | Huge framework/runtime; evidence for architecture separation, not a small C transplant. [Architecture](https://docs.flutter.dev/resources/architectural-overview) · [Impeller](https://docs.flutter.dev/perf/impeller) |
| **Sublime Text** | A polished app can own software and GPU rendering backends. | Proprietary internals; official docs report OpenGL hardware mode and software fallback, but no UI source/LOC or bundle internals. [GPU rendering](https://www.sublimetext.com/docs/gpu_rendering.html) |
| **Ghostty / Alacritty** | Ghostty separates reusable C-ABI core from platform GUI: macOS Swift/AppKit/SwiftUI, Linux Zig+GTK. Alacritty illustrates a custom GPU terminal surface. | Counterexample to forcing one custom UI everywhere: Ghostty uses native UI and GTK. It is a terminal, not a general editor. [Ghostty architecture](https://ghostty.org/docs/about) |
| **Zig/native UI** | Ghostty shows Zig can call C APIs and implement platform shell; the important architectural boundary is core vs shell, not language. | No mature zero-dependency general-purpose C UI toolkit established by these examples. |
| **Slint** | AOT declarative UI can optimize structure and co-locate component memory. | Not C, but useful retained UI model. It is itself a toolkit, contrary to the owner’s chosen constraint. [Slint architecture](https://github.com/slint-ui/slint) |
| **Xilem/Masonry** | Lightweight declarative views can diff into retained widgets, backed by separate Vello, Parley, AccessKit and wgpu components. | Rust ecosystem and multiple dependencies; useful layering precedent, not a C solution. [Xilem](https://docs.rs/xilem/latest/xilem/) |
| **Makepad** | Platform, widgets, shaders, rendering and styling are explicit layers. | Rust-first, GPU-first and substantial; Linux dependencies/platform services remain. [Makepad](https://github.com/makepad/makepad) |

## Recommendation: hybrid retained UI model

Use retained state for panels, focus, accessibility identity, text controls and asynchronous jobs. On each frame, layout visible regions and emit a compact draw list; immediate-mode construction is acceptable for menus, inspectors and transient tools. Canvas objects must **not** be UI widgets; they belong to document scene/index. Keep stable widget IDs and semantic roles. Cache layout/text/render data by generation and invalidate locally.

| Layer | Include | C LOC estimate (excluding tests/platform-specific duplication) |
|---|---|---:|
| `platform.h` contract + event normalization | Window lifecycle, surface, scale, keyboard/mouse/wheel/tablet, timers, clipboard, DnD, files, focus, cursors, IME composition, native dialogs/menu hooks, text shaping hooks | 700–1,200 |
| macOS shim | NSApplication/NSWindow, event bridge, pasteboard, UTType/DnD, NSOpenPanel/NSSavePanel, menus, CoreText, IME marked text, AX adapter, Metal surface | 2,000–4,000 C/Obj-C |
| Win32 shim | HWND/message loop, DPI awareness, clipboard, shell DnD, common dialogs, menus, DirectWrite, TSF IME, UIA adapter, D3D/Metal-equivalent surface | 3,000–5,500 C |
| Linux shim | Wayland + X11 path, xdg-shell, clipboard/DnD, xkbcommon, text-input-v3/IBus, file chooser strategy, AT-SPI bridge, EGL/Vulkan surface | 4,000–8,000 C |
| UI layout + constraints | Flex/grid-like rows, docking/panes, clipping, scroll, text measurement, invalidation, virtual list | 3,000–6,000 |
| Widget/state + accessibility semantics | Buttons, splitters, menus, tree, list, color well, numeric fields, undoable text edit, focus/nav, semantic tree/actions | 5,000–10,000 |
| UI drawing/theme/animation | Shapes/text/icons, tokens, transitions, reduced-motion setting, cursor overlays | 2,000–4,000 |
| renderer backend interface | Surfaces, texture upload, geometry/text batches, fence, resize/scale | 500–1,000 |

**Estimate warning:** total UI/platform shell is roughly **20k–40k LOC** to reach a credible desktop MVP across three platforms; polished accessibility, robust text entry, edge cases and native menus can add another 10k–25k. Not measured project data. Small immediate libraries look tiny because they omit these costs.

### Visual and input rules

- Native menu bar, open/save panels, clipboard and drag/drop are mediated by platform services; keep custom canvas, toolbar and panels branded consistently via tokens.
- Tokens: semantic colors, spacing scale, type scale, border/radius, focus ring, selection, disabled/hover/pressed, contrast modes. Avoid hard-coded values in widgets.
- UI animation only for 120–180 ms panel/selection transitions, with reduced-motion setting and no animation in document fidelity. Use monotonic clock; stop animating when idle.
- Text input uses a dedicated model: UTF-8 buffer, selection/caret, undo, composition range, candidate-window rect, key commands, accessibility value. Never treat keydown Unicode as text; IME can send marked text and commit separately.
- Minimum a11y means semantic roles/names/values/actions, keyboard navigation, focus order, high contrast and screen reader smoke testing in each OS. “We draw pixels ourselves” is no exemption.

## Platform shims and acceptable system dependencies

“Zero dependency” should mean **no bundled third-party runtime libraries**, not “no OS APIs.” macOS frameworks, Win32 system DLLs and Linux system ABI are acceptable. Linux packaging must declare runtime protocol/font dependencies. For one UI-owned renderer, use Metal first on Mac, D3D11 on Windows, Vulkan on Linux where available; CPU framebuffer + OS/GPU blit is the simplest correctness fallback. OpenGL remains a broad fallback but is deprecated on Apple platforms and is not the preferred Mac-first path.

| Platform | Native route | System components considered acceptable | Main costs / caveats |
|---|---|---|---|
| macOS | Objective-C bridge, AppKit/Cocoa events/window, Metal surface and blit; CoreText; NSAccessibility | AppKit, Metal, CoreText, CoreGraphics, CoreFoundation, UniformTypeIdentifiers, CoreServices as OS frameworks | `.m` compile is expected; Obj-C adds no third-party runtime dependency. Pure C via `objc_msgSend` requires ABI-safe casts/selectors and complicates blocks/delegates/IME/accessibility; no meaningful app-size win established. Use minimal Obj-C. |
| Windows | Win32 HWND/message pump + D3D11; DirectWrite/Direct2D selectively, TSF, UIA, shell COM | user32, gdi32, comdlg32, ole32, dwrite, d2d1, dcomp, dwmapi, uxtheme, UIAutomationCore | System APIs stable but COM/TSF/UIA boilerplate large. DWM/DirectComposition is a compositor integration, not required for first pixels. DPI awareness and per-monitor scale must be correct. |
| Linux | Prefer Wayland via libwayland-client + generated xdg-shell; optionally X11/XCB fallback | libc, libwayland-client, xkbcommon, libdecor optional; libX11/XCB and xkbcommon for X11; fontconfig/FreeType/HarfBuzz either distro system libs or vendored optional | Wayland requires protocol negotiation, configure/ack lifecycle, decorations policy and explicit text-input/IME protocol. xdg-shell defines desktop toplevel behavior; text-input-v3 availability varies by compositor. Pure Wayland has no universal file-dialog API: portal DBus is advisable or ship custom chooser. X11 has more legacy APIs but no future-proof Wayland coverage. |

No GTK is needed, but **Wayland client libraries are real system dependencies**. For Mac-first MVP, Linux can initially ship X11/XWayland then add native Wayland; mark that as a temporary support tier, not “Linux done.”

## Honest blocker table

| Risk | Hard blocker? | Cheapest acceptable mitigation |
|---|---|---|
| Accessibility | Blocker for broad/professional deployment, not first private prototype | Semantic widget tree from day one; bridge AX/UIA/AT-SPI per OS; basic keyboard-only + screen reader audit before beta. |
| IME / international text | Blocker for trustworthy text editing globally | Native text-input hooks in platform contract; use CoreText/TSF/IBus pathways; defer advanced text tool, not composition handling. |
| File dialogs/menus | No blocker | Native panels/menus; custom menus only inside app canvas. Wayland use xdg-desktop-portal where installed, documented fallback otherwise. |
| Wayland fragmentation | Not blocker for Mac-first; risk to Linux release | X11 first for MVP; implement xdg-shell path against Mutter/KWin/wlroots test matrix; libdecor or client decorations. |
| Font fallback/rendering | Blocker for text fidelity | OS shaping/fallback backends; don't own font raster stack in MVP. |
| HiDPI | No blocker if designed in | Use logical coordinates internally and scale-aware surfaces; per-monitor scale-change events invalidate UI/canvas tiles. |
| Maintainability | Major ongoing risk | Narrow platform contract, per-platform modules, conformance harness, ownership docs, avoid generic “lowest common denominator” API. |

