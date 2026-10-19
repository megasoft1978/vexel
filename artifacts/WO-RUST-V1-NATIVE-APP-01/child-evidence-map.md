# Native app wave child acceptance evidence map

Date: 2026-10-19. Scope: implemented Rust native-wave child orders, with supporting automatic recovery and CI follow-up.

## Evidence basis

`verification.txt` in this directory records `cargo check --workspace --locked` and `cargo test --workspace --locked -- --test-threads=1` exiting 0; all unit/integration/doc binaries completed, with repository-configured measurement/reproducer ignores disclosed. It also records 51 Python tests passing, ledger validation (2,315 entries after coordinator acceptance bookkeeping), parent readiness validation, CI policy check, `git diff --check`, and independent Sol review CLEAN.

The mappings below identify actual source test modules and functions. Their execution evidence is the **combined workspace pass**, not newly executed per-child commands or retained per-test output. No tests were rerun to create this map. No child test counts are inferred. Readiness validation checks frozen order metadata, not behavioral acceptance. The coordinator reports each child `wo_validate --ready` passed; this report is not a new independently captured readiness transcript.

The parent acceptance covers scoped headless native integration only. GUI acceptance, hosted Actions, performance outside the unchanged local pan evidence, live-client interoperability, release and full product acceptance are not established. Runtime tests do not independently prove dependency/protected-path/line-count criteria; those rely on the recorded review and any explicit source audits. This map does not invent missing audit output.

## Child mappings

### WO-RUST-V1-NATIVE-TIMELINE-UI-01

AC-1–AC-3: transport/selection intents, canvas-target fallback, transient drag and one release edit; integrated presentation is also covered by native_animation_integration. AC-4: source/review boundary; AC-5: authored tests exercised by combined pass.

Test package: `vexel-app`; evidence: combined workspace pass. Source modules and exact test functions:

- `rust/crates/vexel-app/src/native_timeline/interaction.rs`: `escape_discards_drag_preview_without_emitting_an_edit`, `keyboard_transport_emits_intents_only`, `first_track_can_target_the_canvas_selection_without_existing_rows`, `cel_drag_keeps_preview_transient_and_emits_one_edit_on_release`.

Limit: Headless interaction tests do not prove every painted track/key/focus/clipping state or live GUI appearance.

### WO-RUST-V1-NATIVE-DRAWING-01

AC-1: cel boundaries/hidden tracks; AC-2: geometry/local reference checks; AC-3: layer ordering/pixel cap; AC-4: failure regressions; AC-5: deferred tests now included in combined pass.

Test package: `vexel-app`; evidence: combined workspace pass. Source modules and exact test functions:

- `rust/crates/vexel-app/src/animation/native_drawing_tests.rs`: `active_cel_holds_until_next_boundary`, `rejects_non_increasing_cels_and_hidden_tracks_are_skipped`, `asset_must_match_scene_geometry_and_have_safe_local_references`, `composes_layers_in_timeline_order`, `enforces_checked_cumulative_pixel_cap`.

Limit: No independent allocator-exhaustion or visual GUI evidence is claimed.

### WO-RUST-V1-NATIVE-FRAME-01

AC-1: property-first and source preservation; AC-2: cel order, decoded strings, hidden tracks; AC-3: typed invalid-tick/property/drawing/render failures; AC-4: policy/dependency/source boundary is review evidence; AC-5: combined pass.

Test package: `vexel-app`; evidence: combined workspace pass. Source modules and exact test functions:

- `rust/crates/vexel-app/src/animation/native_frame_tests.rs`: `property_snapshot_is_rendered_first_and_source_stays_unchanged`, `drawing_tracks_composite_in_serialized_order_after_the_base`, `escaped_drawing_kind_and_asset_name_are_decoded_before_composition`, `later_cels_change_at_their_authored_ticks_and_hold_afterward`, `hidden_tracks_do_not_resolve_or_render_their_assets`, `reports_invalid_ticks_and_property_errors_with_typed_variants`, `maps_drawing_and_base_render_failures_without_returning_a_frame`.

Limit: The named tests do not establish every allocation-failure branch.

### WO-RUST-V1-NATIVE-PROPERTY-01

