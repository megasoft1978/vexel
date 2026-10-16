# W4 MCP lane

## Built

- Bounded owned JSON parser/serializer, JSON-RPC framing, MCP 2026-07-28
  discovery and 2025-11-25 compatibility.
- Stdio sidecar and owner-thread viewer socket hook, with same-user socket
  checks, a private socket mode, and a visible editor status update after
  agent edits.
- 25 tools, 3 document resources, and an `inspect-document` prompt. Mutations
  route through `document_edit`; transactions clone and commit atomically.
- Explicit path grants from `VEXEL_MCP_ALLOWED_DIRS` or the per-user config
  file. No third-party dependency or network endpoint was added.
- Protocol test, 200,000-case seeded parser/argument fuzz target, and a raw
  stdio test that exercises the full tool surface against a headless broker.

## Limits and unverified items

The JSON, document, page, transaction, and render bounds are documented in
[`../mcp.md`](../mcp.md). The existing public edit API cannot insert a text
node; the published `element_add` schema lists `text`, but the call returns
`invalid_operation`. The active viewer document cannot be closed by MCP, and
`save_as` requires an existing target. Cancellation and progress notifications
are accepted but do not interrupt work or emit progress.

The execution environment rejects Unix-domain socket `bind(2)` with
`EPERM`, including under its writable temporary directories. This prevents
running the raw stdio/headless-broker test here; the test is retained for a
normal macOS runtime where local sockets are available. Report the exact
focused and full-suite commands and outcomes in the implementation handoff.

