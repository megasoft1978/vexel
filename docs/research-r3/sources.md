# Sources

Checked 8 October 2026. Primary project/spec sources preferred. Community reports below are anecdotal evidence, not controlled measurements. Planning estimates in these documents are not source claims.

## Prior art, editor and market
- Donner editor/engine: https://github.com/jwmcglynn/donner
- Donner releases: https://github.com/jwmcglynn/donner/releases
- Donner SVG2 suite design: https://github.com/jwmcglynn/donner/blob/main/docs/design_docs/0057-donner_svg2_test_suite.md
- Inkscape architecture overview: https://wiki.inkscape.org/wiki/Architectural_overview
- Inkscape release index: https://wiki.inkscape.org/wiki/Release_notes
- Inkscape 1.3 performance notes: https://wiki.inkscape.org/wiki/Release_notes/1.3
- Inkscape 1.5 development notes: https://wiki.inkscape.org/wiki/Release_notes/1.5
- Inkscape June 2026 developer list: https://lists.inkscape.org/hyperkitty/list/inkscape-devel%40lists.inkscape.org/2026/6/
- Inkscape license: https://inkscape.org/en/about/license/
- Graphite: https://github.com/GraphiteEditor/Graphite
- Boxy SVG: https://boxy-svg.com/
- Lunacy releases: https://lunacy.docs.icons8.com/release-notes/
- Penpot: https://github.com/penpot/penpot
- SVG-Edit: https://github.com/SVG-Edit/svgedit
- Glaxnimate: https://gitlab.com/mattbas/glaxnimate
- Glaxnimate formats: https://docs.glaxnimate.org/en/formats.html
- Rnote: https://github.com/flxzt/rnote
- Linearity Curve/Move docs: https://support.linearity.io/docs/getting-started/what-is-linearity-curve-and-move-explained
- Affinity Designer: https://affinity.serif.com/en-us/designer/
- Figma SVG export: https://help.figma.com/hc/en-us/articles/360040028114-Export-from-Figma
- Rive editor: https://www.rive.app/editor
- Synfig: https://github.com/synfig/synfig
- Sozi: https://github.com/sozi-projects/Sozi
- Makepad: https://github.com/makepad/makepad
- Zed GPUI: https://github.com/zed-industries/zed/tree/main/crates/gpui
- Xilem: https://github.com/linebender/xilem
- Flutter architecture: https://docs.flutter.dev/resources/architectural-overview
- Vello: https://github.com/linebender/vello
- Vello SVG changelog: https://github.com/linebender/vello_svg/blob/main/CHANGELOG.md
- resvg: https://github.com/linebender/resvg
- PlutoVG: https://github.com/sammycage/plutovg
- PlutoVG Debian SLOC: https://sources.debian.org/src/plutovg/1.3.3-1/
- NanoSVG: https://github.com/memononen/nanosvg
- ThorVG: https://github.com/thorvg/thorvg
- Reddit large SVG report, 2026-08: https://www.reddit.com/r/Inkscape/comments/1w0ue90/methods_for_working_with_large_svgs/
- Reddit 90MB many-node file, 2025-07: https://www.reddit.com/r/Inkscape/comments/1ls879d/
- Reddit 100MB/8GB report, 2026-05: https://www.reddit.com/r/Inkscape/comments/1tpv3tk/inkscape_has_blown_up_my_svg_to_100_mb_what/
- Reddit 780MB save/sidebar report, 2026-08: https://www.reddit.com/r/flatpak/comments/1vps3o0/inkscape_flatpak_is_very_slow_do_things/
- Reddit export preview latency: https://www.reddit.com/r/Inkscape/comments/1dh1zv3/
- HN Inkscape 1.0 lag report: https://news.ycombinator.com/item?id=22855357
- 2026 alternative list (secondary): https://openalternative.co/alternatives/inkscape

