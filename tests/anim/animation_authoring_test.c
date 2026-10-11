#include "svg/animation/authoring.h"
#include "svg/animation/animation.h"
#include "document_edit.h"
#include "viewer_anim_edit.h"
#include "viewer_editor_internal.h"
#include "shape.h"
#include "shape_render.h"

#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define CHECK(expression) do { \
    if (!(expression)) { \
        (void)fprintf(stderr, "%s:%d: check failed: %s\n", \
            __FILE__, __LINE__, #expression); \
        return false; \
    } \
} while (0)

static bool parse_animation(base_bytes source, svg_anim_document *out)
{
    size_t capacity = source.len / 2U + 2U;
    if (capacity > SIZE_MAX / sizeof(svg_xml_token)) return false;
    svg_xml_token *tokens = calloc(capacity, sizeof(*tokens));
    if (tokens == NULL) return false;
    svg_xml_result xml = svg_xml_tokenize(source, svg_xml_default_options(),
        tokens, capacity);
    bool parsed = xml.diagnostic.category == SVG_XML_ERROR_NONE &&
        svg_anim_parse(source, tokens, xml.token_count, out, NULL);
    free(tokens);
    return parsed;
}

static bool apply_patch_copy(base_bytes source, const svg_anim_authoring_patch *patch,
                             uint8_t **out_bytes, size_t *out_length)
{
    if (patch == NULL || out_bytes == NULL || out_length == NULL ||
        patch->offset > source.len ||
        patch->remove_length > source.len - patch->offset) return false;
    size_t length = source.len - patch->remove_length;
    if (!base_size_add(length, patch->replacement_length, &length)) return false;
    uint8_t *output = malloc(length == 0U ? 1U : length);
    if (output == NULL) return false;
    memcpy(output, source.ptr, patch->offset);
    memcpy(output + patch->offset, patch->replacement, patch->replacement_length);
    size_t suffix = source.len - patch->offset - patch->remove_length;
    memcpy(output + patch->offset + patch->replacement_length,
        source.ptr + patch->offset + patch->remove_length, suffix);
    *out_bytes = output;
    *out_length = length;
    return true;
}

static bool apply_and_check_undo(base_bytes source,
                                 const svg_anim_authoring_patch *patch,
                                 uint8_t **out_after, size_t *out_length)
{
    document_edit_result created = document_edit_create(source,
        document_edit_default_limits());
    CHECK(created.error == DOCUMENT_EDIT_OK && created.edit != NULL);
    CHECK(document_edit_replace_source_span(created.edit,
        (document_source_span){patch->offset, patch->remove_length},
        (base_bytes){patch->replacement, patch->replacement_length}) == DOCUMENT_EDIT_OK);
    size_t capacity = source.len + patch->replacement_length + 32U;
    uint8_t *after = malloc(capacity);
    CHECK(after != NULL);
    size_t after_length = 0U;
    CHECK(document_edit_materialize(created.edit, after, capacity, &after_length));
    CHECK(document_edit_undo(created.edit) == DOCUMENT_EDIT_OK);
    size_t restored_length = 0U;
    CHECK(document_edit_materialize(created.edit, after, capacity, &restored_length));
    CHECK(restored_length == source.len &&
        memcmp(after, source.ptr, source.len) == 0);
    CHECK(document_edit_redo(created.edit) == DOCUMENT_EDIT_OK);
    CHECK(document_edit_materialize(created.edit, after, capacity, &restored_length));
    CHECK(restored_length == after_length);
    *out_after = after;
    *out_length = after_length;
    document_edit_destroy(created.edit);
    return true;
}

static bool contains(base_bytes source, const char *text)
{
    size_t length = strlen(text);
    if (length > source.len) return false;
    for (size_t index = 0U; index + length <= source.len; ++index)
        if (memcmp(source.ptr + index, text, length) == 0) return true;
    return false;
}

static double cubic_component(double t, double first, double second)
{
    double inverse = 1.0 - t;
    return 3.0 * inverse * inverse * t * first +
        3.0 * inverse * t * t * second + t * t * t;
}

