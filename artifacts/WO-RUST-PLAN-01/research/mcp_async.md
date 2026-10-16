## MCP Rust migration research (read-only, 2026-10-14)

**Decision — port the owned protocol and bridge; don’t add `rmcp` to the product yet.** The active scope is `src/mcp/`, `app/vexel_mcp_main.c`, `src/app/mcp_broker_hook.c` and `src/app/mcp_broker_io.{c,h}`, plus MCP documentation. The checkout has `src/mcp/README.md`; I found no `docs/mcp/` directory. The unfinished `.worktrees/mcp-owned-capture` candidate remains reference only. No live client session was tested.

### Protocol and SDK facts

The official current MCP release is **2026-07-28**. Its Tasks feature is the `io.modelcontextprotocol/tasks` extension: the client advertises support on each eligible request; the server decides whether to return a task handle. The current lifecycle uses `tasks/get`, `tasks/update`, and `tasks/cancel`; completed results are returned by `tasks/get`. It has no `tasks/list` or `tasks/result`. [2026-07-28 Tasks specification](https://tasks.extensions.modelcontextprotocol.io/specification/2026-07-28/tasks) · [MCP 2026-07-28 release notes](https://blog.modelcontextprotocol.io/posts/2026-07-28/)

The **2025-11-25** release’s Tasks API was experimental core protocol. It used task-augmented requests and included `tasks/list` and `tasks/result`; progress tokens remain usable through task execution. The MCP project explicitly says this lifecycle changed in 2026-07-28. Keep legacy behavior behind negotiated protocol version; do not expose old methods as current Tasks. Legacy JSON-RPC `notifications/cancelled` cancellation is also distinct from modern `tasks/cancel`. [2025-11-25 Tasks specification](https://modelcontextprotocol.io/specification/2025-11-25/basic/utilities/tasks) · [official release migration explanation](https://blog.modelcontextprotocol.io/posts/2026-07-28-release-candidate/)

As of this research date, the official Rust SDK’s latest published version is **`rmcp 3.5.1`, released 2026-10-05**, licensed **Apache-2.0**. Its official README says it implements current Tasks with `TaskManager` and is compatible with the 2025-11-25 protocol release; however, its 3.x migration guide says the old task hint, `tasks/list`, and `tasks/result` were removed. So it is not a drop-in implementation of both Tasks generations. The SDK uses Tokio and has a sizable core dependency set; its published manifest also gives Rust 1.88 and Edition 2024. Apache-2.0 compatibility with Vexel’s MPL-2.0 is a licensing inference, not a legal determination. [rmcp 3.5.1 release metadata](https://docs.rs/crate/rmcp/3.5.1) · [official versioned manifest](https://github.com/modelcontextprotocol/rust-sdk/blob/rmcp-v3.5.1/Cargo.toml) · [official SDK Tasks and migration guide](https://github.com/modelcontextprotocol/rust-sdk)

**Inference/recommendation:** keep the product protocol owned and version-gated. This retains Vexel’s strict bounded JSON, relay framing, established tool schemas and compatibility obligations, while avoiding a broad async/runtime dependency for one local sidecar. Reconsider an exact `rmcp = 3.5.1` pin only if SDK APIs materially simplify interoperability; first review its complete transitive licenses and lockfile. Neither option implies a performance or binary-size gain.

### Rust seams and ownership

Port the existing bounded JSON parser/serializer, protocol dispatch, tools/schemas, broker, access grants, task store, export snapshot/job, Unix socket, and stdio relay as focused modules under `src/mcp/`; document crate APIs under MCP docs. Keep application callbacks in `src/app/` and the CLI composition in `app/`. Preserve Apple Silicon macOS 13+ only, MPL-2.0, custom UI, owned source-preserving SVG/XML/raster behavior, and no application network fetches.

Proposed interfaces: `JsonLineCodec` owns bounded framing; `ProtocolSession::handle(line, backend)` performs version/capability dispatch; `BrokerHost` exposes owner-thread document/project operations; `TaskRegistry` owns task state, retained results, cancellation and expiry; `ExportJob` receives an immutable snapshot and returns bounded encoded bytes/status. The app hook owns broker and viewer state on the UI/event-loop thread. Workers must never hold viewer, broker, document-edit, timeline-session, or borrowed request pointers. Use bounded channels for worker progress/results to owner, with explicit “full/skipped/closed” outcomes and reserved terminal capacity.

Bind each task to an unguessable ID, socket owner identity, and generation. Reject foreign, stale, expired, duplicate, and over-capacity operations without leaking task existence. Cancellation is cooperative; check it at frame boundaries and immediately before file publication. Commit only after owner revalidation of task generation and cancellation state; publish atomically without replacing a concurrently created destination. Keep grants and peer-UID checks in the owner bridge.

### Suggested ready-order sequence

1. Freeze Rust workspace and module API/provenance policy; map current MCP tests and invariants.
2. Port bounded JSON/framing and version-aware protocol dispatch; gate 2026 Tasks capability per request and retain 2025 wire compatibility.
3. Port task registry and Unix socket/relay with bounded queues, owner IDs and generation checks.
4. Port broker and grants behind an owner-thread host interface.
5. Port immutable export snapshots/jobs, cancellation and atomic publication.
6. Integrate app progress/status/cancel control; then run focused protocol, socket, task and publication parity gates.
7. Separately validate Codex/Claude live sessions; no such acceptance is established here.

### Interoperability gates and risks

Require byte/shape fixtures for both protocol versions: modern `resultType:"task"` only with that request’s capability, `tasks/get` terminal result/error and `tasks/cancel`; legacy task/result/progress/cancellation semantics only under 2025 negotiation. Verify stdio remains protocol-only on stdout, partial/coalesced lines and JSON-RPC ID matching remain correct, and bounded overload never silently drops terminal replies. Then prove owner-thread responsiveness, stale-generation rejection, cancellation-before-commit leaving no output, path-grant enforcement, and visible accessible UI cancellation. Unknowns include real client behavior for both Tasks eras and the safe legacy migration/retirement window. No build, test, install, worker, or live-session verification was performed.