# Protocol profile candidate evidence

Candidate: `.cache/WO-RUST-MCP-LIVE-PREVIEW-01/protocol` (detached at `19234a036ed6a8d40c2ccdaec9b8186806d70eca`).

## Changes

- Added `ProtocolSession::live_preview()` and kept the profile across `reset()`.
- Preview `tools/list` contains only `document_list`, `document_info`, `element_add`, `undo`, and `redo`; its `element_add.template` enum is `rect`/`ellipse`.
- Preview rejects calls to other tools and rejects other `element_add` templates before broker dispatch.
- Preview initialization/discovery advertises tools only; task, resource, and prompt capabilities are omitted and resource/prompt methods remain unavailable.
- Default sessions retain the complete schema generator and prior initialization/discovery capability response shape.
- Added four focused tests for tool exposure, template/tool rejection, default behavior, profile reset, and modern discovery capabilities. The modern discovery test also checks preview-specific instructions and that `tasks/get` returns method-not-found.

## Hashes

- `rust/crates/vexel-protocol/src/protocol.rs`: `c8870c56f3bcafd5a4560d6626d5a3ea02d87f57529f1e03945dcbc857ac39d3`
- `rust/crates/vexel-protocol/src/protocol/tool_methods.rs`: `b43f010e5ca1e4a2e01e95860d9f4e48cc979edfea0417d78c6a8bd5a3c31273`
- `rust/crates/vexel-protocol/src/tools.rs`: `198b0ddae062ec7955d03b4605b172e21ad01491ad64d034c4ee6f015638ebcd`
- `rust/crates/vexel-protocol/tests/mcp_preview_profile.rs`: `af3b83f684d34c53e2064c85a092cc08aca509ac3d172495453133d858fed2b2`
- Frozen protocol source inputs were checked against `source-seed.json` before editing: `protocol.rs` `a27ae30ff0c41469d0411c8fe0a51951b48fae2f9166ec40326619d0d490003b`, `tools.rs` `1d59c0e84f96008342c8149e93d098dd8140285f0f15188c36f16983586d5374`, `lib.rs` `0ecd075e2331f73ae3ce91bf7c87dc882f89267a1918e3abcba63b2657a61e08`, `async_protocol.rs` `eb5f084de751c6293e5ef3069c1ec0b13731e92b85ffc8bcff094d1a8b7f737c`, `tests/parity.rs` `a5fd1a5e4ba6a010b1e37cf1132e88f5da166c8519300f96a0123250c1f02e98`.

## Verification

- `rustfmt +stable --edition 2024 --check rust/crates/vexel-protocol/src/protocol.rs rust/crates/vexel-protocol/src/tools.rs rust/crates/vexel-protocol/tests/mcp_preview_profile.rs` — passed.
- `git diff --no-index --check` against the seeded protocol source files — passed; `git diff --check` — passed.
- Full copied workspace `cargo +stable check -p vexel-protocol --locked --offline` and `cargo +stable test -p vexel-protocol --locked --offline` could not resolve the unrelated workspace dependency `objc2-image-io`, absent from the offline registry cache.
- Isolated offline workspace with only local `vexel-base`, `vexel-json`, and `vexel-protocol`: `cargo +stable check -p vexel-protocol --locked --offline` — passed; `cargo +stable test -p vexel-protocol --locked --offline` — passed (4 profile tests and 9 existing parity tests).

No network, UI, shared workspace, manifest, lock, ledger, or application paths were modified.

## Follow-up requested by coordinator

- Preview initialize and server discovery now use preview-specific instructions describing inspection, basic rectangles/ellipses, and undo/redo; default profile instructions are serialized with the same bytes as before.
- Preview `tasks/get` and `tasks/cancel` return method-not-found without forwarding to an async backend.
- Re-ran candidate `cargo +stable check -p vexel-protocol --locked --offline` and `cargo +stable test -p vexel-protocol --locked --offline` in the isolated local-crate workspace: passed (4 profile tests, 9 parity tests). Scoped rustfmt check and `git diff --check`: passed.

## Source limit follow-up

Moved the existing tool-call, task, async-response, and resource-read helpers into `src/protocol/tool_methods.rs`, a private cohesive submodule. This keeps each formatted source file under the 800-line limit without changing behavior. Final formatted line counts: `protocol.rs` 546; `protocol/tool_methods.rs` 454. Re-ran `rustfmt --check`, `git diff --check`, and the isolated offline protocol check/tests; all passed (4 profile tests, 9 parity tests).