AC-1–AC-3: isolated snapshot, ordered scalar/RGBA/transform/visibility writes, smoothstep, target failures, hidden tracks and bounds; AC-4: review/source boundary; AC-5: combined pass.

Test package: `vexel-app`; evidence: combined workspace pass. Source modules and exact test functions:

- `rust/crates/vexel-app/src/animation/native_properties_tests.rs`: `applies_ordered_rgba_and_scalar_writes_to_an_owned_snapshot`, `samples_smoothstep_and_writes_transform_and_visibility`, `rejects_missing_and_duplicate_persistent_ids`, `skips_hidden_tracks_and_rejects_out_of_range_ticks`.

Limit: No exhaustive CSS cascade or allocation-failure claim beyond the identified assertions.

### WO-RUST-V1-NATIVE-SAMPLE-01

AC-1: endpoints/interpolation/serialized order; AC-2: typed values; AC-3: out-of-range rejection; AC-4–AC-5: source/review and combined pass.

Test package: `vexel-timeline`; evidence: combined workspace pass. Source modules and exact test functions:

- `rust/crates/vexel-timeline/src/sample_tests.rs`: `samples_endpoints_and_interpolation_modes_in_serialized_track_order`, `retains_typed_values_and_rejects_ticks_outside_the_timeline`.

Limit: Do not infer exhaustive malformed-value or hidden-track coverage solely from these two test names.

### WO-RUST-V1-DEFAULT-TIMELINE-01

Acceptance: empty new defaults, preservation of unsupported nonempty bytes; related loaded-empty-object preservation is in native_commands_tests. Deferred authored tests are included in combined pass.

Test package: `vexel-app`; evidence: combined workspace pass. Source modules and exact test functions:

- `rust/crates/vexel-app/src/project_session/tests.rs`: `round_trip_preserves_source_ids_order_and_animation`, `unknown_source_survives_byte_for_byte`, `unedited_foreign_source_saves_and_reopens_identically`, `opaque_project_payload_survives_edit_save_as_and_reopen`, `payload_allocation_failure_preserves_replaced_session`, `source_only_write_keeps_default_payload`, `undo_save_redo_and_history_cursor`, `edit_undo_edit_save_reopen_yields_post_edit_state`, `dirty_tracking_edit_save_and_undo_past_saved_point`, `save_without_path_is_typed_error_and_keeps_dirty`, `failed_opens_are_typed_and_leave_the_document_untouched`, `valid_bundle_with_non_svg_scene_is_invalid_scene`, `unwritable_directory_fails_without_touching_state_or_existing_file`, `missing_directory_and_bad_paths_fail_typed`, `resave_replaces_atomically_and_leaves_no_temp_files`, `mcp_preview_state_keeps_working_after_save_and_open`, `shortcuts_map_to_file_actions`, `unsaved_changes_prompt_text`.
- `rust/crates/vexel-app/src/project_session/payload_timeline_tests.rs`: `candidate_round_trip_keeps_manifest_and_opaque_assets_exact`, `candidate_initialization_is_read_only_and_replacement_has_new_identity`, `unsupported_timeline_stays_opaque_and_source_bytes_remain_unchanged`, `invalid_timeline_candidate_fails_without_changing_source`, `every_replacement_allocation_failure_preserves_the_source`, `invalid_asset_names_preserve_the_source`, `duplicate_asset_names_preserve_the_source`, `excessive_asset_count_fails_before_allocation_and_preserves_the_source`, `retained_byte_totals_include_each_opaque_component_exactly_once`.

Limit: Source-only default-write behavior needs its concrete assertions/review, rather than an inference from a workspace exit status.

### WO-RUST-V1-EMPTY-TIMELINE-01

AC-1–AC-2: caller-settings initializer builds decodable v1; AC-3: invalid settings/context; AC-4: empty versus unsupported nonempty distinction; AC-5: combined pass.

Test package: `vexel-timeline`; evidence: combined workspace pass. Source modules and exact test functions:

- `rust/crates/vexel-timeline/src/edit/initialize_tests.rs`: `initializer_builds_valid_empty_v1_payload_from_caller_settings`, `initializer_rejects_invalid_settings_and_invalid_context`, `empty_source_stays_distinct_from_unsupported_nonempty_source`.

