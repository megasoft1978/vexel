# Public API, ownership and parity proof

## Owning resource coverage

| Original storage | Additive owning surface | Payload seen by lower engine |
|---|---|---|
| BoundsSet shapes | JobRasterScratch.bounds | ShapeBounds slice |
| BoundaryMask bits | boundary_bits, Boundary pool | u8 slice, original bit addressing |
| ScanlineIndex events/nodes/roots/shape events/nodes | scanline_events/scanline_nodes/scanline_roots/shape_events/shape_nodes | original ScanlineEvent/ScanlineNode/i32/ShapeRowEvent |
| RowBuffers offsets/active edges/shapes | shape_offsets/active_edges/active_shapes | usize/u32 slices |
| FillCache eligibility/crossings/intervals/runs | eligible_shapes/crossings/intervals/pixel_runs | bool/FillCrossing/FillInterval/PixelRun slices |
| Local edges/seams | local_edges/dash_seams | Edge/bool slices |
| DashCache segments/joins/shapes | JobDashCache with typed buffers | original DashSegment/DashJoin plus allocation-free JobDashRange |
| Nested per-shape grids | JobDashRange.grid_cells/grid_refs | JobDashView.grid resolves original GridCell/u32 slices |
| Cached center/subpixel/stroke cost | center_samples/subpixel_samples/stroke_cost | u8/usize slices |
| Root/child scratch | multiple JobRasterScratch descriptors | all allocated by the SAME RasterJob/ChargeLedger |
| Group scratch | GroupScratch pool | typed u8 buffer, 64 MiB aggregate |
| Packed output | Output pool/JobRenderOutput | checked u8 slice, no unleased Tile transfer |

Engine-only scalar counts/cursors/flags, scanline AVL state, five interval counts,
stride, fill-row work, view mapping and shape ranges remain allocation-free state.
The API changes storage access, not the corresponding algorithms. `JobElement`
is sealed and Copy-only; nested heap storage cannot hide inside an element.

## Reservation and publication

The coordinator constructs one ChargeLedger with its canonical source owner and
explicit job budget, then transfers it into RasterJob. Existing external active
leases are rejected; JobStartError returns the original ledger unchanged. Job
handles bind ledger identity, slot and monotonic insertion serial and never expose
a lease or Vec. Empty live buffers retain a zero-byte registry lease. The source
Arc is retained once by the ledger and charged by actual capacity once.

Slot registry replacement is itself admitted through an active ledger lease.
ChargeLedger's own entry registry uses the existing checked old/new reserve
implementation. Payload allocation mints an entry, verifies requested arithmetic,
local caps, total budget, history and count, then reserves requested bytes in
that active entry BEFORE calling real Vec::try_reserve_exact. Old buffers remain
charged throughout. Exclusive ledger access holds remaining available capacity
until actual capacity is known. Actual capacity is checked against the per-buffer
cap, local aggregate caps and same global budget before any publication. Native
allocator excess can only be observed after return; rejected excess is dropped.
This is Vec-capacity admission, not allocator/RSS measurement or an absolute
allocator transient-size guarantee.

Growth/shrink/append stage original Copy records plus the new suffix in admitted
storage, polling SourceRead per copy and checking control at publication. Errors
preserve old contents/capacity/token/lease. Successful physical reserves remain
in history even if subsequent copy/control fails. Metadata capacity can remain
after a failed operation, honestly reflected in stats. In-capacity append rolls
back just its unpublished suffix. No whole source/cache clone, hidden Vec clone,
infallible Drop allocation, unsafe block or added dependency is used.

## Lifecycle/error coverage

