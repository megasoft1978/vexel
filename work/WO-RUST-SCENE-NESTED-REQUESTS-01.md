---
id: WO-RUST-SCENE-NESTED-REQUESTS-01
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: codex-coordinator
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: e9e02cd4669ef4d20ef0b6c6c9092c62a49e8e9a61f22ac1f1a316c0fdef8a6a
parallel_safe: false
network: false
---
# Bounded nested request scopes for real static SVG patterns

The accepted `SceneBuilder` exposes one active `RequestToken`, but production
SVG patterns can contain nested patterns. A parent pattern must keep its mapped
request alive while a child pattern opens, renders and publishes its own request.
Implement the missing shared capability without weakening existing request,
draft, source, scene-owner, generation, capacity, or rollback checks. This is a
shared Scene contract prerequisite to the full decoder; it is not decoder or
renderer acceptance.

## Required behavior

- Replace the single active request slot with a fixed-capacity, bounded LIFO
  request stack. Maximum active request nesting is 64, matching scene draft
  nesting. Do not add heap allocation or dependencies for request metadata.
- A nested request is legal only for the current top `DraftToken`, its owning
  scene, and the active parent request lineage. Unrelated, stale, foreign,
  wrong-generation, replayed, double-used, or out-of-order tokens fail closed.
- Inner request commit removes only the inner request scope and restores the
  parent request as current. Inner rollback removes only work since the inner
  mark and restores the parent scope and all pre-child prefix handles. A later
  parent rollback also removes committed descendant scenes and work created
  since the parent mark. Cleanup remains unconditional after cancellation.
- Mutations and shape admissions are authorized only by the top request token;
  active parent request state continues to protect its pre-request prefix and
  cannot be committed, rolled back, sealed, frozen, or bypassed while a child
  request is live. `request_identity`, `request_allows_draft`, draft protection,
  scene protection, and static-pattern child minting must all use the correct
  lineage and top-of-stack rules.
- Preserve existing public request/draft APIs, ready/pending/static capability
  boundaries, current cache/publication behavior, and every existing assertion.
  Do not use scene-wide epochs or broad copies as request identity/rollback.

## Required proof

Add `rust/crates/vexel-scene/tests/nested_request_contracts.rs`. Exercise at
least three nested scenes/requests; correct parent-child ticket/source/owner and
mapped-static request behavior; inner success followed by parent success; inner
failure/rollback with parent continuation and unchanged prefix identities;
committed child rollback by its parent; cancellation cleanup at each depth;
wrong, stale, foreign, replayed and out-of-order tokens; rejected parent mutation
while a child request is active; and the exact 64-level boundary plus overflow.
Check live capacities and allocation history after nested rollback. Include a
compile-fail ownership proof where the request borrow must block builder
mutation. Retain all existing fixture-only labels and make no runtime-renderer
claim.

## Allowed paths

- `rust/crates/vexel-scene/src/**`
- `rust/crates/vexel-scene/tests/nested_request_contracts.rs` (new only)
- `rust/crates/vexel-scene/README.md`
- `artifacts/WO-RUST-SCENE-NESTED-REQUESTS-01/**` (except immutable baseline)

Coordinator alone handles accepted-main seeding, manifests/lockfiles, shared
integration, `STATE.md`, `LEDGER.md`, and `ledger.jsonl`.

## Protected paths and execution

All other crates, manifests, existing tests/assertions/literals, prior work
orders/evidence, original C/source oracles, `HEAD`, and index are protected.
No unsafe code, dependency changes, network access, GPL/editor source, system
installs, unsupported OS work, staging/commit/reset/delete, GUI, or UI/E2E tests.
Keep every changed source/test under 800 lines. Preserve exact accepted Scene
inputs and record their hashes before editing. Use an isolated current-source
worktree; one writer for all Scene files. Standard existing subscription only,
Luna appropriate effort; no extra credits. Stop on quota/rate errors.

## Verification

- `cargo +stable check -p vexel-scene --locked --offline`
- `cargo +stable test -p vexel-scene --locked --offline`
- `cargo +stable fmt --all -- --check`
- `python3 scripts/wo_validate.py WO-RUST-SCENE-NESTED-REQUESTS-01 --ready`

Record exact logs, nested API behavior, limits/cancellation/capacity evidence,
source lengths, input and protected-path hashes, and a complete result. Run an
independent Luna High review, then fresh main checks and exact preservation audit
before accepting or seeding the decoder. No decoder, full-renderer, native app,
UI, performance, live MCP, release, or video-demo completion claim.
