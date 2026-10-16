#include "project_timeline_integration.h"
#include "project_timeline_json.h"

#include <stdlib.h>
#include <string.h>

typedef struct scene_id_entry {
    base_bytes value;
    size_t source_offset;
} scene_id_entry;

typedef struct scene_id_collection {
    base_bytes scene;
    scene_id_entry *ids;
    size_t id_capacity;
    size_t id_count;
    size_t pool_capacity;
    size_t pool_used;
    uint8_t *pool;
    bool count_only;
    project_timeline_error error;
    size_t error_offset;
} scene_id_collection;

typedef struct drawing_asset_entry {
    project_bundle_asset asset;
    bool validated;
} drawing_asset_entry;

static project_timeline_integration_result integration_result(
    project_timeline_compatibility compatibility, project_timeline_error error,
    size_t offset, bool canceled)
{
    project_timeline_integration_result result = {0};
    result.compatibility = compatibility;
    result.validation.error = error;
    result.validation.error_offset = offset;
    result.canceled = canceled;
    return result;
}

void project_timeline_context_owner_destroy(project_timeline_context_owner *owner)
{
    if (owner == NULL) return;
    free(owner->scene_ids);
    free(owner->scene_id_pool);
    free(owner->assets);
    *owner = (project_timeline_context_owner){0};
}

static bool is_id_attribute(base_bytes source, const svg_xml_token *token)
{
    return token->kind == SVG_XML_TOKEN_ATTRIBUTE && token->prefix.length == 0U &&
        token->local_name.length == 2U && token->local_name.offset <= source.len &&
        token->local_name.length <= source.len - token->local_name.offset &&
        memcmp(source.ptr + token->local_name.offset, "id", 2U) == 0;
}

static bool collect_id(void *context, const svg_xml_token *token)
{
    scene_id_collection *collection = context;
    if (!is_id_attribute(collection->scene, token)) return true;
    if (collection->id_count >= 1000000U ||
        token->value.length > collection->scene.len - collection->pool_used) {
        collection->error = PROJECT_TIMELINE_LIMIT;
        collection->error_offset = token->value.offset;
        return false;
    }
    if (collection->count_only) {
        collection->id_count++;
        collection->pool_used += token->value.length;
        return true;
    }
    if (collection->id_count >= collection->id_capacity ||
        collection->pool_used > collection->pool_capacity ||
        token->value.length > collection->pool_capacity - collection->pool_used) {
        collection->error = PROJECT_TIMELINE_LIMIT;
        collection->error_offset = token->value.offset;
        return false;
    }
    uint8_t *destination = collection->pool == NULL ? NULL :
        collection->pool + collection->pool_used;
    size_t decoded_length = 0U, error_offset = 0U;
    if (!svg_xml_decode_attribute_value(collection->scene, token->value,
        destination, token->value.length, &decoded_length, &error_offset) ||
        decoded_length == 0U || decoded_length > 256U) {
        collection->error = decoded_length > 256U ? PROJECT_TIMELINE_LIMIT :
            PROJECT_TIMELINE_INVALID_VALUE;
        collection->error_offset = token->value.offset + error_offset;
        return false;
    }
    collection->ids[collection->id_count++] = (scene_id_entry){
        {destination, decoded_length}, token->value.offset};
    collection->pool_used += token->value.length;
    return true;
}

static svg_xml_stream_status visit_scene(base_bytes scene,
    svg_xml_token_visitor_fn visitor, void *visitor_context,
    svg_xml_cancel_check_fn canceled, void *cancel_context,
    svg_xml_result *out_xml_result)
{
    svg_xml_token_stream *stream = NULL;
    svg_xml_stream_status status = svg_xml_token_stream_start_visitor(scene,
        visitor, visitor_context, canceled, cancel_context, &stream);
    if (status != SVG_XML_STREAM_OK) return status;
    do {
        status = svg_xml_token_stream_step(stream, 64U * 1024U);
    } while (status == SVG_XML_STREAM_OK);
    if (out_xml_result != NULL)
        *out_xml_result = svg_xml_token_stream_result(stream);
    svg_xml_token_stream_destroy(stream);
    return status;
}

static int id_value_compare(base_bytes a, base_bytes b)
{
    size_t common = a.len < b.len ? a.len : b.len;
    int order = common == 0U ? 0 : memcmp(a.ptr, b.ptr, common);
    if (order != 0) return order;
    return a.len < b.len ? -1 : a.len > b.len ? 1 : 0;
}

