# WO-RUST-SVG-DECODE-01 R2 — continuation 4 candidate

Status: **incomplete; do not accept or integrate yet.**

Changed decoder request tracking from one pending shape request to a bounded
64-entry LIFO stack. Nested scopes use the accepted
`SceneBuilder::begin_nested_request` API, static pattern tickets bind to the
active scope, and abort unwinds request scopes before draft rollback. Added
fixture-backed scoped production regressions for nested success, failed render,
cancellation, output reclamation, and exact source-owner identity. Full parity
map retains all 113 catalog symbols; no old predicate was removed or renamed.

Verification after the final source split: decoder check passed; package tests
passed **76/76** across 25 result blocks; focused production-engine (10),
production-failure (5), probe/scan (2), capacity journal (2), and M4 boundary
(3) tests passed. Scoped rustfmt and the R2 ready validator passed. Exact logs,
commands, hashes, and the full parity map are in this directory.

Two independent-review findings remain open. The public custom-element helper
has no active draft/request or decoder suspension event, so resolved/missing
clip finalization and typed pattern routing cannot be completed through its
current API. Exact original decoder lookup/paint counters and cache predicates
also cannot be registered against real paths: the accepted paint crate is
registration-only, `DecodeSource` exposes no lookup-probe counter, and the
decoder manifest omits the tile-cache crate. See `blocker-api-proof.md`.

The work-order validator passed. The ledger validator failed because revision 1
of this work order is not terminal; shared ledger state was left unchanged. The
requested accepted-child-requests seed JSON is absent, so its five prior-worker
hashes could not be compared; no Scene source was edited. Current hashes are
recorded, with this limitation explicit.

This is a candidate repair continuation only. No full-product, integration,
renderer, UI, or release completion is claimed.
