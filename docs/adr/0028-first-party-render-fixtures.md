# ADR0028: first-party rendering fixture dependencies

Status: Accepted metadata prerequisite; LunaLow independent review and lockedoffline workspace/protected audit pass. Work order: WO-RUST-RENDER-FIXTURE-SETUP-01.

Add only already accepted local svg-animation and svg-path dev dependencies to
render-owner crates where their direct graph does not expose fixture constructors.
LexicalDocument supplies owned original-source XML tokens; parse_owned returns
actual animation data; parse_path returns actual owned geometry. This permits
original pure algorithm assertions on a real sealed SceneBuilder/SceneView,
without changing production dependencies or introducing a fake provider.

Exact additions and local manifest/source pins are recorded in
render-fixtures.lock.json. Workspace package version0.1.0/MPL2; no new external
packages/versions/checksums/downloads. Runtime render/provider/composition and
full original mixed oracles remain mandatory later. Metadata acceptance requires
independent review/build/protected audit; it is not fixture/engine/app acceptance.
