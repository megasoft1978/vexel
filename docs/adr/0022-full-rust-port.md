# ADR0022: Full first-party Rust port

- Status: accepted (language/scope/owner direction; detailed stack pending research)
- Date:2026-10-14
- Decision owner: project owner

The owner stopped the C implementation and requested a full Rust port, internet
research and maximum useful parallel Luna codex exec execution. Rust supersedes
ADR0001 language/build decision; no shipped first-party C/Objective-C backend is
the finish line. Platform interfaces and system macOS frameworks remain.
Application, custom UI, owned XML/SVG/rasterizer, document/project/recovery,
animation, MCP, developer tools and test harnesses all belong in the port.
Existing C code/candidates/tests/evidence are retained as migration references.

ADR0018 complete product scope and final software acceptance+recorded
Codex character/animation demonstration remain required. macOS13+ AppleSilicon,
MPL2, source preservation, no external SVG fetching, minimal reviewed/pinned
dependencies and no GPL editor source remain locked. Broad UI/E2E stays final.
Rust toolchain/crate choices follow six scoped primary-source research reports
and separate ready implementation orders; no speed, memory or size superiority
is presumed without measured parity evidence.
