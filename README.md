# Vexel

Vexel is a macOS-first vector drawing and animation editor written in Rust. It
has an owned XML/SVG parser and rasterizer, a custom-drawn interface, a native
animation timeline, and a local MCP server so that agents (Codex, Claude Code)
can edit the same open document as the app. Current status and open work are in
[STATUS.md](STATUS.md) and [HANDOFF.md](HANDOFF.md); the product goal is in
[ADR-0018](docs/adr/0018-product-goal-and-completion-gates.md) and the
[delivery plan](docs/product/vexel-goal-and-delivery-plan.md). Changes since
`v1.0.0-alpha` are in [CHANGELOG.md](CHANGELOG.md).

## Features

"Done" means implemented and covered by tests; live-window checks are listed
separately in [docs/ui/live-check.md](docs/ui/live-check.md) and
[docs/e2e-ui.md](docs/e2e-ui.md).

| Feature | Status | Notes |
|---------|--------|-------|
| SVG open and render | Partial | Owned decoder covers a subset. Internal DOCTYPE entities supported; arcs (`A`/`a`) unsupported. Corpus: 21/39 files rendered in the last recorded run ([HANDOFF.md](HANDOFF.md)). |
| Path data | Partial | Q/q, T/t and S/s commands accepted with correct bounds; arcs unsupported. |
| `.vexel` project save and reopen | Done | V1 slice ([HANDOFF.md](HANDOFF.md)). |
| Native Open / Save / Save As panels | Partial | Save panel opens once on Cmd+S. Open panel not verified ([live-check.md](docs/ui/live-check.md)). |
| Canvas editing | Partial | Select with eight resize handles, rotate with live preview, grouping, nested `<use>`, inspector fill/stroke/opacity/transform controls. Pointer drawing not yet verified on a real window ([HANDOFF.md](HANDOFF.md)). |
| Undo / Redo | Done | Journal edit groups for atomic undo. Verified on the real window ([live-check.md](docs/ui/live-check.md)). |
| Animation timeline and playback | Partial | Key editing with cubic-bezier easing, loop, duration. Broader authoring is still open ([HANDOFF.md](HANDOFF.md)). |
| Export: GIF | Done | Native timeline; loops over 120 frames stride-sampled. Export-to-file on quit not yet verified ([live-check.md](docs/ui/live-check.md)). |
| Export: APNG | Done | `apng` via `export_animation` (MCP). |
| Export: PNG frame sequence | Done | `frames` via `export_animation` (MCP). |
| Export: MP4 / MOV | Done | H.264 or HEVC from a native timeline, BT.709 (AVFoundation). |
| Export: HEVC with alpha | Done | `mov` container with `alpha` option. |
| PNG render | Done | `vexel-png`; `render_png` over MCP. |
| Text (CoreText) | Done | ([HANDOFF.md](HANDOFF.md)). |
| Masks, opacity groups, patterns, filters, markers, stroke dashes | Done | Dashes are deferred in animation frames ([HANDOFF.md](HANDOFF.md)). |
| Local MCP server | Partial | 53 live-preview tools, including `batch`. Tool reference in [docs/mcp.md](docs/mcp.md). No verified live session with Codex or Claude Code yet. |
| Blend modes, gradients, JPEG, interlaced PNG | Planned | Typed disclosures, not rendered ([HANDOFF.md](HANDOFF.md)). |
| Documents over 32 MiB | Planned | Refused by design; needs streaming decode ([BLOCKERS.md](BLOCKERS.md)). |
| Signing, notarization | Planned | Owner-gated ([HANDOFF.md](HANDOFF.md)). App icon is new (generated artwork); bundle id is still a placeholder. |

## Build and run

Requirements: Apple Silicon Mac, macOS 13 or later, and the pinned Rust
toolchain (`rust-toolchain.toml`, installed by rustup).