## UI, OS and accessibility
- Clay layout: https://github.com/nicbarker/clay
- Dear ImGui docking: https://github.com/ocornut/imgui/wiki/Docking
- Nuklear: https://github.com/Immediate-Mode-UI/Nuklear
- RAD Debugger UI index: https://deepwiki.com/EpicGamesExt/raddebugger/3-user-interface
- Makepad architecture: https://makepad.rs/guide/start/makepad-framework-architecture
- Sokol: https://github.com/floooh/sokol
- GLFW downloads/source: https://www.glfw.org/download
- SDL3: https://wiki.libsdl.org/SDL3/FrontPage
- Apple CoreText: https://developer.apple.com/documentation/coretext
- Apple AppKit accessibility: https://developer.apple.com/documentation/appkit/accessibility-for-appkit
- Apple Metal storage modes: https://developer.apple.com/documentation/metal/choosing-a-resource-storage-mode-for-apple-gpu-resources
- Apple notarization: https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution
- DirectWrite: https://learn.microsoft.com/en-us/windows/win32/directwrite/programming-guide
- DirectComposition: https://learn.microsoft.com/en-us/windows/win32/directcomp/directcomposition-portal
- Windows UI Automation: https://learn.microsoft.com/en-us/windows/win32/winauto/entry-uiauto-win32
- Wayland xdg-shell: https://gitlab.freedesktop.org/wayland/wayland-protocols/-/blob/main/stable/xdg-shell/xdg-shell.xml
- Wayland text-input-v3: https://gitlab.freedesktop.org/wayland/wayland-protocols/-/blob/main/unstable/text-input/text-input-unstable-v3.xml
- libdecor: https://gitlab.freedesktop.org/libdecor/libdecor
- xkbcommon: https://xkbcommon.org/
- xdg-desktop-portal file chooser: https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.FileChooser.html
- AT-SPI2: https://www.freedesktop.org/wiki/Accessibility/AT-SPI2/

## Rasterization and algorithms
- Blend2D benchmark methodology: https://blend2d.com/performance.html
- Blend2D benchmark images: https://blend2d.com/performance_testing_images.html
- Blend2D Helsinki 2023 PDF: https://blend2d.com/research/blend2d_presentation_helsinki_2023.pdf
- Skia paint stroke API: https://github.com/google/skia/blob/main/include/core/SkPaint.h
- GPU-friendly stroke expansion paper: https://arxiv.org/abs/2405.00127
- Wikimedia SVG benchmark (2021 measurements): https://www.mediawiki.org/wiki/SVG_benchmarks
- Tippecanoe: https://github.com/felt/tippecanoe
- QGIS canvas: https://docs.qgis.org/latest/en/docs/pyqgis_developer_cookbook/canvas.html

## SVG, parsing, safety and tests
- SVG 2: https://www.w3.org/TR/SVG/
- SVG animation: https://www.w3.org/TR/SVG/animate.html
- Chrome SMIL status: https://developer.chrome.com/blog/smil
- MDN SVG animate: https://developer.mozilla.org/en-US/docs/Web/SVG/Element/animate
- Browser support snapshot: https://caniuse.com/svg-smil
- XML 1.0: https://www.w3.org/TR/xml/
- simdjson: https://github.com/simdjson/simdjson
- simdjson methodology: https://github.com/simdjson/simdjson/blob/master/doc/performance.md
- W3C SVG test suite overview: https://www.w3.org/Graphics/SVG/Test/Overview.html
- WPT SVG: https://github.com/web-platform-tests/wpt/tree/master/svg
- Inkscape test files: https://gitlab.com/inkscape/inkscape/-/tree/master/testfiles
- resvg tests: https://github.com/linebender/resvg/tree/main/tests
- CERT C secure coding: https://cmu-sei.github.io/secure-coding-standards/sei-cert-c-coding-standard/front-matter/introduction/
- Clang AddressSanitizer: https://clang.llvm.org/docs/AddressSanitizer.html
- Clang Static Analyzer: https://clang.llvm.org/docs/ClangStaticAnalyzer.html
- CMake: https://cmake.org/cmake/help/latest/

## Prior round reports
- Round 1 research index: `../out/sources.md`
- Round 2 research index: `../out2/sources.md`
