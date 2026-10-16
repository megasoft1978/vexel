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

## Build and run

Requirements: Apple Silicon Mac, macOS 13 or later, the pinned Rust toolchain
(`rust-toolchain.toml`, installed by rustup). No other tools.

```sh
cargo build --release -p vexel-app
./build/rust/aarch64-apple-darwin/release/vexel path/to/file.svg   # open an SVG
./build/rust/aarch64-apple-darwin/release/vexel --mcp-preview       # serve MCP on stdio
```

`--grant-dir <dir>` (repeatable) authorizes a directory for MCP file access.
Tests: `cargo test --workspace --locked`. The workspace lives in `rust/`; see
`HANDOFF.md` for current status and `docs/product/` for the plan. The earlier C
implementation was removed; it is preserved at git tag `legacy-c-final`.

## License

vexel is distributed under the Mozilla Public License 2.0 (MPL-2.0). See
[`LICENSE`](LICENSE) for the license text.
