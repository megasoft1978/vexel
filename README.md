# vexel

vexel is a macOS-first, C17 SVG editor: a custom-drawn editing surface, an owned
SVG parser and rasteriser, SMIL/CSS animation with a docked timeline, and an MCP
server so an agent can drive the same editor. It aims to be a practical
alternative to Inkscape for the documents it supports, with animation and agent
access as first-class features (`docs/adr/0016-product-target-and-gates.md`).

## Status

| Feature | Status |
| --- | --- |
| SVG editor with animation and MCP | Built |

## Usage

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
