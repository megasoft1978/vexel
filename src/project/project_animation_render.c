#include "project_animation_render.h"
#include "project_timeline_json.h"
#include "document/document_edit.h"

#include <locale.h>
#include <math.h>
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef struct property_meta { base_bytes property, type, interpolation; } property_meta;
static locale_t render_c_locale;
static pthread_once_t render_c_locale_once = PTHREAD_ONCE_INIT;

static void render_locale_init(void)
{
    render_c_locale = newlocale(LC_NUMERIC_MASK, "C", (locale_t)0);
}

static bool read_property_meta(const project_timeline_view *timeline,
    size_t index, project_timeline_track_view *track, property_meta *meta)
{
    if (!project_timeline_track_at(timeline, index, track)) return false;
    size_t start = (size_t)(track->payload_json.ptr - timeline->source.ptr);
    json_reader reader = {.source = timeline->source,
        .error_kind = PROJECT_TIMELINE_OK};
    object_fields fields;
    if (!parse_object_fields(&reader,
            (json_span){start, start + track->payload_json.len}, &fields)) return false;
    const json_field *p = get_field(&fields, "property");
    const json_field *v = get_field(&fields, "value_type");
    const json_field *x = get_field(&fields, "interpolation");
    return p != NULL && v != NULL && x != NULL &&
        span_string(&reader, p->value, &meta->property) &&
        span_string(&reader, v->value, &meta->type) &&
        span_string(&reader, x->value, &meta->interpolation);
}

static bool key_components(const project_timeline_view *timeline,
    const project_timeline_item_view *item, size_t count, double *values)
{
    size_t start = (size_t)(item->value_json.ptr - timeline->source.ptr);
    json_reader reader = {.source = timeline->source,
        .error_kind = PROJECT_TIMELINE_OK};
    json_span span = {start, start + item->value_json.len};
    if (count == 1U) {
        bool boolean_value;
        if (span_bool(&reader, span, &boolean_value)) {
            values[0] = boolean_value ? 1.0 : 0.0; return true;
        }
        return span_number_value(&reader, span, &values[0]);
    }
    json_span components[6]; size_t actual = 0U;
    if (!span_array(&reader, span, components, 6U, &actual) || actual != count)
        return false;
    for (size_t i = 0U; i < count; ++i)
        if (!span_number_value(&reader, components[i], &values[i])) return false;
    return true;
}

static bool evaluate_track(const project_timeline_view *timeline, size_t index,
    const property_meta *meta, uint32_t tick, double values[6], size_t *count)
{
    *count = json_string_equal(meta->type, "transform2d") ? 6U :
        json_string_equal(meta->type, "rgba") ? 4U : 1U;
    project_timeline_iterator iterator = {0};
    project_timeline_item_view item = {0}, previous = {0}, next = {0};
    bool have_previous = false, have_next = false;
    while (project_timeline_item_next(timeline, index, &iterator, &item)) {
        if (item.tick <= tick) { previous = item; have_previous = true; }
        else { next = item; have_next = true; break; }
    }
    if (!have_previous && !have_next) return false;
    if (!have_previous) {
        if (!key_components(timeline, &item, *count, values)) return false;
        return true;
    }
    double left[6] = {0}, right[6] = {0};
    if (!key_components(timeline, &previous, *count, left)) return false;
    if (!have_next || json_string_equal(meta->interpolation, "step")) {
        memcpy(values, left, *count * sizeof(double)); return true;
    }
    if (!key_components(timeline, &next, *count, right)) return false;
    double t = (double)(tick - previous.tick) / (double)(next.tick - previous.tick);
    if (json_string_equal(meta->interpolation, "cubic")) t = t * t * (3.0 - 2.0 * t);
    for (size_t i = 0U; i < *count; ++i) values[i] = left[i] + (right[i] - left[i]) * t;
    return true;
}

