# vexel

Vexel is a macOS-first vector drawing and editing application with a goal of
becoming a high-quality alternative to Inkscape, while making 2D cartoon
animation and live AI-agent collaboration first-class workflows. The product
uses a custom-drawn interface and owned SVG parser/rasterizer. Claude Code,
Codex, and compatible agents can work through a local MCP server against the
same open document. See the [product goal and staged delivery plan](docs/product/vexel-goal-and-delivery-plan.md)
and its governing [ADR](docs/adr/0018-product-goal-and-completion-gates.md).

## Status

Vexel now has a versioned `.vexel` project with editable native animation
timelines, drawing cels, property keys, playback, and local MCP control/export.
These workflows are in progress rather than release-complete: SVG/Inkscape
fidelity, large-document performance, project recovery/migration, broader
animation authoring, and long-running MCP progress/cancellation remain open.
The full UI/end-to-end acceptance pass stays at the end of feature work. See the
[delivery plan](docs/product/vexel-goal-and-delivery-plan.md) for measured
corpus status and the remaining work.

## Product showcase

The product goal includes example footage showing Codex creating a sample
character and animating it through live MCP while Vexel updates on screen. The
reviewed video (or coordinated short clips) will demonstrate accepted product
capabilities on the product website and here. Recording and publication come
after feature work and final UI/end-to-end and release acceptance; the showcase
has not yet been recorded or published.

## Rust port

The active implementation is the Rust workspace in `rust/` (see `HANDOFF.md`). The preview
app runs with `cargo run -p vexel-app`; it currently supports a basic-shape MCP live
preview only. The C code below is the legacy reference and will be removed after parity.

## Usage (legacy C build)

```sh
./build/mac-release/bin/svgworkbench file.svg
```

The editor supports fit, zoom, pan, drawing, selection and transform, layer
visibility and locking, a tabbed inspector, task workspaces, an operator log, a
command palette, and clean quit. To build and launch
the Release executable in one step, run `./dev run file.svg`. A headless PPM
frame can be rendered with:

```sh
svgworkbench --render-frame file.svg --size 1280x800 --out frame.ppm
```

## Requirements

- Apple Silicon Mac running macOS 13 or later
- Xcode Command Line Tools (Apple Clang and macOS SDK)
- CMake 3.25 or later and Ninja

No third-party libraries or configure-time downloads are used.

## Configure and build

```sh
./dev build
```

Equivalent commands:

```sh
cmake --preset mac-debug
cmake --build --preset mac-debug
./build/mac-debug/bin/svgworkbench --version
```

The version command prints `svgworkbench 0.0.1`. Additional configure/build
presets are `mac-release` and `mac-size`.

## License

vexel is distributed under the Mozilla Public License 2.0 (MPL-2.0). See
[`LICENSE`](LICENSE) for the license text.
