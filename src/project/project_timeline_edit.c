#define _DARWIN_C_SOURCE 1
#define _USE_EXTENDED_LOCALES_ 1
#include "project_timeline_edit.h"
#include "project_timeline_json.h"

#include <locale.h>
#include <math.h>
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define EDIT_MAX_HISTORY_BYTES (512U * 1024U * 1024U)

typedef struct edit_source {
    uint8_t *data;
    size_t length;
    size_t references;
} edit_source;

typedef struct edit_snapshot {
    edit_source *source;
} edit_snapshot;

struct project_timeline_source_lease {
    edit_source *source;
};

struct project_timeline_edit {
    edit_snapshot *snapshots;
    size_t capacity;
    size_t count;
    size_t cursor;
    size_t bytes_used;
    size_t byte_limit;
};

typedef struct edit_writer {
    uint8_t *data;
    size_t length;
    size_t capacity;
} edit_writer;

static project_timeline_edit_result edit_result(project_timeline_error error,
                                                 size_t offset)
{
    return (project_timeline_edit_result){error, offset};
}

static edit_source *edit_source_create(uint8_t *owned, size_t length)
{
    edit_source *source = malloc(sizeof(*source));
    if (source == NULL) return NULL;
    *source = (edit_source){owned, length, 1U};
    return source;
}

static void edit_source_retain(edit_source *source)
{
    if (source != NULL && source->references < SIZE_MAX) ++source->references;
}

static void edit_source_release(edit_source *source)
{
    if (source == NULL || source->references == 0U) return;
    if (--source->references != 0U) return;
    free(source->data);
    free(source);
}

static bool writer_append(edit_writer *writer, const void *bytes, size_t length)
{
    if (writer == NULL || (bytes == NULL && length != 0U) ||
        writer->length > writer->capacity ||
        length > writer->capacity - writer->length) return false;
    if (length != 0U) memcpy(writer->data + writer->length, bytes, length);
    writer->length += length;
    return true;
}

static bool writer_byte(edit_writer *writer, uint8_t byte)
{
    return writer_append(writer, &byte, 1U);
}

static bool writer_literal(edit_writer *writer, const char *literal, size_t length)
{
    return writer_append(writer, literal, length);
}

static bool string_capacity(size_t length, size_t *out_capacity)
{
    size_t escaped;
    return base_size_mul(length, 6U, &escaped) &&
        base_size_add(escaped, 2U, out_capacity);
}

static bool writer_uint(edit_writer *writer, uint32_t value)
{
    uint8_t digits[10];
    size_t count = 0U;
    do {
        digits[count++] = (uint8_t)('0' + value % 10U);
        value /= 10U;
    } while (value != 0U);
    while (count != 0U)
        if (!writer_byte(writer, digits[--count])) return false;
    return true;
}

static bool writer_string(edit_writer *writer, base_bytes value)
{
    static const uint8_t hex[] = "0123456789abcdef";
    if (!base_bytes_valid(value) || !writer_byte(writer, '"')) return false;
    for (size_t i = 0U; i < value.len; ++i) {
        uint8_t byte = value.ptr[i];
        if (byte == '"' || byte == '\\') {
            uint8_t escaped[2] = {'\\', byte};
            if (!writer_append(writer, escaped, sizeof(escaped))) return false;
        } else if (byte < 0x20U) {
            uint8_t escaped[6] = {'\\', 'u', '0', '0', hex[byte >> 4U], hex[byte & 15U]};
            if (!writer_append(writer, escaped, sizeof(escaped))) return false;
        } else if (!writer_byte(writer, byte)) return false;
    }
    return writer_byte(writer, '"');
}

static bool string_equal(base_bytes a, base_bytes b)
{
    return base_bytes_valid(a) && base_bytes_valid(b) && a.len == b.len &&
        (a.len == 0U || memcmp(a.ptr, b.ptr, a.len) == 0);
}

static bool valid_utf8(base_bytes value)
{
    if (!base_bytes_valid(value)) return false;
    size_t cursor = 0U;
    while (cursor < value.len) {
        uint8_t first = value.ptr[cursor++];
        if (first < 0x80U) continue;
        size_t width = first >= 0xc2U && first <= 0xdfU ? 2U :
            first >= 0xe0U && first <= 0xefU ? 3U :
            first >= 0xf0U && first <= 0xf4U ? 4U : 0U;
        if (width == 0U || width - 1U > value.len - cursor) return false;
        uint8_t second = value.ptr[cursor];
        if ((second & 0xc0U) != 0x80U ||
            (first == 0xe0U && second < 0xa0U) ||
            (first == 0xedU && second >= 0xa0U) ||
            (first == 0xf0U && second < 0x90U) ||
            (first == 0xf4U && second >= 0x90U)) return false;
        for (size_t i = 1U; i < width; ++i)
            if ((value.ptr[cursor + i - 1U] & 0xc0U) != 0x80U) return false;
        cursor += width - 1U;
    }
    return true;
}