Limit: No deterministic allocation-failure injection is identified in this initializer test module.

### WO-RUST-V1-HISTORY-BRIDGE-01

AC-1–AC-2: state/navigation atomicity; AC-3: coalescing boundary; AC-4: redo truncation; AC-5: eviction visibility; AC-6: combined regression pass plus source review.

Test package: `vexel-document-edit`; evidence: combined workspace pass. Source modules and exact test functions:

- `rust/crates/vexel-document-edit/src/history.rs`: `bridge_navigation_is_atomic_and_branching_drops_redo_tokens`, `explicit_boundary_prevents_attribute_coalescing`, `bounded_history_exposes_eviction_through_reachable_state_window`.

Limit: No additional independent memory/performance claim.

### WO-RUST-V1-HISTORY-WRAPPER-01

AC-1: delegation source review; AC-2: identity/reachability/navigation/rejection/boundary tests; AC-3: combined regression pass and source review.

Test package: `vexel-v1-document`; evidence: combined workspace pass. Source modules and exact test functions:

- `rust/crates/vexel-v1-document/src/history_bridge_tests.rs`: `history_state_ids_follow_reachable_cursors`, `history_navigation_restores_reachable_source_states`, `rejected_history_navigation_does_not_mutate_document`, `history_boundary_preserves_the_current_history_state`.

Limit: No live application history interaction claim.

### WO-RUST-V1-PAYLOAD-TIMELINE-01

AC-1: exact manifest/assets; AC-2: read-only candidate and replacement identity; AC-3: deterministic replacement allocation injection, asset rejection/count and exact retained bytes; AC-4: opaque unsupported bytes and combined pass.

Test package: `vexel-app`; evidence: combined workspace pass. Source modules and exact test functions:

- `rust/crates/vexel-app/src/project_session/payload_timeline_tests.rs`: `candidate_round_trip_keeps_manifest_and_opaque_assets_exact`, `candidate_initialization_is_read_only_and_replacement_has_new_identity`, `unsupported_timeline_stays_opaque_and_source_bytes_remain_unchanged`, `invalid_timeline_candidate_fails_without_changing_source`, `every_replacement_allocation_failure_preserves_the_source`, `invalid_asset_names_preserve_the_source`, `duplicate_asset_names_preserve_the_source`, `excessive_asset_count_fails_before_allocation_and_preserves_the_source`, `retained_byte_totals_include_each_opaque_component_exactly_once`.

Limit: Source size/no-dependency criteria rely on review/audit, not runtime tests.

### WO-RUST-V1-PROJECT-TIMELINE-EDIT-01

AC-1: explicit empty initialization/unsupported rejection; AC-2: invalid edits do not publish; AC-3: SVG geometry/safety/name/count and exact bytes; AC-4: atomic owned asset+track candidate; AC-5: combined pass.

Test package: `vexel-app`; evidence: combined workspace pass. Source modules and exact test functions:

- `rust/crates/vexel-app/src/project_session/timeline_edit_tests.rs`: `empty_payload_is_initialized_from_explicit_settings`, `unsupported_nonempty_payload_is_preserved_and_cannot_be_edited`, `edits_return_new_candidate_and_invalid_edit_does_not_publish_partial_state`, `drawing_asset_and_initial_timeline_reference_publish_together`, `candidate_visibility_edit_is_source_preserving_and_decodable`.
- `rust/crates/vexel-app/src/project_session/svg_asset_tests.rs`: `retains_exact_validated_svg_source_bytes`, `rejects_invalid_name_type_unsafe_svg_and_geometry`, `rejects_duplicate_names_and_asset_count_limit`.

Limit: Do not claim every allocation/limit branch is independently injected by these tests.

### WO-RUST-V1-SESSION-COMMANDS-01

AC-1: exact manifest/old assets and unsupported bytes; AC-2: single journal transition/editor boundary; AC-3: decoded sorted unique bounded IDs; AC-4: candidate/admission/no-op rejection preserves history; AC-5: combined pass.

Test package: `vexel-app`; evidence: combined workspace pass. Source modules and exact test functions:

