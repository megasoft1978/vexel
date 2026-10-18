# Vexel

Vexel is a macOS-first vector drawing and animation editor written in Rust. It
has an owned XML/SVG parser and rasterizer, a custom-drawn interface, a native
animation timeline, and a local MCP server so that agents (Codex, Claude Code)
can edit the same open document as the app. Current status and open work are in
[HANDOFF.md](HANDOFF.md); the product goal is in
[ADR-0018](docs/adr/0018-product-goal-and-completion-gates.md) and the
[delivery plan](docs/product/vexel-goal-and-delivery-plan.md).

## Features

Status is taken from [HANDOFF.md](HANDOFF.md) (state as of 2026-10-17).
"Done" means implemented and covered by tests; live-window checks are listed
separately in [docs/ui/live-check.md](docs/ui/live-check.md).

| Feature | Status | Notes |
|---------|--------|-------|
| SVG open and render | Partial | Owned decoder covers a subset. Corpus: 21/39 files render ([HANDOFF.md](HANDOFF.md)). |
| `.vexel` project save and reopen | Done | V1 slice ([HANDOFF.md](HANDOFF.md)). |
| Native Open / Save / Save As panels | Partial | Save panel opens once on Cmd+S (round 5). Open panel not verified ([live-check.md](docs/ui/live-check.md)). |
| Drawing tools | Partial | Pointer drawing not yet verified on a real window ([HANDOFF.md](HANDOFF.md)). |
| Undo / Redo | Done | Verified on the real window (round 4) ([live-check.md](docs/ui/live-check.md)). |
| Animation timeline and playback | Partial | Transport and playback verified live (rows 2 and 7). Broader authoring is still open ([HANDOFF.md](HANDOFF.md)). |
| GIF export | Done | `vexel-gif` ([HANDOFF.md](HANDOFF.md)). Export-to-file on quit not yet verified (row 9, owner) ([live-check.md](docs/ui/live-check.md)). |
| PNG-sequence export | Partial | Not confirmed in [HANDOFF.md](HANDOFF.md). |
| PNG render | Done | `vexel-png` ([HANDOFF.md](HANDOFF.md)). |
| Text (CoreText) | Done | ([HANDOFF.md](HANDOFF.md)). |
| Masks, opacity groups, patterns, filters, markers, stroke dashes | Done | Dashes are deferred in animation frames ([HANDOFF.md](HANDOFF.md)). |
| Local MCP server | Partial | Tools exist ([docs/mcp.md](docs/mcp.md)). Live sessions with Codex and Claude Code are pending. |
| Blend modes, gradients, JPEG, interlaced PNG | Planned | Typed disclosures, not rendered ([HANDOFF.md](HANDOFF.md)). |
| Documents over 32 MiB | Planned | Refused by design; needs streaming decode ([BLOCKERS.md](BLOCKERS.md)). |
| Signing, notarization, app icon | Planned | Owner-gated ([HANDOFF.md](HANDOFF.md)). |

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
`cargo test --workspace --locked`.

## MCP quickstart

`--mcp-preview` serves MCP over stdio from the app binary. `--grant-dir <path>`
(repeatable) grants the file tools access to a directory. Argument parsing is in
[`launch.rs`](rust/crates/vexel-app/src/compose/launch.rs) and the stdio worker
is in [`preview_app.rs`](rust/crates/vexel-app/src/preview_app.rs).

The preview profile exposes 5 tools: `document_list`, `document_info`,
`element_add`, `undo`, and `redo`. The full tool profile is not yet exposed by
a documented flag. Client setup and the tool list are in
[docs/mcp.md](docs/mcp.md); the preview tool list is in
[docs/product/mcp-live-preview.md](docs/product/mcp-live-preview.md).

## Limits and known gaps

Full list: [BLOCKERS.md](BLOCKERS.md) and [HANDOFF.md](HANDOFF.md).

- Files over 32 MiB are refused by design ([BLOCKERS.md](BLOCKERS.md)).
- Corpus: 21 of 39 files render; 2 fail on root size, 6 are unsupported, 9 hit
  limits, 1 hits a render limit ([HANDOFF.md](HANDOFF.md)).
- Corsica and ideogram are known-unsupported ([BLOCKERS.md](BLOCKERS.md)).
- MCP: no verified live session with Codex or Claude Code yet, and long-running
  progress and cancellation are not implemented ([docs/mcp.md](docs/mcp.md)).

## Benchmarks

Results: pending. No benchmark numbers are published. Results will go in
bench/results/ (directory not yet present).

## Status

- macOS only. Apple Silicon, macOS 13 or later.
- Not signed with a Developer ID and not notarized. Release bundles are
  ad-hoc signed ([HANDOFF.md](HANDOFF.md)).
- The app icon and bundle id (`app.vexel.Vexel`) are placeholders
  ([HANDOFF.md](HANDOFF.md)).
- Full UI and end-to-end acceptance is at the end of feature work. The showcase
  recording has not been made ([HANDOFF.md](HANDOFF.md)).

## Documentation

- [HANDOFF.md](HANDOFF.md): current status
- [BLOCKERS.md](BLOCKERS.md): known blockers and gaps
- [docs/mcp.md](docs/mcp.md): MCP server
- [docs/ui/live-check.md](docs/ui/live-check.md): live-window verification
- [docs/product/vexel-goal-and-delivery-plan.md](docs/product/vexel-goal-and-delivery-plan.md): plan

## License

vexel is distributed under the Mozilla Public License 2.0 (MPL-2.0). See
[`LICENSE`](LICENSE) for the license text.
