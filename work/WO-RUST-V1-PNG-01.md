---
id: WO-RUST-V1-PNG-01
revision: 1
status: READY
milestone: V1-SVG-COMPOSITION
owner: project-owner
executor: gpt-6-luna
base_sha: 1d077a5539d44b64586b5416ccf7dab99f9f58f9
spec_sha256: 747355a01360c63c31052b6e9bf6c509afed935db44620813fedebee3f4b9ac2
parallel_safe: true
network: false
---
# PNG encoder crate and the MCP render_png tool

## Goal
Add `vexel-png`, a first-party pure safe Rust RGBA8 PNG encoder with no
dependencies, and a `render_png` MCP tool that renders the open document to a
PNG file inside a granted directory. Closes the HANDOFF gap "`render_png` (no
PNG encoder crate)". No PNG decoding in production code, no other formats.

## Dependencies
Read first: `HANDOFF.md`, `docs/adr/0029`, `vexel-gif` (crate layout and error
style), `vexel-raster` (`Tile`, `rgba()`), `vexel-app/src/render.rs`
(`render`) and `compose` (how a document becomes a `Tile`),
`vexel-app/src/session/{mcp,mcp_tools}.rs`, `preview.rs` (`Operation`, argument
parsing for `export_animation`), `vexel-mcp-access` (`resolve_save_path`,
`resolve_granted_directory`), `vexel-protocol/src/{tools.rs,protocol/
tool_methods.rs}`. The coordinator registers the crate in the workspace
`Cargo.toml` and `vexel-app`'s path dependency before the worker starts
(workers never edit manifests); if the registration is absent, stop.

## Assumptions
- PNG format per the W3C PNG spec, 8-bit RGBA (colour type 6), no interlace,
  one `IHDR`, one or more `IDAT`, `IEND`; filter type 0 only. Alpha is written
  as the tile holds it (the renderer's premultiplied/straight convention must be
  read from `vexel-raster` docs and stated in `result.md`; if the tile is
  premultiplied, un-premultiply with rounding and test it).
- zlib stream: header `0x78 0x01`, deflate blocks, Adler-32 trailer. Compression
  `Stored` (BTYPE 00, blocks <= 65535 bytes) and `Fixed` (BTYPE 01, fixed
  Huffman with greedy hash-chain LZ77, window 32 KiB, bounded work per row).
- A test-only decoder (inflate for stored and fixed blocks, CRC and Adler check,
  unfilter type 0) lives under `tests/` and is not part of the library API.

## Scope
1. `vexel-png`: `encode_rgba8(width, height, rgba: &[u8], Compression) ->
   Result<Vec<u8>, PngError>`; typed errors for zero/oversize dimensions
   (each axis <= 16384, total bytes <= 256 MiB), wrong buffer length, and
   allocation failure (`try_reserve`). `crc32` and `adler32` public for tests.
   `#![forbid(unsafe_code)]`, files <= 800 lines, no dependency.
2. `render_png` tool arguments: `documentId` (string), `path` (string, 1..1023
   bytes, leaf must end in `.png`), optional `width` and `height` (integers
   1..4096, default the document's rendered size capped to 4096 on the long
   axis, aspect preserved). Result JSON `{"path": <resolved>, "width": n,
   "height": n, "bytes": n}`. The path must be a new leaf or existing file in a
   granted directory (`resolve_save_path`); not granted, invalid or too large
   returns the existing typed tool errors, never a panic.
3. Write atomically: encode fully in memory, write to a temp file in the same
   directory, rename over the target; on any failure the existing target is
   unchanged and the temp file is removed. Output bounded to 64 MiB.
4. Routing in `vexel-app`: new `session/mcp_png.rs` (pure function over
   `AppDocument`, policy and arguments), one `Operation::RenderPng{..}` variant
   and parse arm in `preview.rs`, one dispatch arm in `session/mcp.rs`.
5. Protocol: add a `render_png` `ToolSchema`, include it in the preview tool name
   filter and the live-preview allow-list, and validate `path` presence.

## Non-goals
Interlace, palette/grey colour types, filters other than 0, ancillary chunks,
ICC, animation (APNG), GUI export buttons, reading PNG, performance tuning.

## Files allowed to touch
- `rust/crates/vexel-png/**` (new crate, sources and tests; `Cargo.toml` is
  written by the coordinator)
