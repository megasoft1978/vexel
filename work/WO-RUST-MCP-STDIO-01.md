---
id: WO-RUST-MCP-STDIO-01
revision: 1
status: READY
milestone: RUST-MCP-RUNTIME
owner: project-owner
executor: gpt-6-luna
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: e6ce95a467b1dc6d078f2ae87ff18a32df0acd1f1439f664922b518059dc4f03
parallel_safe: true
network: false
---
# Goal and frozen scope
Implement the bounded Rust stdio transport around the already accepted
`vexel-protocol` MCP JSON-RPC session. Add a `vexel-mcp-stdio` library exposing
sync and async-backend serving functions over caller-owned `BufRead`/`Write`
streams. Each complete JSON-RPC line is passed once to the accepted protocol;
nonempty responses are written as one line and flushed. Preserve notifications
with no response, EOF behavior (including one final unterminated frame), CRLF,
and exact response bytes. No global stdin/stdout, process startup, sockets,
app/document backend, task execution, MCP schema changes, or app integration.

Read and retain no more than `vexel_json::MAX_BYTES` per frame. Implement bounded
incremental reading with checked, fallible growth; never call an unbounded
`read_line`/`read_until`. Empty, oversized, or I/O-failed frames return a typed
transport error without calling a backend with truncated bytes. Oversize is a
fatal stream error; do not drain an unbounded peer frame. Handle partial writes
through `write_all`; propagate write/flush failures. Do not print diagnostics to
the protocol stream. The transport remains synchronous: async MCP requests are
handed to the accepted async backend/protocol semantics, not spawned here.

Use only `std`, `vexel-protocol`, and the accepted JSON size constant. Keep the
public surface small and document ownership, blocking behavior, stream framing,
limits, error behavior, and composition obligations. No default/fake app backend.

# Files allowed to touch
- `rust/crates/vexel-mcp-stdio/**` (new crate only)
- `artifacts/WO-RUST-MCP-STDIO-01/**` (except immutable seed)
Coordinator alone prepares the candidate workspace, registers its member and
lockfile, changes this frozen order, and edits main manifests/state/ledgers.

# Protected paths
All existing Rust crates/tests/APIs/manifests except coordinator-owned isolated
workspace registration, legacy C/Objective-C, fixtures/oracles, prior orders and
evidence, app/UI, `STATE.md`, `LEDGER.md`, `ledger.jsonl`, HEAD and index. No new
dependencies beyond accepted local workspace crates, unsafe, network, external
processes, unsupported OS code, GUI, broad UI/E2E, staging, commits, reset,
deletion, or nested workers. Keep every source/test file below 800 lines.

# Verification
In the isolated candidate workspace, using the existing Cargo cache and offline:
- `cargo +stable check -p vexel-mcp-stdio --locked --offline`
- `cargo +stable test -p vexel-mcp-stdio --locked --offline`
- `cargo +stable fmt --package vexel-mcp-stdio -- --check`
- `python3 scripts/wo_validate.py WO-RUST-MCP-STDIO-01 --ready`
- `python3 scripts/ledger.py validate ledger.jsonl`

Tests must exercise actual `vexel_protocol::handle`/`handle_async` calls and parse
the emitted JSON. Cover sync and async routes, multiple requests, CRLF, final
frame at EOF, silent notifications, exact response line framing and flush,
malformed JSON response behavior, empty/over-bound input rejection before
backend dispatch, partial reads/writes, reader/writer errors, and no partial
response on transport failure. Preserve protocol response bytes and session
semantics. Record exact commands, results, scope hashes, and remaining integration
gates. Candidate only until independent review, protected audit, and fresh main
checks. No claim of app control, live Codex interoperability, background task
execution, native UI, or full product completion.

# Output
New safe bounded stdio transport with focused tests, README, and concise exact
verification evidence. Leave all main integration to the coordinator.
