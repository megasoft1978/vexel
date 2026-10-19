---
id: WO-RUST-V1-NATIVE-APP-01
revision: 1
status: READY
milestone: FULL-RUST-PROJECT
owner: project-coordinator
executor: codex-gpt-6-luna
base_sha: 849371835fa4cb8c3e48ff154778a3ec5d7de196
spec_sha256: 3c7d74698670e1adc86504858dc40440bb7d849e835374e990aef11baf0b4047
parallel_safe: true
network: false
---
# Goal
Connect the implemented native timeline-v1 data, journal commands, drawing/property/frame evaluators, and custom timeline panel to the Vexel app. Keep SVG SMIL as a separate selectable mode and preserve opaque unsupported timeline bytes.

# Locked constraints
- Read `OWNER-DIRECTIVES.md`, ADR-0017, ADR-0020, ADR-0029, ADR-0032 and ADR-0033.
- Apple Silicon macOS 13+ only; Rust only; no dependencies or external SVG fetches.
- Preserve source bytes and unknown project data. Native edits publish through the combined ProjectSession journal. Existing native command lineage and old-asset-prefix checks remain mandatory.
- Payload-only edits change combined dirty/recovery/UI snapshot state without falsely changing source-byte revision.
- Keep native playback separate from the SVG SMIL driver; at most one drives the canvas at a time.
- No builds/tests until all implementation and test-authoring slices are complete. Focused rustfmt and `git diff --check` are allowed during authoring.
- No broad UI/E2E during this work order. Leave GUI acceptance explicitly unverified unless a test-owned display is available.

# Interfaces and decisions
- `ProjectSession::timeline_candidate(settings, limits)` returns a bounded read-only candidate for the current scene/payload without journal publication.
- `ProjectTimelineCandidate::view()` returns a decoded borrowed `TimelineView` tied to that candidate and retains its owned scene/asset context.
- `AppDocument::state_revision()` is distinct from the existing source-only `revision()`. It advances on actual combined journal transitions and whole-document replacement.
- `AppDocument` exposes read-only candidate/viewport access and an atomic timeline-edit wrapper; no caller reaches private ProjectSession payload fields.
- `compose::svg_viewport` obtains copied geometry through the same owned SVG decode path used by composition, preserving nonzero viewBox origins.
- Native candidate supports bounded track visibility edits. First drawing-track creation goes through import of its initial SVG cel; no empty drawing track is invented.
- Use existing `NativeTimelineSnapshot` and `NativeTimelineAction` types. The native controller emits import effects; file selection and I/O stay in the import adapter.
- The prior Claude delegation wrapper failed before launch in this session. For the UI slice use Luna high with the `$frontend-design` skill, per the documented fallback; do not retry Claude.

# Slices
Paths are relative to `rust/crates/vexel-app/` unless a crate path is shown.

