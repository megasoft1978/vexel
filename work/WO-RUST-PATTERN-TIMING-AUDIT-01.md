---
id: WO-RUST-PATTERN-TIMING-AUDIT-01
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 3ef6998e4f1f32dcedbf44194795c6db02ec96f5a8c414cf3f3a5a598600ebbb
parallel_safe: true
network: false
---
# Bounded cross-engine static-child timing audit
Read-only integration architecture audit, not another broad rendering plan.
Original pattern.c:239-345 traverses a static child then calls raster_shape_render
BEFORE parent decode.c:214 parses root animation. The child is not passed through
svg_anim_parse. Accepted provisional Scene177 requires actual late parent
animation before ready SceneView; request_mapped_traversal yields pending view.
Determine exact actual safe path needed to render that child during traversal
without fake emptyanimation, earlyparentparse, parentreadyescape, lost source/
mask/style/opacity, or delayed errorpriority. Inspect actual accepted Scene APIs,
current decoder candidate and source-original flow, not historical signatures.
Propose MINIMAL complete implementable boundary/adapter with precise types,
ownedsource/viewport/draft/ticket/job/lifetime/control guards and original child
renderfailure-before-parent-lateanimation proof. Preserve177accepted Scene tests/
compilefail pendingparent guard; explicit static role is different from a ready
animated parent, no weakened originalguard or invented backend. Consider nested
patterns and media callbacks transaction/deferred effect ordering ONLY as needed
for this actual same call chain. No unrelatednewarchitecture/fullengine rewrite.
If existing APIs already suffice, show real exact call path/types and why. If not,
return concrete source-gap patchscope/requirements and required regressioninputs.
Source implementation happens in separate coordinator-owned frozen order.

# Allowed paths
- artifacts/WO-RUST-PATTERN-TIMING-AUDIT-01/** (reports only; input-baseline/runner/order coordinator-owned)
# Protected paths
ALL sources/tests/manifests/locks/workorders/docs/state/ledgers/oldreports/input/
603/157/HEAD/index. READONLY no build/test/network/GUI/agents/reviewCLI/unsafe/
sourceedits/packages/stage/commit/reset/delete/billing. StandardSolMedium named
crossengine timing design, existing allowanceonly; stopquota/rateerror no retry.
# Verification
Exact source/type/call-chain references and declaration hashes; read-only proof.
No test/build/runtime results claimed. Root independent review of evidence before
future sourcework. Avoid whole-tree/history scans, read only relevant functions.
# Output
result.md<=500words, call-chain.json with line/symbol/hash evidence and exact
minimal typed API/adoption/test proposal or proved existing path. Full product
outcomes and remaining gates unchanged; no implementation/acceptance claim.