static int id_compare(const void *left, const void *right)
{
    const scene_id_entry *a = left, *b = right;
    int order = id_value_compare(a->value, b->value);
    if (order != 0) return order;
    return a->source_offset < b->source_offset ? -1 :
        a->source_offset > b->source_offset ? 1 : 0;
}

static int bytes_order(base_bytes a, base_bytes b)
{
    size_t common = a.len < b.len ? a.len : b.len;
    int order = common == 0U ? 0 : memcmp(a.ptr, b.ptr, common);
    if (order != 0) return order;
    return a.len < b.len ? -1 : a.len > b.len ? 1 : 0;
}

static int drawing_asset_compare(const void *left, const void *right)
{
    const drawing_asset_entry *a = left;
    const drawing_asset_entry *b = right;
    return bytes_order(a->asset.name, b->asset.name);
}

static drawing_asset_entry *find_drawing_asset(drawing_asset_entry *assets,
    size_t count, base_bytes name)
{
    size_t low = 0U, high = count;
    while (low < high) {
        size_t middle = low + (high - low) / 2U;
        int order = bytes_order(assets[middle].asset.name, name);
        if (order == 0) return &assets[middle];
        if (order < 0) low = middle + 1U;
        else high = middle;
    }
    return NULL;
}

static bool validate_drawing_assets(const project_timeline_view *timeline,
    drawing_asset_entry *assets, size_t asset_count,
    svg_xml_cancel_check_fn canceled, void *cancel_context,
    project_timeline_integration_result *failure)
{
    for (size_t track_index = 0U; track_index < timeline->track_count;
         ++track_index) {
        project_timeline_track_view track;
        if (!project_timeline_track_at(timeline, track_index, &track)) {
            *failure = integration_result(PROJECT_TIMELINE_COMPATIBILITY_NATIVE_V1,
                PROJECT_TIMELINE_INVALID_SCHEMA, 0U, false);
            return false;
        }
        if (!timeline->drawing_tracks[track_index]) continue;
        project_timeline_iterator iterator = {0};
        project_timeline_item_view item;
        size_t visited = 0U;
        while (project_timeline_item_next(timeline, track_index, &iterator,
                                          &item)) {
            if (canceled != NULL && canceled(cancel_context)) {
                *failure = integration_result(PROJECT_TIMELINE_COMPATIBILITY_NATIVE_V1,
                    PROJECT_TIMELINE_INVALID_ARGUMENT, item.source_offset, true);
                return false;
            }
            uint8_t decoded_name[PROJECT_TIMELINE_MAX_STRING_BYTES];
            size_t decoded_length = 0U;
            if (!project_timeline_string_copy(item.asset,
                    (base_mut_bytes){decoded_name, sizeof(decoded_name)},
                    &decoded_length)) {
                *failure = integration_result(PROJECT_TIMELINE_COMPATIBILITY_NATIVE_V1,
                    PROJECT_TIMELINE_INVALID_VALUE, item.source_offset, false);
                return false;
            }
            drawing_asset_entry *entry = find_drawing_asset(assets, asset_count,
                (base_bytes){decoded_name, decoded_length});
            if (entry == NULL) {
                *failure = integration_result(PROJECT_TIMELINE_COMPATIBILITY_NATIVE_V1,
                    PROJECT_TIMELINE_INVALID_REFERENCE, item.source_offset, false);
                return false;
            }
            if (!entry->validated) {
                bool asset_canceled = false;
                project_drawing_asset_result checked = project_drawing_asset_validate(
                    entry->asset.data, canceled, cancel_context, &asset_canceled);
                if (asset_canceled) {
                    *failure = integration_result(
                        PROJECT_TIMELINE_COMPATIBILITY_NATIVE_V1,
                        PROJECT_TIMELINE_INVALID_ARGUMENT, item.source_offset, true);
                    return false;
                }
                if (canceled != NULL && canceled(cancel_context)) {
                    *failure = integration_result(PROJECT_TIMELINE_COMPATIBILITY_NATIVE_V1,
                        PROJECT_TIMELINE_INVALID_ARGUMENT, item.source_offset, true);
                    return false;
                }
                if (checked.error != SVG_SHAPE_ERROR_NONE) {
                    *failure = integration_result(PROJECT_TIMELINE_COMPATIBILITY_NATIVE_V1,
                        checked.error == SVG_SHAPE_ERROR_LIMIT ? PROJECT_TIMELINE_LIMIT :
                            PROJECT_TIMELINE_INVALID_VALUE,
                        checked.error_offset, false);
                    failure->asset_validation = checked;
                    failure->asset_name = entry->asset.name;
                    return false;
                }
                entry->validated = true;
            }
            visited++;
        }
        if (visited != track.item_count) {
            *failure = integration_result(PROJECT_TIMELINE_COMPATIBILITY_NATIVE_V1,
                PROJECT_TIMELINE_INVALID_SCHEMA, 0U, false);
            return false;
        }
    }
    return true;
}

