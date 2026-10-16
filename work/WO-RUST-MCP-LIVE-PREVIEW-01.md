---
id: WO-RUST-MCP-LIVE-PREVIEW-01
revision: 1
status: READY
milestone: MCP-LIVE-PREVIEW
owner: project-owner
executor: gpt-6-luna
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 20ff1c0d0f8e2144c012a54396fc608a285316b0214bf1a2f070644db29d1c06
parallel_safe: true
network: false
---
# Vexel live MCP drawing preview

Deliver a launchable macOS preview where a real Codex MCP `tools/call` adds a
basic vector shape and the Vexel window visibly updates before the MCP call
returns.

## Context and intent

- Read `OWNER-DIRECTIVES.md`, `STATE.md`, and `docs/product/vexel-v1-vertical-slice.md`.
- Preserve the owner-approved full product goal and V1 scope. This is an
  interim live demonstration, not a substitute release target.
- Reuse the accepted `vexel-mcp-stdio`, `vexel-protocol`, `vexel-json`,
  `vexel-v1-document`, `vexel-ui`, and `vexel-platform-apple` contracts.
- Use a preview-specific protocol profile so `tools/list` advertises only
  behavior implemented by this preview. Full protocol behavior must remain
  unchanged for existing sessions.

## Parallel ownership

Three isolated Luna workers may proceed concurrently with disjoint ownership:

1. Protocol profile worker: `rust/crates/vexel-protocol/src/**` and one new
   focused test file under `rust/crates/vexel-protocol/tests/`. Add
   `ProtocolSession::live_preview()`; its tool profile advertises only
   `document_list`, `document_info`, `element_add` narrowed to `rect`/`ellipse`,
   `undo`, and `redo`. Full/default sessions preserve current schemas and
   responses byte-for-byte. Preview must not advertise unavailable resources,
   prompts, or async task capability.
2. App worker: `rust/crates/vexel-app/src/**`, its package `Cargo.toml`, and
   app README. Implement `--mcp-preview`: create a native window, run the
   bounded stdio server on a background thread, and dispatch tool operations
   to the main UI event loop over bounded synchronous channels. The main loop
   owns the document and window. Implement new blank 512x512 preview, live
   rectangle/ellipse insertion, undo/redo, `document_list` and `document_info`,
   bounded activity feedback, and visible painting of the current preview
   shapes. Every successful add updates the V1 source-owning model and the view
   before returning success. Limit visible preview to 128 shapes and reject
   malformed, unsupported, out-of-bounds, degenerate, or over-limit input
   without changing source, history, pixels, or dirty state. The renderer is an
   explicitly bounded primitive preview and does not claim general SVG decode.
   Keep normal app launch behavior intact. Never write diagnostics to stdout.
3. Docs worker: add `docs/product/mcp-live-preview.md` with the exact Codex MCP
   configuration, startup/close behavior, available tools and a copyable prompt
   to create and revise a simple character. Describe only behavior implemented
   by this order; do not claim animation, SVG import/export, save/reopen, broad
   tool support, or a recorded final demo.

Coordinator alone changes workspace `Cargo.toml`, root `Cargo.lock`, work
orders, source seeds/scopes, `STATE.md`, `LEDGER.md`, `ledger.jsonl`, shared
interfaces, and root integration. Workers use isolated candidate worktrees and
build directories; no overlapping writers.

## Required behavior

1. Normal `vexel` launch remains the existing native shell. `--mcp-preview`
   opens the workbench and starts MCP over caller-owned stdin/stdout using the
   existing bounded stdio transport. Protocol bytes are the only stdout output.
2. The preview MCP profile advertises only implemented tools and no unsupported
   server capabilities. The existing full/default protocol profile keeps its
   frozen tool surface and semantics.
3. A Codex client can initialize, discover the preview tools, inspect the
   current document, then issue successive `element_add` calls for rect/ellipse
   shapes. Each accepted shape is visibly painted in the same open Vexel
   window, and the response is sent only after the corresponding frame has
   been accepted by the native window. `undo` and `redo` restore exact source
   and visible shape state.
4. Shape attributes are bounded, finite, normalized and within a 512x512
   document. Colors are six-digit RGB. A failed call returns an MCP tool error
   with no partial update. Limit an individual JSON-RPC frame by the existing
   transport cap and the preview scene to 128 shapes.
5. The app uses the existing custom UI and design tokens, keeps a dominant
   canvas, and displays recent MCP activity and operation results. It states
   clearly that this is a basic-shape live preview. No misleading active Save,
   Import, Export, animation, or unsupported editing controls.
6. Provide deterministic focused tests for profile tool exposure and unchanged
   default protocol behavior; argument validation; source/history atomicity;
   renderer bounds, mapping and visible pixel changes; and request-to-UI update
   ordering using headless channels. Do not automate or manipulate the user's
   active desktop.

## Allowed paths

- `rust/crates/vexel-protocol/src/**`
- `rust/crates/vexel-protocol/tests/mcp_preview_profile.rs` (new)
- `rust/crates/vexel-app/Cargo.toml` (candidate only; coordinator integrates)
- `rust/crates/vexel-app/src/**`
- `rust/crates/vexel-app/README.md`
- `docs/product/mcp-live-preview.md` (new)
- `artifacts/WO-RUST-MCP-LIVE-PREVIEW-01/**` except frozen seed/scope files
- `.cache/**` for isolated worktrees and build outputs only

## Protected paths and limits

Protect all other Rust crates, app sources outside `rust/crates/vexel-app`,
legacy C/Objective-C, fixtures/oracles/goldens/locks, platform APIs, UI tokens,
root workspace manifests/lock, full V1/product goals, work orders, ledgers,
HEAD and index. No third-party dependency, network, unsupported OS code, broad
UI/E2E suite, visible app launch, commit, staging, deletion, reset, or nested
workers. No placeholder MCP success; no unsupported tool may be advertised.

## Acceptance and verification

- Candidate: locked/offline protocol check/tests, app check/tests, scoped fmt,
  work-order validator, source/scope hash audit and `git diff --check`.
- Coordinator: add only existing local dependencies to `vexel-app`/root lock as
  needed; run `cargo +stable check -p vexel-protocol -p vexel-app --locked --offline`,
  the same focused package tests, package-scoped formatting, `python3
  scripts/wo_validate.py WO-RUST-MCP-LIVE-PREVIEW-01 --ready`, ledger
  validation, scope audit, and `git diff --check` after integration.
- Record exact output and source hashes in `artifacts/WO-RUST-MCP-LIVE-PREVIEW-01/`.
  Obtain one independent code review before main-tree acceptance.
- GUI rendering remains unverified unless an isolated display is available; no
  test may take over the user's active desktop. A build or headless raster test
  is not proof of native window runtime behavior.

## Non-goals for this preview

Animation, GIF export, Inkscape import/export, arbitrary SVG rendering,
native-project save/reopen UI, remote MCP, comprehensive accessibility,
full UI/E2E acceptance, packaged release, and the final recorded website/README
demo remain required later under the unchanged product goal.

## Output

Keep source below 800 lines per file. Preserve candidates and evidence. No
commit. Root integration is accepted only after independent review and fresh
main-tree focused checks. Keep the full product goal active.
