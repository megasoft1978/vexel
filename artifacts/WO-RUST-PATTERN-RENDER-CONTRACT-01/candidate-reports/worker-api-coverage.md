# Worker API coverage

Authority: current accepted Scene185 source, accepted-prerequisite.json,
accepted-cache-coordinator-result.md, accepted-cache-main-verification.json,
timing-audit-result.md and timing-audit-call-chain.json. Original C
src/svg/shape/pattern.c:239–345 and decode.c:198–218 establish the ordering;
actual current Rust declarations establish the implementation names. The media
crate is registration-only; fixtures implement lower adapters explicitly.

| Added or affected surface | Executed evidence in test.log |
| --- | --- |
| PatternTickets::request_mapped_static; StaticPatternRequest::{ticket,child,descriptor}; StaticPatternView::input | static_pattern_contracts: all five tests; actual builder/draft/request/source/descriptor/job, sealed newly owned children, depth-first publication, failures, replay and rollback |
| RasterInput::ready; validate_job; static_generation/static_ticket/static_request/static_draft_nonce; id/viewport/source/source_owner/ignored | nested_depth_first_tiles_mask_closure_and_complete_static_effect_metadata; real_child_failure_wins_and_success_publishes_before_actual_late_animation_parse; wrong-job and work-limit guards |
| RasterInput table getters: edges, edge_colors, edge_shape_ids, shapes, path_subpaths, path_segments, path_numbers, transforms, images, groups, filters, filter_primitives, masks, patterns, fill_paints, stroke_paints, dash_values, stops, clips | Engine::render_flat consumes every getter on actual populated builder tables in nested_depth_first_tiles_mask_closure_and_complete_static_effect_metadata |
| RasterInput checked getters and enumerators: shape/shape_id, transform/transform_id, clip/clip_id, pattern/pattern_id, mask/mask_id, filter/filter_id, group/group_id; edge_shape, fill_paint, stroke_paint, shape_dashes | Same rich fixture; original handle checks shared through generic SceneView algorithms; wrong-scene mask is rejected |
| RasterInput::mask_content | Validated sealed two-level mask closure, same scope propagated, foreign mask handle rejected; no arbitrary SceneId accessor |
| CommonKernelResources::{paint_sample,clip_contains,clip_boundary_edges,clip_work_counts,stroke_sample_work} | Every method invoked by the rich fixture with actual original records and supplied control |
| CommonIndexKernel::build_index; CommonSampleKernel::sample; CommonFlatKernel::render_flat; explicit CommonRasterKernel combined implementation | Rich fixture builds actual same-job scratch/cache, composes disjoint buffers, calls sample and flat; ready and static roles use the same flat fixture implementation |
| CommonSampleInput::{validate,paint_sample} | Actual validated composition cache, original edges/stops and resource object; cancellation at validation and paint entry; same caller control |
| render_common_pattern | Typed fixture same-job output followed by actual complete_mapped; wrong job/cancel/limit rejected before engine; deterministic child failure beats actual invalid animation; successful negative-bounds publication precedes actual failing animation parse |
| ordered_bounds_extent; shared planning/pixel_inverse/paint validation | Negative, zero, inverted-positive, nonfinite and overflowed extents; no planner invocation on invalid input; negative mapping/planning and real mapped publication preserve SourceMap and original bounds |
| ScopedMediaDecodeOperations::{decode_text,decode_image}; decode_scoped_text/decode_scoped_image | Actual borrowed DraftToken pointer equality in both adapters; real begin/commit/rollback_request and admit_text_commands; cumulative limit and late failure leave prefix/token usable; wrong draft and cancellation precede provider |
| ScopedProductionDecodeOperations combined obligations | combined_scoped_operations_retain_real_cache_channel_and_nonpaint_planning_routes dispatches actual cache publication, Fill and Stroke planning and group-filter publication through the same object; separate text/image fixtures exercise media |
| ScopedProductionDecodeSession::{step,resume_pattern,finish}; all ScopedDecodeEvent variants | scoped_session_requires_same_job_static_resume_before_late_parent_finalization owns actual builder/draft/request/provider fields, passes disjoint token borrow, suspends until mapped resume, commits real request, then parses/finalizes animation and freezes graph with published pattern |
| Ownership/type guards | Five new compile-fail doctests: static-to-frame conversion, arbitrary parent lookup, animation access, mutation while borrowed, freeze while borrowed. All five original compile-fail entries remain passing; original test files hash unchanged |

All 199 named entries passed: accepted 185 plus nine runtime and five compile-fail
entries. Fixtures are not production lower-engine adoption, pixel/native renderer
proof or original C callback-equivalence proof. Existing accepted tests supply
additional frozen output/ticket/cache guards; they were not edited. Root owns
independent review, full protected audit and fresh-main acceptance verification.