```sh
cargo build --release -p vexel-app
./build/rust/aarch64-apple-darwin/release/vexel rust/fixtures/v1/anim/loop.svg   # open an SVG
./build/rust/aarch64-apple-darwin/release/vexel --mcp-preview                   # serve MCP on stdio
./build/rust/aarch64-apple-darwin/release/vexel --mcp-preview --grant-dir /Users/me/Artwork
```

The open-SVG command is the one used for the live checks in
[docs/ui/live-check.md](docs/ui/live-check.md). `--mcp-preview` and
`--grant-dir <dir>` (repeatable) are parsed in
[`launch.rs`](rust/crates/vexel-app/src/compose/launch.rs) and handled by
[`preview_app.rs`](rust/crates/vexel-app/src/preview_app.rs). Run tests with
`cargo test --workspace --locked`. The real-window UI script is
`scripts/e2e_ui.sh` (requires `VEXEL_E2E_OK=1`; see [docs/e2e-ui.md](docs/e2e-ui.md)).

## MCP quickstart

`--mcp-preview` serves MCP over stdio from the app binary. `--grant-dir <path>`
(repeatable) grants the file tools access to a directory. Argument parsing is in
[`launch.rs`](rust/crates/vexel-app/src/compose/launch.rs) and the stdio worker
is in [`preview_app.rs`](rust/crates/vexel-app/src/preview_app.rs).

The live preview advertises 53 tools: document open/close/list/info, tree and
element inspection and editing, grouping, undo/redo, `batch`, native timeline
tools, `animation_control`, `export_animation` (gif, mp4, mov, apng, frames), and
`render_png`. Client setup, the tool reference and benchmarks are in
[docs/mcp.md](docs/mcp.md).

## Limits and known gaps

Full list: [BLOCKERS.md](BLOCKERS.md), [HANDOFF.md](HANDOFF.md) and
[STATUS.md](STATUS.md).

- Arc path commands (`A`/`a`) are unsupported.
- Files over 32 MiB are refused by design ([BLOCKERS.md](BLOCKERS.md)).
- Corpus fidelity is partial. Known failing or deferred files include
  columbiarivermap, corsica (`<tspan>` text), ideogram (clip aggregate cap) and
  france-osm (render limit) ([STATUS.md](STATUS.md)).
- MCP: no verified live session with Codex or Claude Code yet, and long-running
  progress and cancellation are not implemented ([docs/mcp.md](docs/mcp.md)).

## Benchmarks

Results: pending. No benchmark numbers are published as results. Per-phase MCP
timings in [docs/mcp.md](docs/mcp.md) are from a headless rig run.

## Status

- macOS only. Apple Silicon, macOS 13 or later.
- Not signed with a Developer ID and not notarized. Release bundles are
  ad-hoc signed ([HANDOFF.md](HANDOFF.md)).
- The app icon is new; bundle id (`app.vexel.Vexel`) is still a placeholder
  ([HANDOFF.md](HANDOFF.md)).
- Full UI and end-to-end acceptance is at the end of feature work. The showcase
  recording has not been made ([HANDOFF.md](HANDOFF.md)).

## Documentation

- [STATUS.md](STATUS.md): current status (live source of truth)
- [CHANGELOG.md](CHANGELOG.md): changes since `v1.0.0-alpha`
- [HANDOFF.md](HANDOFF.md): earlier status snapshot
- [BLOCKERS.md](BLOCKERS.md): known blockers and gaps
- [docs/mcp.md](docs/mcp.md): MCP server and tool reference
- [docs/ui/live-check.md](docs/ui/live-check.md): live-window verification
- [docs/e2e-ui.md](docs/e2e-ui.md): real-window UI e2e script and manual checklist
- [docs/product/vexel-goal-and-delivery-plan.md](docs/product/vexel-goal-and-delivery-plan.md): plan

## License

vexel is distributed under the Mozilla Public License 2.0 (MPL-2.0). See
[`LICENSE`](LICENSE) for the license text.
