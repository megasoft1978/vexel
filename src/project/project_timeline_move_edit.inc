static bool move_tick_span(base_bytes item, size_t *out_start, size_t *out_length)
{
    if (!base_bytes_valid(item) || out_start == NULL || out_length == NULL) return false;
    bool in_string = false, escaped = false;
    for (size_t index = 0U; index < item.len; ++index) {
        uint8_t byte = item.ptr[index];
        if (in_string) {
            if (escaped) escaped = false;
            else if (byte == '\\') escaped = true;
            else if (byte == '"') in_string = false;
            continue;
        }
        if (byte != '"') continue;
        static const uint8_t tick_key[] = "\"tick\"";
        if (sizeof(tick_key) - 1U <= item.len - index &&
            memcmp(item.ptr + index, tick_key, sizeof(tick_key) - 1U) == 0) {
            size_t cursor = index + sizeof(tick_key) - 1U;
            while (cursor < item.len && (item.ptr[cursor] == ' ' ||
                item.ptr[cursor] == '\t' || item.ptr[cursor] == '\r' ||
                item.ptr[cursor] == '\n')) cursor++;
            if (cursor >= item.len || item.ptr[cursor] != ':') {
                index += sizeof(tick_key) - 1U;
                continue;
            }
            cursor++;
            while (cursor < item.len && (item.ptr[cursor] == ' ' ||
                item.ptr[cursor] == '\t' || item.ptr[cursor] == '\r' ||
                item.ptr[cursor] == '\n')) cursor++;
            size_t start = cursor;
            while (cursor < item.len && item.ptr[cursor] >= '0' &&
                item.ptr[cursor] <= '9') cursor++;
            if (cursor == start) return false;
            *out_start = start;
            *out_length = cursor - start;
            return true;
        }
        in_string = true;
    }
    return false;
}

static bool move_write_item(edit_writer *writer, base_bytes source,
    const project_timeline_item_view *item, bool moved, uint32_t new_tick)
{
    if (item->source_offset > source.len ||
        item->source_length > source.len - item->source_offset) return false;
    base_bytes bytes = {source.ptr + item->source_offset, item->source_length};
    if (!moved) return writer_append(writer, bytes.ptr, bytes.len);
    size_t tick_start = 0U, tick_length = 0U;
    if (!move_tick_span(bytes, &tick_start, &tick_length)) return false;
    return writer_append(writer, bytes.ptr, tick_start) &&
        writer_uint(writer, new_tick) &&
        writer_append(writer, bytes.ptr + tick_start + tick_length,
            bytes.len - tick_start - tick_length);
}

