#define PROPERTY_EDIT_VALUE_JSON_LIMIT 512U

static bool property_edit_id_valid(base_bytes id)
{
    if (!base_bytes_valid(id) || id.len == 0U || id.len > 128U) return false;
    for (size_t i = 0U; i < id.len; ++i) {
        uint8_t b = id.ptr[i];
        if (!((b >= 'a' && b <= 'z') || (b >= 'A' && b <= 'Z') ||
              (b >= '0' && b <= '9') || b == '_' || b == '-')) return false;
    }
    return true;
}

static bool property_edit_name_valid(base_bytes value)
{
    return base_bytes_valid(value) && value.len != 0U &&
        value.len <= PROJECT_TIMELINE_MAX_STRING_BYTES && valid_utf8(value);
}

project_timeline_edit_result project_timeline_edit_add_property_track(
    project_timeline_edit *edit, const project_timeline_context *context,
    base_bytes track_id, base_bytes target_id, base_bytes property,
    base_bytes value_type, base_bytes interpolation, base_bytes value_json,
    base_bytes track_name)
{
    if (edit == NULL || !property_edit_id_valid(track_id) ||
        !base_bytes_valid(target_id) || target_id.len == 0U ||
        target_id.len > PROJECT_TIMELINE_MAX_STRING_BYTES ||
        !property_edit_name_valid(property) ||
        !property_edit_name_valid(value_type) ||
        !property_edit_name_valid(interpolation) ||
        !base_bytes_valid(value_json) || value_json.len == 0U ||
        value_json.len > PROPERTY_EDIT_VALUE_JSON_LIMIT ||
        !base_bytes_valid(track_name) ||
        track_name.len > PROJECT_TIMELINE_MAX_STRING_BYTES ||
        !valid_utf8(track_name))
        return edit_result(PROJECT_TIMELINE_INVALID_ARGUMENT, 0U);

    base_bytes source = project_timeline_edit_source(edit);
    project_timeline_view view = {0};
    project_timeline_edit_result checked = validate_source(source, context, &view);
    if (checked.error != PROJECT_TIMELINE_OK) return checked;
    if (view.track_count >= PROJECT_TIMELINE_MAX_TRACKS)
        return edit_result(PROJECT_TIMELINE_LIMIT, view.tracks_offset);
    size_t ignored = 0U;
    if (track_index_for_id(&view, track_id, &ignored))
        return edit_result(PROJECT_TIMELINE_DUPLICATE, view.tracks_offset);
    if (!id_in_context(context, target_id))
        return edit_result(PROJECT_TIMELINE_INVALID_REFERENCE, 0U);

    size_t capacity = 160U;
    const base_bytes parts[] = {track_id, target_id, property, value_type,
        interpolation, track_name};
    for (size_t i = 0U; i < sizeof(parts) / sizeof(parts[0]); ++i) {
        size_t escaped = 0U;
        if (!string_capacity(parts[i].len, &escaped) ||
            !base_size_add(capacity, escaped, &capacity))
            return edit_result(PROJECT_TIMELINE_LIMIT, 0U);
    }
    if (!base_size_add(capacity, value_json.len, &capacity))
        return edit_result(PROJECT_TIMELINE_LIMIT, 0U);
    uint8_t *track_data = malloc(capacity);
    if (track_data == NULL) return edit_result(PROJECT_TIMELINE_LIMIT, 0U);
    edit_writer tw = {track_data, 0U, capacity};
    bool okay = writer_literal(&tw, "{\"id\":", sizeof("{\"id\":") - 1U) &&
        writer_string(&tw, track_id) &&
        writer_literal(&tw, ",\"kind\":\"property\",\"target\":",
            sizeof(",\"kind\":\"property\",\"target\":") - 1U) &&
        writer_string(&tw, target_id) &&
        writer_literal(&tw, ",\"visible\":true,\"property\":",
            sizeof(",\"visible\":true,\"property\":") - 1U) &&
        writer_string(&tw, property) &&
        writer_literal(&tw, ",\"value_type\":", sizeof(",\"value_type\":") - 1U) &&
        writer_string(&tw, value_type) &&
        writer_literal(&tw, ",\"interpolation\":",
            sizeof(",\"interpolation\":") - 1U) &&
        writer_string(&tw, interpolation) &&
        writer_literal(&tw, ",\"keys\":[{\"tick\":0,\"value\":",
            sizeof(",\"keys\":[{\"tick\":0,\"value\":") - 1U) &&
        writer_append(&tw, value_json.ptr, value_json.len) &&
        writer_literal(&tw, "}]", 2U);
    if (okay && track_name.len != 0U)
        okay = writer_literal(&tw, ",\"name\":", sizeof(",\"name\":") - 1U) &&
            writer_string(&tw, track_name);
    if (okay) okay = writer_byte(&tw, '}');
    if (!okay) { free(track_data); return edit_result(PROJECT_TIMELINE_LIMIT, 0U); }

    size_t insert_at = view.track_count == 0U ?
        view.tracks_offset + view.tracks_length - 1U :
        (size_t)(view.track_spans[view.track_count - 1U].ptr - source.ptr) +
            view.track_spans[view.track_count - 1U].len;
    size_t tracks_end = view.tracks_offset + view.tracks_length;
    size_t added = tw.length + (view.track_count == 0U ? 0U : 1U);
    size_t output_length = 0U;
    if (insert_at < view.tracks_offset + 1U || insert_at >= tracks_end ||
        !base_size_add(source.len, added, &output_length) ||
        output_length > PROJECT_BUNDLE_MAX_TIMELINE_BYTES) {
        free(track_data);
        return edit_result(PROJECT_TIMELINE_LIMIT, view.tracks_offset);
    }
    uint8_t *output = malloc(output_length);
    if (output == NULL) { free(track_data); return edit_result(PROJECT_TIMELINE_LIMIT, 0U); }
    edit_writer writer = {output, 0U, output_length};
    okay = append_source(&writer, source, 0U, insert_at) &&
        (view.track_count == 0U || writer_byte(&writer, ',')) &&
        writer_append(&writer, track_data, tw.length) &&
        append_source(&writer, source, insert_at, source.len - insert_at);
    free(track_data);
    if (!okay || writer.length != output_length) {
        free(output);
        return edit_result(PROJECT_TIMELINE_LIMIT, insert_at);
    }
    return commit_candidate(edit, context, output, output_length);
}