static double spline_reference(double progress, const double controls[4])
{
    double low = 0.0, high = 1.0;
    for (size_t index = 0U; index < 60U; ++index) {
        double middle = (low + high) / 2.0;
        if (cubic_component(middle, controls[0], controls[2]) < progress)
            low = middle;
        else high = middle;
    }
    return cubic_component((low + high) / 2.0, controls[1], controls[3]);
}

static bool editor_source_equals(viewer_editor *editor, base_bytes expected)
{
    size_t capacity = viewer_editor_byte_length(editor);
    uint8_t *materialized = malloc(capacity == 0U ? 1U : capacity);
    if (materialized == NULL) return false;
    size_t length = 0U;
    bool okay = viewer_editor_materialize(editor, materialized, capacity, &length) &&
        length == expected.len && memcmp(materialized, expected.ptr, length) == 0;
    free(materialized);
    return okay;
}

static bool append_test_text(char *buffer, size_t capacity, size_t *used,
                             const char *text)
{
    size_t length = strlen(text);
    if (buffer == NULL || used == NULL || *used > capacity ||
        length > capacity - *used) return false;
    memcpy(buffer + *used, text, length);
    *used += length;
    return true;
}

static bool test_key_lifecycle_and_byte_preservation(void)
{
    static const char original[] =
        "<svg xmlns:q='urn:quiet' width='24' height='12'><!--untouched-->"
        "<rect id='r' x='0' y='0' width='4' height='4'>"
        "<animate attributeName='x' values='0;10' dur='2s' q:note='keep' note='verbatim'/>"
        "</rect></svg>";
    base_bytes source = {(const uint8_t *)original, sizeof(original) - 1U};
    svg_anim_document animation = {0};
    CHECK(parse_animation(source, &animation));
    CHECK(animation.track_count == 1U);
    CHECK(svg_anim_authoring_track_editable(&animation, 0U));

    svg_anim_authoring_patch patch = {0};
    CHECK(svg_anim_authoring_add_key(&animation, 0U, 1.0,
        (base_bytes){(const uint8_t *)"5", 1U}, &patch) == SVG_ANIM_AUTHORING_OK);
    uint8_t *after = NULL;
    size_t after_length = 0U;
    CHECK(apply_and_check_undo(source, &patch, &after, &after_length));
    base_bytes edited = {after, after_length};
    CHECK(contains(edited, "<!--untouched-->") &&
        contains(edited, "xmlns:q='urn:quiet'") &&
        contains(edited, "q:note='keep' note='verbatim'") &&
        contains(edited, "values='0;5;10'") &&
        contains(edited, "keyTimes=\"0;0.5;1\""));
    svg_anim_document parsed = {0};
    CHECK(parse_animation(edited, &parsed));
    CHECK(parsed.track_count == 1U && parsed.tracks[0].key_count == 3U);
    svg_anim_document_destroy(&parsed);
    free(after);
    svg_anim_authoring_patch_destroy(&patch);
    svg_anim_document_destroy(&animation);

    CHECK(parse_animation(source, &animation));
    CHECK(svg_anim_authoring_set_key_value(&animation, 0U, 1U,
        (base_bytes){(const uint8_t *)"8", 1U}, &patch) == SVG_ANIM_AUTHORING_OK);
    CHECK(apply_patch_copy(source, &patch, &after, &after_length));
    edited = (base_bytes){after, after_length};
    CHECK(contains(edited, "values='0;8'") && contains(edited, "dur='2s'"));
    svg_anim_document_destroy(&parsed);
    CHECK(parse_animation(edited, &parsed));
    svg_anim_overlay overlay = {0};
    CHECK(svg_anim_eval(&parsed, 1.0, &overlay));
    CHECK(overlay.count == 1U && fabs(overlay.items[0].value.number[0] - 4.0) < 1e-9);
    svg_anim_overlay_destroy(&overlay);
    svg_anim_document_destroy(&parsed);
    svg_shape_document rendered = {0};
    svg_shape_diagnostic diagnostic = {0};
    CHECK(svg_shape_decode(edited, &rendered, &diagnostic) == SVG_SHAPE_ERROR_NONE);
    raster_tile first = {0}, second = {0};
    CHECK(raster_shape_render_at(&rendered, 1.0, &first, &diagnostic));
    CHECK(raster_shape_render_at(&rendered, 1.0, &second, &diagnostic));
    CHECK(first.width == 24U && first.height == 12U &&
        first.byte_count == second.byte_count &&
        memcmp(first.rgba, second.rgba, first.byte_count) == 0);
    CHECK(first.rgba[((size_t)2U * first.width + 5U) * 4U + 3U] == 255U &&
        first.rgba[((size_t)2U * first.width + 1U) * 4U + 3U] == 0U);
    raster_tile_destroy(&first);
    raster_tile_destroy(&second);
    svg_shape_document_destroy(&rendered);
    free(after);
    svg_anim_authoring_patch_destroy(&patch);
    svg_anim_document_destroy(&animation);

    static const char move_source_text[] =
        "<svg><rect x='0'><animate attributeName='x' values='0;5;10' "
        "keyTimes='0;0.5;1' dur='2s'/></rect></svg>";
    base_bytes move_source = {(const uint8_t *)move_source_text,
        sizeof(move_source_text) - 1U};
    CHECK(parse_animation(move_source, &animation));
    CHECK(svg_anim_authoring_move_key(&animation, 0U, 1U, 1.5, &patch) ==
        SVG_ANIM_AUTHORING_OK);
    CHECK(apply_patch_copy(move_source, &patch, &after, &after_length));
    CHECK(contains((base_bytes){after, after_length}, "keyTimes='0;0.75;1'"));
    free(after);
    svg_anim_authoring_patch_destroy(&patch);
    svg_anim_document_destroy(&animation);

    CHECK(parse_animation(source, &animation));
    CHECK(svg_anim_authoring_delete_key(&animation, 0U, 1U, &patch) ==
        SVG_ANIM_AUTHORING_OK);
    CHECK(apply_patch_copy(source, &patch, &after, &after_length));
    CHECK(!contains((base_bytes){after, after_length}, "keyTimes=") &&
        contains((base_bytes){after, after_length}, "values='0'"));
    free(after);
    svg_anim_authoring_patch_destroy(&patch);
    svg_anim_document_destroy(&animation);

    CHECK(parse_animation(source, &animation));
    CHECK(svg_anim_authoring_set_key_value(&animation, 0U, 0U,
        (base_bytes){(const uint8_t *)"0", 1U}, &patch) ==
        SVG_ANIM_AUTHORING_NO_OP);
    CHECK(patch.replacement == NULL);
    CHECK(svg_anim_authoring_set_calc_mode(&animation, 0U,
        SVG_ANIM_CALC_LINEAR, &patch) == SVG_ANIM_AUTHORING_NO_OP);
    CHECK(patch.replacement == NULL);
    CHECK(svg_anim_authoring_set_calc_mode(&animation, 0U,
        (svg_anim_calc_mode)-1, &patch) == SVG_ANIM_AUTHORING_UNSUPPORTED);
    CHECK(svg_anim_authoring_move_key(&animation, 0U, SIZE_MAX, 1.0, &patch) ==
        SVG_ANIM_AUTHORING_UNSUPPORTED);
    CHECK(svg_anim_authoring_set_spline(&animation, 0U, SIZE_MAX,
        (double[4]){0.4, 0.0, 0.6, 1.0}, &patch) ==
        SVG_ANIM_AUTHORING_UNSUPPORTED);
    svg_anim_document_destroy(&animation);
    return true;
}

