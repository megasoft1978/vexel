# WO-RUST-RECOVERY-SESSION-04 acceptance matrix

All criteria below are covered by the named tests and the verified commands recorded in `result.md`. Scope limits are listed at the end.

| Criterion | Frozen acceptance | Evidence expected |
|---|---|---|
| AC-1 | A dirty saved `.vexel` session creates a bounded capsule containing the scene and retained opaque payload; failures preserve live state. | `dirty_recovery_round_trips_scene_and_opaque_payload`; `successful_save_cleans_capsule_and_failed_discard_keeps_recovery_dirty`; session allocation/publication coverage. |
| AC-2 | Canonical identity and open-time metadata baseline drive Matching/Conflict; malformed or mismatched capsules fail without partial adoption. | `open_baseline_rejects_source_changes_during_bundle_load`; `conflict_restore_remains_dirty_and_discard_removes_capsule`; `durable_save_with_unavailable_new_baseline_reports_published_state`; `corrupt_capsule_is_typed_and_does_not_partially_replace_the_session`; `mismatched_capsule_identity_is_preserved_by_capture_and_cleanup`. SHA-256 FIPS known-answer tests are in `recovery_store.rs`. |
| AC-3 | Restore / Discard / Cancel flow preserves the live session on cancel/failure; changed source cannot be overwritten. | `matching_cancel_preserves_the_live_session_and_snapshot`; `restore_adopts_dirty_snapshot_only_after_success`; `discard_adopts_normal_open_and_cancel_is_the_only_prompt_free_stop`; `absent_recovery_opens_normally_without_a_prompt`; `conflict_restore_is_explicit_and_keeps_changed_source_bytes`. Native response mapping: `recovery_modal_responses_map_to_the_requested_choices`. |
| AC-4 | Restored scene and opaque project payload round-trip; restored state is dirty and source bytes remain intact. | `dirty_recovery_round_trips_scene_and_opaque_payload`; `saved_project_recovers_edits_and_save_as_preserves_original`. |
| AC-5 | Save/discard cleanup follows publication outcomes; conflict Save As adopts a new path without overwriting the changed source. | `successful_save_cleans_capsule_and_failed_discard_keeps_recovery_dirty`; `durable_save_with_unavailable_new_baseline_reports_published_state`; `conflict_restore_remains_dirty_and_discard_removes_capsule`; integration test `saved_project_recovers_edits_and_save_as_preserves_original`. |
| AC-6 | New sources remain below 800 lines; no dependency or format change, platform expansion, source loss, or broadened completion claim. | All new Rust files are below 800 lines; no dependency manifest or lockfile changed; scoped format and diff checks pass; Sol review returned CLEAN. |
| AC-7 | Focused and workspace checks, Python ledger/runner tests, ledger validation, diff check, and WO readiness pass. | Exact outcomes are in `result.md`; all listed commands exited 0. |

## SHA-256 provenance

The store key is SHA-256 derived from the canonicalized existing source path. The in-repo implementation is `project_session/recovery_store.rs::sha256`; `sha256_matches_fips_known_answers` verifies the empty-string and `abc` known-answer vectors. No dependency was added. Provenance: [NIST FIPS 180-4](https://doi.org/10.6028/NIST.FIPS.180-4).

## Explicit deferred scope

This order does not claim automatic/periodic/background snapshots, app-launch recovery scan or presentation, missing-source restoration, retention, broad UI/E2E coverage, or performance compliance. These remain follow-up work. `AppDocument::open_path` and MCP open remain normal-open routes and do not imply a native recovery dialog.
