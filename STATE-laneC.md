# Lane C state

Lane C owns the document model, spatial index, document tests, and document
module documentation. Renderer/SVG and UI implementation paths belong to other
lanes; keep shared CMake edits limited to registering Lane C targets.

## Completed

- `WO-M5-01` candidate builds a bounded source-backed XML element model with
  document-scoped stable handles, source-backed tag/attribute/range views,
  subtree tombstones, and a read-only paged tree interface for Lane B. Debug,
  Release, ASan+UBSan, UBSan, full verification, focused tests, and XML fuzz
  smoke pass. Fresh read-only review round 5 passed with no blockers; merge is
  pending under the repository lock.

## Next

1. Review and merge `WO-M5-01` under the repository merge lock.
2. Implement attribute overlays and unknown namespace/subtree preservation
   (`M5-02`), then minimal-diff save and recovery journaling (`M5-03/04`).
3. Implement spatial hit testing/index build measurements (`M7-01`), transform
   operations (`M7-03`), transactional undo/redo (`M9-01`), and document-only
   style/layer/path operations (`M8-01..03`).

The tree view header is read-only and allocation-free so Lane B can query node
ranges without creating a second model.