static int classify_hex_value(uint8_t byte)
{
    if (byte >= '0' && byte <= '9') return (int)(byte - '0');
    if (byte >= 'a' && byte <= 'f') return (int)(byte - 'a') + 10;
    if (byte >= 'A' && byte <= 'F') return (int)(byte - 'A') + 10;
    return -1;
}

static bool classify_utf8_sequence(const uint8_t *bytes, size_t length,
                                   size_t *cursor)
{
    uint8_t first = bytes[*cursor];
    size_t width = first >= 0xc2U && first <= 0xdfU ? 2U :
        first >= 0xe0U && first <= 0xefU ? 3U :
        first >= 0xf0U && first <= 0xf4U ? 4U : 0U;
    if (width == 0U || width > length - *cursor) return false;
    uint8_t second = bytes[*cursor + 1U];
    if ((second & 0xc0U) != 0x80U ||
        (first == 0xe0U && second < 0xa0U) ||
        (first == 0xedU && second >= 0xa0U) ||
        (first == 0xf0U && second < 0x90U) ||
        (first == 0xf4U && second >= 0x90U)) return false;
    for (size_t i = 2U; i < width; ++i)
        if ((bytes[*cursor + i] & 0xc0U) != 0x80U) return false;
    *cursor += width;
    return true;
}

static bool build_scene_ids(base_bytes scene, svg_xml_cancel_check_fn canceled,
    void *cancel_context, base_bytes **out_ids, size_t *out_count,
    uint8_t **out_pool, project_timeline_integration_result *failure)
{
    *out_ids = NULL;
    *out_count = 0U;
    *out_pool = NULL;
    scene_id_collection count = {.scene = scene, .count_only = true};
    svg_xml_result xml_result = {0};
    svg_xml_stream_status status = visit_scene(scene, collect_id, &count,
        canceled, cancel_context, &xml_result);
    if (status != SVG_XML_STREAM_COMPLETE) {
        *failure = integration_result(PROJECT_TIMELINE_COMPATIBILITY_NATIVE_V1,
            count.error == PROJECT_TIMELINE_OK ? PROJECT_TIMELINE_INVALID_SCHEMA : count.error,
            count.error == PROJECT_TIMELINE_OK ? xml_result.diagnostic.offset :
                count.error_offset, status == SVG_XML_STREAM_CANCELED &&
                count.error == PROJECT_TIMELINE_OK);
        return false;
    }
    if (count.id_count > SIZE_MAX / sizeof(scene_id_entry) ||
        count.id_count > SIZE_MAX / sizeof(base_bytes)) {
        *failure = integration_result(PROJECT_TIMELINE_COMPATIBILITY_NATIVE_V1,
            PROJECT_TIMELINE_LIMIT, 0U, false);
        return false;
    }
    scene_id_entry *ids = count.id_count == 0U ? NULL :
        calloc(count.id_count, sizeof(*ids));
    uint8_t *pool = count.pool_used == 0U ? NULL : malloc(count.pool_used);
    if ((count.id_count != 0U && ids == NULL) ||
        (count.pool_used != 0U && pool == NULL)) {
        free(ids);
        free(pool);
        *failure = integration_result(PROJECT_TIMELINE_COMPATIBILITY_NATIVE_V1,
            PROJECT_TIMELINE_LIMIT, 0U, false);
        return false;
    }
    scene_id_collection fill = {.scene = scene, .ids = ids,
        .id_capacity = count.id_count, .pool_capacity = count.pool_used,
        .pool = pool};
    status = visit_scene(scene, collect_id, &fill, canceled, cancel_context,
                         &xml_result);
    if (status != SVG_XML_STREAM_COMPLETE || fill.id_count != count.id_count) {
        free(ids);
        free(pool);
        *failure = integration_result(PROJECT_TIMELINE_COMPATIBILITY_NATIVE_V1,
            fill.error == PROJECT_TIMELINE_OK ? PROJECT_TIMELINE_INVALID_SCHEMA : fill.error,
            fill.error == PROJECT_TIMELINE_OK ? xml_result.diagnostic.offset :
                fill.error_offset, status == SVG_XML_STREAM_CANCELED &&
                fill.error == PROJECT_TIMELINE_OK);
        return false;
    }
    if (fill.id_count > 1U) qsort(ids, fill.id_count, sizeof(*ids), id_compare);
    for (size_t i = 1U; i < fill.id_count; ++i) {
        if (id_value_compare(ids[i - 1U].value, ids[i].value) == 0) {
            *failure = integration_result(PROJECT_TIMELINE_COMPATIBILITY_NATIVE_V1,
                PROJECT_TIMELINE_DUPLICATE, ids[i].source_offset, false);
            free(ids);
            free(pool);
            return false;
        }
    }
    base_bytes *sorted_ids = fill.id_count == 0U ? NULL :
        calloc(fill.id_count, sizeof(*sorted_ids));
    if (fill.id_count != 0U && sorted_ids == NULL) {
        free(ids);
        free(pool);
        *failure = integration_result(PROJECT_TIMELINE_COMPATIBILITY_NATIVE_V1,
            PROJECT_TIMELINE_LIMIT, 0U, false);
        return false;
    }
    for (size_t i = 0U; i < fill.id_count; ++i) sorted_ids[i] = ids[i].value;
    free(ids);
    *out_ids = sorted_ids;
    *out_count = fill.id_count;
    *out_pool = pool;
    return true;
}