static bool id_in_context(const project_timeline_context *context, base_bytes id)
{
    if (context == NULL || !base_bytes_valid(id)) return false;
    size_t low = 0U, high = context->scene_id_count;
    while (low < high) {
        size_t mid = low + (high - low) / 2U;
        size_t common = context->scene_ids[mid].len < id.len ?
            context->scene_ids[mid].len : id.len;
        int order = common == 0U ? 0 : memcmp(context->scene_ids[mid].ptr,
                                              id.ptr, common);
        if (order == 0) order = context->scene_ids[mid].len < id.len ? -1 :
            context->scene_ids[mid].len > id.len ? 1 : 0;
        if (order == 0) return true;
        if (order < 0) low = mid + 1U;
        else high = mid;
    }
    return false;
}

static bool asset_exists(const project_timeline_context *context, base_bytes name)
{
    if (context == NULL) return false;
    for (size_t i = 0U; i < context->asset_count; ++i) {
        const project_timeline_asset_ref *asset = &context->assets[i];
        if (string_equal(asset->name, name) &&
            asset->media_type.len == sizeof("image/svg+xml") - 1U &&
            memcmp(asset->media_type.ptr, "image/svg+xml", sizeof("image/svg+xml") - 1U) == 0)
            return true;
    }
    return false;
}

static bool track_index_for_id(const project_timeline_view *view, base_bytes wanted,
                               size_t *out_index)
{
    uint8_t decoded[PROJECT_TIMELINE_MAX_STRING_BYTES];
    for (size_t i = 0U; i < view->track_count; ++i) {
        project_timeline_track_view track;
        size_t length = 0U;
        if (!project_timeline_track_at(view, i, &track) ||
            !project_timeline_string_copy(track.id,
                (base_mut_bytes){decoded, sizeof(decoded)}, &length)) return false;
        if (length == wanted.len && memcmp(decoded, wanted.ptr, length) == 0) {
            *out_index = i;
            return true;
        }
    }
    return false;
}

static project_timeline_edit_result validate_source(base_bytes source,
    const project_timeline_context *context, project_timeline_view *out_view)
{
    project_timeline_result decoded = project_timeline_decode(source, context, out_view);
    return edit_result(decoded.error, decoded.error_offset);
}

project_timeline_edit_limits project_timeline_edit_default_limits(void)
{
    return (project_timeline_edit_limits){PROJECT_TIMELINE_EDIT_DEFAULT_COMMANDS,
                                          PROJECT_TIMELINE_EDIT_DEFAULT_BYTES};
}

project_timeline_edit_result project_timeline_edit_create(
    base_bytes source, const project_timeline_context *context,
    project_timeline_edit_limits limits, project_timeline_edit **out_edit)
{
    if (out_edit != NULL) *out_edit = NULL;
    if (out_edit == NULL || limits.history_commands == 0U ||
        limits.history_commands > PROJECT_TIMELINE_EDIT_MAX_COMMANDS ||
        limits.history_bytes == 0U || limits.history_bytes > EDIT_MAX_HISTORY_BYTES)
        return edit_result(PROJECT_TIMELINE_INVALID_ARGUMENT, 0U);
    project_timeline_view view = {0};
    project_timeline_edit_result checked = validate_source(source, context, &view);
    if (checked.error != PROJECT_TIMELINE_OK) return checked;
    if (view.empty || source.len > limits.history_bytes)
        return edit_result(PROJECT_TIMELINE_LIMIT, 0U);
    project_timeline_edit *edit = calloc(1U, sizeof(*edit));
    if (edit == NULL) return edit_result(PROJECT_TIMELINE_LIMIT, 0U);
    size_t capacity = limits.history_commands + 1U;
    if (capacity > SIZE_MAX / sizeof(*edit->snapshots)) {
        free(edit);
        return edit_result(PROJECT_TIMELINE_LIMIT, 0U);
    }
    edit->snapshots = calloc(capacity, sizeof(*edit->snapshots));
    uint8_t *copy = malloc(source.len);
    if (edit->snapshots == NULL || copy == NULL) {
        free(copy);
        free(edit->snapshots);
        free(edit);
        return edit_result(PROJECT_TIMELINE_LIMIT, 0U);
    }
    memcpy(copy, source.ptr, source.len);
    edit_source *initial = edit_source_create(copy, source.len);
    if (initial == NULL) {
        free(copy);
        free(edit->snapshots);
        free(edit);
        return edit_result(PROJECT_TIMELINE_LIMIT, 0U);
    }
    edit->snapshots[0] = (edit_snapshot){initial};
    edit->capacity = capacity;
    edit->count = 1U;
    edit->byte_limit = limits.history_bytes;
    edit->bytes_used = source.len;
    *out_edit = edit;
    return edit_result(PROJECT_TIMELINE_OK, 0U);
}