static bool test_track_timing_edit_and_limits(void)
{
    static const char source_text[] =
        "<svg><!--keep--><rect id='r'><animate attributeName='x' "
        "values='0;10' dur='2s' data-extra='unchanged'/></rect></svg>";
    base_bytes source = {(const uint8_t *)source_text, sizeof(source_text) - 1U};
    svg_anim_document animation = {0};
    svg_anim_authoring_patch patch = {0};
    CHECK(parse_animation(source, &animation));
    CHECK(svg_anim_authoring_set_timing(&animation, 0U, 0.5, 3.0, &patch) ==
        SVG_ANIM_AUTHORING_OK);
    uint8_t *after = NULL;
    size_t after_length = 0U;
    CHECK(apply_patch_copy(source, &patch, &after, &after_length));
    base_bytes edited = {after, after_length};
    CHECK(contains(edited, "begin=\"0.5s\"") && contains(edited, "dur='3s'") &&
        contains(edited, "data-extra='unchanged'") && contains(edited, "<!--keep-->"));
    svg_anim_document_destroy(&animation);
    svg_anim_authoring_patch_destroy(&patch);
    CHECK(parse_animation(edited, &animation));
    CHECK(fabs(animation.tracks[0].begin_offset - 0.5) < 1e-12 &&
        fabs(animation.tracks[0].duration_seconds - 3.0) < 1e-12);
    CHECK(svg_anim_authoring_set_timing(&animation, 0U, 0.5, 3.0, &patch) ==
        SVG_ANIM_AUTHORING_NO_OP);
    CHECK(patch.replacement == NULL);
    CHECK(svg_anim_authoring_set_timing(&animation, 0U, -1.0, 3.0, &patch) ==
        SVG_ANIM_AUTHORING_INVALID_ARGUMENT);
    CHECK(svg_anim_authoring_set_timing(&animation, 0U, 1.0, 3600.0, &patch) ==
        SVG_ANIM_AUTHORING_INVALID_ARGUMENT);
    svg_anim_document_destroy(&animation);
    free(after);

    char full_values[8192];
    size_t used = 0U;
    CHECK(append_test_text(full_values, sizeof(full_values), &used,
        "<svg><rect><animate attributeName='x' values='"));
    for (size_t index = 0U; index < SVG_ANIM_MAX_VALUES; ++index) {
        if (index > 0U) CHECK(append_test_text(full_values, sizeof(full_values),
            &used, ";"));
        CHECK(append_test_text(full_values, sizeof(full_values), &used, "0"));
    }
    CHECK(append_test_text(full_values, sizeof(full_values), &used,
        "' dur='2s'/></rect></svg>"));
    CHECK(parse_animation((base_bytes){(const uint8_t *)full_values, used}, &animation));
    CHECK(animation.tracks[0].key_count == SVG_ANIM_MAX_VALUES);
    CHECK(svg_anim_authoring_add_key(&animation, 0U, 1.0,
        (base_bytes){(const uint8_t *)"1", 1U}, &patch) == SVG_ANIM_AUTHORING_LIMIT);
    uint8_t *too_large = malloc(64U * 1024U + 1U);
    CHECK(too_large != NULL);
    memset(too_large, '1', 64U * 1024U + 1U);
    CHECK(svg_anim_authoring_set_key_value(&animation, 0U, 0U,
        (base_bytes){too_large, 64U * 1024U + 1U}, &patch) ==
        SVG_ANIM_AUTHORING_INVALID_ARGUMENT);
    free(too_large);
    svg_anim_document_destroy(&animation);
    return true;
}