project_timeline_integration_result project_timeline_validate_bundle_owned(
    const project_bundle_view *bundle, base_bytes scene_svg,
    svg_xml_cancel_check_fn canceled, void *cancel_context,
    project_timeline_view *out_view,
    project_timeline_context_owner *out_context)
{
    if (out_view != NULL) memset(out_view, 0, sizeof(*out_view));
    if (out_context != NULL) *out_context = (project_timeline_context_owner){0};
    if (canceled != NULL && canceled(cancel_context))
        return integration_result(PROJECT_TIMELINE_COMPATIBILITY_EMPTY,
            PROJECT_TIMELINE_INVALID_ARGUMENT, 0U, true);
    if (bundle == NULL || !base_bytes_valid(bundle->timeline_json) ||
        !base_bytes_valid(scene_svg) || scene_svg.len == 0U ||
        scene_svg.len > PROJECT_BUNDLE_MAX_SCENE_BYTES ||
        bundle->asset_count > PROJECT_BUNDLE_MAX_ASSET_COUNT)
        return integration_result(PROJECT_TIMELINE_COMPATIBILITY_EMPTY,
            PROJECT_TIMELINE_INVALID_ARGUMENT, 0U, false);
    if (bundle->timeline_json.len == 0U) {
        if (out_view != NULL) out_view->empty = true;
        return integration_result(PROJECT_TIMELINE_COMPATIBILITY_EMPTY,
            PROJECT_TIMELINE_OK, 0U, false);
    }
    bool native_v1 = false;
    size_t classify_offset = 0U;
    bool was_canceled = false;
    project_timeline_error classification = project_timeline_json_classify(
        bundle->timeline_json, &native_v1, &classify_offset, canceled,
        cancel_context, &was_canceled);
    if (was_canceled || (canceled != NULL && canceled(cancel_context)))
        return integration_result(PROJECT_TIMELINE_COMPATIBILITY_OPAQUE,
            classification, classify_offset, true);
    if (classification != PROJECT_TIMELINE_OK)
        return integration_result(PROJECT_TIMELINE_COMPATIBILITY_OPAQUE,
            classification, classify_offset, false);
    if (!native_v1)
        return integration_result(PROJECT_TIMELINE_COMPATIBILITY_OPAQUE,
            PROJECT_TIMELINE_OK, 0U, false);
    project_timeline_context empty_context = {0};
    project_timeline_view classified_view = {0};
    project_timeline_result classified = project_timeline_decode_cancellable(
        bundle->timeline_json, &empty_context, &classified_view, canceled,
        cancel_context, &was_canceled);
    if (was_canceled || (canceled != NULL && canceled(cancel_context)))
        return integration_result(PROJECT_TIMELINE_COMPATIBILITY_NATIVE_V1,
            classified.error, classified.error_offset, true);
    if (classified.error != PROJECT_TIMELINE_OK &&
        classified.error != PROJECT_TIMELINE_INVALID_REFERENCE)
        return integration_result(PROJECT_TIMELINE_COMPATIBILITY_NATIVE_V1,
            classified.error, classified.error_offset, false);

    base_bytes *scene_ids = NULL;
    uint8_t *scene_id_pool = NULL;
    size_t scene_id_count = 0U;
    project_timeline_integration_result failure = {0};
    if (!build_scene_ids(scene_svg, canceled, cancel_context, &scene_ids,
                         &scene_id_count, &scene_id_pool, &failure)) return failure;

    project_timeline_asset_ref *assets = bundle->asset_count == 0U ? NULL :
        calloc(bundle->asset_count, sizeof(*assets));
    drawing_asset_entry *drawing_assets = bundle->asset_count == 0U ? NULL :
        calloc(bundle->asset_count, sizeof(*drawing_assets));
    if (bundle->asset_count != 0U && (assets == NULL || drawing_assets == NULL)) {
        free(assets);
        free(drawing_assets);
        free(scene_ids);
        free(scene_id_pool);
        return integration_result(PROJECT_TIMELINE_COMPATIBILITY_NATIVE_V1,
            PROJECT_TIMELINE_LIMIT, 0U, false);
    }
    bool assets_valid = true;
    for (size_t i = 0U; i < bundle->asset_count; ++i) {
        project_bundle_asset asset;
        if (!project_bundle_asset_at(bundle, i, &asset)) {
            assets_valid = false;
            break;
        }
        assets[i] = (project_timeline_asset_ref){asset.name, asset.media_type};
        drawing_assets[i].asset = asset;
    }
    if (!assets_valid) {
        free(assets);
        free(drawing_assets);
        free(scene_ids);
        free(scene_id_pool);
        return integration_result(PROJECT_TIMELINE_COMPATIBILITY_NATIVE_V1,
            PROJECT_TIMELINE_INVALID_ARGUMENT, 0U, false);
    }
    project_timeline_context context = {scene_ids, scene_id_count,
        assets, bundle->asset_count};
    project_timeline_view decoded = {0};
    project_timeline_result validation = project_timeline_decode_cancellable(
        bundle->timeline_json, &context, &decoded, canceled, cancel_context,
        &was_canceled);
    if (validation.error == PROJECT_TIMELINE_OK && bundle->asset_count > 1U)
        qsort(drawing_assets, bundle->asset_count, sizeof(*drawing_assets),
              drawing_asset_compare);
    project_timeline_integration_result asset_failure = {0};
    bool drawings_valid = validation.error != PROJECT_TIMELINE_OK ||
        validate_drawing_assets(&decoded, drawing_assets, bundle->asset_count,
            canceled, cancel_context, &asset_failure);
    bool was_canceled_final = was_canceled ||
        (canceled != NULL && canceled(cancel_context));
    free(drawing_assets);
    bool valid = drawings_valid && validation.error == PROJECT_TIMELINE_OK &&
        !was_canceled_final;
    if (!valid) {
        free(assets);
        free(scene_ids);
        free(scene_id_pool);
    } else if (out_context != NULL) {
        out_context->scene_ids = scene_ids;
        out_context->scene_id_pool = scene_id_pool;
        out_context->assets = assets;
        out_context->context = (project_timeline_context){scene_ids,
            scene_id_count, assets, bundle->asset_count};
    } else {
        free(assets);
        free(scene_ids);
        free(scene_id_pool);
    }
    if (!drawings_valid) return asset_failure;
    if (validation.error == PROJECT_TIMELINE_OK && !was_canceled_final && out_view != NULL)
        *out_view = decoded;
    return integration_result(PROJECT_TIMELINE_COMPATIBILITY_NATIVE_V1,
        validation.error, validation.error_offset, was_canceled_final);
}