void project_timeline_edit_destroy(project_timeline_edit *edit)
{
    if (edit == NULL) return;
    for (size_t i = 0U; i < edit->count; ++i)
        edit_source_release(edit->snapshots[i].source);
    free(edit->snapshots);
    free(edit);
}

base_bytes project_timeline_edit_source(const project_timeline_edit *edit)
{
    if (edit == NULL || edit->count == 0U || edit->cursor >= edit->count)
        return (base_bytes){NULL, 0U};
    const edit_snapshot *snapshot = &edit->snapshots[edit->cursor];
    return (base_bytes){snapshot->source->data, snapshot->source->length};
}

project_timeline_edit_result project_timeline_edit_source_lease_create(
    const project_timeline_edit *edit,
    project_timeline_source_lease **out_lease)
{
    if (out_lease != NULL) *out_lease = NULL;
    if (edit == NULL || out_lease == NULL || edit->count == 0U ||
        edit->cursor >= edit->count || edit->snapshots[edit->cursor].source == NULL)
        return edit_result(PROJECT_TIMELINE_INVALID_ARGUMENT, 0U);
    project_timeline_source_lease *lease = malloc(sizeof(*lease));
    if (lease == NULL) return edit_result(PROJECT_TIMELINE_LIMIT, 0U);
    lease->source = edit->snapshots[edit->cursor].source;
    edit_source_retain(lease->source);
    *out_lease = lease;
    return edit_result(PROJECT_TIMELINE_OK, 0U);
}

base_bytes project_timeline_source_lease_bytes(
    const project_timeline_source_lease *lease)
{
    return lease == NULL || lease->source == NULL ? (base_bytes){NULL, 0U} :
        (base_bytes){lease->source->data, lease->source->length};
}

void project_timeline_source_lease_destroy(project_timeline_source_lease *lease)
{
    if (lease == NULL) return;
    edit_source_release(lease->source);
    free(lease);
}

size_t project_timeline_edit_undo_count(const project_timeline_edit *edit)
{
    return edit == NULL ? 0U : edit->cursor;
}

size_t project_timeline_edit_redo_count(const project_timeline_edit *edit)
{
    return edit == NULL || edit->cursor >= edit->count ? 0U :
        edit->count - edit->cursor - 1U;
}

static project_timeline_edit_result commit_candidate(project_timeline_edit *edit,
    const project_timeline_context *context, uint8_t *candidate, size_t length)
{
    base_bytes current = project_timeline_edit_source(edit);
    if (current.len == length && memcmp(current.ptr, candidate, length) == 0) {
        free(candidate);
        return edit_result(PROJECT_TIMELINE_OK, 0U);
    }
    uint8_t *compact = realloc(candidate, length);
    if (compact == NULL) {
        free(candidate);
        return edit_result(PROJECT_TIMELINE_LIMIT, 0U);
    }
    candidate = compact;
    if (length > edit->byte_limit) {
        free(candidate);
        return edit_result(PROJECT_TIMELINE_LIMIT, 0U);
    }
    project_timeline_view view = {0};
    project_timeline_edit_result checked = validate_source(
        (base_bytes){candidate, length}, context, &view);
    if (checked.error != PROJECT_TIMELINE_OK) {
        free(candidate);
        return checked;
    }
    size_t first = 0U, retained_bytes = 0U;
    for (size_t i = 0U; i <= edit->cursor; ++i) {
        if (!base_size_add(retained_bytes, edit->snapshots[i].source->length,
                           &retained_bytes)) {
            free(candidate);
            return edit_result(PROJECT_TIMELINE_LIMIT, 0U);
        }
    }
    size_t retained_count = edit->cursor + 1U;
    while (retained_count + 1U > edit->capacity ||
           retained_bytes > edit->byte_limit - length) {
        if (retained_count <= 1U) {
            free(candidate);
            return edit_result(PROJECT_TIMELINE_LIMIT, 0U);
        }
        retained_bytes -= edit->snapshots[first++].source->length;
        retained_count--;
    }
    edit_source *next_source = edit_source_create(candidate, length);
    if (next_source == NULL) {
        free(candidate);
        return edit_result(PROJECT_TIMELINE_LIMIT, 0U);
    }
    for (size_t i = edit->cursor + 1U; i < edit->count; ++i)
        edit_source_release(edit->snapshots[i].source);
    edit->count = edit->cursor + 1U;
    for (size_t i = 0U; i < first; ++i)
        edit_source_release(edit->snapshots[i].source);
    if (first != 0U) {
        memmove(edit->snapshots, edit->snapshots + first,
            retained_count * sizeof(*edit->snapshots));
        edit->cursor -= first;
    }
    edit->count = retained_count;
    edit->snapshots[edit->count++] = (edit_snapshot){next_source};
    edit->cursor = edit->count - 1U;
    edit->bytes_used = retained_bytes + length;
    return edit_result(PROJECT_TIMELINE_OK, 0U);
}