- `rust/crates/vexel-app/src/session/mcp_png.rs` (new), `rust/crates/vexel-app/
  tests/mcp_render_png.rs` (new), `rust/crates/vexel-app/src/session/mod.rs`
  (one `mod` line), `rust/crates/vexel-app/src/session/mcp.rs` (dispatch arm
  only), `rust/crates/vexel-app/src/preview.rs` (variant and parse arm only)
- `rust/crates/vexel-protocol/src/tools.rs` (new schema entry and filter only),
  `rust/crates/vexel-protocol/src/protocol/tool_methods.rs`
- `rust/crates/vexel-protocol/tests/render_png_listing.rs` (new)
- `artifacts/WO-RUST-V1-PNG-01/**`

Shared files: `mcp.rs`, `preview.rs` and `tools.rs` are also edited by
WO-RUST-V1-POLISH-01 in other hunks; touch only the hunks named here. The
coordinator applies both patches sequentially with `git apply --3way`.

## Protected paths
Workspace/lock manifests and every `Cargo.toml`, other crates, `vexel-gif`,
`vexel-render`, `vexel-platform-apple`, C/ObjC, `tests/`, goldens, oracles,
`docs/**`, `STATE.md`, `LEDGER*.md`, `ledger.jsonl`, other work orders, git
HEAD/index. No unsafe, external dependencies, network, installs, GUI launch,
commit, stage, reset or deletion.

## Acceptance criteria
1. Known vectors: `crc32("123456789") == 0xCBF43926`; `adler32("Wikipedia") ==
   0x11E60398`; the 8-byte PNG signature and chunk layout of a 1x1 image are
   byte-exact against a hand-written expected array.
2. Round trip with the test-only decoder, for both `Stored` and `Fixed`: 1x1,
   3x5, 64x64 gradient, 257x3, 300x300 (crosses the 65535-byte stored block
   boundary), flat colour, and a deterministic
   pseudo-random 128x128; decoded pixels equal input exactly, CRCs and Adler
   verify.
3. `Fixed` output for the flat 256x256 image is smaller than `Stored`; `Fixed`
   output for random data is never more than 1.1x the stored size.
4. Errors: width 0, height 0, 16385, wrong length, and a length overflow case
   each return the matching `PngError` variant with no panic and no allocation
   attempt beyond the bound.
5. `render_png` through `McpHost`: a granted directory and a document with a
   rect and ellipse produce a file whose decode has the requested size and
   matches the in-process render of the same document byte-for-byte (same
   conversion applied); an ungranted path, `..` escape, non-`.png` leaf,
   `width` 0 or 5000, a missing document id and a directory target each return
   a typed error and create no file.
6. Atomicity: a failure injected by a read-only target directory leaves an
   existing `.png` unchanged and no temp file behind.
7. Protocol: preview `tools/list` includes `render_png` with its schema;
   `tools/call` with bad arguments returns `invalid_arguments`; the non-preview
   list is unchanged and the existing protocol tests pass unmodified.
8. All new tests run without a display; existing `vexel-app` and `vexel-protocol`
   tests pass (counts may only increase); no `unsafe`;
   `cargo tree -p vexel-png` shows no dependency.
9. `result.md` (<=300 words) states the alpha convention, compression ratios
   measured on the test images, and what is unexecuted (live MCP client use).

If an AC cannot be met, stop with the exact blocker; no test weakening.

## Verification (frozen; repo-local CARGO_HOME/CARGO_TARGET_DIR, locked, offline)
- `cargo +stable check -p vexel-png -p vexel-app -p vexel-protocol --locked --offline`
- `cargo +stable test -p vexel-png --locked --offline`
- `cargo +stable test -p vexel-app --locked --offline`
- `cargo +stable test -p vexel-protocol --locked --offline`
- `cargo +stable fmt --manifest-path Cargo.toml --package vexel-png --package vexel-app -- --check`
- `python3 scripts/wo_validate.py WO-RUST-V1-PNG-01 --ready`
- `python3 scripts/ledger.py validate ledger.jsonl`
- `git diff --check`

## Evidence
`artifacts/WO-RUST-V1-PNG-01/`: `result.md`, `verification.json`, `ac-map.json`,
`changed-path-audit.json`.

## Worker and review
Worker: gpt-6-luna High, one writer, existing allowance only; stop on quota
errors. Candidate only until a fresh read-only reviewer reruns the frozen
commands and checks each AC. The coordinator registers the crate, integrates
the patch, reruns main-tree checks and a real MCP `render_png` call, then
updates STATE.md, LEDGER.md and ledger.jsonl.