static bool append_number(char *buffer, size_t capacity, size_t *used, double value)
{
    if (pthread_once(&render_c_locale_once, render_locale_init) != 0) return false;
    if (render_c_locale == (locale_t)0 || *used >= capacity) return false;
    locale_t prior = uselocale(render_c_locale);
    int n = snprintf(buffer + *used, capacity - *used, "%.17g", value);
    (void)uselocale(prior);
    if (n < 0 || (size_t)n >= capacity - *used) return false;
    *used += (size_t)n; return true;
}

static bool set_target_attribute(document_edit *edit, base_bytes target,
    base_bytes name, const char *value, size_t length)
{
    uint8_t id[256]; size_t id_length = 0U; document_handle node;
    if (!project_timeline_string_copy(target, (base_mut_bytes){id, sizeof(id)}, &id_length) ||
        id_length == 0U || !document_edit_find_id(edit, (base_bytes){id, id_length}, &node)) return false;
    return document_edit_set_attribute(edit, node, name,
        (base_bytes){(const uint8_t *)value, length}) == DOCUMENT_EDIT_OK;
}

static bool apply_property(document_edit *edit, base_bytes target,
    const property_meta *meta, double values[6], size_t count)
{
    char text[192]; size_t used = 0U; base_bytes name = meta->property;
    if (json_string_equal(meta->type, "boolean")) {
        const char *v = values[0] != 0.0 ? "visible" : "hidden";
        return set_target_attribute(edit, target, name, v, strlen(v));
    }
    if (json_string_equal(meta->type, "rgba")) {
        uint32_t c[3];
        for (size_t i = 0U; i < 3U; ++i) { c[i] = (uint32_t)(values[i] * 255.0 + 0.5); if (c[i] > 255U) c[i] = 255U; }
        int n = snprintf(text, sizeof(text), "#%02x%02x%02x", c[0], c[1], c[2]);
        if (n != 7 || !set_target_attribute(edit, target, name, text, 7U)) return false;
        name = json_string_equal(meta->property, "fill") ?
            (base_bytes){(const uint8_t *)"fill-opacity", 12U} :
            (base_bytes){(const uint8_t *)"stroke-opacity", 14U};
        used = 0U;
        return append_number(text, sizeof(text), &used, values[3]) &&
            set_target_attribute(edit, target, name, text, used);
    }
    if (json_string_equal(meta->type, "transform2d")) {
        memcpy(text, "matrix(", 7U); used = 7U;
        for (size_t i = 0U; i < count; ++i) {
            if (i != 0U) text[used++] = ' ';
            if (!append_number(text, sizeof(text), &used, values[i])) return false;
        }
        text[used++] = ')';
    } else if (!append_number(text, sizeof(text), &used, values[0])) return false;
    return set_target_attribute(edit, target, name, text, used);
}

static bool make_property_scene(const svg_shape_document *scene,
    const project_timeline_view *timeline, uint32_t tick,
    svg_shape_document *out, document_edit **out_edit, uint8_t **out_bytes)
{
    *out_edit = NULL; *out_bytes = NULL;
    bool found = false;
    for (size_t i = 0U; i < timeline->track_count; ++i) if (!timeline->drawing_tracks[i]) { found = true; break; }
    if (!found) return true;
    document_edit_result created = document_edit_create(scene->source, document_edit_default_limits());
    if (created.error != DOCUMENT_EDIT_OK || created.edit == NULL) return false;
    *out_edit = created.edit;
    for (size_t i = 0U; i < timeline->track_count; ++i) {
        if (timeline->drawing_tracks[i]) continue;
        project_timeline_track_view track; property_meta meta; double values[6] = {0}; size_t count = 0U;
        if (!read_property_meta(timeline, i, &track, &meta)) return false;
        if (!track.visible) continue;
        if (
            !evaluate_track(timeline, i, &meta, tick, values, &count) ||
            !apply_property(*out_edit, track.target, &meta, values, count)) return false;
    }
    size_t length = document_edit_byte_length(*out_edit);
    if (length == 0U || length > 64U * 1024U * 1024U) return false;
    *out_bytes = malloc(length);
    if (*out_bytes == NULL || !document_edit_materialize(*out_edit, *out_bytes, length, &length)) return false;
    svg_shape_diagnostic diagnostic = {0};
    return svg_shape_decode((base_bytes){*out_bytes, length}, out, &diagnostic) == SVG_SHAPE_ERROR_NONE;
}