static bool append_source(edit_writer *writer, base_bytes source,
                          size_t offset, size_t length)
{
    if (offset > source.len || length > source.len - offset) return false;
    return writer_append(writer, source.ptr + offset, length);
}

project_timeline_edit_result project_timeline_edit_add_drawing_track(
    project_timeline_edit *edit, const project_timeline_context *context,
    base_bytes track_id, base_bytes target_id, base_bytes asset_name,
    base_bytes track_name)
{
    if (edit == NULL || !base_bytes_valid(track_id) || track_id.len == 0U ||
        track_id.len > 128U || !base_bytes_valid(target_id) ||
        !base_bytes_valid(track_name) ||
        track_name.len > PROJECT_TIMELINE_MAX_STRING_BYTES || !valid_utf8(track_name))
        return edit_result(PROJECT_TIMELINE_INVALID_ARGUMENT, 0U);
    for (size_t i = 0U; i < track_id.len; ++i) {
        uint8_t b = track_id.ptr[i];
        if (!((b >= 'a' && b <= 'z') || (b >= 'A' && b <= 'Z') ||
              (b >= '0' && b <= '9') || b == '_' || b == '-'))
            return edit_result(PROJECT_TIMELINE_INVALID_VALUE, i);
    }
    base_bytes source = project_timeline_edit_source(edit);
    project_timeline_view view = {0};
    project_timeline_edit_result checked = validate_source(source, context, &view);
    if (checked.error != PROJECT_TIMELINE_OK) return checked;
    if (!id_in_context(context, target_id) || !asset_exists(context, asset_name))
        return edit_result(PROJECT_TIMELINE_INVALID_REFERENCE, 0U);
    size_t existing = 0U;
    if (view.track_count >= PROJECT_TIMELINE_MAX_TRACKS)
        return edit_result(PROJECT_TIMELINE_LIMIT, 0U);
    if (track_index_for_id(&view, track_id, &existing))
        return edit_result(PROJECT_TIMELINE_DUPLICATE, 0U);
    size_t track_capacity = 128U, part;
    if (!string_capacity(track_id.len, &part) ||
        !base_size_add(track_capacity, part, &track_capacity) ||
        !string_capacity(target_id.len, &part) ||
        !base_size_add(track_capacity, part, &track_capacity) ||
        !string_capacity(asset_name.len, &part) ||
        !base_size_add(track_capacity, part, &track_capacity) ||
        !string_capacity(track_name.len, &part) ||
        !base_size_add(track_capacity, part, &track_capacity))
        return edit_result(PROJECT_TIMELINE_LIMIT, 0U);
    uint8_t *track_json = malloc(track_capacity);
    if (track_json == NULL) return edit_result(PROJECT_TIMELINE_LIMIT, 0U);
    edit_writer tw = {track_json, 0U, track_capacity};
    bool okay = writer_literal(&tw, "{\"id\":", sizeof("{\"id\":") - 1U) &&
        writer_string(&tw, track_id) &&
        writer_literal(&tw, ",\"kind\":\"drawing\",\"target\":",
            sizeof(",\"kind\":\"drawing\",\"target\":") - 1U) &&
        writer_string(&tw, target_id) &&
        writer_literal(&tw, ",\"visible\":true,\"cels\":[{\"tick\":0,\"asset\":",
            sizeof(",\"visible\":true,\"cels\":[{\"tick\":0,\"asset\":") - 1U) &&
        writer_string(&tw, asset_name) && writer_literal(&tw, "}]", 2U);
    if (okay && track_name.len != 0U)
        okay = writer_literal(&tw, ",\"name\":", sizeof(",\"name\":") - 1U) &&
            writer_string(&tw, track_name);
    if (okay) okay = writer_byte(&tw, '}');
    if (!okay) {
        free(track_json);
        return edit_result(PROJECT_TIMELINE_LIMIT, 0U);
    }
    size_t array_start = view.tracks_offset;
    size_t array_end = array_start + view.tracks_length;
    size_t insert_at = view.track_count == 0U ? array_end - 1U :
        (size_t)(view.track_spans[view.track_count - 1U].ptr - source.ptr) +
        view.track_spans[view.track_count - 1U].len;
    size_t extra = tw.length + (view.track_count == 0U ? 0U : 1U);
    size_t output_length;
    if (insert_at < array_start + 1U || insert_at >= array_end ||
        !base_size_add(source.len, extra, &output_length) ||
        output_length > PROJECT_BUNDLE_MAX_TIMELINE_BYTES) {
        free(track_json);
        return edit_result(PROJECT_TIMELINE_LIMIT, array_start);
    }
    uint8_t *output = malloc(output_length);
    if (output == NULL) {
        free(track_json);
        return edit_result(PROJECT_TIMELINE_LIMIT, 0U);
    }
    edit_writer w = {output, 0U, output_length};
    okay = append_source(&w, source, 0U, insert_at) &&
        (view.track_count == 0U || writer_byte(&w, ',')) &&
        writer_append(&w, track_json, tw.length) &&
        append_source(&w, source, insert_at, source.len - insert_at);
    free(track_json);
    if (!okay || w.length != output_length) {
        free(output);
        return edit_result(PROJECT_TIMELINE_LIMIT, insert_at);
    }
    return commit_candidate(edit, context, output, output_length);
}