- `rust/crates/vexel-app/src/project_session/native_commands_tests.rs`: `scene_ids_are_decoded_sorted_unique_and_bounded`, `session_exposes_current_sorted_ids`, `timeline_noop_and_operation_rejection_preserve_payload_and_journal`, `timeline_change_is_one_undoable_payload_transition`, `drawing_import_and_first_track_reference_commit_together`, `unsupported_payload_edit_is_rejected_without_publication`, `returned_candidate_and_edit_budget_rejections_preserve_journal_state`, `separately_opened_unsupported_candidate_is_rejected_before_publication`, `returning_the_unchanged_candidate_is_a_semantic_noop`, `rejected_operation_and_payload_admission_preserve_redo_and_checkpoint`, `payload_commit_preserves_separate_editor_undo_steps`, `drawing_import_preserves_manifest_and_all_existing_asset_bytes`, `loaded_empty_object_stays_opaque_and_byte_exact`, `returned_candidate_cannot_drop_or_modify_existing_opaque_assets`.

Limit: No GUI authoring or live MCP claim.

### WO-RUST-V1-SESSION-JOURNAL-01

Acceptance: rejected admission skips closure; partial-error undo; paired chronological navigation; no-op/branch; save checkpoint; failed open/recovery preservation. Combined pass also includes session/recovery caller tests.

Test package: `vexel-app`; evidence: combined workspace pass. Source modules and exact test functions:

- `rust/crates/vexel-app/src/project_session/journal_integration_tests.rs`: `scene_admission_failure_does_not_run_edit_or_change_combined_history`, `partial_scene_error_is_recorded_and_remains_undoable`, `scene_and_payload_states_undo_and_redo_in_chronological_order`, `payload_noop_preserves_redo_and_payload_edit_branches_history`, `save_checkpoints_combined_scene_and_payload_state`, `failed_open_and_recovery_replacement_preserve_live_history`.

Limit: The six named tests alone do not enumerate every coalescing/eviction/open-success route; lower journal and full wave review supply broader evidence.

### WO-RUST-V1-APP-JOURNAL-01

AC-1: identity/checkpoint; AC-2: no-op/partial-error/admission; AC-3: exact shared accounting and rejection; AC-4: chronology/branch/coalescing/eviction; AC-5: fallible reservations, staged clones and bounds; AC-6: combined pass.

Test package: `vexel-app`; evidence: combined workspace pass. Source modules and exact test functions:

- `rust/crates/vexel-app/src/project_session/history_tests.rs`: `scene_payload_scene_undo_redo_preserves_global_chronology`, `payload_branch_discards_redo_and_never_reuses_identity`, `scene_coalescing_replaces_only_scene_tail_and_payload_is_hard_boundary`, `checkpoint_and_dirty_state_follow_app_identity_through_navigation`, `dropped_and_noop_scene_admissions_preserve_redo_and_identity`, `partial_error_scene_observation_is_committed_and_unreachable_is_rejected`, `payload_admission_rejection_preserves_journal_and_candidate`, `shared_snapshot_bytes_count_once_and_identity_size_conflicts_reject`, `exact_payload_memory_bound_is_admitted_and_one_byte_over_is_rejected`, `scene_admission_fails_before_edit_when_id_cannot_advance`, `bounded_eviction_and_reachability_filter_preserve_current_state`, `stale_navigation_and_commit_ids_are_rejected_without_cursor_change`, `independent_scene_edits_remain_separate_undo_steps`, `initialization_rejects_payload_over_byte_bound`, `initialization_reservation_failure_preserves_input_and_initial_identity`, `admission_reservation_failures_preserve_cursor_ids_checkpoint_and_redo`, `scene_payload_clone_is_staged_before_mutation_and_finalization_never_clones`.

Limit: Source size and absence of dependencies are review/audit criteria.

## Parent integration and supporting recovery evidence

`rust/crates/vexel-app/tests/native_animation_integration.rs` is Cargo integration target `native_animation_integration` in `vexel-app`. Its eight tests cover atomic payload revision/no-op, mixed undo/redo, recovery/save/reopen, import cancellation/viewport mismatch, opaque payload preservation, escaped timeline strings, stale-target disabled state, and deterministic integer ticks/stale results. This is combined-workspace evidence, not a separate invocation.