project_timeline_integration_result project_timeline_validate_bundle(
    const project_bundle_view *bundle, base_bytes scene_svg,
    svg_xml_cancel_check_fn canceled, void *cancel_context,
    project_timeline_view *out_view)
{
    return project_timeline_validate_bundle_owned(bundle, scene_svg, canceled,
        cancel_context, out_view, NULL);
}

static bool syntax_string(json_reader *reader, base_bytes *out_content)
{
    if (reader->cursor >= reader->source.len ||
        reader->source.ptr[reader->cursor++] != '"')
        return fail(reader, PROJECT_TIMELINE_MALFORMED_JSON);
    size_t start = reader->cursor;
    while (reader->cursor < reader->source.len) {
        if (!json_reader_poll_cancel(reader)) return false;
        uint8_t byte = reader->source.ptr[reader->cursor];
        if (byte == '"') {
            size_t end = reader->cursor++;
            if (out_content != NULL)
                *out_content = (base_bytes){reader->source.ptr + start, end - start};
            return true;
        }
        if (byte < 0x20U) return fail(reader, PROJECT_TIMELINE_MALFORMED_JSON);
        if (byte == '\\') {
            reader->cursor++;
            if (reader->cursor >= reader->source.len)
                return fail(reader, PROJECT_TIMELINE_MALFORMED_JSON);
            uint8_t escaped = reader->source.ptr[reader->cursor++];
            if (escaped == '"' || escaped == '\\' || escaped == '/' || escaped == 'b' ||
                escaped == 'f' || escaped == 'n' || escaped == 'r' || escaped == 't')
                continue;
            if (escaped != 'u' || reader->source.len - reader->cursor < 4U)
                return fail(reader, PROJECT_TIMELINE_MALFORMED_JSON);
            uint32_t code = 0U;
            for (size_t i = 0U; i < 4U; ++i) {
                int digit = classify_hex_value(reader->source.ptr[reader->cursor++]);
                if (digit < 0) return fail(reader, PROJECT_TIMELINE_MALFORMED_JSON);
                code = (code << 4U) | (uint32_t)digit;
            }
            if (code >= 0xd800U && code <= 0xdbffU) {
                if (reader->source.len - reader->cursor < 6U ||
                    reader->source.ptr[reader->cursor++] != '\\' ||
                    reader->source.ptr[reader->cursor++] != 'u')
                    return fail(reader, PROJECT_TIMELINE_MALFORMED_JSON);
                uint32_t low = 0U;
                for (size_t i = 0U; i < 4U; ++i) {
                    int digit = classify_hex_value(reader->source.ptr[reader->cursor++]);
                    if (digit < 0) return fail(reader, PROJECT_TIMELINE_MALFORMED_JSON);
                    low = (low << 4U) | (uint32_t)digit;
                }
                if (low < 0xdc00U || low > 0xdfffU)
                    return fail(reader, PROJECT_TIMELINE_MALFORMED_JSON);
            } else if (code >= 0xdc00U && code <= 0xdfffU)
                return fail(reader, PROJECT_TIMELINE_MALFORMED_JSON);
        } else if (byte >= 0x80U) {
            if (!classify_utf8_sequence(reader->source.ptr, reader->source.len,
                                        &reader->cursor))
                return fail(reader, PROJECT_TIMELINE_MALFORMED_JSON);
        } else reader->cursor++;
    }
    return fail(reader, PROJECT_TIMELINE_MALFORMED_JSON);
}

