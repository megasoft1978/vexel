# Core semantic crosswalk (candidate only)

Scope: all 168 assigned baseline rows. Each row below is classified exactly once and retains original disposition `pending_port`. No candidate accepts a replacement or claims parity or port completion.

## Evidence method

Source inspection reproduces the review finding: prior rows repeated broad crate/work-order labels without behavior-specific correspondence. Legacy symbols were inspected read-only at `legacy-c-final` because the legacy paths are absent from the working tree; Rust paths refer to the current tree. The immutable baseline remains the accounting authority. These are candidate relationships, not a declaration that the tag and current Rust implement identical behavior.

The keyed groups below identify concrete symbols, an accepted ledger event and its recorded result/evidence, and named frozen oracles. A named oracle is a comparison obligation unless an explicit fixture/assertion link is stated. Historical test outcomes are reported only as existing evidence; nothing was executed for this mapping.

`WO-RUST-SVG-DECODE-01` has no accepted ledger event and ends blocked/superseded (ledger lines 1843–1844). Decoder filenames and original_* Rust tests alone do not support candidate classification. `WO-SVG-CLIP-01` is legacy C acceptance, not Rust replacement evidence.

## Keyed behavior groups

<a id="d-model"></a>

### D-MODEL — Document tree and retained spans

- Legacy behavior: `document_model_parse`, `document_model_root`, `document_model_attribute_at`, `document_model_remove_subtree`.
- Rust candidate: `rust/crates/vexel-document/src/parse.rs::{parse_document,parse_owned_document}`; `src/model.rs::DocumentModel::{root,attribute_at,remove_subtree}`.
- Accepted evidence: `WO-RUST-DOCUMENT-MODEL-01`, `ledger.jsonl:1240`, run `WO-RUST-DOCUMENT-MODEL-01-r1-accepted-integrated`; recorded result/evidence `artifacts/WO-RUST-DOCUMENT-MODEL-01/recheck-v1-result.md`. Acceptance remains limited to the scope of that event.
- Frozen oracle: `tests/document/test_document_model.c`. Rust comparison lead: `rust/crates/vexel-document/src/tests.rs`.
- Remaining obligation: Owned/borrowed lifetime representation differs; complete node identity, invalid-handle, allocation and source-span assertion correspondence remains open.

<a id="d-edit"></a>

### D-EDIT — Piece table and source edits

- Legacy behavior: `document_edit_create`, `document_edit_materialize`, `document_edit_set_attribute`, `document_edit_replace_xml_region`, `document_edit_replace_source_span`.
- Rust candidate: `rust/crates/vexel-document-edit/src/piece_table.rs::Editor::{create,materialize,validate_replace,commit_replace}`; `src/ops.rs::Editor::{set_attribute,replace_xml_region,replace_source_span}`.
- Accepted evidence: `WO-RUST-DOCUMENT-EDIT-01`, `ledger.jsonl:1306`, run `WO-RUST-DOCUMENT-EDIT-01-r1-accepted-integrated`; recorded result/evidence `artifacts/WO-RUST-DOCUMENT-EDIT-01/review.md`. Acceptance remains limited to the scope of that event.
- Frozen oracle: `tests/edit/test_document_edit.c` (`test_queries_and_attributes`, `test_replace_xml_region`, `test_replace_source_span`). Rust comparison lead: `rust/crates/vexel-document-edit/tests/original_fixtures.rs::{test_queries_and_attributes,test_replace_xml_region,test_replace_source_span}`; `tests/edit_contract.rs::rejected_parse_keeps_source_history_and_snapshot_is_stable`.
- Remaining obligation: Direct fixture/assertion port is stated in original_fixtures.rs; this report does not establish completeness of private storage, allocator hooks or all atomic failure paths.

<a id="d-group"></a>

### D-GROUP — Grouped edits and insertion

- Legacy behavior: `document_edit_group_siblings`, `document_edit_ungroup_siblings`, `document_edit_insert_template`, `document_edit_insert_xml_fragment`, `document_edit_insert_text_element`.
- Rust candidate: `rust/crates/vexel-document-edit/src/group.rs::Editor::{group_siblings,ungroup_siblings}`; `src/insert.rs::Editor::{insert_template,insert_xml_fragment,insert_text_element}`.
- Accepted evidence: `WO-RUST-DOCUMENT-EDIT-01`, `ledger.jsonl:1306`, run `WO-RUST-DOCUMENT-EDIT-01-r1-accepted-integrated`; recorded result/evidence `artifacts/WO-RUST-DOCUMENT-EDIT-01/review.md`. Acceptance remains limited to the scope of that event.
- Frozen oracle: `tests/edit/test_document_edit.c` (`test_structural_operations`, `test_empty_text_and_xml_fragment`, `test_insert_text_element`). Rust comparison lead: `rust/crates/vexel-document-edit/tests/original_fixtures.rs::{test_structural_operations,test_empty_text_and_xml_fragment,test_insert_text_element}`.
- Remaining obligation: Generated IDs, escaping and a single undo step have concrete test leads; sibling-boundary, malformed-fragment and allocation cases still need a complete case matrix.

<a id="d-history"></a>

### D-HISTORY — History, snapshots and view pagination

- Legacy behavior: `document_edit_undo`, `document_edit_redo`, `document_edit_history_jump`, `document_edit_snapshot_create`; `document_view_range`.
- Rust candidate: `rust/crates/vexel-document-edit/src/piece_table.rs::Editor::{undo,redo}`; `src/history.rs::Editor::history_jump`; `src/snapshot.rs::{Editor::snapshot,EditSnapshot::materialize}`; `src/view.rs::document_view_range`.
- Accepted evidence: `WO-RUST-DOCUMENT-EDIT-01`, `ledger.jsonl:1306`, run `WO-RUST-DOCUMENT-EDIT-01-r1-accepted-integrated`; recorded result/evidence `artifacts/WO-RUST-DOCUMENT-EDIT-01/review.md`. Acceptance remains limited to the scope of that event.
- Frozen oracle: `tests/edit/document_edit_history_test.c`; `tests/edit/document_edit_snapshot_test.c`; `tests/edit/test_document_edit.c::test_history_bounds`. Rust comparison lead: `rust/crates/vexel-document-edit/tests/history_snapshot_view.rs::{check_recent_entries_and_jump,test_snapshot_stability_and_worker_read,document_view_range_original_pagination}`; `tests/original_fixtures.rs::test_history_bounds`.
- Remaining obligation: Snapshot ownership is Arc-based; borrowed C lifetime, eviction/coalescing limits and worker-read/failure guarantees are not accepted here as equivalent.

<a id="d-save"></a>

### D-SAVE — Source-preserving save

- Legacy behavior: `source_save_noop`; `document_edit_save_as`.
- Rust candidate: `rust/crates/vexel-document-io/src/source_save.rs::source_save_noop`; `rust/crates/vexel-document-edit/src/piece_table.rs::Editor::save_as`.
- Accepted evidence: `WO-RUST-SOURCE-SAVE-01`, `ledger.jsonl:1270`, run `WO-RUST-SOURCE-SAVE-01-r1-accepted-integrated`; recorded result/evidence `artifacts/WO-RUST-SOURCE-SAVE-01/recheck-v1-result.md`. Acceptance remains limited to the scope of that event.
- Frozen oracle: `tests/document/test_source_save.c`; `tests/edit/test_document_edit.c`. Rust comparison lead: `rust/crates/vexel-document-io/tests/source_save.rs`; `rust/crates/vexel-document-edit/tests/native_save.rs`.
- Remaining obligation: The accepted source-save result supplies the noop-save lead; D-EDIT supplies the Editor::save_as integration lead; per-hook write/sync/rename failure, result offsets and filesystem atomicity still require frozen-case correspondence.

<a id="geom"></a>

### GEOM — Finite affine geometry

- Legacy behavior: `geom_finite_point`, `geom_affine_compose`, `geom_affine_map_point`, `geom_affine_map_bounds`.
- Rust candidate: `rust/crates/vexel-geometry/src/lib.rs::{finite_point,affine_compose,affine_map_point,affine_map_bounds}`.
- Accepted evidence: `WO-RUST-GEOMETRY-01`, `ledger.jsonl:1064`, run `WO-RUST-GEOMETRY-01-accepted`; recorded result/evidence `artifacts/WO-RUST-GEOMETRY-01/verification-integrated.txt`. Acceptance remains limited to the scope of that event.
- Frozen oracle: `tests/svg/shape/test_shape.c` affine composition/point/bounds assertions (legacy lines 654–681). Rust comparison lead: `rust/crates/vexel-geometry/src/lib.rs::tests::{original_composition_point_and_bounds_assertions,original_iterative_right_first_composition_assertion}`.
- Remaining obligation: Only finite affine behavior is linked; this does not replace the complete shape suite or prove every floating-point edge case.