N1 viewport, N2 visibility, N3 controller, N4 driver/stage, N5 import, N6 presentation, N7 host, and N8 integration are slices of the parent order; they are not separate child ledger IDs. Relevant additional source test locations include `src/animation/native_driver_tests.rs`, `src/animation/native_stage_tests.rs`, inline tests in `src/native_animation/import.rs` and `src/native_animation/host.rs`, and `vexel-timeline/src/edit/visibility_tests.rs`. Their inclusion does not turn untested GUI behavior into accepted evidence.

### WO-RUST-RECOVERY-AUTO-01 revision 3

AC-1–AC-3 map to immutable snapshot and policy/worker tests in `rust/crates/vexel-app/src/project_session/recovery_snapshot_tests.rs`, `src/recovery_policy_tests.rs`, and `src/recovery_job_tests.rs`. AC-4 lifecycle publication barriers and route behavior additionally map to integration target `recovery_auto`, source `rust/crates/vexel-app/tests/recovery_auto.rs`, test `automatic_recovery_routes_open_save_and_close_without_losing_capsules`. Worker shutdown/status and error behavior require the worker/policy assertions and integrated review; do not infer exhaustive route/failure injection from that single integration test. The shared workspace pass includes these modules. Individual package commands are not separately recorded in parent verification. GUI/status appearance and acknowledged-barrier latency remain unverified.

AC-5 epoch/revision/status behavior additionally relies on the coordinator integration and review; the identified policy/worker tests alone do not prove every user-facing status string. AC-6 lifecycle regression evidence includes the recovery integration target and combined workspace pass. AC-7 workspace/Python/ledger/diff are explicit in parent verification; per-child readiness is coordinator-reported, and a separate formatting transcript is not present there.

Exact supporting test functions:

- `rust/crates/vexel-app/src/project_session/recovery_snapshot_tests.rs`: `snapshot_shares_immutable_scene_and_payload_across_later_edits`, `snapshot_preparation_skips_clean_and_untitled_sessions`, `dirty_saved_native_snapshot_captures_open_baseline_and_identity`, `clean_session_skips_snapshot_preparation_without_store_access`.
- `rust/crates/vexel-app/src/recovery_policy_tests.rs`: `clean_session_is_never_eligible`, `first_snapshot_waits_for_initial_debounce`, `later_edits_do_not_postpone_first_snapshot`, `scheduled_attempts_observe_fifteen_second_cadence`, `failed_attempt_retries_on_the_same_bounded_cadence`, `becoming_clean_resets_debounce_and_cadence`, `pending_snapshot_is_latest_wins_and_bounded_to_one_value`.
- `rust/crates/vexel-app/src/recovery_job_tests.rs`: `latest_pending_job_replaces_older_work_and_result_slot_is_bounded`, `pause_clears_pending_and_waits_until_active_publication_finishes`, `shutdown_drops_pending_and_joins_active_task`, `publication_errors_remain_typed_in_the_bounded_result`.

### WO-RUST-CI-PAN-01

AC-1 requires unchanged workload/threshold/oracle and a scoped workflow diff; runtime success alone is insufficient. AC-2 has isolated local 12,022.2 ms evidence against 15,657 ms in the frozen order, plus the combined serialized workspace pass. AC-3 has workspace/Python/ledger/diff and policy evidence in parent verification; coordinator reports child readiness passed, while a separate formatting transcript is not recorded there. AC-4 has recorded CLEAN review but **the matching successful hosted Actions run after commit/push remains unproven**. `artifacts/WO-RUST-CI-PAN-01/ac-map.md` now reflects the local workspace/review evidence; it does not close hosted AC-4. Do not mark complete acceptance until the hosted gate succeeds.

## Acceptance limits

This document maps available evidence; it does not append ledger transitions or assert that every individual AC has an isolated proof. In particular, allocation-failure exhaustiveness, complete UI painting/accessibility, source-size/protected-path audits without transcripts, and hosted CI must not be inferred from an exit-zero workspace run. Historical recovery session orders -01/-02 remain superseded.
