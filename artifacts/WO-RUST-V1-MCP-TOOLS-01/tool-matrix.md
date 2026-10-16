# Tool matrix

Tests: `T:` = rust/crates/vexel-app/tests, protocol = vexel-protocol/tests/mcp_preview_profile.rs.

| tool | success test | rejection tests |
|---|---|---|
| save | T:mcp_tools_files `save_as_then_open_in_a_fresh_document_is_byte_identical`, `path_tool_failures_...` (deleted-file re-save) | `path_tool_failures_...` (untitled, ungranted path), `missing_grant_refuses_every_path_tool`, edit: `wrong_document_id_is_rejected_by_save_tools_and_export` |
| save_as | `save_as_then_open_...` | `path_tool_failures_...` (ext, ungranted, missing parent, oversize, empty), `symlinks_cannot_escape_the_grant`, `missing_grant_...` |
| document_open | `save_as_then_open_...`, `open_svg_replaces_the_document_and_ids_continue_after_it` | `path_tool_failures_...` (truncated, corrupt, flipped byte, bad svg, wrong ext, missing, ungranted, oversize, empty), `symlinks_...`, `missing_grant_...` |
| attribute_set | T:mcp_tools_edit `attribute_set_fill_stroke_width_and_translate`, `attribute_set_translate_both_spellings`, `attribute_set_changes_only_the_named_element` | `attribute_set_rejections` (colour, oversize, unknown element, name, width bounds/NaN, transform forms, wrong documentId) |
| element_delete | `element_delete_removes_one_element` | `element_delete_rejections` |
| element_add line, path | `element_add_line_and_path_use_stable_ids` | `element_add_line_and_path_rejections` (NaN/inf, colour, width, oversize d, bad d, extra key, geometry, parent, circle/g/text, wrong documentId) |
| export_svg | `export_svg_returns_source_and_changes_nothing`, files: `export_svg_decodes_and_equals_the_source` | `export_svg_too_large_is_typed`, `wrong_document_id_...` |
| grants | files: `grant_dir_flag_parsing`, `explicit_grants_combine_with_environment_and_config_file`, `export_job_and_tool_share_the_grant_policy` | same tests |
| list / protocol | edit: `library_target_links_and_lists_routed_tools`, `render_png_is_not_routed`; protocol: `live_preview_advertises_only_supported_tools_and_narrow_shapes`, `live_preview_forwards_routed_file_edit_and_line_path_calls` | |
| driver resync (AC7) | edit: `every_routed_edit_resyncs_the_shared_driver`, `host_driver_follows_routed_edits` | |