<a id="tile"></a>

### TILE — RGBA tile allocation

- Legacy behavior: `raster_tile_create`, `raster_tile_destroy`.
- Rust candidate: `rust/crates/vexel-raster/src/lib.rs::Tile::{new,byte_count,rgba,rgba_mut}`; Rust drop owns destruction.
- Accepted evidence: `WO-RUST-RASTER-01`, `ledger.jsonl:1067`, run `WO-RUST-RASTER-01-accepted`; recorded result/evidence `artifacts/WO-RUST-RASTER-01/verification-integrated.txt`. Acceptance remains limited to the scope of that event.
- Frozen oracle: `tests/svg/shape/test_shape.c` tile construction/render consumers. Rust comparison lead: `rust/crates/vexel-raster/tests/tile.rs`.
- Remaining obligation: Only tile allocation/storage is adjacent; no tile-cache, renderer, allocation-hook or pixel-golden acceptance follows.

<a id="spatial"></a>

### SPATIAL — Static spatial queries

- Legacy behavior: `spatial_index_create`, `spatial_index_count`, `spatial_index_query`.
- Rust candidate: `rust/crates/vexel-spatial/src/lib.rs::SpatialIndex::{new,count,query}` with `Bounds` and `Record`.
- Accepted evidence: `WO-RUST-SPATIAL-01`, `ledger.jsonl:1117`, run `WO-RUST-SPATIAL-01-accepted-integrated`; recorded result/evidence `artifacts/WO-RUST-SPATIAL-01/recheck-v1-result.md`. Acceptance remains limited to the scope of that event.
- Frozen oracle: `tests/svg/spatial/spatial_cache_test.c` (consumer/cache oracle). Rust comparison lead: `rust/crates/vexel-spatial/tests/spatial.rs`.
- Remaining obligation: Static index queries have accepted scoped evidence; mutable cache invalidation, generation handling and the entire cache oracle remain unresolved.

<a id="xml-reader"></a>

### XML-READER — Incremental tag reader and UTF-8 scanning

- Legacy behavior: `svg_xml_reader_start`, `svg_xml_reader_step`, `svg_xml_reader_get_progress`.
- Rust candidate: `rust/crates/vexel-xml/src/lib.rs::XmlReader::{new,step,progress}` and `utf8_next`.
- Accepted evidence: `WO-RUST-XML-01`, `ledger.jsonl:1076`, run `WO-RUST-XML-01-accepted`; recorded result/evidence `artifacts/WO-RUST-XML-01/verification-integrated.txt`. Acceptance remains limited to the scope of that event.
- Frozen oracle: `tests/svg/xml/test_xml_tokenizer.c` (adjacent tokenizer oracle, not full reader equivalence). Rust comparison lead: `rust/crates/vexel-xml/src/tests.rs`.
- Remaining obligation: The acceptance explicitly excludes a full tokenizer claim; callback progression and exact diagnostics require separate reader assertions.

<a id="xml-token"></a>

### XML-TOKEN — Tokenization and attribute decoding

- Legacy behavior: `svg_xml_tokenize`, `svg_xml_tokenize_large`, `svg_xml_decode_attribute_value`.
- Rust candidate: `rust/crates/vexel-xml/src/tokenizer.rs::{tokenize,tokenize_large,decode_attribute_value}`.
- Accepted evidence: `WO-RUST-XML-02`, `ledger.jsonl:1191`, run `WO-RUST-XML-02-accepted-integrated`; recorded result/evidence `artifacts/WO-RUST-XML-02/recheck-v4-result.md`. Acceptance remains limited to the scope of that event.
- Frozen oracle: `tests/svg/xml/test_xml_tokenizer.c`. Rust comparison lead: `rust/crates/vexel-xml/src/tests.rs`; `tests/mutation_fuzz.rs`.
- Remaining obligation: Doctype/entity expansion, CLI protocol and full frozen diagnostic/limit assertion mapping are not established by these candidates.

<a id="xml-stream"></a>

### XML-STREAM — Incremental token storage

- Legacy behavior: `svg_xml_token_stream_start`, `svg_xml_token_stream_start_visitor`, `svg_xml_token_stream_step`, `svg_xml_token_stream_take`.
- Rust candidate: `rust/crates/vexel-xml/src/stream.rs::XmlTokenStream::{new,new_visitor,step,take,visitor_storage_bytes}`.
- Accepted evidence: `WO-RUST-XML-STORAGE-01`, `ledger.jsonl:1228`, run `WO-RUST-XML-STORAGE-01-r1-accepted-integrated`; recorded result/evidence `artifacts/WO-RUST-XML-STORAGE-01/recheck-v1-result.md`. Acceptance remains limited to the scope of that event.
- Frozen oracle: `tests/svg/xml/test_xml_tokenizer.c`; `tests/svg/index/test_token_stream.c`. Rust comparison lead: `rust/crates/vexel-xml/src/stream_tests.rs`.
- Remaining obligation: Exact cancellation frontier, visitor ownership, byte-budget/storage limits and every C stream assertion still need a case matrix.

<a id="color"></a>

### COLOR — CSS color parsing

- Legacy behavior: `svg_css_parse_color`.
- Rust candidate: `rust/crates/vexel-svg-color/src/lib.rs::parse_color`.
- Accepted evidence: `WO-RUST-SVG-COLOR-01`, `ledger.jsonl:1200`, run `WO-RUST-SVG-COLOR-01-r1-accepted-integrated`; recorded result/evidence `artifacts/WO-RUST-SVG-COLOR-01/review.md`. Acceptance remains limited to the scope of that event.
- Frozen oracle: `tests/svg/css/test_css.c`; `tests/svg/shape/test_shape.c`. Rust comparison lead: `rust/crates/vexel-svg-color/src/lib.rs` unit tests.
- Remaining obligation: Accepted parser scope is a lead for color formats; all caller fallback, malformed span and complete frozen color assertions remain pending.

<a id="css"></a>

### CSS — CSS sheet and cascade

- Legacy behavior: `svg_css_sheet_create`, `svg_css_compute`, `svg_css_keyword_equals`, `svg_css_is_style_element`.
- Rust candidate: `rust/crates/vexel-svg-css/src/lib.rs::{CssSheet::create,CssSheet::compute,keyword_equals,is_style_element}`.
- Accepted evidence: `WO-RUST-SVG-CSS-01`, `ledger.jsonl:1283`, run `WO-RUST-SVG-CSS-01-r1-accepted-integrated`; recorded result/evidence `artifacts/WO-RUST-SVG-CSS-01/recheck-v1-result.md`. Acceptance remains limited to the scope of that event.
- Frozen oracle: `tests/svg/css/test_css.c`. Rust comparison lead: `rust/crates/vexel-svg-css/tests/css.rs`; `tests/legacy_oracle.rs` compares a saved Rust baseline, not a C process.
- Remaining obligation: Selector/declaration/cascade behavior is linked; legacy allocation, capacity and every C assertion are not inferred from the Rust-baseline comparison.

<a id="index"></a>

### INDEX — Progressive SVG index

- Legacy behavior: `svg_index_start`, `svg_index_step`, `svg_index_advance_generation`, `svg_index_make_preview_source`.
- Rust candidate: `rust/crates/vexel-svg-index/src/lib.rs::IndexSession::{start,step,advance_generation,make_preview_source}`.
- Accepted evidence: `WO-RUST-SVG-INDEX-01`, `ledger.jsonl:1246`, run `WO-RUST-SVG-INDEX-01-r1-accepted-integrated`; recorded result/evidence `artifacts/WO-RUST-SVG-INDEX-01/recheck-v3-result.md`. Acceptance remains limited to the scope of that event.
- Frozen oracle: `tests/svg/index/test_index.c`; `tests/svg/index/test_token_stream.c`. Rust comparison lead: `rust/crates/vexel-svg-index/tests/index_contract.rs`; `src/index_tests.rs`.
- Remaining obligation: Generation cancellation, preview bytes and the Python wrapper are not fully reconciled; index storage is not the spatial cache.

<a id="reference"></a>

### REFERENCE — Definition lookup and use attributes