project_timeline_edit_result project_timeline_edit_set_property_key(
    project_timeline_edit *edit, const project_timeline_context *context,
    base_bytes track_id, uint32_t tick, base_bytes value_json)
{
    if (edit == NULL || !base_bytes_valid(track_id) || track_id.len == 0U ||
        !base_bytes_valid(value_json) || value_json.len == 0U)
        return edit_result(PROJECT_TIMELINE_INVALID_ARGUMENT, 0U);
    base_bytes source = project_timeline_edit_source(edit);
    project_timeline_view view = {0};
    project_timeline_edit_result checked = validate_source(source, context, &view);
    if (checked.error != PROJECT_TIMELINE_OK) return checked;
    size_t track_index = 0U;
    if (!track_index_for_id(&view, track_id, &track_index))
        return edit_result(PROJECT_TIMELINE_INVALID_REFERENCE, 0U);
    if (view.drawing_tracks[track_index])
        return edit_result(PROJECT_TIMELINE_INVALID_ARGUMENT, 0U);
    if (tick >= view.duration_ticks)
        return edit_result(PROJECT_TIMELINE_INVALID_VALUE, 0U);

    if (value_json.len > PROPERTY_EDIT_VALUE_JSON_LIMIT)
        return edit_result(PROJECT_TIMELINE_LIMIT, 0U);
    uint8_t key_data[640U];
    edit_writer key = {key_data, 0U, sizeof(key_data)};
    if (!writer_literal(&key, "{\"tick\":", sizeof("{\"tick\":") - 1U) ||
        !writer_uint(&key, tick) ||
        !writer_literal(&key, ",\"value\":", sizeof(",\"value\":") - 1U) ||
        !writer_append(&key, value_json.ptr, value_json.len) ||
        !writer_byte(&key, '}'))
        return edit_result(PROJECT_TIMELINE_LIMIT, 0U);

    base_bytes array = view.item_spans[track_index];
    size_t array_start = (size_t)(array.ptr - source.ptr);
    size_t splice = array_start + array.len - 1U, remove_length = 0U;
    bool replacing = false, found = false;
    project_timeline_iterator iterator = {0};
    project_timeline_item_view item;
    while (project_timeline_item_next(&view, track_index, &iterator, &item)) {
        if (item.tick == tick) {
            splice = item.source_offset;
            remove_length = item.source_length;
            replacing = true;
            found = true;
            break;
        }
        if (item.tick > tick) {
            splice = item.source_offset;
            found = true;
            break;
        }
    }
    if (!found && iterator.next_index != view.item_counts[track_index])
        return edit_result(PROJECT_TIMELINE_INVALID_SCHEMA, array_start);
    if (!replacing) {
        size_t total = 0U;
        for (size_t i = 0U; i < view.track_count; ++i)
            if (!base_size_add(total, view.item_counts[i], &total))
                return edit_result(PROJECT_TIMELINE_LIMIT, array_start);
        if (view.item_counts[track_index] >= PROJECT_TIMELINE_MAX_ITEMS_PER_TRACK ||
            total >= PROJECT_TIMELINE_MAX_ITEMS)
            return edit_result(PROJECT_TIMELINE_LIMIT, array_start);
    }
    bool comma = !replacing && view.item_counts[track_index] != 0U;
    size_t insert_length = key.length;
    if (comma && !base_size_add(insert_length, 1U, &insert_length))
        return edit_result(PROJECT_TIMELINE_LIMIT, splice);
    size_t output_length = 0U;
    if (remove_length > source.len ||
        !base_size_add(source.len - remove_length, insert_length, &output_length) ||
        output_length > PROJECT_BUNDLE_MAX_TIMELINE_BYTES)
        return edit_result(PROJECT_TIMELINE_LIMIT, splice);
    uint8_t *output = malloc(output_length);
    if (output == NULL) return edit_result(PROJECT_TIMELINE_LIMIT, 0U);
    edit_writer writer = {output, 0U, output_length};
    bool okay = append_source(&writer, source, 0U, splice) &&
        (!comma || found || writer_byte(&writer, ',')) &&
        writer_append(&writer, key.data, key.length) &&
        (!comma || !found || writer_byte(&writer, ',')) &&
        append_source(&writer, source, splice + remove_length,
            source.len - splice - remove_length);
    if (!okay || writer.length != output_length) {
        free(output);
        return edit_result(PROJECT_TIMELINE_LIMIT, splice);
    }
    return commit_candidate(edit, context, output, output_length);
}

