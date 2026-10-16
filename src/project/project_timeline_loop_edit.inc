project_timeline_edit_result project_timeline_edit_set_loop(
    project_timeline_edit *edit, const project_timeline_context *context,
    project_timeline_loop_mode mode)
{
    static const char *const names[] = {"once", "loop", "ping_pong"};
    if (edit == NULL || mode < PROJECT_TIMELINE_LOOP_ONCE ||
        mode > PROJECT_TIMELINE_LOOP_PING_PONG)
        return edit_result(PROJECT_TIMELINE_INVALID_ARGUMENT, 0U);
    base_bytes source = project_timeline_edit_source(edit);
    project_timeline_view view = {0};
    project_timeline_edit_result checked = validate_source(source, context, &view);
    if (checked.error != PROJECT_TIMELINE_OK) return checked;
    if (view.loop_mode == mode) return edit_result(PROJECT_TIMELINE_OK, 0U);
    json_reader reader = {.source = source, .error_kind = PROJECT_TIMELINE_OK};
    if (!scan_value(&reader, 0U))
        return edit_result(reader.error_kind, reader.error);
    object_fields root = {0};
    if (!parse_object_fields(&reader, (json_span){0U, reader.cursor}, &root))
        return edit_result(reader.error_kind, reader.error);
    const json_field *field = get_field(&root, "loop");
    if (field == NULL) return edit_result(PROJECT_TIMELINE_INVALID_SCHEMA, 0U);
    size_t replacement_length = strlen(names[(size_t)mode]) + 2U;
    size_t source_length = field->value.end - field->value.start;
    size_t output_length = source.len - source_length;
    if (!base_size_add(output_length, replacement_length, &output_length) ||
        output_length > PROJECT_BUNDLE_MAX_TIMELINE_BYTES)
        return edit_result(PROJECT_TIMELINE_LIMIT, field->value.start);
    uint8_t *output = malloc(output_length);
    if (output == NULL) return edit_result(PROJECT_TIMELINE_LIMIT, 0U);
    edit_writer writer = {output, 0U, output_length};
    bool okay = append_source(&writer, source, 0U, field->value.start) &&
        writer_byte(&writer, '"') &&
        writer_literal(&writer, names[(size_t)mode], replacement_length - 2U) &&
        writer_byte(&writer, '"') &&
        append_source(&writer, source, field->value.end,
            source.len - field->value.end);
    if (!okay || writer.length != output_length) {
        free(output);
        return edit_result(PROJECT_TIMELINE_LIMIT, field->value.start);
    }
    return commit_candidate(edit, context, output, output_length);
}
