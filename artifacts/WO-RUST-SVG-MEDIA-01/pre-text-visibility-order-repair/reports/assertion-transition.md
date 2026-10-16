# Scene153 continuation: defect assertions replaced by correctness assertions

The coordinator archived the terminal writer and seeded only accepted Scene153
files, as recorded in immutable accepted-integration-source-seed.json. No old
seed was restored. The frozen R2 scope remains 24 symbols and 12 original cases.
Historical blocker/review/log artifacts are retained; they describe the previous
API and candidate. There are no remaining bug-expecting assertions in current
tests. Filenames ending in `_gap.rs` retain review continuity only.

| Archived defect test | Current required assertion |
| --- | --- |
| accepted_public_api_rejects_request_savepoints_inside_outer_draft | accepted_request_savepoint_preserves_prefix_and_outer_draft_usability: begin a real request, append, rollback only its suffix, reject the discarded handle, retain the prefix handle, then append/commit a second request and outer draft. |
| image_late_shape_limit_documents_residual_pixels_and_missing_atomicity | image_late_shape_limit_restores_prefix_without_residual_pixels: the original shape-limit error remains, no decoded image survives, prefix edges/source/handle survive and the outer draft commits successfully. |
| image_late_cancellation_documents_residual_pixels_and_missing_atomicity | image_late_cancellation_restores_prefix_without_residual_pixels: cancellation after pixels requires the same cleanup and usable token; no outer rollback repairs it. |
| image_late_capacity_admission_failure_documents_residual_pixels | image_late_capacity_failure_restores_prefix_without_residual_pixels: use measured actual builder capacity admission, retain ReservedBytes classification, release request pixels, preserve prefix and outer commit. |
| text_late_capacity_failure_documents_retained_layout_state_and_cache | text_late_capacity_failure_restores_layout_state_cache_and_prefix: after glyph work fails admission, all external state fields and previous cache counters/owned buffers remain exact, command admissions rewind and the prefix commits. |
| text_second_run_limit_documents_retained_first_run_state_and_cache | text_second_run_limit_restores_first_run_state_cache_and_prefix: failure of the second run removes the first run too, restores state/cache/document totals and permits another real outer append/commit. |
| text_finalization_cancel_documents_retained_shape_after_glyph_work | text_finalization_cancel_restores_shape_state_cache_and_allows_retry: remove the just-published shape and cache admission, preserve prior buffers, restore counters, then successfully retry the same request and commit. |
| document_command_limit_is_missing_on_repeated_cached_layout_uses | seventeenth_cached_layout_use_hits_document_limit_and_restores_counters: 16 nonempty rectangle layout uses fill exactly 4 Mi document commands; cached use 17 returns TextCommands at the original run offset, with all earlier shapes/state/cache/counters unchanged. A late canceled use at count eight also restores its nonzero checkpoint. |

The budget fixture now publishes actual rectangle outlines. Its complete layout
contains 262144 valid commands, as allowed by original validation; cache hits
still count a full document use. Native providers are not substituted.

Four additional tests sweep every observed control-call boundary for both media
types through commit (Canceled, Limit and Overflow), or exercise every provider
failure class and malformed late text output. These require full request
restoration and successful subsequent outer use. Original provider failure
mapping is Limit/OOM/otherwise Unsupported; source-map argument validation keeps
its separate accepted InvalidArgument classification.

The accepted cumulative command API lives on SceneBuilder, not TextState. Text
state is restored externally; the builder restores its private document budget.
Cache rollback removes only appended entries and restores admission counters;
reserved capacity and physical allocation/work history are not rewound.

## Independent style/diagnostic review continuation

All prior 34 named tests and their functional predicates remain. Two incorrect
expectations were corrected to the frozen original, rather than preserved as
bug requirements:

* `media_parity::text_entities_whitespace_and_position_lists_match_source_contract`:
  over-cap list formerly expected `Limit(TextBytes)`; now requires `Number` and
  the exact first excess item's offset. Every other predicate is unchanged.
* `request_failures::image_provider_failures_preserve_original_classes_offsets_and_outer_prefix`:
  all eight provider error classes and cleanup/prefix predicates remain; offset
  now requires `parse_data_uri(...).encoded.offset`, matching image.c's successful
  `data_payload(source, href, &href)` mutation. Invalid/empty URI failures still
  use the original href start. No production/native provider was fabricated.

The 14 new style_publication tests require correct diagnostics, visibility and
provider/callback ordering, actual Scene colors/stroke/clip/group outputs,
source-host retention, live suffix publication, and rollback at every observed
control boundary for Canceled/Limit/Overflow. They are fixture delegate proofs,
not native rendering or full decoder paint/definition resolver proofs. Exact
pre-geometry shape-capacity admission remains the separately reported public
shared contract gap; no assertion assumes that missing operation exists.
