# Vexel

Vexel is a native SVG viewer and editor for Apple Silicon Macs, written in C17 with a custom-drawn interface and an owned SVG parser and rasterizer. It is an evolving workbench, with bounded rendering and editing of a supported SVG subset.

[Project website and demo](https://megasoft1978.github.io/vexel/) · [Demo assets and provenance](docs/examples/README.md)

## Current capabilities

- Open, inspect, save, and edit SVG source while preserving unrelated XML bytes and unknown content.
- Zoom and pan; layers and attribute Inspector; multiple selection, transforms, shape and text tools, supported path node editing, snapping, rulers/guides, clipboard, undo/redo, command palette, and PNG export.
- Docked animation timeline with play/pause, seeking, preview modes, speed, track scrolling, keyframe and spline easing editing for supported tracks.
- SMIL numeric geometry/opacity, solid colors, transform articulation, simple timing, repetition, linear/discrete/spline/paced evaluation, and bounded line-based motion paths. Editing accepts a narrower subset than playback.
- MCP access to documents, source-preserving mutations, animation tools, sampled PNG frames, and live app controls. GUI and MCP share selection and undo history.

## Build and run

Requires Apple Silicon, macOS 13+, Xcode Command Line Tools, CMake 3.25+, and Ninja. No configure-time downloads or third-party library dependencies are used.

```sh
./dev build
./dev run docs/examples/un-cuore-per-due-vector-rig-v2.svg
# Or build Release explicitly:
cmake --preset mac-release
cmake --build --preset mac-release
./build/mac-release/bin/svgworkbench docs/examples/un-cuore-per-due-vector-rig-v2.svg
```

Launch without a path for the Welcome/Open flow. `./dev bundle` builds the macOS application bundle. Run `./dev t --all` for the complete local test tier.

```sh
./build/mac-debug/bin/svgworkbench --render-frame file.svg --size 1280x800 --time 1.5 --out frame.ppm
# The destination directory must already exist:
./build/mac-debug/bin/svgworkbench --render-animation file.svg --fps 8 --duration 10 --out frames/
```

CLI frame sequences use PPM, flatten transparency over white, and have frame/pixel limits. See [app contracts](src/app/README.md) and [animation contracts](src/svg/animation/README.md).

## MCP

The running app owns the local same-user socket; `build/mac-debug/bin/vexel-mcp` relays MCP JSON-RPC over stdio. Both processes use the same `VEXEL_MCP_SOCKET` path. Grant filesystem access through `VEXEL_MCP_ALLOWED_DIRS` or the private `~/.config/vexel/mcp-allowed-dirs.txt` file. See [MCP setup and limits](src/mcp/README.md).

Live records (`active` and `window:N`) share the app's current document. Headless documents are owned by the broker; live UI controls return `unavailable` for headless records. `VEXEL_MCP_DISABLE_LIVE=1` disables live control. Source/element handles expire after mutations: refresh the tree/query before reusing them. Update coordinated timing attributes in one transaction so intermediate invalid animation states are never committed.

MCP frame rendering succeeded during the supplied demo workflow, while built-in full-page animation export failed. The supplied GIF is an external workflow output, not evidence that the built-in exporter succeeded. This documentation/UI order does not change MCP or implement an exporter.

## Demo: Un cuore per due

![Vector animation preview](docs/examples/un-cuore-per-due-vector-rig-preview-v3.gif)

[Animated SVG](docs/examples/un-cuore-per-due-vector-rig-v2.svg) · [GIF preview](docs/examples/un-cuore-per-due-vector-rig-preview-v3.gif) · [Contact sheet](docs/examples/v3-contact-sheet-8fps.png)

The demo is a vector trace/reconstruction with transform articulation and squash/stretch. It does not establish exact original-drawing fidelity, bone skinning, or path morphing. Its 12 named spline-eased tracks describe an 11.01-second SVG scene. The GIF contains 88 frames at 120 ms each, an encoded loop of 10.56 seconds. [Asset metadata, hashes, and rights](docs/examples/README.md).

## Limitations and license

SVG support is incomplete: SVGZ, external media/resources, scripts, CSS animation/event timing, curved motion paths, and active path morphing are unsupported. Some group opacity, filter/mask, pattern and clip cases remain limited; renderer acceptance work is deferred, not declared complete. Large files and export work have explicit resource limits. No exact SVG fidelity or performance claim is made.

Vexel software is [MPL-2.0](LICENSE). Supplied artwork has separate provenance; the software license does not grant rights to that artwork.