static bool make_cel(edit_writer *writer, uint32_t tick, base_bytes asset,
                     base_bytes label)
{
    if (!writer_literal(writer, "{\"tick\":", sizeof("{\"tick\":") - 1U) ||
        !writer_uint(writer, tick) ||
        !writer_literal(writer, ",\"asset\":", sizeof(",\"asset\":") - 1U) ||
        !writer_string(writer, asset))
        return false;
    if (label.len != 0U &&
        (!writer_literal(writer, ",\"label\":", sizeof(",\"label\":") - 1U) ||
         !writer_string(writer, label)))
        return false;
    return writer_byte(writer, '}');
}

static bool cel_size(uint32_t tick, base_bytes asset, base_bytes label, size_t *out)
{
    size_t capacity = 48U, escaped;
    if (!string_capacity(asset.len, &escaped) ||
        !base_size_add(capacity, escaped, &capacity) ||
        !string_capacity(label.len, &escaped) ||
        !base_size_add(capacity, escaped, &capacity)) return false;
    (void)tick;
    *out = capacity;
    return true;
}

project_timeline_edit_result project_timeline_edit_set_cel(
    project_timeline_edit *edit, const project_timeline_context *context,
    base_bytes track_id, uint32_t tick, base_bytes asset_name,
    base_bytes label)
{
    if (edit == NULL || !base_bytes_valid(track_id) ||
        !base_bytes_valid(label) || label.len > PROJECT_TIMELINE_MAX_STRING_BYTES ||
        !valid_utf8(label))
        return edit_result(PROJECT_TIMELINE_INVALID_ARGUMENT, 0U);
    base_bytes source = project_timeline_edit_source(edit);
    project_timeline_view view = {0};
    project_timeline_edit_result checked = validate_source(source, context, &view);
    if (checked.error != PROJECT_TIMELINE_OK) return checked;
    if (!asset_exists(context, asset_name))
        return edit_result(PROJECT_TIMELINE_INVALID_REFERENCE, 0U);
    size_t track_index = 0U;
    if (!track_index_for_id(&view, track_id, &track_index))
        return edit_result(PROJECT_TIMELINE_INVALID_REFERENCE, 0U);
    if (!view.drawing_tracks[track_index])
        return edit_result(PROJECT_TIMELINE_INVALID_ARGUMENT, 0U);
    if (tick >= view.duration_ticks)
        return edit_result(PROJECT_TIMELINE_INVALID_VALUE, 0U);
    size_t cel_capacity;
    if (!cel_size(tick, asset_name, label, &cel_capacity))
        return edit_result(PROJECT_TIMELINE_LIMIT, 0U);
    uint8_t *cel_data = malloc(cel_capacity);
    if (cel_data == NULL) return edit_result(PROJECT_TIMELINE_LIMIT, 0U);
    edit_writer cel = {cel_data, 0U, cel_capacity};
    if (!make_cel(&cel, tick, asset_name, label)) {
        free(cel_data);
        return edit_result(PROJECT_TIMELINE_LIMIT, 0U);
    }
    size_t array_start = (size_t)(view.item_spans[track_index].ptr - source.ptr);
    size_t array_length = view.item_spans[track_index].len;
    size_t splice_at = array_start + array_length - 1U;
    size_t removed_length = 0U;
    bool replacing = false, append_exposure = true, located = false;
    project_timeline_iterator iterator = {0};
    project_timeline_item_view item;
    while (project_timeline_item_next(&view, track_index, &iterator, &item)) {
        if (tick == item.tick) {
            splice_at = item.source_offset;
            removed_length = item.source_length;
            replacing = true;
            append_exposure = false;
            located = true;
            break;
        }
        if (tick < item.tick) {
            splice_at = item.source_offset;
            append_exposure = false;
            located = true;
            break;
        }
    }
    if (!located && iterator.next_index != view.item_counts[track_index]) {
        free(cel_data);
        return edit_result(PROJECT_TIMELINE_INVALID_SCHEMA, array_start);
    }
    if (!replacing) {
        size_t total_items = 0U;
        for (size_t i = 0U; i < view.track_count; ++i) {
            if (!base_size_add(total_items, view.item_counts[i], &total_items)) {
                free(cel_data);
                return edit_result(PROJECT_TIMELINE_LIMIT, array_start);
            }
        }
        if (view.item_counts[track_index] >= PROJECT_TIMELINE_MAX_ITEMS_PER_TRACK ||
            total_items >= PROJECT_TIMELINE_MAX_ITEMS) {
            free(cel_data);
            return edit_result(PROJECT_TIMELINE_LIMIT, array_start);
        }
    }
    bool add_comma = !replacing && view.item_counts[track_index] != 0U;
    size_t insert_length = cel.length;
    if (add_comma && !base_size_add(insert_length, 1U, &insert_length)) {
        free(cel_data);
        return edit_result(PROJECT_TIMELINE_LIMIT, array_start);
    }
    size_t output_length;
    if (removed_length > source.len ||
        !base_size_add(source.len - removed_length, insert_length, &output_length) ||
        output_length > PROJECT_BUNDLE_MAX_TIMELINE_BYTES) {
        free(cel_data);
        return edit_result(PROJECT_TIMELINE_LIMIT, array_start);
    }
    uint8_t *output = malloc(output_length);
    if (output == NULL) {
        free(cel_data);
        return edit_result(PROJECT_TIMELINE_LIMIT, 0U);
    }
    edit_writer w = {output, 0U, output_length};
    bool okay = append_source(&w, source, 0U, splice_at) &&
        (!add_comma || !append_exposure || writer_byte(&w, ',')) &&
        writer_append(&w, cel_data, cel.length) &&
        (!add_comma || append_exposure || writer_byte(&w, ',')) &&
        append_source(&w, source, splice_at + removed_length,
            source.len - splice_at - removed_length);
    free(cel_data);
    if (!okay || w.length != output_length) {
        free(output);
        return edit_result(PROJECT_TIMELINE_LIMIT, splice_at);
    }
    return commit_candidate(edit, context, output, output_length);
}