static bool syntax_number(json_reader *reader)
{
    size_t start = reader->cursor;
    if (reader->source.ptr[reader->cursor] == '-') reader->cursor++;
    if (reader->cursor >= reader->source.len)
        return fail(reader, PROJECT_TIMELINE_MALFORMED_JSON);
    if (reader->source.ptr[reader->cursor] == '0') reader->cursor++;
    else {
        uint8_t first = reader->source.ptr[reader->cursor];
        if (first < '1' || first > '9')
            return fail(reader, PROJECT_TIMELINE_MALFORMED_JSON);
        while (reader->cursor < reader->source.len &&
            reader->source.ptr[reader->cursor] >= '0' &&
            reader->source.ptr[reader->cursor] <= '9') {
            reader->cursor++;
            if (!json_reader_poll_cancel(reader)) return false;
        }
    }
    if (reader->cursor < reader->source.len && reader->source.ptr[reader->cursor] == '.') {
        size_t digits = ++reader->cursor;
        while (reader->cursor < reader->source.len &&
            reader->source.ptr[reader->cursor] >= '0' &&
            reader->source.ptr[reader->cursor] <= '9') {
            reader->cursor++;
            if (!json_reader_poll_cancel(reader)) return false;
        }
        if (digits == reader->cursor) return fail(reader, PROJECT_TIMELINE_MALFORMED_JSON);
    }
    if (reader->cursor < reader->source.len &&
        (reader->source.ptr[reader->cursor] == 'e' || reader->source.ptr[reader->cursor] == 'E')) {
        reader->cursor++;
        if (reader->cursor < reader->source.len &&
            (reader->source.ptr[reader->cursor] == '+' || reader->source.ptr[reader->cursor] == '-'))
            reader->cursor++;
        size_t digits = reader->cursor;
        while (reader->cursor < reader->source.len &&
            reader->source.ptr[reader->cursor] >= '0' &&
            reader->source.ptr[reader->cursor] <= '9') {
            reader->cursor++;
            if (!json_reader_poll_cancel(reader)) return false;
        }
        if (digits == reader->cursor) return fail(reader, PROJECT_TIMELINE_MALFORMED_JSON);
    }
    return reader->cursor > start;
}

