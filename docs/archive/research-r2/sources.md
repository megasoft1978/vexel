# Sources

Live research checked 8 October 2026. Links are primary project/spec/documentation pages unless noted. Published benchmark values retain their source version and caveats; planning estimates in the report are our estimates, not source claims.

## UI and platform

- Sokol source/docs: https://github.com/floooh/sokol
- SDL3 front page/license: https://wiki.libsdl.org/SDL3/FrontPage
- GLFW download/source: https://www.glfw.org/download
- raylib: https://github.com/raysan5/raylib
- Clay C layout/LOC claims: https://github.com/nicbarker/clay
- microui: https://github.com/rxi/microui
- Nuklear: https://github.com/Immediate-Mode-UI/Nuklear
- Dear ImGui: https://github.com/ocornut/imgui
- RAD Debugger UI research (secondary source index): https://deepwiki.com/EpicGamesExt/raddebugger/3-user-interface
- Casey Muratori, “Immediate-Mode Graphical User Interfaces”: https://caseymuratori.com/blog_0001
- Blender GHOST modernization pull request / C API tradeoff: https://projects.blender.org/blender/blender/pulls/151792
- Flutter architecture: https://docs.flutter.dev/resources/architectural-overview
- Flutter Impeller: https://docs.flutter.dev/perf/impeller
- Sublime Text GPU rendering: https://www.sublimetext.com/docs/gpu_rendering.html
- Ghostty architecture: https://ghostty.org/docs/about
- Slint repo/architecture: https://github.com/slint-ui/slint
- Xilem: https://docs.rs/xilem/latest/xilem/
- Makepad repo: https://github.com/makepad/makepad
- Makepad architecture guide: https://makepad.rs/guide/start/makepad-framework-architecture
- Win32 DirectWrite guide: https://learn.microsoft.com/en-us/windows/win32/directwrite/programming-guide
- Apple CoreText: https://developer.apple.com/documentation/coretext
- Microsoft DirectComposition: https://learn.microsoft.com/en-us/windows/win32/directcomp/directcomposition-portal
- Wayland xdg-shell protocol: https://gitlab.freedesktop.org/wayland/wayland-protocols/-/blob/main/stable/xdg-shell/xdg-shell.xml
- Wayland text-input-v3 protocol: https://gitlab.freedesktop.org/wayland/wayland-protocols/-/blob/main/unstable/text-input/text-input-unstable-v3.xml
- xdg-desktop-portal FileChooser: https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.FileChooser.html
- Apple accessibility for apps: https://developer.apple.com/documentation/appkit/accessibility-for-appkit
- Windows UI Automation: https://learn.microsoft.com/en-us/windows/win32/winauto/entry-uiauto-win32
- Linux AT-SPI: https://www.freedesktop.org/wiki/Accessibility/AT-SPI2/

## Renderer and text

- Blend2D performance comparison methodology: https://blend2d.com/performance.html
- Blend2D benchmark rendered samples: https://blend2d.com/performance_testing_images.html
- Blend2D 2023 presentation: https://blend2d.com/research/blend2d_presentation_helsinki_2023.pdf
- Wikimedia SVG benchmarks (includes 2021 comparison tables): https://www.mediawiki.org/wiki/SVG_benchmarks
- Vello: https://github.com/linebender/vello
- Skia benchmark tool source: https://github.com/google/skia/blob/main/tools/skpbench/skpbench.cpp
- Skia project: https://skia.org/
- resvg: https://github.com/linebender/resvg
- PlutoVG: https://github.com/sammycage/plutovg
- ThorVG: https://github.com/thorvg/thorvg
- HarfBuzz: https://github.com/harfbuzz/harfbuzz
- FreeType license: https://github.com/freetype/freetype/blob/master/LICENSE.TXT
- stb collection / stb_truetype: https://github.com/nothings/stb

## XML, SVG and corpora

- W3C SVG test suite overview: https://www.w3.org/Graphics/SVG/Test/Overview.html
- pugixml benchmark: https://pugixml.org/benchmark.html
- pugixml manual / compact mode and memory example: https://pugixml.org/docs/manual.html
- simdjson performance claims: https://github.com/simdjson/simdjson/
- simdjson performance methodology/allocator notes: https://github.com/simdjson/simdjson/blob/master/doc/performance.md
- Natural Earth downloads: https://www.naturalearthdata.com/downloads/
- Natural Earth-derived Wikimedia map file: https://commons.wikimedia.org/wiki/File:NED_worldmap_110m.svg
- resvg test suite: https://github.com/linebender/resvg/tree/main/tests
- Wikimedia list of resvg SVG test files: https://commons.wikimedia.org/wiki/User:JoKalliauer/SVG_test_suites
- Inkscape testfiles: https://gitlab.com/inkscape/inkscape/-/tree/master/testfiles
- SVG 2 specification: https://www.w3.org/TR/SVG/
- XML 1.0 specification: https://www.w3.org/TR/xml/
- Expat documentation: https://libexpat.github.io/doc/api/latest/
- libxml2 parser API: https://gnome.pages.gitlab.gnome.org/libxml2/html/parser_8h.html
- NanoSVG: https://github.com/memononen/nanosvg
- LunaSVG: https://github.com/sammycage/lunasvg
- PlutoSVG: https://github.com/sammycage/plutosvg

## Bench/build sources

- Inkscape 1.3 release notes, async canvas performance claims: https://wiki.inkscape.org/wiki/Release_notes/1.3
- Inkscape 1.5 notes/development: https://wiki.inkscape.org/wiki/Release_notes/1.5
- Inkscape release index: https://wiki.inkscape.org/wiki/Release_notes
- Apple notarization: https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution
- Apple notarization workflow: https://developer.apple.com/documentation/Security/customizing-the-notarization-workflow
- Zig C/C++ compiler usage: https://ziglang.org/learn/overview/#cross-compiling
- CMake: https://cmake.org/cmake/help/latest/
- Clang sanitizers: https://clang.llvm.org/docs/AddressSanitizer.html
- Clang static analyzer: https://clang.llvm.org/docs/ClangStaticAnalyzer.html
- Inkscape license page and repository COPYING were reviewed in round 1: https://inkscape.org/en/about/license/ · https://github.com/inkscape/inkscape/blob/master/COPYING

## Prior art

- Rive editor: https://www.rive.app/editor
- Graphite: https://github.com/GraphiteEditor/Graphite
- Rnote: https://github.com/flxzt/rnote
- Penpot: https://github.com/penpot/penpot
- Inkscape renderer / release notes are above; round-1 source list remains at `../out/sources.md`.

## Unknowns explicitly retained

- No apples-to-apples current benchmark for Blend2D, Vello, Skia, resvg, librsvg and Inkscape on one named Mac with editor interactions was located.
- No fixed, attributable 1 GB public SVG test file was verified. Generate and pin one.
- No canonical current binary/LOC/RSS figures for proprietary Sublime, Rive, Lunacy, Affinity, Illustrator or commercial editor internals were found.
- No measured LOC or size forecast for this project exists. All layer/roadmap LOC, dates and gates are estimates to revise after M0.
- A direct primary-source current size inventory for SDL3/GLFW/raylib/RAD Debugger was not established; report avoids pretending source LOC equals shipped binary size.