static bool test_untouched_numeric_lexemes_survive_key_edits(void)
{
    static const char source_text[] =
        "<svg><rect x='0'><animate attributeName='x' "
        "values='0.00;10.000' dur='2s'/></rect></svg>";
    base_bytes source = {(const uint8_t *)source_text, sizeof(source_text) - 1U};
    svg_anim_document animation = {0};
    CHECK(parse_animation(source, &animation));
    svg_anim_authoring_patch patch = {0};
    CHECK(svg_anim_authoring_add_key(&animation, 0U, 1.0,
        (base_bytes){(const uint8_t *)"5", 1U}, &patch) == SVG_ANIM_AUTHORING_OK);
    uint8_t *after = NULL;
    size_t after_length = 0U;
    CHECK(apply_patch_copy(source, &patch, &after, &after_length));
    CHECK(contains((base_bytes){after, after_length}, "values='0.00;5;10.000'"));
    free(after);
    svg_anim_authoring_patch_destroy(&patch);
    svg_anim_document_destroy(&animation);
    return true;
}

static bool test_editor_animation_commands_undo_and_order(void)
{
    static const char source_text[] =
        "<svg width='24' height='12'><rect id='r' x='0' y='0' width='4' height='4'>"
        "<animate id='first' attributeName='x' values='0;10' dur='2s' data-order='first'/>"
        "<animate id='second' attributeName='x' values='10;20' dur='2s' data-order='second'/>"
        "</rect></svg>";
    base_bytes original = {(const uint8_t *)source_text, sizeof(source_text) - 1U};
    viewer_editor *editor = viewer_editor_create(original, UI_THEME_DARK, 640U, 480U);
    CHECK(editor != NULL);
    svg_anim_document animation = {0};
    CHECK(parse_animation(original, &animation));
    uint32_t flags = viewer_editor_animation_set_key_value(editor, &animation,
        0U, 1U, (base_bytes){(const uint8_t *)"8", 1U});
    CHECK((flags & VIEWER_EDITOR_MUTATED) != 0U);
    uint8_t *key_source = malloc(viewer_editor_byte_length(editor));
    CHECK(key_source != NULL);
    size_t key_length = 0U;
    CHECK(viewer_editor_materialize(editor, key_source,
        viewer_editor_byte_length(editor), &key_length));
    base_bytes key_edited = {key_source, key_length};
    CHECK(contains(key_edited, "values='0;8'"));
    CHECK((viewer_editor_key(editor, 0U, PLATFORM_MODIFIER_COMMAND, "z", 1U) &
        VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(editor_source_equals(editor, original));
    CHECK((viewer_editor_key(editor, 0U,
        PLATFORM_MODIFIER_COMMAND | PLATFORM_MODIFIER_SHIFT, "z", 1U) &
        VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(editor_source_equals(editor, key_edited));
    svg_anim_document_destroy(&animation);
    CHECK(parse_animation(key_edited, &animation));

    flags = viewer_editor_animation_set_timing(editor, &animation, 0U, 0.5, 3.0);
    CHECK((flags & VIEWER_EDITOR_MUTATED) != 0U);
    uint8_t *timed_source = malloc(viewer_editor_byte_length(editor));
    CHECK(timed_source != NULL);
    size_t timed_length = 0U;
    CHECK(viewer_editor_materialize(editor, timed_source,
        viewer_editor_byte_length(editor), &timed_length));
    base_bytes timed = {timed_source, timed_length};
    svg_anim_document_destroy(&animation);
    CHECK(parse_animation(timed, &animation));
    flags = viewer_editor_animation_set_timing(editor, &animation, 0U, 0.5, 3.0);
    CHECK((flags & VIEWER_EDITOR_MUTATED) == 0U);
    CHECK((viewer_editor_key(editor, 0U, PLATFORM_MODIFIER_COMMAND, "z", 1U) &
        VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(editor_source_equals(editor, key_edited));
    CHECK((viewer_editor_key(editor, 0U,
        PLATFORM_MODIFIER_COMMAND | PLATFORM_MODIFIER_SHIFT, "z", 1U) &
        VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(editor_source_equals(editor, timed));
    svg_anim_document_destroy(&animation);
    CHECK(parse_animation(timed, &animation));

    document_handle target;
    document_source_span target_span;
    CHECK(document_edit_find_id(editor->edit,
        (base_bytes){(const uint8_t *)"r", 1U}, &target));
    CHECK(document_edit_source_span(editor->edit, target, &target_span));
    CHECK(viewer_editor_select_at_source_offset(editor, target_span.offset));
    flags = viewer_editor_animation_add_track(editor, &animation,
        SVG_ANIM_ATTR_Y, SVG_ANIM_KIND_ANIMATE, SVG_ANIM_TRANSFORM_TRANSLATE,
        (base_bytes){(const uint8_t *)"0", 1U},
        (base_bytes){(const uint8_t *)"5", 1U}, 1.0, 2.0);
    CHECK((flags & VIEWER_EDITOR_MUTATED) != 0U);
    uint8_t *with_track = malloc(viewer_editor_byte_length(editor));
    CHECK(with_track != NULL);
    size_t with_track_length = 0U;
    CHECK(viewer_editor_materialize(editor, with_track,
        viewer_editor_byte_length(editor), &with_track_length));
    base_bytes added = {with_track, with_track_length};
    svg_anim_document_destroy(&animation);
    CHECK(parse_animation(added, &animation));
    CHECK(animation.track_count == 3U &&
        animation.tracks[0].attribute == SVG_ANIM_ATTR_X &&
        animation.tracks[1].attribute == SVG_ANIM_ATTR_X &&
        animation.tracks[2].attribute == SVG_ANIM_ATTR_Y &&
        animation.tracks[0].element_offset < animation.tracks[1].element_offset &&
        animation.tracks[1].element_offset < animation.tracks[2].element_offset &&
        memcmp(added.ptr + animation.tracks[0].id.offset, "first", 5U) == 0 &&
        memcmp(added.ptr + animation.tracks[1].id.offset, "second", 6U) == 0);
    svg_anim_document_destroy(&animation);
    CHECK((viewer_editor_key(editor, 0U, PLATFORM_MODIFIER_COMMAND, "z", 1U) &
        VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(editor_source_equals(editor, timed));
    CHECK((viewer_editor_key(editor, 0U,
        PLATFORM_MODIFIER_COMMAND | PLATFORM_MODIFIER_SHIFT, "z", 1U) &
        VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(editor_source_equals(editor, added));
    CHECK(parse_animation(added, &animation));
    flags = viewer_editor_animation_remove_track(editor, &animation, 2U);
    CHECK((flags & VIEWER_EDITOR_MUTATED) != 0U);
    svg_anim_document_destroy(&animation);
    CHECK((viewer_editor_key(editor, 0U, PLATFORM_MODIFIER_COMMAND, "z", 1U) &
        VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(editor_source_equals(editor, added));
    svg_anim_document_destroy(&animation);
    viewer_editor_destroy(editor);
    free(with_track);
    free(timed_source);
    free(key_source);
    return true;
}

static bool test_color_transform_and_track_creation(void)
{
    static const char color_source[] =
        "<svg><rect fill='red'><animate attributeName='fill' values='red;blue' dur='1s'/></rect></svg>";
    base_bytes color_bytes = {(const uint8_t *)color_source, sizeof(color_source) - 1U};
    svg_anim_document animation = {0};
    CHECK(parse_animation(color_bytes, &animation));
    svg_anim_authoring_patch patch = {0};
    CHECK(svg_anim_authoring_set_key_value(&animation, 0U, 1U,
        (base_bytes){(const uint8_t *)"#00FF00", 7U}, &patch) == SVG_ANIM_AUTHORING_OK);
    uint8_t *after = NULL;
    size_t after_length = 0U;
    CHECK(apply_patch_copy(color_bytes, &patch, &after, &after_length));
    CHECK(contains((base_bytes){after, after_length}, "values='red;#00FF00'"));
    free(after);
    svg_anim_authoring_patch_destroy(&patch);
    svg_anim_document_destroy(&animation);

    static const char transform_source[] =
        "<svg><g><animateTransform attributeName='transform' type='translate' values='0 0;8 4' dur='1s'/></g></svg>";
    base_bytes transform_bytes = {(const uint8_t *)transform_source,
        sizeof(transform_source) - 1U};
    CHECK(parse_animation(transform_bytes, &animation));
    CHECK(svg_anim_authoring_set_key_value(&animation, 0U, 1U,
        (base_bytes){(const uint8_t *)"4 2", 3U}, &patch) == SVG_ANIM_AUTHORING_OK);
    CHECK(apply_patch_copy(transform_bytes, &patch, &after, &after_length));
    CHECK(contains((base_bytes){after, after_length}, "values='0 0;4 2'"));
    free(after);
    svg_anim_authoring_patch_destroy(&patch);
    svg_anim_document_destroy(&animation);

    static const char target[] = "<svg><rect id='target' x='1'/></svg>";
    uint8_t *fragment = NULL;
    size_t fragment_length = 0U;
    CHECK(svg_anim_authoring_create_track(
        (base_bytes){(const uint8_t *)target, sizeof(target) - 1U},
        10U, SVG_ANIM_ATTR_X, SVG_ANIM_KIND_ANIMATE,
        SVG_ANIM_TRANSFORM_TRANSLATE,
        (base_bytes){(const uint8_t *)"1", 1U},
        (base_bytes){(const uint8_t *)"9", 1U}, 0.5, 2.0,
        &fragment, &fragment_length) == SVG_ANIM_AUTHORING_OK);
    CHECK(contains((base_bytes){fragment, fragment_length}, "begin=\"0s\"") &&
        contains((base_bytes){fragment, fragment_length}, "values=\"1;9;9\"") &&
        contains((base_bytes){fragment, fragment_length}, "dur=\"2s\""));
    document_edit_result edit = document_edit_create(
        (base_bytes){(const uint8_t *)target, sizeof(target) - 1U},
        document_edit_default_limits());
    CHECK(edit.error == DOCUMENT_EDIT_OK && edit.edit != NULL);
    document_handle rect;
    CHECK(document_edit_find_id(edit.edit,
        (base_bytes){(const uint8_t *)"target", 6U}, &rect));
    CHECK(document_edit_insert_xml_fragment(edit.edit, rect,
        (base_bytes){fragment, fragment_length}, &rect) == DOCUMENT_EDIT_OK);
    size_t capacity = sizeof(target) + fragment_length + 64U;
    uint8_t *materialized = malloc(capacity);
    CHECK(materialized != NULL);
    size_t length = 0U;
    CHECK(document_edit_materialize(edit.edit, materialized, capacity, &length));
    CHECK(parse_animation((base_bytes){materialized, length}, &animation));
    CHECK(animation.track_count == 1U && animation.tracks[0].key_count == 3U);
    svg_anim_document_destroy(&animation);
    free(materialized);
    free(fragment);
    document_edit_destroy(edit.edit);
    return true;
}

static bool test_spline_isolation_modes_and_rejection(void)
{
    static const char source_text[] =
        "<svg><rect><animate attributeName='x' values='0;1;2;3' dur='1s' "
        "calcMode='spline' keySplines='0.10 0.20 0.30 0.40;  0.22 0.32 0.42 0.52 ;0.40 0.50 0.60 0.70'/></rect></svg>";
    base_bytes source = {(const uint8_t *)source_text, sizeof(source_text) - 1U};
    svg_anim_document animation = {0};
    CHECK(parse_animation(source, &animation));
    CHECK(animation.tracks[0].spline_count == 3U);
    double controls[4] = {0.91, 0.11, 0.72, 0.82};
    svg_anim_authoring_patch patch = {0};
    CHECK(svg_anim_authoring_set_spline(&animation, 0U, 1U, controls, &patch) ==
        SVG_ANIM_AUTHORING_OK);
    uint8_t *after = NULL;
    size_t after_length = 0U;
    CHECK(apply_patch_copy(source, &patch, &after, &after_length));
    base_bytes edited = {after, after_length};
    CHECK(contains(edited, "keySplines='0.10 0.20 0.30 0.40;") &&
        contains(edited, ";0.40 0.50 0.60 0.70'"));
    CHECK(parse_animation(edited, &animation));
    CHECK(animation.tracks[0].calc_mode == SVG_ANIM_CALC_SPLINE);
    CHECK(fabs(animation.tracks[0].splines[1][0] - controls[0]) < 1e-12 &&
        fabs(animation.tracks[0].splines[1][2] - controls[2]) < 1e-12);
    svg_anim_overlay overlay = {0};
    CHECK(svg_anim_eval(&animation, 0.5, &overlay));
    CHECK(overlay.count == 1U && fabs(overlay.items[0].value.number[0] -
        (1.0 + spline_reference(0.5, controls))) < 1e-8);
    svg_anim_overlay_destroy(&overlay);

    svg_anim_authoring_patch mode_patch = {0};
    CHECK(svg_anim_authoring_set_calc_mode(&animation, 0U,
        SVG_ANIM_CALC_LINEAR, &mode_patch) == SVG_ANIM_AUTHORING_OK);
    uint8_t *linear_bytes = NULL;
    size_t linear_length = 0U;
    CHECK(apply_patch_copy(edited, &mode_patch, &linear_bytes, &linear_length));
    svg_anim_authoring_patch_destroy(&mode_patch);
    svg_anim_document_destroy(&animation);
    CHECK(parse_animation((base_bytes){linear_bytes, linear_length}, &animation));
    CHECK(svg_anim_eval(&animation, 0.2, &overlay));
    CHECK(overlay.count == 1U && fabs(overlay.items[0].value.number[0] - 0.6) < 1e-9);
    svg_anim_overlay_destroy(&overlay);
    CHECK(svg_anim_authoring_set_calc_mode(&animation, 0U,
        SVG_ANIM_CALC_DISCRETE, &mode_patch) == SVG_ANIM_AUTHORING_OK);
    uint8_t *discrete_bytes = NULL;
    size_t discrete_length = 0U;
    CHECK(apply_patch_copy((base_bytes){linear_bytes, linear_length}, &mode_patch,
        &discrete_bytes, &discrete_length));
    svg_anim_authoring_patch_destroy(&mode_patch);
    svg_anim_document_destroy(&animation);
    CHECK(parse_animation((base_bytes){discrete_bytes, discrete_length}, &animation));
    CHECK(svg_anim_eval(&animation, 0.2, &overlay));
    CHECK(overlay.count == 1U && overlay.items[0].value.number[0] == 0.0);
    svg_anim_overlay_destroy(&overlay);
    svg_anim_document_destroy(&animation);
    free(discrete_bytes);
    free(linear_bytes);
    free(after);
    svg_anim_authoring_patch_destroy(&patch);

    CHECK(parse_animation(source, &animation));
    CHECK(svg_anim_authoring_set_spline(&animation, 0U, 1U,
        (double[4]){1.1, 0.0, 0.5, 1.0}, &patch) ==
        SVG_ANIM_AUTHORING_INVALID_ARGUMENT);
    CHECK(svg_anim_authoring_set_calc_mode(&animation, 0U,
        SVG_ANIM_CALC_LINEAR, &patch) == SVG_ANIM_AUTHORING_OK);
    CHECK(apply_patch_copy(source, &patch, &after, &after_length));
    CHECK(!contains((base_bytes){after, after_length}, "keySplines=") &&
        contains((base_bytes){after, after_length}, "calcMode='linear'"));
    free(after);
    svg_anim_authoring_patch_destroy(&patch);
    svg_anim_document_destroy(&animation);

    static const char paced_text[] =
        "<svg><rect><animate attributeName='x' values='0;1' dur='1s' calcMode='paced'/></rect></svg>";
    CHECK(parse_animation((base_bytes){(const uint8_t *)paced_text,
        sizeof(paced_text) - 1U}, &animation));
    CHECK(!svg_anim_authoring_track_editable(&animation, 0U));
    CHECK(svg_anim_authoring_set_spline(&animation, 0U, 0U,
        (double[4]){0.4, 0.0, 0.6, 1.0}, &patch) ==
        SVG_ANIM_AUTHORING_UNSUPPORTED);
    svg_anim_document_destroy(&animation);
    return true;
}

int main(void)
{
    if (!test_key_lifecycle_and_byte_preservation() ||
        !test_track_timing_edit_and_limits() ||
        !test_untouched_numeric_lexemes_survive_key_edits() ||
        !test_editor_animation_commands_undo_and_order() ||
        !test_color_transform_and_track_creation() ||
        !test_spline_isolation_modes_and_rejection()) return EXIT_FAILURE;
    (void)puts("animation authoring: all checks passed");
    return EXIT_SUCCESS;
}
