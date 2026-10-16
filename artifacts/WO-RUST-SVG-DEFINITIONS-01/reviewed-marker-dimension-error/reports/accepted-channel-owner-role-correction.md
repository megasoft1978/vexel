# Coordinator correction: owner anchor and definition are different roles

The equality finding was overbroad. The frozen accepted mask test at
rust/crates/vexel-scene/tests/render_integration_contracts.rs:588-616 uses
a group lexical owner but dispatches its child path, and must still pass.
The actual decoder definition_work.rs:95-100 mints the typed owner from host
(or current element), then lines129-137 dispatches the separately resolved
definition. Pattern/clip/filter URLs likewise can reference a different element.
DefinitionOwner.element is therefore a canonical planning/target anchor, not
universally the resolved definition ID. Both must be independently canonical
and source/draft/target-coherent, with returned descriptor bound to requested
definition and mapped source bound to actual owning shape as already validated.
Blanket equality changes valid behavior; neither original tests nor original
source should change to satisfy it. R2 new equality-only assertions encode that
wrong rule and must be replaced by correct role tests. No frozen assertion changes.
