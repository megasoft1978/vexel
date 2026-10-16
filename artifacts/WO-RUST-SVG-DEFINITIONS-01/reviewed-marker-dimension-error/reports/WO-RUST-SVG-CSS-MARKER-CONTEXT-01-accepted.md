# Accepted exact marker context API

CssPaint::without_inherited_markers copies the computed context, clears only the
four marker properties and preserves source identity/all other property values
and spans. Parents remain intact; explicit children still cascade normally.
Owned legacy marker.c performs the same four clears; no allocation/general setter.
All63accepted CSS entries remain byte-identical; three newcases pass for exact
clearing/span retention, rawshorthand/explicitchild and initial/idempotent context.
Main lockedoffline check/test/WO and ledger/diff/preservation audit pass.

Initial newfixture wrongly expected CSS itself to expand shorthand; corrected
to actual legacy rawCSS semantics without changing engine/assertions. Initial
review wrongly inferred readonly prevented inspection; second source review
verified method but deferred evidence judgment. Both preserved; evidence recheck
LunaLow ACCEPT independently hash-compared all10original test files and read actual
passing command logs. Acceptance covers only this API; full definitions/renderer/
app/GUI/finaltestedrelease/authenticMCPrecording remain required.