typedef struct onion_patch {
    size_t start;
    size_t end;
    uint8_t replacement[32];
    size_t replacement_length;
} onion_patch;

static locale_t edit_c_locale;
static pthread_once_t edit_locale_once = PTHREAD_ONCE_INIT;

static void initialize_edit_locale(void)
{
    edit_c_locale = newlocale(LC_NUMERIC_MASK, "C", (locale_t)0);
}

static bool onion_patch_uint(onion_patch *patch, json_span span, uint32_t value)
{
    if (patch == NULL || span.start > span.end) return false;
    patch->start = span.start;
    patch->end = span.end;
    uint8_t reverse[10];
    size_t count = 0U;
    do {
        reverse[count++] = (uint8_t)('0' + value % 10U);
        value /= 10U;
    } while (value != 0U);
    patch->replacement_length = count;
    for (size_t index = 0U; index < count; ++index)
        patch->replacement[index] = reverse[count - index - 1U];
    return true;
}

static bool onion_patch_opacity(onion_patch *patch, json_span span, double opacity)
{
    if (patch == NULL || span.start > span.end || !isfinite(opacity) ||
        opacity < 0.0 || opacity > 1.0) return false;
    (void)pthread_once(&edit_locale_once, initialize_edit_locale);
    if (edit_c_locale == (locale_t)0) return false;
    locale_t previous = uselocale(edit_c_locale);
    if (previous == (locale_t)0) return false;
    int amount = snprintf((char *)patch->replacement,
        sizeof(patch->replacement), "%.17g", opacity);
    (void)uselocale(previous);
    if (amount <= 0 || (size_t)amount >= sizeof(patch->replacement)) return false;
    patch->start = span.start;
    patch->end = span.end;
    patch->replacement_length = (size_t)amount;
    return true;
}

