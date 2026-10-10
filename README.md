# vexel

**A native macOS workbench for very large SVG files.**
Written in C17. No third-party libraries. Your source bytes stay yours.

> **Status: early development (pre-alpha).** vexel is a working SVG *viewer*,
> not an editor yet. It opens and renders a bounded SVG subset in a native
> window; editing, saving and broad SVG coverage are still being added. See
> [Status](#status) for what works today.

## Why vexel

Most SVG tools parse a file into their own model and write a different file back.
On big, hand-tuned or tool-generated documents that means slow opens, lost
metadata and noisy diffs. vexel is designed around the opposite approach.

- **Source-preserving.** The document is held as borrowed source bytes plus
  spans. Unknown elements, namespaces and formatting are never rewritten. A
  no-op save writes back the exact bytes, via an atomic temp-file rename.
- **Responsive on large files.** Indexing runs in byte-budgeted steps on a job
  scheduler so the main thread never blocks on a big document. Rendering is
  tile-based with a spatial index and a tile cache.
- **Safe on untrusted input.** Every parser path has explicit size, depth and
  work limits, checked arithmetic and no unbounded recursion. Scripts, external
  entities and external resources are never loaded or executed. The XML
  tokenizer and shape decoder are fuzzed under ASan and UBSan.
- **Small and self-contained.** C17, CMake and Ninja, no dependencies, no
  configure-time downloads. A Release `svgworkbench` is about 200 KB.
- **Apple Silicon native.** Metal presentation, custom-drawn UI with keyboard
  semantics, platform code behind narrow `platform/` interfaces.
- **Checked against an independent renderer.** Output is compared with pinned
  librsvg renders under fixed pixel thresholds. librsvg is used only to produce
  test baselines; it is not a dependency.
- **Decisions are written down.** Architecture decisions live in
  [`docs/adr/`](docs/adr), and performance methodology in
  [`docs/PERF.md`](docs/PERF.md).

## Status

| Area | State |
| --- | --- |
| Bounded XML tokenizer, streaming index | Built, fuzzed |
| Lossless no-op save (atomic rename) | Built |
| Document model with stable node handles | Built |
| Spatial index, tile cache, job scheduler | Built |
| Scalar rasterizer: fills, strokes, dashes, CSS paint cascade | Built (subset) |
| Metal presentation, custom UI, keyboard focus and semantics | Built |
| Interactive viewer: open, pan, zoom, fit (`svgworkbench file.svg`) | Built |
| `<use>`/`<symbol>`, linear/radial gradients, `clipPath` | Built (subset) |
| Attribute edits, minimal-diff save, undo/redo | Planned |
| Text rendering, accessibility adapters | Planned |

Rendering covers a bounded subset of SVG. In the current 58-file real-world
corpus only 2 files preview fully; the rest are reported as unsupported, not
guessed at. This is deliberate: unsupported content is reported, never silently
dropped. Measured numbers and their methodology are in
[`docs/PERF.md`](docs/PERF.md); no comparative speed claims are made.

## Requirements

- Apple Silicon Mac running macOS 13 or later
- Xcode Command Line Tools (Apple Clang and the macOS SDK)
- CMake 3.25 or later and Ninja

vexel supports macOS on Apple Silicon only for now.

## Usage

```sh
./dev run path/to/file.svg     # build Release and open the viewer
```

Drag or scroll to pan, pinch or Cmd+scroll to zoom, `+`/`-` zoom, `0` fits,
Cmd+Q quits. Unsupported SVG shows an explicit error frame naming the reason.
`svgworkbench --render-frame file.svg --size 1280x800 --out frame.ppm` renders
the same frame headlessly.

## Build and test

```sh
./dev build          # configure and build Debug
./dev verify         # build plus the fast test suite
./build/mac-debug/bin/svgworkbench --version
```

Other commands: `./dev asan`, `./dev ubsan`, `./dev shape-fuzz`,
`./dev bench-corpus`. Release is
`cmake --preset mac-release && cmake --build --preset mac-release`.

## Repository layout

- `app/` executable composition and native smoke tests
- `src/` base, platform, XML/SVG, raster, document, spatial and UI modules
- `tests/` unit, golden, oracle, fuzz and UI tests
- `bench/` corpus metadata, generators and benchmark runners
- `docs/` coding standard, ADRs, performance and test-driver notes

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

Copyright (c) 2026 Gabriele Ferreri. vexel is licensed under the
[Mozilla Public License 2.0](LICENSE).