static project_animation_render_result render_result(
    project_animation_render_error error, size_t track_index,
    size_t source_offset)
{
    return (project_animation_render_result){
        .error = error,
        .track_index = track_index,
        .source_offset = source_offset
    };
}

static bool bundle_asset_find(const project_bundle_view *bundle,
                              base_bytes name,
                              project_bundle_asset *out_asset)
{
    for (size_t index = 0U; index < bundle->asset_count; ++index) {
        if (!project_bundle_asset_at(bundle, index, out_asset)) return false;
        if (bytes_compare(out_asset->name, name) == 0) return true;
    }
    return false;
}

static bool active_exposure(const project_timeline_view *timeline,
                            size_t track_index, uint32_t tick,
                            project_timeline_item_view *out_item)
{
    project_timeline_iterator iterator = {0};
    project_timeline_item_view item, selected = {0};
    bool found = false;
    while (project_timeline_item_next(timeline, track_index, &iterator, &item)) {
        if (item.tick > tick) break;
        selected = item;
        found = true;
    }
    if (found) *out_item = selected;
    return found;
}

static bool source_over(raster_tile *destination, const raster_tile *source)
{
    if (destination->width != source->width ||
        destination->height != source->height ||
        destination->byte_count != source->byte_count ||
        destination->rgba == NULL || source->rgba == NULL) return false;
    for (size_t offset = 0U; offset < destination->byte_count; offset += 4U) {
        uint32_t source_alpha = source->rgba[offset + 3U];
        uint32_t destination_alpha = destination->rgba[offset + 3U];
        uint32_t inverse = 255U - source_alpha;
        for (size_t channel = 0U; channel < 3U; ++channel) {
            uint32_t source_value =
                ((uint32_t)source->rgba[offset + channel] * source_alpha + 127U) /
                255U;
            uint32_t destination_value = destination->rgba[offset + channel];
            uint32_t value = source_value +
                (destination_value * inverse + 127U) / 255U;
            destination->rgba[offset + channel] =
                (uint8_t)(value > 255U ? 255U : value);
        }
        uint32_t alpha = source_alpha +
            (destination_alpha * inverse + 127U) / 255U;
        destination->rgba[offset + 3U] = (uint8_t)(alpha > 255U ? 255U : alpha);
    }
    return true;
}