| ID | Owner / tier | Owned paths | Work | Deferred verification |
|---|---|---|---|---|
| N0 | Coordinator, Luna high | `src/project_session/{mod.rs,native_commands.rs,timeline_edit.rs,timeline_edit_tests.rs}`, `src/session/{mod.rs,native_animation.rs,tests_native_animation.rs}`, `src/file_ui.rs`, `src/file_ui/lifecycle_routes.rs`, `src/compose/mod.rs`, `src/lib.rs`, new `src/native_animation/mod.rs`; shared declarations/registrations only | Add read-only candidate and view access, AppDocument state revision and native wrappers, and state-revision-aware recovery observation. Preserve source-only revision and journal semantics. Freeze `native_animation` shared intent/effect/render-result contracts and register modules. Preserve lineage and exact old-asset-prefix enforcement. Expose bounded track visibility candidate operation after N2 signature lands. | `cargo test -p vexel-app --locked` |
| N1 | Luna medium | New `src/compose/viewport.rs` | Expose copied decoded viewport through existing owned decoder; preserve output dimensions, viewBox origin/extents and aspect data; propagate cancellation and decode errors. No second parser. Coordinator registers module. | `cargo test -p vexel-app --locked compose` |
| N2 | Luna medium | `../vexel-timeline/src/edit.rs`, `../vexel-timeline/src/edit/ops.rs`, new `../vexel-timeline/src/edit/visibility_tests.rs` | Add source-preserving bounded `set_track_visibility`, validate missing track and limits, preserve unknown fields and timeline order. Do not add empty-track creation. | `cargo test -p vexel-timeline --locked` |
| N3 | Luna high | New `src/native_animation/{controller.rs,snapshot.rs,commands.rs}` | Own stable selection, tick/viewport/playback mode and candidate cache keyed by state revision. Build the immutable existing native panel snapshot. Dispatch supported intents through AppDocument atomic APIs; reconcile stale selections on undo/open. Emit an explicit import effect. Disable unsupported actions with a useful reason. | `cargo test -p vexel-app --locked native_animation` |
| N4 | Luna high | New `src/animation/{native_driver.rs,native_stage.rs}` and focused tests | Use existing injected playback clock and `render_native_frame`; quantize to integer ticks, clamp play-once to final tick, honor loop mode, use decoded viewport geometry, bound queued jobs, reject stale state-revision/tick/size results, and retain last complete frame on failure. Preserve/display onion settings, but do not composite onion frames; report preview unavailable. Keep SMIL and native publication mutually exclusive. | `cargo test -p vexel-app --locked animation::native_` |
| N5 | Luna medium | New `src/native_animation/import.rs` and focused tests | Handle import effect through existing `FileUi::backend()`/`with_lifecycle()` and actual PanelBackend interfaces. Read bounded local SVG bytes, derive expected geometry from current scene, validate safety/geometry, generate collision-safe asset/track IDs, and commit asset+initial track/cel atomically. Cancel/failure leaves project and history unchanged. | `cargo test -p vexel-app --locked native_animation::import` |
| N6 | Luna high + `$frontend-design` | `src/native_timeline/{model.rs,layout.rs,paint.rs,interaction.rs}`, new `presentation.rs`, focused UI tests | Add explicit Native / SVG SMIL selector and native footer presentation using existing design tokens. Consume immutable snapshots and emit intents only. Preserve clipping, focus, disabled/error states, transient drag preview and one edit intent on release. Keep session, I/O and rendering outside UI. | `cargo test -p vexel-app --locked native_timeline` |
| N7 | Luna high | `src/main.rs`, new `src/native_animation/host.rs` and focused tests | Wire controller, import effects, native driver and selected presentation into the event/render loop. Route footer input once; reset transient state on replacement; refresh history/dirty/menu/status from combined state revision; cancel stale work; keep source files below 800 lines. | `cargo check -p vexel-app --locked` |
| N8 | Luna medium | New `tests/native_animation_integration.rs` | Author headless integration coverage for atomic actions, payload-only revision/recovery, mixed undo/redo, import cancellation and geometry rejection, save/reopen, unsupported-payload preservation, stale frames and deterministic tick boundaries. | `cargo test -p vexel-app --locked --test native_animation_integration` |

# Parallel order
- N0 starts first and freezes shared contracts. N1 and N2 may proceed in parallel with N0 on disjoint files. N6 starts after its immutable presentation/action contract is frozen. N4 can proceed on its owned new files; viewport integration waits for N1.
- N3 starts after N0 interfaces freeze; it waits for N2 visibility signature before dispatching visibility.
- N5 starts after N0 and N1 signatures freeze. N7 host wiring waits for N3-N6. N8 test authoring follows stable implementation APIs.
- One writer per module. Coordinator alone owns shared declarations, module registrations, work orders, manifests, locks and ledger.

# Acceptance
- Native and SMIL timelines remain separate and selectable.
- Supported native actions persist through the combined journal and survive save/reopen; undo/redo restores scene and payload chronology.
- Import adds an asset and its initial cel/track atomically, with local safe SVG validation against the current scene viewport.
- Playback updates the canvas at deterministic integer ticks; source edits, payload edits, seek, pause, mode switch and document replacement invalidate/cancel stale frames correctly.
- Unsupported timeline data remains byte-for-byte preserved and authoring is disabled with a visible reason.
- Focused tests are authored but unrun until the complete implementation wave is integrated.

# Final verification
After all implementation/test authoring is complete, run from `rust/`:

```sh
cargo check --workspace --locked
cargo test --workspace --locked
```

Then from repository root:

```sh
python3 -m unittest tests.test_ledger tests.test_wo_runner
python3 scripts/ledger.py validate ledger.jsonl
python3 scripts/wo_validate.py --ready WO-RUST-V1-NATIVE-APP-01
git diff --check
```

Report headless checks and GUI acceptance separately. Broad UI/E2E remains deferred by OWNER-DIRECTIVES.md.