static bool syntax_value(json_reader *reader, unsigned depth)
{
    if (depth > 32U) return fail(reader, PROJECT_TIMELINE_LIMIT);
    skip_space(reader);
    if (reader->cursor >= reader->source.len)
        return fail(reader, PROJECT_TIMELINE_MALFORMED_JSON);
    uint8_t byte = reader->source.ptr[reader->cursor];
    if (byte == '"') return syntax_string(reader, NULL);
    if (byte == '-' || (byte >= '0' && byte <= '9')) return syntax_number(reader);
    if (byte == '[') {
        reader->cursor++;
        skip_space(reader);
        if (reader->cursor < reader->source.len && reader->source.ptr[reader->cursor] == ']') {
            reader->cursor++;
            return true;
        }
        for (;;) {
            if (!syntax_value(reader, depth + 1U)) return false;
            skip_space(reader);
            if (reader->cursor >= reader->source.len)
                return fail(reader, PROJECT_TIMELINE_MALFORMED_JSON);
            uint8_t delimiter = reader->source.ptr[reader->cursor++];
            if (delimiter == ']') return true;
            if (delimiter != ',') return fail(reader, PROJECT_TIMELINE_MALFORMED_JSON);
        }
    }
    if (byte == '{') {
        reader->cursor++;
        skip_space(reader);
        if (reader->cursor < reader->source.len && reader->source.ptr[reader->cursor] == '}') {
            reader->cursor++;
            return true;
        }
        for (;;) {
            if (!syntax_string(reader, NULL)) return false;
            skip_space(reader);
            if (reader->cursor >= reader->source.len || reader->source.ptr[reader->cursor++] != ':')
                return fail(reader, PROJECT_TIMELINE_MALFORMED_JSON);
            if (!syntax_value(reader, depth + 1U)) return false;
            skip_space(reader);
            if (reader->cursor >= reader->source.len)
                return fail(reader, PROJECT_TIMELINE_MALFORMED_JSON);
            uint8_t delimiter = reader->source.ptr[reader->cursor++];
            if (delimiter == '}') return true;
            if (delimiter != ',') return fail(reader, PROJECT_TIMELINE_MALFORMED_JSON);
            skip_space(reader);
        }
    }
    static const char *const literals[] = {"true", "false", "null"};
    for (size_t i = 0U; i < sizeof(literals) / sizeof(literals[0]); ++i) {
        size_t length = strlen(literals[i]);
        if (reader->source.len - reader->cursor >= length &&
            memcmp(reader->source.ptr + reader->cursor, literals[i], length) == 0) {
            reader->cursor += length;
            return true;
        }
    }
    return fail(reader, PROJECT_TIMELINE_MALFORMED_JSON);
}

static bool content_is(base_bytes raw, const char *literal)
{
    char decoded[32];
    size_t length = 0U;
    size_t expected = strlen(literal);
    return expected < sizeof(decoded) && project_timeline_string_copy(raw,
        (base_mut_bytes){(uint8_t *)decoded, sizeof(decoded)}, &length) &&
        length == expected && memcmp(decoded, literal, length) == 0;
}

static bool root_key_is(base_bytes raw, const char *literal)
{
    return content_is(raw, literal);
}