static project_animation_render_result render_frame_composed(
    const svg_shape_document *scene, const project_bundle_view *bundle,
    const project_timeline_view *timeline, uint32_t tick, raster_tile *out_tile)
{
    if (out_tile == NULL)
        return render_result(PROJECT_ANIMATION_RENDER_INVALID_ARGUMENT, 0U, 0U);
    if (out_tile->rgba != NULL || out_tile->width != 0U ||
        out_tile->height != 0U || out_tile->byte_count != 0U)
        return render_result(PROJECT_ANIMATION_RENDER_INVALID_ARGUMENT, 0U, 0U);
    if (scene == NULL || bundle == NULL || timeline == NULL ||
        scene->width == 0U || scene->height == 0U ||
        scene->width > RASTER_TILE_MAX_AXIS ||
        scene->height > RASTER_TILE_MAX_AXIS ||
        !base_bytes_valid(timeline->source) ||
        (bundle->asset_count != 0U && !base_bytes_valid(bundle->encoded)))
        return render_result(PROJECT_ANIMATION_RENDER_INVALID_ARGUMENT, 0U, 0U);
    if (timeline->empty || timeline->duration_ticks == 0U ||
        tick >= timeline->duration_ticks)
        return render_result(PROJECT_ANIMATION_RENDER_INVALID_TIMELINE, 0U, 0U);

    svg_shape_diagnostic diagnostic = {SVG_SHAPE_ERROR_NONE, 0U};
    if (!raster_shape_render(scene, out_tile, &diagnostic)) {
        project_animation_render_result failure =
            render_result(PROJECT_ANIMATION_RENDER_RASTER_FAILED, 0U,
                          diagnostic.offset);
        failure.diagnostic = diagnostic;
        return failure;
    }
    size_t composite_bytes = out_tile->byte_count;
    if (composite_bytes > PROJECT_ANIMATION_RENDER_MAX_COMPOSITE_BYTES) {
        raster_tile_destroy(out_tile);
        return render_result(PROJECT_ANIMATION_RENDER_LIMIT, 0U, 0U);
    }
    for (size_t offset = 0U; offset < out_tile->byte_count; offset += 4U) {
        uint32_t alpha = out_tile->rgba[offset + 3U];
        for (size_t channel = 0U; channel < 3U; ++channel)
            out_tile->rgba[offset + channel] = (uint8_t)(
                ((uint32_t)out_tile->rgba[offset + channel] * alpha + 127U) /
                255U);
    }

    for (size_t track_index = 0U; track_index < timeline->track_count;
         ++track_index) {
        if (!timeline->drawing_tracks[track_index]) continue;
        project_timeline_track_view track;
        if (!project_timeline_track_at(timeline, track_index, &track)) {
            raster_tile_destroy(out_tile);
            return render_result(PROJECT_ANIMATION_RENDER_INVALID_TIMELINE,
                                 track_index, 0U);
        }
        if (!track.visible) continue;
        project_timeline_item_view exposure;
        if (!active_exposure(timeline, track_index, tick, &exposure)) continue;

        uint8_t name_bytes[PROJECT_BUNDLE_MAX_ASSET_NAME_BYTES];
        size_t name_length = 0U;
        if (!project_timeline_string_copy(exposure.asset,
                (base_mut_bytes){name_bytes, sizeof(name_bytes)}, &name_length) ||
            name_length == 0U) {
            raster_tile_destroy(out_tile);
            return render_result(PROJECT_ANIMATION_RENDER_INVALID_REFERENCE,
                                 track_index, exposure.source_offset);
        }
        base_bytes name = {name_bytes, name_length};
        project_bundle_asset asset = {0};
        if (!bundle_asset_find(bundle, name, &asset)) {
            raster_tile_destroy(out_tile);
            return render_result(PROJECT_ANIMATION_RENDER_INVALID_REFERENCE,
                                 track_index, exposure.source_offset);
        }
        static const uint8_t svg_type[] = "image/svg+xml";
        if (asset.media_type.len != sizeof(svg_type) - 1U ||
            memcmp(asset.media_type.ptr, svg_type, sizeof(svg_type) - 1U) != 0) {
            raster_tile_destroy(out_tile);
            return render_result(PROJECT_ANIMATION_RENDER_UNSUPPORTED_MEDIA,
                                 track_index, exposure.source_offset);
        }

        svg_shape_document drawing = {0};
        diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_NONE, 0U};
        svg_shape_error decoded = svg_shape_decode(asset.data, &drawing,
                                                   &diagnostic);
        if (decoded != SVG_SHAPE_ERROR_NONE) {
            raster_tile_destroy(out_tile);
            project_animation_render_result failure =
                render_result(PROJECT_ANIMATION_RENDER_DECODE_FAILED,
                              track_index, diagnostic.offset);
            failure.diagnostic = diagnostic;
            return failure;
        }
        if (drawing.width != scene->width || drawing.height != scene->height ||
            drawing.view_x != scene->view_x || drawing.view_y != scene->view_y ||
            drawing.view_width != scene->view_width ||
            drawing.view_height != scene->view_height) {
            svg_shape_document_destroy(&drawing);
            raster_tile_destroy(out_tile);
            return render_result(PROJECT_ANIMATION_RENDER_DIMENSION_MISMATCH,
                                 track_index, exposure.source_offset);
        }
        if (out_tile->byte_count >
                PROJECT_ANIMATION_RENDER_MAX_COMPOSITE_BYTES - composite_bytes) {
            svg_shape_document_destroy(&drawing);
            raster_tile_destroy(out_tile);
            return render_result(PROJECT_ANIMATION_RENDER_LIMIT, track_index,
                                 exposure.source_offset);
        }

        raster_tile layer = {0};
        diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_NONE, 0U};
        bool rendered = raster_shape_render(&drawing, &layer, &diagnostic);
        svg_shape_document_destroy(&drawing);
        if (!rendered) {
            raster_tile_destroy(&layer);
            raster_tile_destroy(out_tile);
            project_animation_render_result failure =
                render_result(PROJECT_ANIMATION_RENDER_RASTER_FAILED,
                              track_index, diagnostic.offset);
            failure.diagnostic = diagnostic;
            return failure;
        }
        composite_bytes += layer.byte_count;
        bool composited = source_over(out_tile, &layer);
        raster_tile_destroy(&layer);
        if (!composited) {
            raster_tile_destroy(out_tile);
            return render_result(PROJECT_ANIMATION_RENDER_LIMIT,
                                 track_index, exposure.source_offset);
        }
    }
    return render_result(PROJECT_ANIMATION_RENDER_OK, 0U, 0U);
}