| Operation/error | Proof |
|---|---|
| source once, Send job, Drop cleanup | source_once_owned_drop_and_no_self_reference; weak owner expires while token copy survives |
| actual spare capacity/history/reuse | actual_spare_capacity_reuse_growth_clear_release_history |
| grow requires old+new overlap before allocation | overlap_rejected_before_alloc_and_old_buffer_unchanged |
| native reserve failure | real_try_reserve_failure_and_checked_byte_overflow_preserve_state uses actual try_reserve_exact above isize::MAX; no global OOM claim |
| arithmetic/token/history/count overflow | new byte-overflow integration plus two private boundary-state unit proofs |
| registry pressure / registration rollback | metadata_pressure_leaves_no_published_buffers_or_external_lease_escape; scratch_registration_failure_rolls_back_tokens_not_physical_history |
| wrong source/job/stale/double | stale_double_wrong_job_and_equal_bytes_wrong_source_rejected; empty-source Arc identity proof |
| cancellation after actual allocation / during old copy / suffix | cancel_before_publication_retains_real_allocation_history_not_contents; growth_copy_loop_cancellation_and_in_capacity_append_are_atomic; cancellation_inside_append_preserves_old_capacity_and_contents |
| nested actual capacity/global grid caps/reuse | nested_grids_share_source_budget_actual_capacity_and_stale_checks; global_grid_capacity_limits_admit_before_allocation_and_release_for_reuse |
| physical shrink/clear/release | physical_shrink_reserves_overlap_and_preserves_history_and_values |
| wrong pool/duplicate/mismatched descriptor | wrong_pool_output_dash_and_grid_are_rejected; duplicate_or_wrong_scratch_descriptor_cannot_partially_release; output_layout_caps_and_wrong_scratch_roles_are_rejected |
| discarded descriptor cleanup | release_all_reclaims_discarded_descriptors_and_invalidates_saved_tokens |
| same-job real index row writes + child/output | actual_lower_index_sample_and_child_output_share_one_job |

`truncate` retains capacity; `clear_capacity` keeps the same empty live token;
`release` invalidates token copies. Descriptor Drop is NOT resource release.
Scratch release prevalidates uniqueness/roles/liveness without work control,
then releases its fixed buffers. Nested grids are independently job-owned and
may be shared; release them explicitly or use release_all/finish/Drop. release_all
reclaims every payload even after all descriptor copies are discarded. finish
also destroys both registries and ends this job's source charge, returning zero
job-live bytes and preserved history. Source bytes may remain owned elsewhere.

## Control and compatibility

ControlledKernelResources requires every paint/clip/stroke method explicitly,
with WorkControl and Result. There is no default implementation, blanket legacy
adapter, fake paint, precharge substitute, Unsupported backend or smaller stop
limit. ControlledSampleInput passes the actual control into paint; lower typed
ControlledSampleKernel/ControlledIndexKernel/ControlledFlatKernel expose actual
job resources. New sample/index fixtures exercise these public interfaces.

The explicit TEST provider loops over stop data, charges PaintVisit at each real
iteration and propagates cancellation/work limit from inside the loop. It has
no production registration. The 4x4 fixture passes the same (s+0.5)/4 lattice,
stop slices and (sum+8)/16 rounding. This proves interface/data/control behavior,
not complete paint/stroke/render oracle parity. The full lower engines and root
coordinator providers remain separate scopes, as frozen in the order.

Legacy public structs/fields/required methods are unchanged. SceneView gains an
immutable source-owner accessor, implemented through a private borrowed Arc
witness; overlays/children preserve it, including empty allocations. No original
86 test file/assertion/literal was edited. All original 86 run alongside 25 new
checks. Protected hashes are recorded separately; full 603/157 source/oracle
acceptance, full app/runtime/performance/UI/live MCP/release are NOT claimed.

Final source-limit audit: shape_bounds.c defines 200M boundary marking WORK,
not bits. RasterJob::charge_boundary enforces that checked/cancelable per-mask counter
in its private live slot, so copied handles cannot fork/reset it; boundary
storage uses original engine-derived element bounds and the job budget, with no
new global byte default. The boundary-limit integration proof covers max/overflow/
cancellation and failed-charge identity.