project_timeline_edit_result project_timeline_edit_set_onion(
    project_timeline_edit *edit, const project_timeline_context *context,
    uint32_t before, uint32_t after, double opacity)
{
    if (edit == NULL || before > 8U || after > 8U || !isfinite(opacity) ||
        opacity < 0.0 || opacity > 1.0)
        return edit_result(PROJECT_TIMELINE_INVALID_ARGUMENT, 0U);
    base_bytes source = project_timeline_edit_source(edit);
    project_timeline_view view = {0};
    project_timeline_edit_result checked = validate_source(source, context, &view);
    if (checked.error != PROJECT_TIMELINE_OK) return checked;
    if (view.onion_before == before && view.onion_after == after &&
        view.onion_opacity == opacity)
        return edit_result(PROJECT_TIMELINE_OK, 0U);

    json_reader reader = {.source = source, .error_kind = PROJECT_TIMELINE_OK};
    if (!scan_value(&reader, 0U))
        return edit_result(reader.error_kind, reader.error);
    size_t root_end = reader.cursor;
    object_fields root = {0};
    if (!parse_object_fields(&reader, (json_span){0U, root_end}, &root))
        return edit_result(reader.error_kind, reader.error);
    const json_field *onion_field = get_field(&root, "onion");
    object_fields onion = {0};
    if (onion_field == NULL || !parse_object_fields(&reader,
            onion_field->value, &onion))
        return edit_result(PROJECT_TIMELINE_INVALID_SCHEMA,
            onion_field == NULL ? 0U : onion_field->value.start);
    const json_field *before_field = get_field(&onion, "before");
    const json_field *after_field = get_field(&onion, "after");
    const json_field *opacity_field = get_field(&onion, "opacity");
    if (before_field == NULL || after_field == NULL || opacity_field == NULL)
        return edit_result(PROJECT_TIMELINE_INVALID_SCHEMA,
            onion_field->value.start);
    onion_patch patches[3] = {0};
    if (!onion_patch_uint(&patches[0], before_field->value, before) ||
        !onion_patch_uint(&patches[1], after_field->value, after) ||
        !onion_patch_opacity(&patches[2], opacity_field->value, opacity))
        return edit_result(PROJECT_TIMELINE_INVALID_VALUE,
            opacity_field->value.start);
    for (size_t index = 1U; index < 3U; ++index) {
        onion_patch item = patches[index];
        size_t position = index;
        while (position > 0U && patches[position - 1U].start > item.start) {
            patches[position] = patches[position - 1U];
            position--;
        }
        patches[position] = item;
    }
    size_t removed = 0U, added = 0U;
    for (size_t index = 0U; index < 3U; ++index) {
        if (patches[index].start > patches[index].end ||
            patches[index].end > source.len ||
            (index > 0U && patches[index - 1U].end > patches[index].start) ||
            !base_size_add(removed, patches[index].end - patches[index].start,
                &removed) ||
            !base_size_add(added, patches[index].replacement_length, &added))
            return edit_result(PROJECT_TIMELINE_INVALID_SCHEMA,
                onion_field->value.start);
    }
    if (removed > source.len) return edit_result(PROJECT_TIMELINE_LIMIT,
        onion_field->value.start);
    size_t output_length = source.len - removed;
    if (!base_size_add(output_length, added, &output_length) ||
        output_length > PROJECT_BUNDLE_MAX_TIMELINE_BYTES)
        return edit_result(PROJECT_TIMELINE_LIMIT, onion_field->value.start);
    uint8_t *output = malloc(output_length);
    if (output == NULL) return edit_result(PROJECT_TIMELINE_LIMIT, 0U);
    edit_writer writer = {output, 0U, output_length};
    size_t cursor = 0U;
    bool okay = true;
    for (size_t index = 0U; okay && index < 3U; ++index) {
        okay = append_source(&writer, source, cursor,
                patches[index].start - cursor) &&
            writer_append(&writer, patches[index].replacement,
                patches[index].replacement_length);
        cursor = patches[index].end;
    }
    okay = okay && append_source(&writer, source, cursor, source.len - cursor);
    if (!okay || writer.length != output_length) {
        free(output);
        return edit_result(PROJECT_TIMELINE_LIMIT, onion_field->value.start);
    }
    return commit_candidate(edit, context, output, output_length);
}