project_timeline_edit_result project_timeline_edit_add_property_track_key(
    project_timeline_edit *edit, const project_timeline_context *context,
    base_bytes track_id, base_bytes target_id, base_bytes property,
    base_bytes value_type, base_bytes interpolation, base_bytes initial_json,
    uint32_t tick, base_bytes value_json, base_bytes track_name)
{
    if (edit == NULL || tick == 0U || !base_bytes_valid(initial_json) ||
        !base_bytes_valid(value_json))
        return edit_result(PROJECT_TIMELINE_INVALID_ARGUMENT, 0U);
    base_bytes before = project_timeline_edit_source(edit);
    if (before.len > (SIZE_MAX - 1024U) / 2U)
        return edit_result(PROJECT_TIMELINE_LIMIT, 0U);
    project_timeline_edit *candidate = NULL;
    project_timeline_edit_result changed = project_timeline_edit_create(before,
        context, (project_timeline_edit_limits){1U, before.len * 2U + 1024U},
        &candidate);
    if (changed.error != PROJECT_TIMELINE_OK) return changed;
    changed = project_timeline_edit_add_property_track(candidate, context,
        track_id, target_id, property, value_type, interpolation, initial_json,
        track_name);
    if (changed.error == PROJECT_TIMELINE_OK)
        changed = project_timeline_edit_set_property_key(candidate, context,
            track_id, tick, value_json);
    if (changed.error == PROJECT_TIMELINE_OK) {
        base_bytes after = project_timeline_edit_source(candidate);
        uint8_t *copy = after.len == 0U ? NULL : malloc(after.len);
        if (copy == NULL) changed = edit_result(PROJECT_TIMELINE_LIMIT, 0U);
        else {
            memcpy(copy, after.ptr, after.len);
            changed = commit_candidate(edit, context, copy, after.len);
        }
    }
    project_timeline_edit_destroy(candidate);
    return changed;
}
