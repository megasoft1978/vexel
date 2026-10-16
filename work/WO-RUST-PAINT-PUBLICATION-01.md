---
id: WO-RUST-PAINT-PUBLICATION-01
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 641a3dcb3a4d1bcd879c733e2c1df58114ac3979e39dee225c7f8b09b362067b
parallel_safe: true
network: false
---
# Entire named shared publication gap
Close confirmed PAINT complete-resolver blocker: reusable source/definition/
viewport gradient template identity and graph stop publication lifecycle. Current
GradientCacheEntry owns local template stops, its stop_range/paint.stops are local,
while ResolvedPaint returns owned stops with no template identity. Never overload
that local range with graph indices or append duplicates per painted instance.

Define opaque source-coherent template identity from actual retained immutable
source/definition/viewport, matching original C template cache semantics. Permit
complete ResolvedPaint carrying explicit template identity or an equivalent safe
mechanism; original66scene test files must remain byte-identical. Existing
PaintCache and GradientCacheEntry literal construction must remain compatible.
No new unchecked ID constructor, synthetic identity, replacement gradient resolver
or mutable source escape. Actual original source/viewport/instance distinction
retained, including singular/bbox/emptyshape error behavior.

SceneBuilder owns transactional per-scene stop publication lookup: publish once,
reuse correct range for same template/scene across instances, never cross-source/
scene/stale slot ownership. Active draft parent guards remain, ordinary rollback
preserves independent child scenes, nested LIFO parent rollback invalidates all
created descendants/publications, rollback removes only publication suffix and
preserves earlier stable ranges. Appending ordinary unrelated stops must not break
reusable prior template ranges. Reused slots after rollback cannot authorize stale
cache graph ranges. Same-key conflicting stop contents must reject or distinguish
safe identity; never silently return another paint's stops.

Original stops bounds65536, gradients32768, local stops256, reference/work/byte
caps preserved, no smaller effective admission due per-instance stop duplication.
Use bounded efficient keyed lookup, explicit checked arithmetic/fallible reserve,
actual nested metadata/vector capacity/old-new overlap/history, cancellation in
all variable loops and before publication. Scene/mark/draft/freeze measurement
includes actual metadata and source counted once. Allocation may retain unused
capacity/history on failed reserve, but all logical tables/cache records stay
unchanged on error/cancel/limit. No selfreferences/foreign objects/newdeps/unsafe.

# Files allowed to touch
- rust/crates/vexel-scene/src/**
- rust/crates/vexel-scene/tests/paint_publication.rs (new only)
- rust/crates/vexel-scene/README.md (API/ownership note)
- artifacts/WO-RUST-PAINT-PUBLICATION-01/** (except immutable baseline/scope/seed)
Coordinator owns frozen order/shared integration/STATE/LEDGER/fullplan. Actual
writer works only in isolated current-source tree, one writer/shared module.

# Protected paths
All manifests/locks/config/oldRusttests (66scene entries)/otherRust/peers/
source603/157oracles/previousorders/evidence/immutable catalogs/index/state/docs/
ledgers. Source/tests<800, split cohesive new metadata helper modules as needed.
No network/newdep/install/GPL/editorSource/unsafe/unsupportedOS/Cshim/stage/
commit/reset/delete/agents/GUI/broadUIE2E or resource/service/billing switch.
CSS marker method separatelyunderreview; do not modify that source. Native queue
repair and animation source repair remain isolated. PAINT priorcandidate terminal.

# Verification
cargo +stable check -p vexel-scene --locked --offline
cargo +stable test -p vexel-scene --locked --offline
python3 scripts/wo_validate.py WO-RUST-PAINT-PUBLICATION-01 --ready
Original66tests byte-identical. New meaningful proof for reuse/no duplication,
viewport/definition/source/scene distinction, ordinary/nested rollback/prefix
preservation/slot reuse, stale/wrong identities, cancellation/failing allocation/
limits/transactional cleanup/actual capacity/source accounting/freeze, and
resolved paint payload publication semantics. No weaker oracle or fake engine.
Root independent LunaHigh review and freshmain protected audits/checks before
consumers resume; candidate exit not accepted. Standard existing allowance only.

# Output
Complete tested public shared publication API/ownership/rollback and current
source map; result.md<=450words, verification.json, parity/adaptation/API report.
Actual gradient parser/resolver/paint algorithm stays PAINT-owned and required;
full engine/native/UI/liveMCP/performance/tooling/finalQA/release/demo incomplete.