project_timeline_edit_result project_timeline_edit_move_item(
    project_timeline_edit *edit, const project_timeline_context *context,
    base_bytes track_id, uint32_t from_tick, uint32_t to_tick)
{
    if (edit == NULL || !base_bytes_valid(track_id) || track_id.len == 0U)
        return edit_result(PROJECT_TIMELINE_INVALID_ARGUMENT, 0U);
    base_bytes source = project_timeline_edit_source(edit);
    project_timeline_view view = {0};
    project_timeline_edit_result checked = validate_source(source, context, &view);
    if (checked.error != PROJECT_TIMELINE_OK) return checked;
    size_t track_index = 0U;
    if (!track_index_for_id(&view, track_id, &track_index))
        return edit_result(PROJECT_TIMELINE_INVALID_REFERENCE, 0U);
    size_t count = view.item_counts[track_index];
    if (count == 0U || count > PROJECT_TIMELINE_MAX_ITEMS_PER_TRACK ||
        count > SIZE_MAX / sizeof(project_timeline_item_view) ||
        count > SIZE_MAX / sizeof(size_t))
        return edit_result(PROJECT_TIMELINE_LIMIT, 0U);
    project_timeline_item_view *items = malloc(count * sizeof(*items));
    size_t *order = malloc(count * sizeof(*order));
    if (items == NULL || order == NULL) {
        free(items);
        free(order);
        return edit_result(PROJECT_TIMELINE_LIMIT, 0U);
    }
    size_t found_index = SIZE_MAX;
    project_timeline_iterator iterator = {0};
    project_timeline_item_view item;
    size_t item_count = 0U;
    while (project_timeline_item_next(&view, track_index, &iterator, &item)) {
        if (item_count >= count) {
            free(items);
            free(order);
            return edit_result(PROJECT_TIMELINE_INVALID_SCHEMA, item.source_offset);
        }
        items[item_count] = item;
        if (item.tick == from_tick) found_index = item_count;
        item_count++;
    }
    if (iterator.next_index != count || item_count != count) {
        free(items);
        free(order);
        return edit_result(PROJECT_TIMELINE_INVALID_SCHEMA, 0U);
    }
    if (found_index == SIZE_MAX) {
        free(items);
        free(order);
        return edit_result(PROJECT_TIMELINE_INVALID_REFERENCE, 0U);
    }
    if (from_tick == to_tick) {
        free(items);
        free(order);
        return edit_result(PROJECT_TIMELINE_OK, 0U);
    }
    if (from_tick == 0U || to_tick >= view.duration_ticks) {
        free(items);
        free(order);
        return edit_result(PROJECT_TIMELINE_INVALID_VALUE, 0U);
    }
    for (size_t index = 0U; index < count; ++index) {
        if (index != found_index && items[index].tick == to_tick) {
            size_t error_offset = items[index].source_offset;
            free(items);
            free(order);
            return edit_result(PROJECT_TIMELINE_DUPLICATE, error_offset);
        }
    }
    size_t ordered_count = 0U;
    bool inserted = false;
    for (size_t index = 0U; index < count; ++index) {
        if (index == found_index) continue;
        if (!inserted && to_tick < items[index].tick) {
            order[ordered_count++] = SIZE_MAX;
            inserted = true;
        }
        order[ordered_count++] = index;
    }
    if (!inserted) order[ordered_count++] = SIZE_MAX;
    if (ordered_count != count) {
        free(items);
        free(order);
        return edit_result(PROJECT_TIMELINE_INVALID_SCHEMA, 0U);
    }
    base_bytes item_array = view.item_spans[track_index];
    if (item_array.len < 2U || item_array.ptr < source.ptr ||
        (size_t)(item_array.ptr - source.ptr) > source.len ||
        item_array.len > source.len - (size_t)(item_array.ptr - source.ptr)) {
        free(items);
        free(order);
        return edit_result(PROJECT_TIMELINE_INVALID_SCHEMA, 0U);
    }
    project_timeline_item_view moved = items[found_index];
    base_bytes moved_bytes = {source.ptr + moved.source_offset, moved.source_length};
    size_t tick_start = 0U, tick_length = 0U;
    if (!move_tick_span(moved_bytes, &tick_start, &tick_length)) {
        free(items);
        free(order);
        return edit_result(PROJECT_TIMELINE_INVALID_SCHEMA, moved.source_offset);
    }
    size_t digit_count = 1U;
    for (uint32_t value = to_tick; value >= 10U; value /= 10U) digit_count++;
    size_t moved_length = moved.source_length - tick_length;
    if (!base_size_add(moved_length, digit_count, &moved_length)) {
        free(items);
        free(order);
        return edit_result(PROJECT_TIMELINE_LIMIT, moved.source_offset);
    }
    size_t new_array_length = item_array.len - moved.source_length;
    if (!base_size_add(new_array_length, moved_length, &new_array_length)) {
        free(items);
        free(order);
        return edit_result(PROJECT_TIMELINE_LIMIT, 0U);
    }
    size_t output_length = source.len - item_array.len;
    if (!base_size_add(output_length, new_array_length, &output_length) ||
        output_length > PROJECT_BUNDLE_MAX_TIMELINE_BYTES) {
        free(items);
        free(order);
        return edit_result(PROJECT_TIMELINE_LIMIT, 0U);
    }
    uint8_t *output = malloc(output_length);
    if (output == NULL) {
        free(items);
        free(order);
        return edit_result(PROJECT_TIMELINE_LIMIT, 0U);
    }
    size_t array_offset = (size_t)(item_array.ptr - source.ptr);
    size_t prefix_end = items[0].source_offset;
    size_t suffix_start = items[count - 1U].source_offset +
        items[count - 1U].source_length;
    edit_writer writer = {output, 0U, output_length};
    bool okay = append_source(&writer, source, 0U, prefix_end);
    for (size_t position = 0U; okay && position < count; ++position) {
        size_t ordered_index = order[position];
        if (ordered_index == SIZE_MAX)
            okay = move_write_item(&writer, source, &moved, true, to_tick);
        else
            okay = move_write_item(&writer, source, &items[ordered_index], false, 0U);
        if (okay && position + 1U < count) {
            size_t gap_start = items[position].source_offset +
                items[position].source_length;
            size_t gap_end = items[position + 1U].source_offset;
            if (gap_end < gap_start) okay = false;
            else okay = append_source(&writer, source, gap_start, gap_end - gap_start);
        }
    }
    okay = okay && append_source(&writer, source, suffix_start,
        array_offset + item_array.len - suffix_start) &&
        append_source(&writer, source, array_offset + item_array.len,
            source.len - array_offset - item_array.len) &&
        writer.length == output_length;
    free(items);
    free(order);
    if (!okay) {
        free(output);
        return edit_result(PROJECT_TIMELINE_LIMIT, array_offset);
    }
    return commit_candidate(edit, context, output, output_length);
}
