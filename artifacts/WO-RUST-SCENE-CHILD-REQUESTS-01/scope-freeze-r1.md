---
id: WO-RUST-SCENE-CHILD-REQUESTS-01
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: codex-coordinator
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 216a81dd054f064970ea1412f8470ca2cf67b1c2030035161a9ad8bd72c96157
parallel_safe: false
network: false
---
# Explicit nested request scopes for SVG pattern children

The accepted `SceneBuilder::begin_request` intentionally rejects reentrant
requests; a frozen accepted test asserts that behavior. Preserve it. Production
SVG pattern contents can themselves contain patterns, so add a distinct,
explicit child-request API that opens a nested request only from a live parent
request into the current descendant draft. This order is a shared Scene API
prerequisite to the decoder, not decoder or renderer acceptance.

## Required behavior

- Keep `begin_request(&DraftToken, ...)` single-scope semantics unchanged,
  including rejection while any request is active. Preserve every frozen test,
  public struct literal, and compile-fail ownership assertion byte-for-byte.
- Add a public checked API, named `begin_nested_request` or a clearly equivalent
  additive name, that takes the live parent draft and request token plus the
  current descendant draft. It may open only a new descendant scene/draft under
  that exact top parent request. Require builder owner, source, generation,
  draft ancestry, scene lineage, and request LIFO order to match.
- Track request scopes in a fixed-capacity array with an explicit depth; maximum
  combined nested request/draft depth is 64. No heap allocation/dependency for
  request metadata. A caller must explicitly commit or roll back each top scope.
- Inner commit removes only the inner request scope and restores its parent as
  current. Inner rollback restores the inner mark and preserves parent prefix
  handles/data and parent request usability. Parent rollback after committed
  descendants removes all descendant scenes/content created since its mark.
  Cancellation/error cleanup is unconditional and valid after the work control
  has stopped.
- Mutations/admissions authorize only against the top request. Every active
  request protects its pre-request prefix and prevents draft commit/rollback,
  scene seal/freeze, reentrancy through the old API, or out-of-order cleanup.
  `request_identity`, `request_allows_draft`, draft/scene protection, text
  accounting, static child request minting, commit, rollback, and abort must all
  follow the full lineage.
- Retain bounded metadata, checked arithmetic, honest capacity/history accounting,
  original error priority, no broad snapshots, no scene-wide epoch identity, and
  existing ready/pending/static capability boundaries.

## Required proof

Add `rust/crates/vexel-scene/tests/child_request_contracts.rs`. Leave the frozen
`decode_publication_contracts.rs` assertion rejecting nested `begin_request`
unchanged; prove the additive child API succeeds for the same legitimate nested
descendant case. Exercise at least three nested scenes/requests, child static
pattern request capability, inner success then outer success, inner rollback
then parent continuation with unchanged prefix identities, committed descendant
cleanup by parent rollback, cancellation at each nesting depth, wrong/stale/
foreign/replayed/out-of-order token rejection, parent mutation blocking while a
child request is active, and the exact 64-level boundary plus overflow. Check
capacity/allocation history after rollback. Add compile-fail proof that a live
static request borrow blocks builder mutation. Keep fixture-only labels; no
runtime renderer claim.

## Allowed paths

- `rust/crates/vexel-scene/src/**`
- `rust/crates/vexel-scene/tests/child_request_contracts.rs` (new only)
- `rust/crates/vexel-scene/README.md`
- `artifacts/WO-RUST-SCENE-CHILD-REQUESTS-01/**` (except immutable baseline)

Coordinator alone handles accepted-main seeding, workspace manifests/lock,
shared integration, `STATE.md`, `LEDGER.md`, and `ledger.jsonl`.

## Protected paths and execution

All other crates, manifests, existing tests/assertions/literals, previous work
orders/evidence, original C/source oracles, `HEAD`, and index are protected. No
unsafe code, dependency change, network access, GPL/editor source, system install,
unsupported OS work, staging/commit/reset/delete, GUI, or UI/E2E tests. Keep all
changed source/tests under 800 lines. Preserve exact current accepted-main inputs
and record their hashes before editing. Use an isolated current-source worktree;
one writer for all Scene files. Set `CARGO_HOME` to the repo-local accepted
offline cache when running Cargo. Existing subscription only, Luna appropriate
effort; no extra credits. Stop on quota/rate errors.

## Verification

- `CARGO_HOME=<repo-local-cache> cargo +stable check --manifest-path Cargo.toml -p vexel-scene --locked --offline`
- `CARGO_HOME=<repo-local-cache> cargo +stable test --manifest-path Cargo.toml -p vexel-scene --locked --offline`
- `cargo +stable fmt --all -- --check`
- `python3 scripts/wo_validate.py WO-RUST-SCENE-CHILD-REQUESTS-01 --ready`

Record exact commands/logs, nested API behavior, limit/cancellation/capacity
proof, source lengths, input and protected-path hashes, and concise results. Run
an independent Sol Medium review, then fresh main checks and exact preservation
audit before accepting or seeding the decoder. No decoder, full-renderer, native
app, UI, performance, live MCP, release, or video-demo completion claim.