project_timeline_error project_timeline_json_classify(base_bytes source,
    bool *out_native_v1, size_t *out_error_offset,
    bool (*cancel_check)(void *), void *cancel_context, bool *out_canceled)
{
    if (out_canceled != NULL) *out_canceled = false;
    if (out_native_v1 == NULL || out_error_offset == NULL ||
        !base_bytes_valid(source)) return PROJECT_TIMELINE_INVALID_ARGUMENT;
    *out_native_v1 = false;
    *out_error_offset = 0U;
    if (source.len > PROJECT_BUNDLE_MAX_TIMELINE_BYTES) return PROJECT_TIMELINE_LIMIT;
    json_reader syntax = {.source = source, .error_kind = PROJECT_TIMELINE_OK,
        .cancel_check = cancel_check, .cancel_context = cancel_context};
    if (source.len == 0U) return PROJECT_TIMELINE_OK;
    if (!syntax_value(&syntax, 0U)) {
        if (out_canceled != NULL) *out_canceled = syntax.canceled;
        *out_error_offset = syntax.error;
        return syntax.error_kind;
    }
    skip_space(&syntax);
    if (syntax.cursor != source.len) {
        *out_error_offset = syntax.cursor;
        return PROJECT_TIMELINE_MALFORMED_JSON;
    }
    json_reader root = {.source = source, .error_kind = PROJECT_TIMELINE_OK,
        .cancel_check = cancel_check, .cancel_context = cancel_context};
    skip_space(&root);
    if (root.cursor >= source.len || source.ptr[root.cursor++] != '{')
        return PROJECT_TIMELINE_OK;
    bool known_schema = false;
    bool have_version = false;
    json_span version = {0U, 0U};
    skip_space(&root);
    if (root.cursor < source.len && source.ptr[root.cursor] == '}') return PROJECT_TIMELINE_OK;
    for (;;) {
        base_bytes key;
        if (!syntax_string(&root, &key)) return PROJECT_TIMELINE_MALFORMED_JSON;
        skip_space(&root);
        if (root.cursor >= source.len || source.ptr[root.cursor++] != ':')
            return PROJECT_TIMELINE_MALFORMED_JSON;
        skip_space(&root);
        size_t value_start = root.cursor;
        if (root_key_is(key, "schema")) {
            if (root.cursor < source.len && source.ptr[root.cursor] == '"') {
                base_bytes schema;
                if (syntax_string(&root, &schema) && content_is(schema, "vexel.animation"))
                    known_schema = true;
                else if (root.cursor == value_start)
                    return PROJECT_TIMELINE_MALFORMED_JSON;
            } else if (!syntax_value(&root, 1U)) return PROJECT_TIMELINE_MALFORMED_JSON;
        } else if (root_key_is(key, "version") && !have_version) {
            if (!syntax_value(&root, 1U)) return PROJECT_TIMELINE_MALFORMED_JSON;
            version = (json_span){value_start, root.cursor};
            have_version = true;
        } else if (!syntax_value(&root, 1U)) return PROJECT_TIMELINE_MALFORMED_JSON;
        skip_space(&root);
        if (root.cursor >= source.len) return PROJECT_TIMELINE_MALFORMED_JSON;
        uint8_t delimiter = source.ptr[root.cursor++];
        if (delimiter == '}') break;
        if (delimiter != ',') return PROJECT_TIMELINE_MALFORMED_JSON;
        skip_space(&root);
    }
    if (!known_schema) return PROJECT_TIMELINE_OK;
    *out_native_v1 = true;
    if (!have_version) return PROJECT_TIMELINE_OK;
    size_t cursor = version.start;
    if (cursor >= version.end || source.ptr[cursor] < '0' || source.ptr[cursor] > '9')
        return PROJECT_TIMELINE_OK;
    uint64_t value = 0U;
    while (cursor < version.end && source.ptr[cursor] >= '0' && source.ptr[cursor] <= '9') {
        uint64_t digit = (uint64_t)(source.ptr[cursor++] - '0');
        if (value > (UINT32_MAX - digit) / 10U) {
            *out_native_v1 = false;
            return PROJECT_TIMELINE_OK;
        }
        value = value * 10U + digit;
    }
    if (cursor != version.end) return PROJECT_TIMELINE_OK;
    if (value != 1U) *out_native_v1 = false;
    return PROJECT_TIMELINE_OK;
}

