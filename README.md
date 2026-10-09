# vexel

vexel is a macOS-first, C17 workbench for large SVG documents. The initial
application is a native SVG viewer for the supported document subset.

## Status

| Feature | Status |
| --- | --- |
| Interactive SVG viewer | Built |

## Usage

```sh
./build/mac-release/bin/svgworkbench file.svg
```

The viewer supports fit, zoom, pan, resize, and clean quit. To build and launch
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
