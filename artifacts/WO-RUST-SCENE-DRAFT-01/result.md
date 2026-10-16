# Accepted scoped scene-draft finalization gate

Opaque nonborrowing tokens authorize only new records in bounded64-level LIFO
scopes, including scope-created children. Deferred shape/color/transform/clip/
mask/filter/group/fill/stroke finalization validates source/ranges/handles,
capacity overlap and cancellation before publication. Pending scopes cannot
freeze/seal; failed append or commit remains rollbackable. Rollback drops owned
descendants and restores ignored-notice counts/first offsets, preserving actual
retained capacity, allocation history and monotonic handle identities.

Initial LunaHigh review found ignored diagnostic state outside rollback; that
review and source snapshot are preserved in initial-review/. Added private token
snapshots and two meaningful nested-notice regressions; independent recheck ACCEPT.
Fresh main lockedoffline check and57focused tests pass:43existing byte-preserved
assertions/tests plus14draft cases. Work-order/ledger/diff and603legacy/157frozen
oracles/HEAD/index audit pass. No broad UI/E2E or actual decoder/renderer/native
window/application/performance/release/demo acceptance follows from this gate.
