---
id: WO-RUST-V1-NATIVE-DRAWING-01
revision: 1
status: READY
milestone: FULL-RUST-PROJECT
owner: project-coordinator
executor: codex-gpt-6-luna
base_sha: 849371835fa4cb8c3e48ff154778a3ec5d7de196
spec_sha256: 45ca1159041ac0d10047464dd7a124644b0c404619ee423d096205821fcfd2bf
parallel_safe: true
network: false
---
# Goal
Resolve, validate, render, and compose native timeline-v1 SVG drawing assets.

# Context and contract
Read `OWNER-DIRECTIVES.md`, ADR-0020 and ADR-0032, project asset table APIs, and owned SVG decode/render APIs. A cel references an immutable named complete `image/svg+xml` asset. Validate referenced assets fail-closed for scripts, external resources, duplicate IDs, and unresolved local refs. Require matching scene root pixel dimensions and every viewBox coordinate; no scale/crop. Resolve last cel at or before tick. Target ID is identity-only and supplies no placement. Render root coordinates and composite visible active layers in timeline order using premultiplied source-over with integer `+127` rounding. Enforce the cumulative 512 MiB frame-pixel cap and return no partial frame on failure.

# Allowed paths
- New `rust/crates/vexel-app/src/animation/native_drawing.rs`
- New `rust/crates/vexel-app/src/animation/native_drawing_tests.rs`
- Work-order artifact directory only

# Protected paths
All existing source, module registries, interfaces, manifests/locks, property/sampling files, app frame/MCP/session integration, recovery files, ADRs, work orders, ledger, C/Python evidence, and external paths. Do not run tests/builds; author focused tests only.

# Acceptance
- AC-1: selects active cels exactly at integer ticks and skips tracks with no prior cel or hidden state.
- AC-2: validates referenced SVG assets and exact geometry compatibility without fetching resources.
- AC-3: composes in timeline order with deterministic premultiplied integer source-over and enforces checked cumulative cap.
- AC-4: failure never exposes a partial frame; tests cover active-cel boundaries, mismatch, unsafe refs, ordering, and cap.
- AC-5: no new dependency; source files under 800 lines; tests left unrun until implementation phase ends.

# Deferred verification
`cargo test -p vexel-app --locked`
