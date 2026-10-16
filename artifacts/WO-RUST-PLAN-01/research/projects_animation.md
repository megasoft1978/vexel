## Research result — document, project, animation and recovery port

**Evidence scope:** I read `OWNER-DIRECTIVES.md`, `AGENTS.md`, `work/WO-RUST-PLAN-01.md`, the Rust port goal, ADR-0019–0021, and the named document/project paths and related app timeline/recovery paths. This was read-only; I ran no builds or tests. Current implementation evidence is from repository docs and C source; the Rust plan remains research-stage.

### Existing behavior and remaining work

The current C reference already has a borrowed-source document model, source-preserving piece table, bounded undo/redo with save-state tracking, edit snapshots, and atomic save adapters. Project code includes the bounded v1 bundle codec, timeline validation/edit history/session, drawing assets, frame composition, asset overlays, and recovery capsule codec/storage. The app docs also describe background recovery capture and discovery, basic opacity/transform keys, drawing-track/cel edits and retiming.

Missing or explicitly limited authoring/product work includes broader property-track/key editing, richer drawing/cel operations, onion-skin preview, loop sampling, animation export and playback UI completion, and recovery retention/expiry. ADR-0019 also requires full file-open/save, dirty-state, compatibility and recovery integration before claiming native-project support. These are **C reference capabilities and gaps**, not evidence of Rust parity. Confirm exact current app integration against implementation orders before porting.

### Decisions and interfaces

**Recommended ownership boundary (inference):**

- `src/document`: own immutable source storage, piece descriptors/blobs, command history, saved-state identity and snapshots. Expose an owned `Document` and transaction API: `begin`/apply validated edit/`commit` or rollback, plus `undo`, `redo`, `mark_saved`, snapshot and materialize. A rejected edit must preserve source, history cursor and redo branch. Snapshots own/pin all bytes they need across worker jobs.
- `src/project`: own bounded container and recovery codecs, asset table/overlay, validation contexts and file lifecycle. Decode into owned project state while retaining exact raw metadata/scene/timeline/asset bytes; expose typed views only when schema is recognized. Keep unknown supported payloads byte-identical and unsupported versions read-only. Separate codec from filesystem adapter.
- `src/animation`: own typed v1 validation/edit history/evaluation over integer ticks. Timeline edits transact as a unit and publish only after candidate serialization and validation. Animation scene IDs and asset names are stable serialized identifiers, never transient handles.
- `app` timeline/recovery: own UI/session orchestration and worker scheduling; call document/project/animation APIs, route undo deliberately, and surface conflicts/errors. Keep platform-specific durable publication behind macOS `platform/` interfaces.

These boundaries fit the current borrowed C contracts and ADRs; API names are proposed, not established.

### Primary-source technical evidence

- Rust `std::fs` docs (version shown **1.99.0**, dated **2026-09-28**) document rename replacement behavior and TOCTOU cautions, including use of atomic operations such as `File::create_new`: [std::fs](https://doc.rust-lang.org/std/fs/). `File::sync_all` attempts to sync file contents and metadata; it is more expensive than close and does not by itself specify parent-directory durability: [File::sync_all](https://doc.rust-lang.org/std/fs/struct.File.html#method.sync_all). **Inference:** preserve the existing same-directory temp → write → file sync → rename/no-replace publish → parent-directory sync sequence in the macOS adapter; do not treat `rename` alone as durable publication.
- `crc32fast` **1.5.2**, docs fetched **2026-10-14**, computes IEEE CRC-32 and supports incremental hashing; its published license is **MIT OR Apache-2.0**: [crate docs](https://docs.rs/crc32fast/latest/crc32fast/), [crate metadata](https://docs.rs/crate/crc32fast/latest/source/Cargo.toml). **Recommendation:** consider this one pinned dependency for ADR-compatible CRC, subject to provenance/manifest review. CRC remains corruption detection, not authentication, as ADR-0019/21 state.
- `serde_json` **1.0.154**, docs fetched **2026-10-14**, offers `to_writer` and typed/untyped representations; license **MIT OR Apache-2.0**: [docs](https://docs.rs/serde_json/latest/serde_json/), [crate source](https://docs.rs/crate/serde_json/latest/source/). **Recommendation:** don’t use generic deserialize/reserialize for lossless project edits: it cannot preserve arbitrary original formatting/unknown bytes as required. Retain an owned bounded span parser/patcher for timeline authoring; consider serde only for newly generated metadata if its dependency cost is accepted.
- **Inference:** integer ticks, rational frame rates, explicit interpolation and immutable drawing assets in ADR-0020 are the deterministic animation contract. I found no primary Rust library requirement for this model; keep evaluation owned and testable against the schema contract rather than adding an animation dependency.

### Ready work-order sequence and parallel scopes

1. Freeze Rust APIs and shared ownership/error types; coordinator owns workspace manifests, locks, interfaces and ledgers.
2. Port `src/document` model, piece table, transactional edits, history, snapshots and save-state behavior.
3. Port `src/project` v1 codec/assets and `src/animation` bounded schema validation as disjoint lanes after byte/view contracts freeze.
4. Port timeline edit/session/evaluation and project save/open integration; preserve opaque payload passthrough.
5. Port recovery capsule codec, then macOS atomic storage and app capture/discovery/conflict UI.
6. Port app timeline authoring/playback/onion-skin/export gaps; focused parity checks per order, broad UI/E2E at final acceptance.

Do not split writers within a module; gate each order on byte-for-byte fixtures, undo/redo branch behavior, save/reopen, corruption/limit rejection, opaque-payload round trips, and failure-safe publication. No speed or size gain is established. Unknowns include current Rust workspace/toolchain decisions, exact parity fixture inventory, recovery memory at maximum project size, and retention policy.