project_timeline_edit_result project_timeline_edit_remove_cel(
    project_timeline_edit *edit, const project_timeline_context *context,
    base_bytes track_id, uint32_t tick)
{
    if (edit == NULL || !base_bytes_valid(track_id))
        return edit_result(PROJECT_TIMELINE_INVALID_ARGUMENT, 0U);
    base_bytes source = project_timeline_edit_source(edit);
    project_timeline_view view = {0};
    project_timeline_edit_result checked = validate_source(source, context, &view);
    if (checked.error != PROJECT_TIMELINE_OK) return checked;
    size_t track_index = 0U;
    if (!track_index_for_id(&view, track_id, &track_index))
        return edit_result(PROJECT_TIMELINE_INVALID_REFERENCE, 0U);
    if (!view.drawing_tracks[track_index])
        return edit_result(PROJECT_TIMELINE_INVALID_ARGUMENT, 0U);
    size_t remove_start = 0U, remove_end = 0U;
    bool found = false;
    project_timeline_iterator iterator = {0};
    project_timeline_item_view item, previous = {0};
    size_t index = 0U;
    while (project_timeline_item_next(&view, track_index, &iterator, &item)) {
        if (item.tick == tick) {
            found = true;
            if (view.item_counts[track_index] <= 1U)
                return edit_result(PROJECT_TIMELINE_INVALID_VALUE, 0U);
            if (index == 0U) {
                project_timeline_item_view next = {0};
                if (!project_timeline_item_next(&view, track_index, &iterator, &next))
                    return edit_result(PROJECT_TIMELINE_INVALID_SCHEMA, 0U);
                remove_start = item.source_offset;
                remove_end = next.source_offset;
            } else {
                size_t comma = previous.source_offset + previous.source_length;
                while (comma < item.source_offset && source.ptr[comma] != ',') comma++;
                if (comma == item.source_offset)
                    return edit_result(PROJECT_TIMELINE_INVALID_SCHEMA, item.source_offset);
                remove_start = comma;
                remove_end = item.source_offset + item.source_length;
            }
            break;
        }
        previous = item;
        index++;
    }
    if (!found && iterator.next_index != view.item_counts[track_index])
        return edit_result(PROJECT_TIMELINE_INVALID_SCHEMA, 0U);
    if (!found) return edit_result(PROJECT_TIMELINE_INVALID_REFERENCE, 0U);
    size_t output_length = source.len - (remove_end - remove_start);
    uint8_t *output = malloc(output_length);
    if (output == NULL) return edit_result(PROJECT_TIMELINE_LIMIT, 0U);
    edit_writer w = {output, 0U, output_length};
    bool okay = append_source(&w, source, 0U, remove_start) &&
        append_source(&w, source, remove_end, source.len - remove_end);
    if (!okay || w.length != output_length) {
        free(output);
        return edit_result(PROJECT_TIMELINE_LIMIT, remove_start);
    }
    return commit_candidate(edit, context, output, output_length);
}

static project_timeline_edit_result move_history(project_timeline_edit *edit,
    const project_timeline_context *context, bool redo)
{
    if (edit == NULL) return edit_result(PROJECT_TIMELINE_INVALID_ARGUMENT, 0U);
    if ((!redo && edit->cursor == 0U) ||
        (redo && edit->cursor + 1U >= edit->count))
        return edit_result(PROJECT_TIMELINE_INVALID_VALUE, 0U);
    size_t cursor = redo ? edit->cursor + 1U : edit->cursor - 1U;
    project_timeline_view view = {0};
    project_timeline_edit_result checked = validate_source(
        (base_bytes){edit->snapshots[cursor].source->data,
                     edit->snapshots[cursor].source->length}, context, &view);
    if (checked.error != PROJECT_TIMELINE_OK) return checked;
    edit->cursor = cursor;
    return edit_result(PROJECT_TIMELINE_OK, 0U);
}

project_timeline_edit_result project_timeline_edit_undo(
    project_timeline_edit *edit, const project_timeline_context *context)
{
    return move_history(edit, context, false);
}

project_timeline_edit_result project_timeline_edit_redo(
    project_timeline_edit *edit, const project_timeline_context *context)
{
    return move_history(edit, context, true);
}

#include "project_timeline_loop_edit.inc"
#include "project_timeline_property_edit.inc"
#include "project_timeline_move_edit.inc"