- Legacy behavior: `svg_reference_index_create`, `svg_reference_index_find`, `svg_reference_definition_end`, `svg_reference_xlink_is_bound`.
- Rust candidate: `rust/crates/vexel-svg-reference/src/lib.rs::{ReferenceIndex::create,ReferenceIndex::find,definition_end,xlink_is_bound,use_attributes_supported}`.
- Accepted evidence: `WO-RUST-SVG-REFERENCE-01`, `ledger.jsonl:1267`, run `WO-RUST-SVG-REFERENCE-01-r1-accepted-integrated`; recorded result/evidence `artifacts/WO-RUST-SVG-REFERENCE-01/recheck-v1-result.md`. Acceptance remains limited to the scope of that event.
- Frozen oracle: `tests/svg/shape/test_reference.c`. Rust comparison lead: `rust/crates/vexel-svg-reference/tests/reference.rs`; `tests/svg_compat01.rs`.
- Remaining obligation: Lookup behavior is linked; duplicate IDs, namespace/unsupported-attribute priority and full referenced-instance decode remain pending.

<a id="paint"></a>

### PAINT — Gradient resolution and sampling

- Legacy behavior: `svg_paint_edge_span`, `svg_paint_resolve_fill`, `svg_paint_sample`, `svg_paint_cache_destroy`.
- Rust candidate: `rust/crates/vexel-svg-paint/src/paint.rs::{edge_span,sample}`; `src/resolver.rs::{resolve_fill,resolve_cached_fill}`; cache storage uses Rust ownership.
- Accepted evidence: `WO-RUST-SVG-PAINT-01`, `ledger.jsonl:1536`, run `WO-RUST-SVG-PAINT-01-r2-owned-source-accepted`; recorded result/evidence `artifacts/WO-RUST-SVG-PAINT-01/r2-coordinator-result.md`. Acceptance remains limited to the scope of that event.
- Frozen oracle: `tests/svg/shape/test_paint_edge_span.c`; `tests/svg/shape/gradient_stroke_test.c`. Rust comparison lead: `rust/crates/vexel-svg-paint/tests/{paint_parity.rs,resolver_parity.rs,cache_handoff.rs}`.
- Remaining obligation: Acceptance is the owned paint source gate only; complete render integration, cache lifetime and pixel assertions are not inferred.

<a id="anim"></a>

### ANIM — SMIL parse and evaluation

- Legacy behavior: `svg_anim_parse`, `svg_anim_eval`, `svg_anim_attribute_name`, `svg_anim_diagnostic_name`, `svg_anim_resolve_motion_path`.
- Rust candidate: `rust/crates/vexel-svg-animation/src/parse.rs::parse`; `src/eval.rs::evaluate`; `src/lib.rs::{attribute_name,diagnostic_name}`; `src/motion.rs::resolve_motion_path`.
- Accepted evidence: `WO-RUST-SVG-ANIMATION-01`, `ledger.jsonl:1295`, run `WO-RUST-SVG-ANIMATION-01-r1-accepted-integrated`; recorded result/evidence `artifacts/WO-RUST-SVG-ANIMATION-01/recheck-v1-result.md`. Acceptance remains limited to the scope of that event.
- Frozen oracle: `tests/anim` oracles are owned by project-animation-mcp; cross-reference only, plus `tests/svg/shape/test_shape.c` render consumer. Rust comparison lead: `rust/crates/vexel-svg-animation/tests/animation.rs`.
- Remaining obligation: Parsing/evaluation is distinct from animation authoring and per-frame renderer/clip ownership. Authoring rows are unresolved here.

<a id="clip"></a>

### CLIP — Clip geometry and ownership

- Legacy behavior: `svg_clip_create_rect`, `svg_clip_contains`, `svg_clip_local_bounds`, `svg_clip_apply_group`, `svg_clip_clone`, `svg_clip_update_owner`.
- Rust candidate: `rust/crates/vexel-svg-definitions/src/clip.rs::{create_rect,contains,local_bounds,apply_group,clone_clips,update_owner,visit_boundary_edges,work_units}`.
- Accepted evidence: `WO-RUST-SVG-DEFINITIONS-01`, `ledger.jsonl:1608`, run `WO-RUST-SVG-DEFINITIONS-01-main40-accepted`; recorded result/evidence `artifacts/WO-RUST-SVG-DEFINITIONS-01/coordinator-result.md`. Acceptance remains limited to the scope of that event.
- Frozen oracle: `tests/svg/shape/clip_definition_scope_test.c`; `tests/svg/shape/test_clip_work_budget.c`. Rust comparison lead: `rust/crates/vexel-svg-definitions/tests/{definition_plans.rs,clip_rollback.rs,production_planner.rs}`.
- Remaining obligation: Accepted definitions source gate does not establish complete decoder traversal or every frozen clip budget assertion.

<a id="filter"></a>

### FILTER — Filter pixel primitives

- Legacy behavior: `raster_shape_filter_apply`.
- Rust candidate: `rust/crates/vexel-render/src/filter/mod.rs::apply_filter_chain`; `filter/blur.rs::{blur,offset,drop_shadow}`; `filter/color.rs::{composite_pixel,color_matrix}`.
- Accepted evidence: `WO-RUST-SVG-FILTER-01`, `ledger.jsonl:1934`, run `wo-rust-svg-filter-01-accepted-2026-10-16`; recorded result/evidence `work/WO-RUST-SVG-FILTER-01.md`. Acceptance remains limited to the scope of that event.
- Frozen oracle: `tests/realworld/fixtures/w5` filter fixtures and frozen `tests/svg/shape/w5_reference.c` renderer oracle. Rust comparison lead: `rust/crates/vexel-render/tests/filter_parity.rs`; `tests/filter_units.rs`; 29 fixture results described in accepted ledger.
- Remaining obligation: Existing result.md records candidate-stage limitations; the accepted ledger records subsequent review. No claim that all legacy filter allocation/budget/failure behavior is reconciled.

## Per-row crosswalk

`partial candidate` links only the behavior stated by its group. `unresolved` records a missing mapping even when adjacent Rust source exists. Every row retains original `pending_port`; test paths named in groups above are oracle cross-references, not duplicate classifications.

