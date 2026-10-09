# Lane C state

Lane C owns the document model, spatial index, document tests, and document
module documentation. Renderer/SVG and UI implementation paths belong to other
lanes; keep shared CMake edits limited to registering Lane C targets.

## Completed

- `WO-M5-01` merged to main at `f3c36f6cc03d66f4f67db74315f89dc570b46eba`. It adds a bounded source-backed XML element model, document-scoped stable handles, source-backed tag/attribute/range views, subtree tombstones, and the read-only paged tree interface for Lane B. Debug/Release, ASan+UBSan, UBSan, full verification (13 CTests, 201 Python tests), focused tests, and XML tokenizer fuzz smoke pass. Fresh read-only review round 5 passed with no blockers; rebase verification is in `artifacts/WO-M5-01/merge-verify.txt`.

## Next

1. Implement attribute overlays and unknown namespace/subtree preservation
   (`M5-02`), then minimal-diff save and recovery journaling (`M5-03/04`).
2. Implement spatial hit testing/index build measurements (`M7-01`), transform
   operations (`M7-03`), transactional undo/redo (`M9-01`), and document-only
   style/layer/path operations (`M8-01..03`).

The tree view header is read-only and allocation-free so Lane B can query node
ranges without creating a second model.
