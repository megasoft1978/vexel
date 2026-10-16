# Frozen R1 implementation and public evidence

Single isolated shared source writer. Only allowed scene owning/control source,
README, the sole new test file, and this order's unfrozen artifacts are changed.
The supplied media/decoder/sample blockers and accepted R2/raster prerequisites
were read. No lower peer engine, manifests, ledgers, order or source oracle changed.

| Gap / actual API | Public evidence in render_integration_contracts.rs |
| --- | --- |
| Cumulative document text use: `admit_text_commands`, `document_text_commands`, original 4 Mi constant | `sixteen_cached_layout_uses_and_exact_seventeenth_failure_with_no_suffix`: provider-produced 262144-command cache used sixteen times; seventeenth exact limit/offset with unchanged scene, logical counter, memory and cache size |
| Request/draft/child ownership of counter | `failed_provider_publication_cancel_abort_and_nested_draft_restore_logical_use`: actual provider failure, late publication failure, physical history, canceled admission, nested child rollback, abort/stale request, successful use, outer draft rollback; `text_priority_foreign_offset_huge_count_and_scene_mark_independence`: foreign draft/source offset, huge count/error priority, independent mark behavior |
| Safe same-job composition: `compose`, `RasterComposition::with_buffers/stats/work/charge_work` | `validated_once_repeated_grid_reads_actual_row_fill_and_output_writes`: real frozen scene and admitted same-source job; exactly four initial stroke validation reports, 32 actual stroke/sample row loops, fill runs/active-edge writes and packed output. No allocation/stat changes per row |
| Immutable validated cache: `ValidatedDashView::segments/joins/shapes/grid_at`, underlying checked `JobDashView::grid_at` | Same actual row fixture and overflow/failure fixtures; no cache/Arc clone, unsafe, self-reference or detached ownership. Legacy replaceable JobDashView and ControlledSampleInput explicitly retain their validation |
| Alias/source/pool/serial and cleanup | `composition_alias_pool_foreign_stale_source_and_cancel_guards`, `composition_panic_cancel_partial_work_and_ownership_are_honest`: exact alias, foreign job, equal bytes foreign owner, Dash/Grid write denial, stale output/cache, control interruption, panic and honest retained partial output/work. release_all/finish reclaim discarded handles |
| Native failure/arithmetic/history | `real_native_capacity_arithmetic_and_overlap_failure_preserve_composable_cache`, `transient_overlap_admission_and_canceled_growth_leave_original_cache_and_history`, `checked_quantum_and_same_job_work_overflow_stop_before_unreportable_work`: real std capacity rejection above isize::MAX, checked byte/work overflow, preallocation budget failure, canceled real replacement with retained physical history and original payload/cache |
| Original overlap topology: shared input/sealed canonical validator | `original_post_close_smooth_and_explicit_geometry_metadata_freeze_and_frame`: exact implicit/explicit cubic edge equality, repeated Z, later closes, multiple subpaths, nonidentity transform, second-shape offset publication, unchanged segments/numbers/subpaths, actual seal/freeze/frame. `post_close_forged_overlap_segment_ranges_foreign_arithmetic_and_cancel_reject`: wrong closed flag/first/range/segment/provenance/source/endpoint and cancellation |
| Typed actual mask target: `mask_post_target`, `MaskPostTarget::group/shapes/mask/validate`, `AttachTargetedMask`, `apply_mask_post`; production post validation | `nonempty_mask_expansion_post_attaches_exact_real_group_and_range`, `mask_target_wrong_source_owner_scope_range_postkind_cancel_and_stale_request`, `actual_mask_planner_shared_validation_nonempty_expansion_wrong_flavor_and_source_coherence`: real sealed nonempty child, source-derived owner, exact group/range, admitted nonempty planner expansion, untouched other group, foreign/unrelated source, wrong range/scope/post/flavor, stale reused insertion and cancellation |
| Honest bounded atomic quantum: `DecodeQuantum::new/atomic/compute_style/find_reference/append_shape/commit_request/commit_draft/work/overshoot`, `AtomicStep` | `genuine_shared_atomic_style_reference_append_commit_quantum_and_resume`: actual CSS/reference/append/request/draft calls; each exact metered report includes overshoot; boundary does no operation and returns owned pending input. `atomic_error_cancel_overflow_work_visible_and_no_partial_append_commit`, overflow fixture: exact stopped charge site/error and cleanup; no fake yield/reparse |
| Static ownership | Two compiled negative rustdoc checks: release_all cannot occur while composition is live; mutable callback slice cannot escape |

## Actual scope and adaptations

Text admission remains separate from provider/cache/glyph validation, at the
original post-layout/pre-outline point; MEDIA must restore its own cursor/cache
state on failure. Global document use follows original mask/pattern sync_work;
only nesting/request checkpoints restore it, never independent scene-local marks.

Production sample adapters must retain one composition scope across repeated rows
and adapt their full algorithm to its immutable cache accessors. All capacities
are admitted before scope entry; mutable callbacks cannot grow or release them.
Error/cancel/panic means unfinished output, not successful renderer publication.

Mask planners/adapters must use actual sealed child MaskId and actual existing
group range to mint a post target. Production refuses legacy ownerless AttachMask.
Post validation rejects mismatched mask flavor and context before handoff; apply
validates again before group mutation. Legacy fixture literals remain intact.

Quantum is explicitly soft: one full bounded synchronous shared API may cross
it. Scope/caps, charge reports including failed stop sites, boundary/pending data,
cleanup, worker-thread obligation and unproven main-thread/large-file performance
are specified in README. No reduced SVG bound or claimed latency is introduced.

No remaining protected dependency prevents the five shared operations. Lower
MEDIA/SAMPLE/DECODE adaptations, original decoder assertion rerun after accepted
seeding, actual renderer/native providers/root integration, 603/157 oracle execution,
large-file measurements and final application acceptance remain their own gates.
This worker did not run or edit those peer tests or claim their acceptance.

Independent coordinator review, fresh main checks, ledger acceptance and peer
source seeding are pending. This writer starts no agents and writes no root ledger.