| Frozen row | Classification | Concrete lead / remaining behavior |
|---|---|---|
| `src/base/base.c` | unresolved | Checked arithmetic, slices, arenas and allocator hooks lack a scoped accepted replacement/assertion map. Original disposition: `pending_port`. |
| `src/base/base.h` | unresolved | Checked arithmetic, slices, arenas and allocator hooks lack a scoped accepted replacement/assertion map. Original disposition: `pending_port`. |
| `tests/base/test_base.c` | unresolved | Checked arithmetic, slices, arenas and allocator hooks lack a scoped accepted replacement/assertion map. Original disposition: `pending_port`. |
| `tests/fuzz/shape/shape_fuzz.c` | unresolved | Shape-fuzz seeds, mutation/input protocol and campaign verdicts have no accepted Rust harness correspondence. Original disposition: `pending_port`. |
| `tests/fuzz/shape/shape_fuzz.py` | unresolved | Shape-fuzz seeds, mutation/input protocol and campaign verdicts have no accepted Rust harness correspondence. Original disposition: `pending_port`. |
| `tests/test_document_model.py` | unresolved | Python invocation, fixture generation/selection, process protocol and assertions have no accepted Rust harness correspondence. Original disposition: `pending_port`. |
| `tests/test_literal_lengths.py` | unresolved | Python invocation, fixture generation/selection, process protocol and assertions have no accepted Rust harness correspondence. Original disposition: `pending_port`. |
| `tests/test_shape_render.py` | unresolved | Python invocation, fixture generation/selection, process protocol and assertions have no accepted Rust harness correspondence. Original disposition: `pending_port`. |
| `tests/test_source_save.py` | unresolved | Python invocation, fixture generation/selection, process protocol and assertions have no accepted Rust harness correspondence. Original disposition: `pending_port`. |
| `tests/test_spatial_cache.py` | unresolved | Python invocation, fixture generation/selection, process protocol and assertions have no accepted Rust harness correspondence. Original disposition: `pending_port`. |
| `tests/test_svg_index.py` | unresolved | Python invocation, fixture generation/selection, process protocol and assertions have no accepted Rust harness correspondence. Original disposition: `pending_port`. |
| `tests/test_xml_tokenizer.py` | unresolved | Python invocation, fixture generation/selection, process protocol and assertions have no accepted Rust harness correspondence. Original disposition: `pending_port`. |
| `src/document/document_edit.c` | partial candidate | Legacy symbols linked by the group: `document_edit_create`. Behavior group [D-EDIT](#d-edit). Only the named behavior/symbols in this group are mapped; the rest of this translation unit remains unproven against its frozen oracle. Original disposition: `pending_port`. |
| `src/document/document_edit.h` | partial candidate | Legacy symbols linked by the group: `document_edit_create`, `document_edit_materialize`, `document_edit_set_attribute`. Behavior group [D-EDIT](#d-edit). Public declarations/types are candidate contracts; ABI, representation, ownership and all error cases require separate reconciliation. Original disposition: `pending_port`. |
| `src/document/document_edit_group.c` | partial candidate | Legacy symbols linked by the group: `document_edit_group_siblings`, `document_edit_ungroup_siblings`. Behavior group [D-GROUP](#d-group). Only the named behavior/symbols in this group are mapped; the rest of this translation unit remains unproven against its frozen oracle. Original disposition: `pending_port`. |
| `src/document/document_edit_history.c` | partial candidate | Legacy symbols linked by the group: `document_edit_undo`, `document_edit_redo`, `document_edit_history_jump`. Behavior group [D-HISTORY](#d-history). Only the named behavior/symbols in this group are mapped; the rest of this translation unit remains unproven against its frozen oracle. Original disposition: `pending_port`. |
| `src/document/document_edit_insert.c` | partial candidate | Legacy symbols linked by the group: `document_edit_insert_template`, `document_edit_insert_xml_fragment`. Behavior group [D-GROUP](#d-group). Only the named behavior/symbols in this group are mapped; the rest of this translation unit remains unproven against its frozen oracle. Original disposition: `pending_port`. |
| `src/document/document_edit_internal.h` | unresolved | Adjacent group [D-EDIT](#d-edit) is a lead only: no concrete symbol contract in this private/helper unit was linked to its accepted Rust result. Preserve layout, ownership, error/limit and helper behavior until mapped. Original disposition: `pending_port`. |
| `src/document/document_edit_ops.c` | partial candidate | Legacy symbols linked by the group: `document_edit_set_attribute`. Behavior group [D-EDIT](#d-edit). Only the named behavior/symbols in this group are mapped; the rest of this translation unit remains unproven against its frozen oracle. Original disposition: `pending_port`. |
| `src/document/document_edit_piece_table.c` | partial candidate | Legacy symbols linked by the group: `document_edit_materialize`. Behavior group [D-EDIT](#d-edit). Only the named behavior/symbols in this group are mapped; the rest of this translation unit remains unproven against its frozen oracle. Original disposition: `pending_port`. |
| `src/document/document_edit_region.c` | partial candidate | Legacy symbols linked by the group: `document_edit_replace_xml_region`, `document_edit_replace_source_span`. Behavior group [D-EDIT](#d-edit). Only the named behavior/symbols in this group are mapped; the rest of this translation unit remains unproven against its frozen oracle. Original disposition: `pending_port`. |
| `src/document/document_edit_snapshot.c` | partial candidate | Legacy symbols linked by the group: `document_edit_snapshot_create`. Behavior group [D-HISTORY](#d-history). Only the named behavior/symbols in this group are mapped; the rest of this translation unit remains unproven against its frozen oracle. Original disposition: `pending_port`. |
| `src/document/document_edit_text_insert.c` | partial candidate | Legacy symbols linked by the group: `document_edit_insert_xml_fragment`, `document_edit_insert_text_element`. Behavior group [D-GROUP](#d-group). Only the named behavior/symbols in this group are mapped; the rest of this translation unit remains unproven against its frozen oracle. Original disposition: `pending_port`. |
| `src/document/document_model.c` | partial candidate | Legacy symbols linked by the group: `document_model_parse`, `document_model_root`, `document_model_attribute_at`. Behavior group [D-MODEL](#d-model). Only the named behavior/symbols in this group are mapped; the rest of this translation unit remains unproven against its frozen oracle. Original disposition: `pending_port`. |
| `src/document/document_model.h` | partial candidate | Legacy symbols linked by the group: `document_model_parse`, `document_model_root`, `document_model_attribute_at`. Behavior group [D-MODEL](#d-model). Public declarations/types are candidate contracts; ABI, representation, ownership and all error cases require separate reconciliation. Original disposition: `pending_port`. |
| `src/document/document_view.h` | partial candidate | Legacy symbols linked by the group: `document_view_range`. Behavior group [D-HISTORY](#d-history). Public declarations/types are candidate contracts; ABI, representation, ownership and all error cases require separate reconciliation. Original disposition: `pending_port`. |
| `src/document/source_save.c` | partial candidate | Legacy symbols linked by the group: `source_save_noop`. Behavior group [D-SAVE](#d-save). Only the named behavior/symbols in this group are mapped; the rest of this translation unit remains unproven against its frozen oracle. Original disposition: `pending_port`. |
| `src/document/source_save.h` | partial candidate | Legacy symbols linked by the group: `source_save_noop`. Behavior group [D-SAVE](#d-save). Public declarations/types are candidate contracts; ABI, representation, ownership and all error cases require separate reconciliation. Original disposition: `pending_port`. |
| `src/geom/geometry.c` | partial candidate | Legacy symbols linked by the group: `geom_finite_point`, `geom_affine_compose`, `geom_affine_map_point`. Behavior group [GEOM](#geom). Only the named behavior/symbols in this group are mapped; the rest of this translation unit remains unproven against its frozen oracle. Original disposition: `pending_port`. |
| `src/geom/geometry.h` | partial candidate | Legacy symbols linked by the group: `geom_finite_point`, `geom_affine_compose`, `geom_affine_map_point`. Behavior group [GEOM](#geom). Public declarations/types are candidate contracts; ABI, representation, ownership and all error cases require separate reconciliation. Original disposition: `pending_port`. |
| `src/raster/shape_bounds.c` | unresolved | Complete raster unit mapping is missing. WO-RUST-RENDER-PERF-01 accepts selected active-edge/grid behavior, explicitly omitting legacy adaptive sizing, counters and work probes; tile/job contracts cannot establish this unit. Original disposition: `pending_port`. |
| `src/raster/shape_bounds.h` | unresolved | Complete raster unit mapping is missing. WO-RUST-RENDER-PERF-01 accepts selected active-edge/grid behavior, explicitly omitting legacy adaptive sizing, counters and work probes; tile/job contracts cannot establish this unit. Original disposition: `pending_port`. |
| `src/raster/shape_filter.c` | partial candidate | Legacy symbols linked by the group: `raster_shape_filter_apply`. Behavior group [FILTER](#filter). Only the named behavior/symbols in this group are mapped; the rest of this translation unit remains unproven against its frozen oracle. Original disposition: `pending_port`. |
| `src/raster/shape_filter.h` | partial candidate | Legacy symbols linked by the group: `raster_shape_filter_apply`. Behavior group [FILTER](#filter). Public declarations/types are candidate contracts; ABI, representation, ownership and all error cases require separate reconciliation. Original disposition: `pending_port`. |
| `src/raster/shape_render.c` | unresolved | Complete raster unit mapping is missing. WO-RUST-RENDER-PERF-01 accepts selected active-edge/grid behavior, explicitly omitting legacy adaptive sizing, counters and work probes; tile/job contracts cannot establish this unit. Original disposition: `pending_port`. |
| `src/raster/shape_render.h` | unresolved | Complete raster unit mapping is missing. WO-RUST-RENDER-PERF-01 accepts selected active-edge/grid behavior, explicitly omitting legacy adaptive sizing, counters and work probes; tile/job contracts cannot establish this unit. Original disposition: `pending_port`. |
| `src/raster/shape_render_animation.c` | unresolved | Complete raster unit mapping is missing. WO-RUST-RENDER-PERF-01 accepts selected active-edge/grid behavior, explicitly omitting legacy adaptive sizing, counters and work probes; tile/job contracts cannot establish this unit. Original disposition: `pending_port`. |
| `src/raster/shape_render_animation_clips.c` | unresolved | Complete raster unit mapping is missing. WO-RUST-RENDER-PERF-01 accepts selected active-edge/grid behavior, explicitly omitting legacy adaptive sizing, counters and work probes; tile/job contracts cannot establish this unit. Original disposition: `pending_port`. |
| `src/raster/shape_render_animation_clips.h` | unresolved | Complete raster unit mapping is missing. WO-RUST-RENDER-PERF-01 accepts selected active-edge/grid behavior, explicitly omitting legacy adaptive sizing, counters and work probes; tile/job contracts cannot establish this unit. Original disposition: `pending_port`. |
| `src/raster/shape_render_dash_cache.c` | unresolved | Complete raster unit mapping is missing. WO-RUST-RENDER-PERF-01 accepts selected active-edge/grid behavior, explicitly omitting legacy adaptive sizing, counters and work probes; tile/job contracts cannot establish this unit. Original disposition: `pending_port`. |
| `src/raster/shape_render_fill.c` | unresolved | Complete raster unit mapping is missing. WO-RUST-RENDER-PERF-01 accepts selected active-edge/grid behavior, explicitly omitting legacy adaptive sizing, counters and work probes; tile/job contracts cannot establish this unit. Original disposition: `pending_port`. |
| `src/raster/shape_render_fill.h` | unresolved | Complete raster unit mapping is missing. WO-RUST-RENDER-PERF-01 accepts selected active-edge/grid behavior, explicitly omitting legacy adaptive sizing, counters and work probes; tile/job contracts cannot establish this unit. Original disposition: `pending_port`. |
| `src/raster/shape_render_group.c` | unresolved | Complete raster unit mapping is missing. WO-RUST-RENDER-PERF-01 accepts selected active-edge/grid behavior, explicitly omitting legacy adaptive sizing, counters and work probes; tile/job contracts cannot establish this unit. Original disposition: `pending_port`. |
| `src/raster/shape_render_internal.h` | unresolved | Complete raster unit mapping is missing. WO-RUST-RENDER-PERF-01 accepts selected active-edge/grid behavior, explicitly omitting legacy adaptive sizing, counters and work probes; tile/job contracts cannot establish this unit. Original disposition: `pending_port`. |
| `src/raster/shape_render_preflight.c` | unresolved | Complete raster unit mapping is missing. WO-RUST-RENDER-PERF-01 accepts selected active-edge/grid behavior, explicitly omitting legacy adaptive sizing, counters and work probes; tile/job contracts cannot establish this unit. Original disposition: `pending_port`. |
| `src/raster/shape_render_preflight.h` | unresolved | Complete raster unit mapping is missing. WO-RUST-RENDER-PERF-01 accepts selected active-edge/grid behavior, explicitly omitting legacy adaptive sizing, counters and work probes; tile/job contracts cannot establish this unit. Original disposition: `pending_port`. |
| `src/raster/shape_render_sample.c` | unresolved | Complete raster unit mapping is missing. WO-RUST-RENDER-PERF-01 accepts selected active-edge/grid behavior, explicitly omitting legacy adaptive sizing, counters and work probes; tile/job contracts cannot establish this unit. Original disposition: `pending_port`. |
| `src/raster/shape_render_stroke.c` | unresolved | Complete raster unit mapping is missing. WO-RUST-RENDER-PERF-01 accepts selected active-edge/grid behavior, explicitly omitting legacy adaptive sizing, counters and work probes; tile/job contracts cannot establish this unit. Original disposition: `pending_port`. |
| `src/raster/shape_render_stroke_grid.c` | unresolved | Complete raster unit mapping is missing. WO-RUST-RENDER-PERF-01 accepts selected active-edge/grid behavior, explicitly omitting legacy adaptive sizing, counters and work probes; tile/job contracts cannot establish this unit. Original disposition: `pending_port`. |
| `src/raster/shape_scanline.c` | unresolved | Complete raster unit mapping is missing. WO-RUST-RENDER-PERF-01 accepts selected active-edge/grid behavior, explicitly omitting legacy adaptive sizing, counters and work probes; tile/job contracts cannot establish this unit. Original disposition: `pending_port`. |
| `src/raster/shape_scanline.h` | unresolved | Complete raster unit mapping is missing. WO-RUST-RENDER-PERF-01 accepts selected active-edge/grid behavior, explicitly omitting legacy adaptive sizing, counters and work probes; tile/job contracts cannot establish this unit. Original disposition: `pending_port`. |
| `src/raster/shape_scanline_fill.c` | unresolved | Complete raster unit mapping is missing. WO-RUST-RENDER-PERF-01 accepts selected active-edge/grid behavior, explicitly omitting legacy adaptive sizing, counters and work probes; tile/job contracts cannot establish this unit. Original disposition: `pending_port`. |
| `src/raster/shape_scanline_fill.h` | unresolved | Complete raster unit mapping is missing. WO-RUST-RENDER-PERF-01 accepts selected active-edge/grid behavior, explicitly omitting legacy adaptive sizing, counters and work probes; tile/job contracts cannot establish this unit. Original disposition: `pending_port`. |
| `src/raster/tile.c` | partial candidate | Legacy symbols linked by the group: `raster_tile_create`, `raster_tile_destroy`. Behavior group [TILE](#tile). Only the named behavior/symbols in this group are mapped; the rest of this translation unit remains unproven against its frozen oracle. Original disposition: `pending_port`. |
| `src/raster/tile.h` | partial candidate | Legacy symbols linked by the group: `raster_tile_create`, `raster_tile_destroy`. Behavior group [TILE](#tile). Public declarations/types are candidate contracts; ABI, representation, ownership and all error cases require separate reconciliation. Original disposition: `pending_port`. |
| `src/raster/tile_cache.c` | unresolved | Tile-cache keying, eviction, ownership and failure oracle mapping is missing; TILE concerns allocation only. Original disposition: `pending_port`. |
| `src/raster/tile_cache.h` | unresolved | Tile-cache keying, eviction, ownership and failure oracle mapping is missing; TILE concerns allocation only. Original disposition: `pending_port`. |
| `src/spatial/index.c` | partial candidate | Legacy symbols linked by the group: `spatial_index_create`, `spatial_index_count`, `spatial_index_query`. Behavior group [SPATIAL](#spatial). Only the named behavior/symbols in this group are mapped; the rest of this translation unit remains unproven against its frozen oracle. Original disposition: `pending_port`. |
| `src/spatial/index.h` | partial candidate | Legacy symbols linked by the group: `spatial_index_create`, `spatial_index_count`, `spatial_index_query`. Behavior group [SPATIAL](#spatial). Public declarations/types are candidate contracts; ABI, representation, ownership and all error cases require separate reconciliation. Original disposition: `pending_port`. |
| `src/svg/animation/animation.h` | partial candidate | Legacy symbols linked by the group: `svg_anim_parse`, `svg_anim_eval`, `svg_anim_attribute_name`. Behavior group [ANIM](#anim). Public declarations/types are candidate contracts; ABI, representation, ownership and all error cases require separate reconciliation. Original disposition: `pending_port`. |
| `src/svg/animation/animation_internal.h` | unresolved | Private data layout/lifetime/helper correspondence and a named accepted result plus frozen assertion map were not established. Original disposition: `pending_port`. |
| `src/svg/animation/authoring.c` | unresolved | SMIL authoring patch creation/edit/removal has no symbol/result/oracle correspondence in ANIM parse/evaluation evidence. Original disposition: `pending_port`. |
| `src/svg/animation/authoring.h` | unresolved | SMIL authoring patch creation/edit/removal has no symbol/result/oracle correspondence in ANIM parse/evaluation evidence. Original disposition: `pending_port`. |
| `src/svg/animation/authoring_create.c` | unresolved | SMIL authoring patch creation/edit/removal has no symbol/result/oracle correspondence in ANIM parse/evaluation evidence. Original disposition: `pending_port`. |
| `src/svg/animation/authoring_internal.h` | unresolved | SMIL authoring patch creation/edit/removal has no symbol/result/oracle correspondence in ANIM parse/evaluation evidence. Original disposition: `pending_port`. |
| `src/svg/animation/evaluate.c` | partial candidate | Legacy symbols linked by the group: `svg_anim_eval`. Behavior group [ANIM](#anim). Only the named behavior/symbols in this group are mapped; the rest of this translation unit remains unproven against its frozen oracle. Original disposition: `pending_port`. |
| `src/svg/animation/motion_path.c` | partial candidate | Legacy symbols linked by the group: `svg_anim_resolve_motion_path`. Behavior group [ANIM](#anim). Only the named behavior/symbols in this group are mapped; the rest of this translation unit remains unproven against its frozen oracle. Original disposition: `pending_port`. |
| `src/svg/animation/smil.c` | partial candidate | Legacy symbols linked by the group: `svg_anim_attribute_name`, `svg_anim_diagnostic_name`. Behavior group [ANIM](#anim). Only the named behavior/symbols in this group are mapped; the rest of this translation unit remains unproven against its frozen oracle. Original disposition: `pending_port`. |
| `src/svg/animation/smil_tracks.c` | partial candidate | Legacy symbols linked by the group: `svg_anim_parse`, `svg_anim_resolve_motion_path`. Behavior group [ANIM](#anim). Only the named behavior/symbols in this group are mapped; the rest of this translation unit remains unproven against its frozen oracle. Original disposition: `pending_port`. |
| `src/svg/clip/clip.c` | partial candidate | Legacy symbols linked by the group: `svg_clip_create_rect`, `svg_clip_contains`, `svg_clip_local_bounds`. Behavior group [CLIP](#clip). Only the named behavior/symbols in this group are mapped; the rest of this translation unit remains unproven against its frozen oracle. Original disposition: `pending_port`. |
| `src/svg/clip/clip.h` | partial candidate | Legacy symbols linked by the group: `svg_clip_create_rect`, `svg_clip_contains`, `svg_clip_local_bounds`. Behavior group [CLIP](#clip). Public declarations/types are candidate contracts; ABI, representation, ownership and all error cases require separate reconciliation. Original disposition: `pending_port`. |
| `src/svg/css/color.c` | partial candidate | Legacy symbols linked by the group: `svg_css_parse_color`. Behavior group [COLOR](#color). Only the named behavior/symbols in this group are mapped; the rest of this translation unit remains unproven against its frozen oracle. Original disposition: `pending_port`. |
| `src/svg/css/color.h` | partial candidate | Legacy symbols linked by the group: `svg_css_parse_color`. Behavior group [COLOR](#color). Public declarations/types are candidate contracts; ABI, representation, ownership and all error cases require separate reconciliation. Original disposition: `pending_port`. |
| `src/svg/css/css.c` | partial candidate | Legacy symbols linked by the group: `svg_css_sheet_create`, `svg_css_compute`, `svg_css_is_style_element`. Behavior group [CSS](#css). Only the named behavior/symbols in this group are mapped; the rest of this translation unit remains unproven against its frozen oracle. Original disposition: `pending_port`. |
| `src/svg/css/css.h` | partial candidate | Legacy symbols linked by the group: `svg_css_sheet_create`, `svg_css_compute`, `svg_css_keyword_equals`. Behavior group [CSS](#css). Public declarations/types are candidate contracts; ABI, representation, ownership and all error cases require separate reconciliation. Original disposition: `pending_port`. |
| `src/svg/index/index.c` | partial candidate | Legacy symbols linked by the group: `svg_index_start`, `svg_index_step`, `svg_index_advance_generation`. Behavior group [INDEX](#index). Only the named behavior/symbols in this group are mapped; the rest of this translation unit remains unproven against its frozen oracle. Original disposition: `pending_port`. |
| `src/svg/index/index.h` | partial candidate | Legacy symbols linked by the group: `svg_index_start`, `svg_index_step`, `svg_index_advance_generation`. Behavior group [INDEX](#index). Public declarations/types are candidate contracts; ABI, representation, ownership and all error cases require separate reconciliation. Original disposition: `pending_port`. |
| `src/svg/paint/paint.c` | partial candidate | Legacy symbols linked by the group: `svg_paint_edge_span`, `svg_paint_resolve_fill`, `svg_paint_sample`. Behavior group [PAINT](#paint). Only the named behavior/symbols in this group are mapped; the rest of this translation unit remains unproven against its frozen oracle. Original disposition: `pending_port`. |
| `src/svg/paint/paint.h` | partial candidate | Legacy symbols linked by the group: `svg_paint_edge_span`, `svg_paint_resolve_fill`, `svg_paint_sample`. Behavior group [PAINT](#paint). Public declarations/types are candidate contracts; ABI, representation, ownership and all error cases require separate reconciliation. Original disposition: `pending_port`. |
| `src/svg/reference/reference.c` | partial candidate | Legacy symbols linked by the group: `svg_reference_index_create`, `svg_reference_index_find`, `svg_reference_definition_end`. Behavior group [REFERENCE](#reference). Only the named behavior/symbols in this group are mapped; the rest of this translation unit remains unproven against its frozen oracle. Original disposition: `pending_port`. |
| `src/svg/reference/reference.h` | partial candidate | Legacy symbols linked by the group: `svg_reference_index_create`, `svg_reference_index_find`, `svg_reference_definition_end`. Behavior group [REFERENCE](#reference). Public declarations/types are candidate contracts; ABI, representation, ownership and all error cases require separate reconciliation. Original disposition: `pending_port`. |
| `src/svg/shape/decode.c` | unresolved | Decoder/traversal/media/style/definition integration lacks an accepted per-symbol mapping here; the full decoder order is blocked/superseded. Accepted filter/pattern/text slices cannot close this unit. Original disposition: `pending_port`. |
| `src/svg/shape/document.c` | unresolved | Decoder/traversal/media/style/definition integration lacks an accepted per-symbol mapping here; the full decoder order is blocked/superseded. Accepted filter/pattern/text slices cannot close this unit. Original disposition: `pending_port`. |
| `src/svg/shape/filter.c` | unresolved | Decoder/traversal/media/style/definition integration lacks an accepted per-symbol mapping here; the full decoder order is blocked/superseded. Accepted filter/pattern/text slices cannot close this unit. Original disposition: `pending_port`. |
| `src/svg/shape/ignored.c` | unresolved | Decoder/traversal/media/style/definition integration lacks an accepted per-symbol mapping here; the full decoder order is blocked/superseded. Accepted filter/pattern/text slices cannot close this unit. Original disposition: `pending_port`. |
| `src/svg/shape/image.c` | unresolved | Decoder/traversal/media/style/definition integration lacks an accepted per-symbol mapping here; the full decoder order is blocked/superseded. Accepted filter/pattern/text slices cannot close this unit. Original disposition: `pending_port`. |
| `src/svg/shape/marker.c` | unresolved | Decoder/traversal/media/style/definition integration lacks an accepted per-symbol mapping here; the full decoder order is blocked/superseded. Accepted filter/pattern/text slices cannot close this unit. Original disposition: `pending_port`. |
| `src/svg/shape/mask.c` | unresolved | Decoder/traversal/media/style/definition integration lacks an accepted per-symbol mapping here; the full decoder order is blocked/superseded. Accepted filter/pattern/text slices cannot close this unit. Original disposition: `pending_port`. |
| `src/svg/shape/media.h` | unresolved | Decoder/traversal/media/style/definition integration lacks an accepted per-symbol mapping here; the full decoder order is blocked/superseded. Accepted filter/pattern/text slices cannot close this unit. Original disposition: `pending_port`. |
| `src/svg/shape/pattern.c` | unresolved | Decoder/traversal/media/style/definition integration lacks an accepted per-symbol mapping here; the full decoder order is blocked/superseded. Accepted filter/pattern/text slices cannot close this unit. Original disposition: `pending_port`. |
| `src/svg/shape/selection.c` | unresolved | Decoder/traversal/media/style/definition integration lacks an accepted per-symbol mapping here; the full decoder order is blocked/superseded. Accepted filter/pattern/text slices cannot close this unit. Original disposition: `pending_port`. |
| `src/svg/shape/shape.c` | unresolved | Decoder/traversal/media/style/definition integration lacks an accepted per-symbol mapping here; the full decoder order is blocked/superseded. Accepted filter/pattern/text slices cannot close this unit. Original disposition: `pending_port`. |
| `src/svg/shape/shape.h` | unresolved | Decoder/traversal/media/style/definition integration lacks an accepted per-symbol mapping here; the full decoder order is blocked/superseded. Accepted filter/pattern/text slices cannot close this unit. Original disposition: `pending_port`. |
| `src/svg/shape/shape_alloc.c` | unresolved | Decoder/traversal/media/style/definition integration lacks an accepted per-symbol mapping here; the full decoder order is blocked/superseded. Accepted filter/pattern/text slices cannot close this unit. Original disposition: `pending_port`. |
| `src/svg/shape/shape_alloc.h` | unresolved | Decoder/traversal/media/style/definition integration lacks an accepted per-symbol mapping here; the full decoder order is blocked/superseded. Accepted filter/pattern/text slices cannot close this unit. Original disposition: `pending_port`. |
| `src/svg/shape/shape_clip_apply.c` | unresolved | Decoder/traversal/media/style/definition integration lacks an accepted per-symbol mapping here; the full decoder order is blocked/superseded. Accepted filter/pattern/text slices cannot close this unit. Original disposition: `pending_port`. |
| `src/svg/shape/shape_custom.c` | unresolved | Decoder/traversal/media/style/definition integration lacks an accepted per-symbol mapping here; the full decoder order is blocked/superseded. Accepted filter/pattern/text slices cannot close this unit. Original disposition: `pending_port`. |
| `src/svg/shape/shape_geometry.c` | unresolved | Decoder/traversal/media/style/definition integration lacks an accepted per-symbol mapping here; the full decoder order is blocked/superseded. Accepted filter/pattern/text slices cannot close this unit. Original disposition: `pending_port`. |
| `src/svg/shape/shape_internal.h` | unresolved | Decoder/traversal/media/style/definition integration lacks an accepted per-symbol mapping here; the full decoder order is blocked/superseded. Accepted filter/pattern/text slices cannot close this unit. Original disposition: `pending_port`. |
| `src/svg/shape/shape_path.c` | unresolved | Decoder/traversal/media/style/definition integration lacks an accepted per-symbol mapping here; the full decoder order is blocked/superseded. Accepted filter/pattern/text slices cannot close this unit. Original disposition: `pending_port`. |
| `src/svg/shape/shape_style.c` | unresolved | Decoder/traversal/media/style/definition integration lacks an accepted per-symbol mapping here; the full decoder order is blocked/superseded. Accepted filter/pattern/text slices cannot close this unit. Original disposition: `pending_port`. |
| `src/svg/shape/text.c` | unresolved | Decoder/traversal/media/style/definition integration lacks an accepted per-symbol mapping here; the full decoder order is blocked/superseded. Accepted filter/pattern/text slices cannot close this unit. Original disposition: `pending_port`. |
| `src/svg/shape/text_cache.c` | unresolved | Decoder/traversal/media/style/definition integration lacks an accepted per-symbol mapping here; the full decoder order is blocked/superseded. Accepted filter/pattern/text slices cannot close this unit. Original disposition: `pending_port`. |
| `src/svg/shape/transform.c` | unresolved | Decoder/traversal/media/style/definition integration lacks an accepted per-symbol mapping here; the full decoder order is blocked/superseded. Accepted filter/pattern/text slices cannot close this unit. Original disposition: `pending_port`. |
| `src/svg/shape/traverse.c` | unresolved | Decoder/traversal/media/style/definition integration lacks an accepted per-symbol mapping here; the full decoder order is blocked/superseded. Accepted filter/pattern/text slices cannot close this unit. Original disposition: `pending_port`. |
| `src/svg/shape/traverse_internal.h` | unresolved | Decoder/traversal/media/style/definition integration lacks an accepted per-symbol mapping here; the full decoder order is blocked/superseded. Accepted filter/pattern/text slices cannot close this unit. Original disposition: `pending_port`. |
| `src/svg/shape/traverse_switch.c` | unresolved | Decoder/traversal/media/style/definition integration lacks an accepted per-symbol mapping here; the full decoder order is blocked/superseded. Accepted filter/pattern/text slices cannot close this unit. Original disposition: `pending_port`. |
| `src/svg/shape/traverse_use.c` | unresolved | Decoder/traversal/media/style/definition integration lacks an accepted per-symbol mapping here; the full decoder order is blocked/superseded. Accepted filter/pattern/text slices cannot close this unit. Original disposition: `pending_port`. |
| `src/svg/shape/viewport.c` | unresolved | Decoder/traversal/media/style/definition integration lacks an accepted per-symbol mapping here; the full decoder order is blocked/superseded. Accepted filter/pattern/text slices cannot close this unit. Original disposition: `pending_port`. |
| `src/svg/xml/xml_chars.c` | unresolved | Adjacent group [XML-READER](#xml-reader) is a lead only: no concrete symbol contract in this private/helper unit was linked to its accepted Rust result. Preserve layout, ownership, error/limit and helper behavior until mapped. Original disposition: `pending_port`. |
| `src/svg/xml/xml_doctype.h` | unresolved | Doctype/entity parsing is deferred by WO-RUST-SVG-GRAMMAR-01; XML-TOKEN does not establish this behavior. Original disposition: `pending_port`. |
| `src/svg/xml/xml_doctype_parse.inc` | unresolved | Doctype/entity parsing is deferred by WO-RUST-SVG-GRAMMAR-01; XML-TOKEN does not establish this behavior. Original disposition: `pending_port`. |
| `src/svg/xml/xml_internal.h` | unresolved | Private data layout/lifetime/helper correspondence and a named accepted result plus frozen assertion map were not established. Original disposition: `pending_port`. |
| `src/svg/xml/xml_reader.c` | partial candidate | Legacy symbols linked by the group: `svg_xml_reader_start`, `svg_xml_reader_step`, `svg_xml_reader_get_progress`. Behavior group [XML-READER](#xml-reader). Only the named behavior/symbols in this group are mapped; the rest of this translation unit remains unproven against its frozen oracle. Original disposition: `pending_port`. |
| `src/svg/xml/xml_reader.h` | partial candidate | Legacy symbols linked by the group: `svg_xml_reader_start`, `svg_xml_reader_step`, `svg_xml_reader_get_progress`. Behavior group [XML-READER](#xml-reader). Public declarations/types are candidate contracts; ABI, representation, ownership and all error cases require separate reconciliation. Original disposition: `pending_port`. |
| `src/svg/xml/xml_stream_internal.inc` | partial candidate | Legacy symbols linked by the group: `svg_xml_token_stream_start_visitor`, `svg_xml_token_stream_take`. Behavior group [XML-STREAM](#xml-stream). Only the named behavior/symbols in this group are mapped; the rest of this translation unit remains unproven against its frozen oracle. Original disposition: `pending_port`. |
| `src/svg/xml/xml_tokenizer.c` | unresolved | Adjacent group [XML-TOKEN](#xml-token) is a lead only: no concrete symbol contract in this private/helper unit was linked to its accepted Rust result. Preserve layout, ownership, error/limit and helper behavior until mapped. Original disposition: `pending_port`. |
| `src/svg/xml/xml_tokenizer.h` | partial candidate | Legacy symbols linked by the group: `svg_xml_tokenize`, `svg_xml_tokenize_large`, `svg_xml_decode_attribute_value`. Behavior group [XML-TOKEN](#xml-token). Public declarations/types are candidate contracts; ABI, representation, ownership and all error cases require separate reconciliation. Original disposition: `pending_port`. |
| `src/svg/xml/xml_tokenizer_api.c` | partial candidate | Legacy symbols linked by the group: `svg_xml_tokenize`, `svg_xml_tokenize_large`, `svg_xml_decode_attribute_value`. Behavior group [XML-TOKEN](#xml-token). Only the named behavior/symbols in this group are mapped; the rest of this translation unit remains unproven against its frozen oracle. Original disposition: `pending_port`. |
| `src/svg/xml/xml_tokenizer_private.h` | unresolved | Private data layout/lifetime/helper correspondence and a named accepted result plus frozen assertion map were not established. Original disposition: `pending_port`. |
| `tests/document/test_document_model.c` | unresolved | Frozen assertions remain the oracle; no complete named Rust case/result correspondence for this file was established. Original disposition: `pending_port`. |
| `tests/document/test_source_save.c` | unresolved | Frozen assertions remain the oracle; no complete named Rust case/result correspondence for this file was established. Original disposition: `pending_port`. |
| `tests/edit/document_edit_history_test.c` | partial candidate | Behavior group [D-HISTORY](#d-history). Only the named behavior/symbols in this group are mapped; the rest of this translation unit remains unproven against its frozen oracle. Original disposition: `pending_port`. |
| `tests/edit/document_edit_snapshot_test.c` | partial candidate | Behavior group [D-HISTORY](#d-history). Only the named behavior/symbols in this group are mapped; the rest of this translation unit remains unproven against its frozen oracle. Original disposition: `pending_port`. |
| `tests/edit/test_document_edit.c` | partial candidate | Behavior group [D-EDIT](#d-edit). Explicit direct assertion-port lead: `original_fixtures.rs::{test_queries_and_attributes,test_replace_xml_region,test_replace_source_span}`; support/allocation and exhaustive coverage remain pending. Original disposition: `pending_port`. |
| `tests/raster/cache/cache_contract.c` | unresolved | Tile-cache keying, eviction, ownership and failure oracle mapping is missing; TILE concerns allocation only. Original disposition: `pending_port`. |
| `tests/realworld/__init__.py` | unresolved | Corpus generator/package/runner behavior is not established by accepted pixel fixtures; fixture identity, selection and verdict semantics remain open. Original disposition: `pending_port`. |
| `tests/realworld/generate.py` | unresolved | Corpus generator/package/runner behavior is not established by accepted pixel fixtures; fixture identity, selection and verdict semantics remain open. Original disposition: `pending_port`. |
| `tests/realworld/test_corpus.py` | unresolved | Corpus generator/package/runner behavior is not established by accepted pixel fixtures; fixture identity, selection and verdict semantics remain open. Original disposition: `pending_port`. |
| `tests/svg/__init__.py` | unresolved | Python invocation, fixture generation/selection, process protocol and assertions have no accepted Rust harness correspondence. Original disposition: `pending_port`. |
| `tests/svg/css/test_css.c` | unresolved | Frozen assertions remain the oracle; no complete named Rust case/result correspondence for this file was established. Original disposition: `pending_port`. |
| `tests/svg/index/test_index.c` | unresolved | Frozen assertions remain the oracle; no complete named Rust case/result correspondence for this file was established. Original disposition: `pending_port`. |
| `tests/svg/index/test_token_stream.c` | unresolved | Frozen assertions remain the oracle; no complete named Rust case/result correspondence for this file was established. Original disposition: `pending_port`. |
| `tests/svg/shape/__init__.py` | unresolved | Python invocation, fixture generation/selection, process protocol and assertions have no accepted Rust harness correspondence. Original disposition: `pending_port`. |
| `tests/svg/shape/clip_definition_scope_test.c` | unresolved | Frozen support/reference/CLI protocol is preserved; no accepted runner or complete assertion correspondence was established. Original disposition: `pending_port`. |
| `tests/svg/shape/gradient_stroke_test.c` | unresolved | Frozen assertions remain the oracle; no complete named Rust case/result correspondence for this file was established. Original disposition: `pending_port`. |
| `tests/svg/shape/marker_realloc_test.c` | unresolved | Frozen assertions remain the oracle; no complete named Rust case/result correspondence for this file was established. Original disposition: `pending_port`. |
| `tests/svg/shape/media_render_cli.c` | unresolved | Frozen support/reference/CLI protocol is preserved; no accepted runner or complete assertion correspondence was established. Original disposition: `pending_port`. |
| `tests/svg/shape/paint_clip_generator.py` | unresolved | Python invocation, fixture generation/selection, process protocol and assertions have no accepted Rust harness correspondence. Original disposition: `pending_port`. |
| `tests/svg/shape/path_number_test.c` | unresolved | Frozen assertions remain the oracle; no complete named Rust case/result correspondence for this file was established. Original disposition: `pending_port`. |
| `tests/svg/shape/png_utils.py` | unresolved | Python invocation, fixture generation/selection, process protocol and assertions have no accepted Rust harness correspondence. Original disposition: `pending_port`. |
| `tests/svg/shape/raster_budget_test.c` | unresolved | Frozen assertions remain the oracle; no complete named Rust case/result correspondence for this file was established. Original disposition: `pending_port`. |
| `tests/svg/shape/raster_large_render_test.c` | unresolved | Frozen assertions remain the oracle; no complete named Rust case/result correspondence for this file was established. Original disposition: `pending_port`. |
| `tests/svg/shape/shape_render_cli.c` | unresolved | Frozen support/reference/CLI protocol is preserved; no accepted runner or complete assertion correspondence was established. Original disposition: `pending_port`. |
| `tests/svg/shape/shape_render_reference.c` | unresolved | Frozen support/reference/CLI protocol is preserved; no accepted runner or complete assertion correspondence was established. Original disposition: `pending_port`. |
| `tests/svg/shape/shape_render_reference.h` | unresolved | Frozen support/reference/CLI protocol is preserved; no accepted runner or complete assertion correspondence was established. Original disposition: `pending_port`. |
| `tests/svg/shape/shape_render_reference_cli.c` | unresolved | Frozen support/reference/CLI protocol is preserved; no accepted runner or complete assertion correspondence was established. Original disposition: `pending_port`. |
| `tests/svg/shape/shape_render_stats_cli.c` | unresolved | Frozen support/reference/CLI protocol is preserved; no accepted runner or complete assertion correspondence was established. Original disposition: `pending_port`. |
| `tests/svg/shape/stroke_dash_storage_test.c` | unresolved | Frozen assertions remain the oracle; no complete named Rust case/result correspondence for this file was established. Original disposition: `pending_port`. |
| `tests/svg/shape/test_clip_work_budget.c` | unresolved | Frozen support/reference/CLI protocol is preserved; no accepted runner or complete assertion correspondence was established. Original disposition: `pending_port`. |
| `tests/svg/shape/test_decode_budget.c` | unresolved | Frozen assertions remain the oracle; no complete named Rust case/result correspondence for this file was established. Original disposition: `pending_port`. |
| `tests/svg/shape/test_ignored.c` | unresolved | Frozen assertions remain the oracle; no complete named Rust case/result correspondence for this file was established. Original disposition: `pending_port`. |
| `tests/svg/shape/test_m4_06_evidence.c` | unresolved | Frozen assertions remain the oracle; no complete named Rust case/result correspondence for this file was established. Original disposition: `pending_port`. |
| `tests/svg/shape/test_m4_06_evidence.h` | unresolved | Frozen assertions remain the oracle; no complete named Rust case/result correspondence for this file was established. Original disposition: `pending_port`. |
| `tests/svg/shape/test_paint_edge_span.c` | unresolved | Frozen assertions remain the oracle; no complete named Rust case/result correspondence for this file was established. Original disposition: `pending_port`. |
| `tests/svg/shape/test_reference.c` | unresolved | Frozen support/reference/CLI protocol is preserved; no accepted runner or complete assertion correspondence was established. Original disposition: `pending_port`. |
| `tests/svg/shape/test_reference.h` | unresolved | Frozen support/reference/CLI protocol is preserved; no accepted runner or complete assertion correspondence was established. Original disposition: `pending_port`. |
| `tests/svg/shape/test_render_frame_ignored.py` | unresolved | Python invocation, fixture generation/selection, process protocol and assertions have no accepted Rust harness correspondence. Original disposition: `pending_port`. |
| `tests/svg/shape/test_shape.c` | unresolved | Frozen assertions remain the oracle; no complete named Rust case/result correspondence for this file was established. Original disposition: `pending_port`. |
| `tests/svg/shape/test_w5_render.py` | unresolved | Python invocation, fixture generation/selection, process protocol and assertions have no accepted Rust harness correspondence. Original disposition: `pending_port`. |
| `tests/svg/shape/text_image_test.c` | unresolved | Frozen assertions remain the oracle; no complete named Rust case/result correspondence for this file was established. Original disposition: `pending_port`. |
| `tests/svg/shape/w5_reference.c` | unresolved | Frozen support/reference/CLI protocol is preserved; no accepted runner or complete assertion correspondence was established. Original disposition: `pending_port`. |
| `tests/svg/spatial/spatial_cache_test.c` | unresolved | Frozen assertions remain the oracle; no complete named Rust case/result correspondence for this file was established. Original disposition: `pending_port`. |
| `tests/svg/test_text_image_render.py` | unresolved | Python invocation, fixture generation/selection, process protocol and assertions have no accepted Rust harness correspondence. Original disposition: `pending_port`. |
| `tests/svg/xml/compare_expat.py` | unresolved | Python invocation, fixture generation/selection, process protocol and assertions have no accepted Rust harness correspondence. Original disposition: `pending_port`. |
| `tests/svg/xml/test_xml_tokenizer.c` | unresolved | Frozen assertions remain the oracle; no complete named Rust case/result correspondence for this file was established. Original disposition: `pending_port`. |
| `tests/svg/xml/xml_tokenizer_cli.c` | unresolved | Frozen support/reference/CLI protocol is preserved; no accepted runner or complete assertion correspondence was established. Original disposition: `pending_port`. |
| `tests/svg/xml/xml_tokenizer_fuzz.c` | unresolved | Frozen assertions remain the oracle; no complete named Rust case/result correspondence for this file was established. Original disposition: `pending_port`. |

## Disjoint follow-up packages and dependencies

1. **Base contracts:** establish checked arithmetic, slices, arena lifecycle and hook/failure oracles. This is independent of SVG feature work; do not duplicate accepted Rust primitives without a contract gap.
2. **Document model and save:** reconcile D-MODEL and D-SAVE using frozen node/span and write/sync/rename cases; depend on XML-TOKEN storage/span identity. Then reconcile D-EDIT, D-GROUP and D-HISTORY independently against edit/history/snapshot oracles.
3. **Geometry and index/cache:** reconcile GEOM finite affine cases, then SPATIAL query ordering and bounds. Close tile-cache and spatial-cache generations/invalidation separately; TILE allocation evidence does not close either cache.
4. **XML contracts:** reconcile XML-READER, XML-TOKEN and XML-STREAM diagnostics, budgets, cancellation and attribute decoding first. Doctype/entities and CLI/fuzz protocols remain separate unresolved packages.
5. **SVG semantics:** reconcile COLOR/CSS, INDEX, REFERENCE, PAINT, CLIP and ANIM separately using their frozen cases. Animation authoring is separate from parse/evaluation. Full decode/traversal/definitions/media integration depends on these contracts and requires a new accepted mapping; preserve blocked decoder evidence.
6. **Raster algorithms and integration:** reconcile FILTER against recorded fixtures and failure/budget behavior. Separately map fill/scanline, stroke/dash/grid, bounds, groups, animation clips and sampling to accepted algorithm slices; depend on decoded geometry/style/definition ownership. Preserve legacy work/counter behavior as an explicit gap where the accepted performance result omits it.
7. **Test/harness migration:** port frozen Python/C test support, reference/stats CLIs, fuzz harness and corpus generation only after defining fixture, argument and verdict contracts. Accepted Rust pixel fixtures do not establish these runner contracts.

## Verification boundary

No source, tests, work orders, dependencies, inventory, reconciliation or ledger was edited. No tests, builds, benchmarks or GUI checks were run. This report changes only semantic evidence and candidate classifications; every original disposition remains `pending_port` and all completeness/parity claims remain open.
