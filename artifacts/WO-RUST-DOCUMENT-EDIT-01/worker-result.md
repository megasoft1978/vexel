# WO-RUST-DOCUMENT-EDIT-01 candidate

Status: complete, uncommitted candidate for independent root review; not accepted or integrated.

Implemented the entire named C editor and document-view scope. The parity ledger maps all 112 function symbols, internal types, limits, ownership rules, errors, and test coverage with no pending implementation entries.

The piece table now shares immutable blob owners and charges actual Vec capacity until the final piece, command, or snapshot releases it. Descriptor and history capacity allocations are fallible and prepared before publication. Routine commits invalidate the cache; model/token/span rebuilding occurs once on demand. Returned-handle edits prepare their eventual cache before committing. Snapshots remain readable concurrently and after editor destruction.

Preserved root-only fragment auto-ID policy, decoded public lookup and the C collision helper’s raw fast path, verbatim nested duplicate IDs, source gaps, namespaces, quotes, labels, state IDs, coalescing, trimming, redo branches, XML-region validation, and saved-cursor behavior. Save retains original diagnostics/native flags and marks saved on OK or rename while reporting durability failure.

Safe adaptations are documented precisely in parity-map.json: typed null exclusion, owned Arc lifetimes, allocation failure Results, transactional history jumps, and measured outputs without partial writes. Only standard Arc control-block allocation remains infallible.

Rust 1.99 locked/offline check and tests passed: 2 unit, 50 integration, and 1 compile-fail documentation test. Both original 200,000-case seeded sequences passed. Native save fixtures remain under .cache/edit-save-*.

Work-order validation, the 1,280-entry ledger validation, and diff checks passed. All 16 named legacy source/assertion files match the original dirty-source baseline. Shared manifests, peers, ledgers, docs, and the frozen order were not edited. Full 100 MB integration remains the separately scoped feature specified by the order.
