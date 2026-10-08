# vexel

vexel is a macOS-first, C17 workbench for large SVG documents. The initial
scaffold builds a small command-line executable; application features will be
added through reviewed work orders.

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