project_animation_render_result project_animation_render_frame(
    const svg_shape_document *scene, const project_bundle_view *bundle,
    const project_timeline_view *timeline, uint32_t tick, raster_tile *out_tile)
{
    if (scene == NULL || timeline == NULL)
        return render_result(PROJECT_ANIMATION_RENDER_INVALID_ARGUMENT, 0U, 0U);
    bool has_properties = false;
    for (size_t i = 0U; i < timeline->track_count; ++i)
        if (!timeline->drawing_tracks[i]) { has_properties = true; break; }
    if (!has_properties)
        return render_frame_composed(scene, bundle, timeline, tick, out_tile);
    svg_shape_document evaluated = {0};
    document_edit *edit = NULL;
    uint8_t *source = NULL;
    if (!make_property_scene(scene, timeline, tick, &evaluated, &edit, &source)) {
        svg_shape_document_destroy(&evaluated);
        document_edit_destroy(edit);
        free(source);
        return render_result(PROJECT_ANIMATION_RENDER_INVALID_TIMELINE, 0U, 0U);
    }
    project_animation_render_result result = render_frame_composed(
        &evaluated, bundle, timeline, tick, out_tile);
    svg_shape_document_destroy(&evaluated);
    document_edit_destroy(edit);
    free(source);
    return result;
}

const char *project_animation_render_error_name(
    project_animation_render_error error)
{
    switch (error) {
        case PROJECT_ANIMATION_RENDER_OK: return "ok";
        case PROJECT_ANIMATION_RENDER_INVALID_ARGUMENT: return "invalid_argument";
        case PROJECT_ANIMATION_RENDER_INVALID_TIMELINE: return "invalid_timeline";
        case PROJECT_ANIMATION_RENDER_INVALID_REFERENCE: return "invalid_reference";
        case PROJECT_ANIMATION_RENDER_UNSUPPORTED_MEDIA: return "unsupported_media";
        case PROJECT_ANIMATION_RENDER_DIMENSION_MISMATCH: return "dimension_mismatch";
        case PROJECT_ANIMATION_RENDER_DECODE_FAILED: return "decode_failed";
        case PROJECT_ANIMATION_RENDER_RASTER_FAILED: return "raster_failed";
        case PROJECT_ANIMATION_RENDER_OUT_OF_MEMORY: return "out_of_memory";
        case PROJECT_ANIMATION_RENDER_LIMIT: return "limit";
    }
    return "unknown";
}
