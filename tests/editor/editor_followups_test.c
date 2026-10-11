#include "viewer_editor.h"
#include "viewer_editor_internal.h"

#include "svg/shape/shape.h"

#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

#define CHECK(condition) do { if (!(condition)) { \
    (void)fprintf(stderr, "FAIL %s:%d: %s\n", __FILE__, __LINE__, #condition); \
    goto done; } } while (0)
#define FOLLOWUP_EDGE_COMPARISON_CAP 512U

static const uint8_t cubic_source[] =
    "<svg xmlns='http://www.w3.org/2000/svg' width='64' height='64' "
    "viewBox='0 0 64 64'><path id='curve' d='M 8 40 C 18 0 28 80 38 40' "
    "fill='none' stroke='#172033' stroke-width='1'/></svg>";

static const uint8_t quadratic_source[] =
    "<svg xmlns='http://www.w3.org/2000/svg' width='64' height='64' "
    "viewBox='0 0 64 64'><path id='curve' d='M 8 40 Q 23 10 38 40' "
    "fill='none' stroke='#172033' stroke-width='1'/></svg>";

static const uint8_t arc_source[] =
    "<svg xmlns='http://www.w3.org/2000/svg' width='64' height='64' "
    "viewBox='0 0 64 64'><path id='curve' d='M 8 40 A 20 20 0 0 1 38 40' "
    "fill='none' stroke='#172033' stroke-width='1'/></svg>";

static const uint8_t unsupported_path_source[] =
    "<svg xmlns='http://www.w3.org/2000/svg' width='64' height='64' "
    "viewBox='0 0 64 64'><path id='curve' d='M 8 40 R 12 30' "
    "fill='none' stroke='#172033'/></svg>";

static const uint8_t node_command_source[] =
    "<svg xmlns='http://www.w3.org/2000/svg' width='64' height='64' "
    "viewBox='0 0 64 64'><path id='curve' "
    "d='M 4 8 L 10 8 C 12 6 14 10 16 8 S 22 6 24 8 "
    "Q 26 6 28 8 T 32 8 A 4 4 0 0 1 40 8' "
    "fill='none' stroke='#172033'/></svg>";

static const uint8_t smooth_node_source[] =
    "<svg xmlns='http://www.w3.org/2000/svg' width='64' height='64' "
    "viewBox='0 0 64 64'><path id='curve' "
    "d='M 4 10 C 8 10 12 10 16 10 C 18 15 20 15 22 12' "
    "fill='none' stroke='#172033'/></svg>";

static const uint8_t multi_source[] =
    "<svg xmlns='http://www.w3.org/2000/svg' width='64' height='64' "
    "viewBox='0 0 64 64'><rect id='first' x='8' y='8' width='16' height='12' "
    "fill='#4f7cff'/><!-- keep this comment --><ellipse id='second' cx='42' "
    "cy='42' rx='8' ry='8' fill='#ef5261'/></svg>";

static const uint8_t distribution_source[] =
    "<svg xmlns='http://www.w3.org/2000/svg' width='64' height='64' "
    "viewBox='0 0 64 64'><rect id='a' x='4' y='8' width='4' height='4'/>"
    "<rect id='b' x='24' y='8' width='4' height='4'/>"
    "<rect id='c' x='54' y='8' width='4' height='4'/></svg>";

static const uint8_t groups_source[] =
    "<svg xmlns='http://www.w3.org/2000/svg' width='64' height='64' "
    "viewBox='0 0 64 64'><g id='first-group'><rect id='a' x='2' y='2' "
    "width='4' height='4'/><!-- inside --></g><!-- between groups -->"
    "<g id='second-group'>\n<ellipse id='b' cx='20' cy='20' rx='4' ry='4'/>"
    "</g></svg>";

static const uint8_t ungrouped_source[] =
    "<svg xmlns='http://www.w3.org/2000/svg' width='64' height='64' "
    "viewBox='0 0 64 64'><rect id='a' x='2' y='2' width='4' height='4'/>"
    "<!-- inside --><!-- between groups -->\n"
    "<ellipse id='b' cx='20' cy='20' rx='4' ry='4'/></svg>";

static bool decode(const uint8_t *bytes, size_t length,
                   svg_shape_document *out_document)
{
    svg_shape_diagnostic diagnostic = {0};
    return svg_shape_decode((base_bytes){bytes, length}, out_document,
        &diagnostic) == SVG_SHAPE_ERROR_NONE;
}

static double point_segment_distance(geom_point point, geom_edge edge)
{
    double dx = edge.b.x - edge.a.x;
    double dy = edge.b.y - edge.a.y;
    double length_squared = dx * dx + dy * dy;
    double projection = length_squared > 0.0 ?
        ((point.x - edge.a.x) * dx + (point.y - edge.a.y) * dy) /
            length_squared : 0.0;
    if (projection < 0.0) projection = 0.0;
    if (projection > 1.0) projection = 1.0;
    return hypot(point.x - (edge.a.x + projection * dx),
        point.y - (edge.a.y + projection * dy));
}

static double point_to_document_edges(geom_point point,
    const svg_shape_document *document)
{
    double nearest = INFINITY;
    for (size_t index = 0U; index < document->edge_count; ++index) {
        double distance = point_segment_distance(point, document->edges[index]);
        if (distance < nearest) nearest = distance;
    }
    return nearest;
}

static bool document_edges_near(const svg_shape_document *left,
    const svg_shape_document *right, double tolerance)
{
    if (left == NULL || right == NULL || left->edges == NULL ||
        right->edges == NULL || left->edge_count == 0U ||
        right->edge_count == 0U ||
        left->edge_count > FOLLOWUP_EDGE_COMPARISON_CAP ||
        right->edge_count > FOLLOWUP_EDGE_COMPARISON_CAP || !isfinite(tolerance) ||
        tolerance < 0.0) return false;
    for (size_t index = 0U; index < left->edge_count; ++index) {
        if (point_to_document_edges(left->edges[index].a, right) > tolerance ||
            point_to_document_edges(left->edges[index].b, right) > tolerance)
            return false;
    }
    for (size_t index = 0U; index < right->edge_count; ++index) {
        if (point_to_document_edges(right->edges[index].a, left) > tolerance ||
            point_to_document_edges(right->edges[index].b, left) > tolerance)
            return false;
    }
    return true;
}

static bool materialize(viewer_editor *editor, uint8_t *buffer,
                        size_t capacity, size_t *out_length)
{
    return viewer_editor_materialize(editor, buffer, capacity, out_length);
}

static bool source_contains(viewer_editor *editor, const char *needle)
{
    uint8_t buffer[32768];
    size_t length = 0U, needle_length = strlen(needle);
    if (!materialize(editor, buffer, sizeof(buffer), &length) ||
        needle_length > length) return false;
    for (size_t index = 0U; index <= length - needle_length; ++index)
        if (memcmp(buffer + index, needle, needle_length) == 0) return true;
    return false;
}

static size_t source_count(viewer_editor *editor, const char *needle)
{
    uint8_t buffer[32768];
    size_t length = 0U, needle_length = strlen(needle), count = 0U;
    if (!materialize(editor, buffer, sizeof(buffer), &length) ||
        needle_length > length) return 0U;
    for (size_t index = 0U; index <= length - needle_length; ++index)
        if (memcmp(buffer + index, needle, needle_length) == 0) ++count;
    return count;
}

static bool test_curve_node_round_trip(void)
{
    viewer_editor *editor = NULL;
    svg_shape_document before = {0}, after = {0}, restored_document = {0};
    uint8_t changed[8192];
    size_t changed_length = 0U;
    int result = 1;
    editor = viewer_editor_create_styled((base_bytes){cubic_source,
        sizeof(cubic_source) - 1U}, UI_THEME_DARK, 640U, 640U, false);
    CHECK(editor != NULL && decode(cubic_source, sizeof(cubic_source) - 1U,
        &before));
    size_t path_offset = (size_t)(strstr((const char *)cubic_source, "<path") -
        (const char *)cubic_source);
    CHECK(viewer_editor_select_at_source_offset(editor, path_offset));
    CHECK(viewer_editor_set_draw_tool(editor, VIEWER_EDITOR_TOOL_PEN));
    uint32_t inserted = editor_canvas_path_add_node(editor, &before,
        (geom_point){23.0, 40.0}, 2.0);
    CHECK((inserted & VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(source_count(editor, "C ") == 2U);
    CHECK(materialize(editor, changed, sizeof(changed), &changed_length));
    CHECK(decode(changed, changed_length, &after));
    CHECK(document_edges_near(&before, &after, 0.5));
    CHECK(editor_canvas_path_node_begin(editor, &after,
        (geom_point){23.0, 40.0}, 1.0));
    CHECK((editor_canvas_path_node_delete(editor, &after) &
        VIEWER_EDITOR_MUTATED) != 0U);
    uint8_t restored[8192];
    size_t restored_length = 0U;
    CHECK(materialize(editor, restored, sizeof(restored), &restored_length));
    CHECK(decode(restored, restored_length, &restored_document));
    CHECK(document_edges_near(&before, &restored_document, 0.5));
    result = 0;
done:
    svg_shape_document_destroy(&restored_document);
    svg_shape_document_destroy(&after);
    svg_shape_document_destroy(&before);
    viewer_editor_destroy(editor);
    return result == 0;
}

static bool curve_split_round_trip(const uint8_t *source, size_t source_length,
    geom_point insertion, const char *curve_command)
{
    viewer_editor *editor = NULL;
    svg_shape_document before = {0}, after = {0}, restored_document = {0};
    uint8_t changed[8192], restored[8192];
    size_t changed_length = 0U, restored_length = 0U;
    int result = 1;
    editor = viewer_editor_create_styled((base_bytes){source, source_length},
        UI_THEME_DARK, 640U, 640U, false);
    CHECK(editor != NULL && decode(source, source_length, &before));
    size_t path_offset = (size_t)(strstr((const char *)source, "<path") -
        (const char *)source);
    CHECK(viewer_editor_select_at_source_offset(editor, path_offset));
    CHECK(viewer_editor_set_draw_tool(editor, VIEWER_EDITOR_TOOL_PEN));
    CHECK((editor_canvas_path_add_node(editor, &before, insertion, 2.0) &
        VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(source_count(editor, curve_command) == 2U);
    CHECK(materialize(editor, changed, sizeof(changed), &changed_length));
    CHECK(decode(changed, changed_length, &after));
    CHECK(document_edges_near(&before, &after, 0.5));
    CHECK((viewer_editor_key(editor, 0U, 0U, "n", 1U) &
        VIEWER_EDITOR_HANDLED) != 0U);
    viewer_editor_canvas_overlay_state overlay = {0};
    CHECK(editor_canvas_path_nodes_overlay(editor, &after, &overlay));
    CHECK(overlay.path_point_count == 3U);
    CHECK(editor_canvas_path_node_begin(editor, &after, overlay.path_points[1], 1.0));
    CHECK((editor_canvas_path_node_delete(editor, &after) &
        VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(materialize(editor, restored, sizeof(restored), &restored_length));
    CHECK(decode(restored, restored_length, &restored_document));
    CHECK(document_edges_near(&before, &restored_document, 0.5));
    result = 0;
done:
    svg_shape_document_destroy(&restored_document);
    svg_shape_document_destroy(&after);
    svg_shape_document_destroy(&before);
    viewer_editor_destroy(editor);
    return result == 0;
}

static bool test_quadratic_and_arc_node_round_trip(void)
{
    if (!curve_split_round_trip(quadratic_source,
            sizeof(quadratic_source) - 1U, (geom_point){23.0, 25.0}, "Q "))
        return false;
    svg_shape_document document = {0};
    if (!decode(arc_source, sizeof(arc_source) - 1U, &document)) return false;
    if (document.path_segment_count < 2U) {
        svg_shape_document_destroy(&document);
        return false;
    }
    const svg_shape_path_segment *arc = &document.path_segments[1];
    if (arc->edge_count == 0U || arc->first_edge > document.edge_count ||
        arc->edge_count > document.edge_count - arc->first_edge) {
        svg_shape_document_destroy(&document);
        return false;
    }
    svg_shape_document_destroy(&document);
    return curve_split_round_trip(arc_source, sizeof(arc_source) - 1U,
        (geom_point){23.0, 33.228756555322953}, "A ");
}

static bool test_align_and_distribute_shortcuts(void)
{
    viewer_editor *editor = NULL;
    svg_shape_document document = {0}, aligned_document = {0};
    int result = 1;
    editor = viewer_editor_create_styled((base_bytes){multi_source,
        sizeof(multi_source) - 1U}, UI_THEME_DARK, 640U, 640U, false);
    CHECK(editor != NULL && decode(multi_source, sizeof(multi_source) - 1U,
        &document));
    CHECK(viewer_editor_select_all(editor, &document));
    uint32_t aligned = viewer_editor_align_selection(editor, &document,
        VIEWER_EDITOR_ALIGN_LEFT);
    CHECK((aligned & VIEWER_EDITOR_MUTATED) != 0U);
    uint8_t aligned_source[8192];
    size_t aligned_length = 0U;
    CHECK(materialize(editor, aligned_source, sizeof(aligned_source),
        &aligned_length) && decode(aligned_source, aligned_length,
        &aligned_document));
    CHECK(editor->canvas_selection_count == 2U);
    svg_shape_selection aligned_first, aligned_second;
    CHECK(svg_shape_element_at_offset(&aligned_document,
        editor->canvas_selection_offsets[0], &aligned_first));
    CHECK(svg_shape_element_at_offset(&aligned_document,
        editor->canvas_selection_offsets[1], &aligned_second));
    CHECK(fabs(aligned_first.user_bounds.x - aligned_second.user_bounds.x) < 1.0e-6);
    svg_shape_document_destroy(&aligned_document);
    CHECK((viewer_editor_key(editor, 0U, PLATFORM_MODIFIER_COMMAND, "z", 1U) &
        VIEWER_EDITOR_MUTATED) != 0U);
    uint8_t restored[8192];
    size_t restored_length = 0U;
    CHECK(materialize(editor, restored, sizeof(restored), &restored_length));
    CHECK(restored_length == sizeof(multi_source) - 1U &&
        memcmp(restored, multi_source, restored_length) == 0);
    svg_shape_document_destroy(&document);
    viewer_editor_destroy(editor);
    editor = viewer_editor_create_styled((base_bytes){distribution_source,
        sizeof(distribution_source) - 1U}, UI_THEME_DARK, 640U, 640U, false);
    CHECK(editor != NULL && decode(distribution_source,
        sizeof(distribution_source) - 1U, &document));
    CHECK(viewer_editor_select_all(editor, &document));
    uint32_t distributed = viewer_editor_distribute_selection(editor,
        &document, VIEWER_EDITOR_DISTRIBUTE_HORIZONTAL);
    CHECK((distributed & VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(source_contains(editor, "transform=\"translate(5 0)\""));
    result = 0;
done:
    svg_shape_document_destroy(&aligned_document);
    svg_shape_document_destroy(&document);
    viewer_editor_destroy(editor);
    return result == 0;
}

static bool test_multiselect_group_round_trip(void)
{
    viewer_editor *editor = NULL;
    svg_shape_document document = {0};
    uint8_t buffer[8192];
    size_t length = 0U;
    int result = 1;
    editor = viewer_editor_create_styled((base_bytes){multi_source,
        sizeof(multi_source) - 1U}, UI_THEME_DARK, 640U, 640U, false);
    CHECK(editor != NULL && decode(multi_source, sizeof(multi_source) - 1U,
        &document));
    CHECK(viewer_editor_select_all(editor, &document));
    CHECK((viewer_editor_key(editor, 0U, PLATFORM_MODIFIER_COMMAND, "g", 1U) &
        VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(source_contains(editor, "<g><rect id='first'"));
    CHECK(source_contains(editor, "<!-- keep this comment --><ellipse id='second'"));
    CHECK(source_contains(editor, "fill='#ef5261'/></g></svg>"));
    CHECK((viewer_editor_key(editor, 0U,
        PLATFORM_MODIFIER_COMMAND | PLATFORM_MODIFIER_SHIFT, "g", 1U) &
        VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(materialize(editor, buffer, sizeof(buffer), &length));
    CHECK(length == sizeof(multi_source) - 1U &&
        memcmp(buffer, multi_source, length) == 0);
    CHECK((viewer_editor_key(editor, 0U, PLATFORM_MODIFIER_COMMAND, "z", 1U) &
        VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(source_contains(editor, "<g><rect id='first'"));
    result = 0;
done:
    svg_shape_document_destroy(&document);
    viewer_editor_destroy(editor);
    return result == 0;
}

static bool test_multiselect_cap(void)
{
    enum { RECTANGLE_COUNT = 129 };
    viewer_editor *editor = NULL;
    svg_shape_document document = {0};
    int result = 1;
    uint8_t source[32768];
    size_t used = 0U;
    const char *prefix = "<svg xmlns='http://www.w3.org/2000/svg' width='320' "
        "height='220' viewBox='0 0 320 220'>";
    const char *suffix = "</svg>";
    size_t prefix_length = strlen(prefix), suffix_length = strlen(suffix);
    CHECK(prefix_length <= sizeof(source));
    memcpy(source, prefix, prefix_length);
    used = prefix_length;
    for (size_t index = 0U; index < RECTANGLE_COUNT; ++index) {
        size_t x = (index % 16U) * 18U;
        size_t y = (index / 16U) * 18U;
        int amount = snprintf((char *)source + used, sizeof(source) - used,
            "<rect x='%zu' y='%zu' width='4' height='4'/>", x, y);
        CHECK(amount > 0 && (size_t)amount < sizeof(source) - used);
        used += (size_t)amount;
    }
    CHECK(suffix_length <= sizeof(source) - used);
    memcpy(source + used, suffix, suffix_length);
    used += suffix_length;
    editor = viewer_editor_create_styled((base_bytes){source, used},
        UI_THEME_DARK, 640U, 640U, false);
    CHECK(editor != NULL && decode(source, used, &document));
    CHECK(viewer_editor_select_all(editor, &document));
    CHECK(editor->canvas_selection_count == EDITOR_CANVAS_SELECTION_CAP);
    CHECK((viewer_editor_nudge_selection(editor, 1.0, 0.0) &
        VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(source_count(editor, "transform=\"translate(1 0)\"") ==
        EDITOR_CANVAS_SELECTION_CAP);
    result = 0;
done:
    svg_shape_document_destroy(&document);
    viewer_editor_destroy(editor);
    return result == 0;
}

static bool test_multigroup_ungroup_round_trip(void)
{
    viewer_editor *editor = NULL;
    svg_shape_document document = {0};
    uint8_t materialized[8192];
    size_t length = 0U;
    int result = 1;
    editor = viewer_editor_create_styled((base_bytes){groups_source,
        sizeof(groups_source) - 1U}, UI_THEME_DARK, 640U, 640U, false);
    CHECK(editor != NULL && decode(groups_source, sizeof(groups_source) - 1U,
        &document));
    size_t first = (size_t)(strstr((const char *)groups_source,
        "<g id='first-group'") - (const char *)groups_source);
    size_t second = (size_t)(strstr((const char *)groups_source,
        "<g id='second-group'") - (const char *)groups_source);
    CHECK(viewer_editor_select_at_source_offset(editor, first));
    CHECK(editor_canvas_selection_toggle(editor, second));
    CHECK(editor->canvas_selection_count == 2U);
    CHECK((viewer_editor_key(editor, 0U,
        PLATFORM_MODIFIER_COMMAND | PLATFORM_MODIFIER_SHIFT, "g", 1U) &
        VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(materialize(editor, materialized, sizeof(materialized), &length));
    CHECK(length == sizeof(ungrouped_source) - 1U &&
        memcmp(materialized, ungrouped_source, length) == 0);
    CHECK((viewer_editor_key(editor, 0U, PLATFORM_MODIFIER_COMMAND, "z", 1U) &
        VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(materialize(editor, materialized, sizeof(materialized), &length));
    CHECK(length == sizeof(groups_source) - 1U &&
        memcmp(materialized, groups_source, length) == 0);
    result = 0;
done:
    svg_shape_document_destroy(&document);
    viewer_editor_destroy(editor);
    return result == 0;
}

static bool test_shift_nudge_shortcut(void)
{
    viewer_editor *editor = NULL;
    svg_shape_document document = {0};
    int result = 1;
    editor = viewer_editor_create_styled((base_bytes){multi_source,
        sizeof(multi_source) - 1U}, UI_THEME_DARK, 640U, 640U, false);
    CHECK(editor != NULL && decode(multi_source, sizeof(multi_source) - 1U,
        &document));
    CHECK(viewer_editor_select_all(editor, &document));
    CHECK((viewer_editor_key(editor, 124U, PLATFORM_MODIFIER_SHIFT, NULL, 0U) &
        VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(source_count(editor, "transform=\"translate(10 0)\"") == 2U);
    result = 0;
done:
    svg_shape_document_destroy(&document);
    viewer_editor_destroy(editor);
    return result == 0;
}

static bool test_layer_visibility_is_preview_only(void)
{
    viewer_editor *editor = NULL;
    svg_shape_document preview_document = {0};
    uint8_t materialized[8192], preview[8192];
    size_t length = 0U, layer_index = SIZE_MAX;
    size_t preview_length = 0U;
    document_handle layer = {0};
    int result = 1;
    editor = viewer_editor_create_styled((base_bytes){multi_source,
        sizeof(multi_source) - 1U}, UI_THEME_DARK, 640U, 640U, true);
    CHECK(editor != NULL && document_edit_find_id(editor->edit,
        (base_bytes){(const uint8_t *)"first", 5U}, &layer));
    for (size_t index = 0U; index < editor->outline_count; ++index) {
        if (editor_same_handle(editor->outline[index], layer)) {
            layer_index = index;
            break;
        }
    }
    CHECK(layer_index != SIZE_MAX);
    CHECK(materialize(editor, materialized, sizeof(materialized), &length));
    CHECK(length == sizeof(multi_source) - 1U &&
        memcmp(materialized, multi_source, length) == 0);
    CHECK(!viewer_editor_is_dirty(editor));
    CHECK(editor_outline_toggle_visibility(editor, layer_index));
    CHECK(!viewer_editor_is_dirty(editor));
    CHECK(materialize(editor, materialized, sizeof(materialized), &length));
    CHECK(length == sizeof(multi_source) - 1U &&
        memcmp(materialized, multi_source, length) == 0);
    CHECK(viewer_editor_materialize_preview(editor, preview, sizeof(preview),
        &preview_length));
    CHECK(preview_length > length && decode(preview, preview_length,
        &preview_document));
    CHECK(preview_document.shape_count == 1U);
    svg_shape_document_destroy(&preview_document);
    CHECK(editor_outline_toggle_visibility(editor, layer_index));
    CHECK(!viewer_editor_is_dirty(editor));
    CHECK(materialize(editor, materialized, sizeof(materialized), &length));
    CHECK(length == sizeof(multi_source) - 1U &&
        memcmp(materialized, multi_source, length) == 0);
    CHECK(viewer_editor_materialize_preview(editor, preview, sizeof(preview),
        &preview_length));
    CHECK(preview_length == sizeof(multi_source) - 1U &&
        memcmp(preview, multi_source, preview_length) == 0);
    result = 0;
done:
    svg_shape_document_destroy(&preview_document);
    viewer_editor_destroy(editor);
    return result == 0;
}

static bool test_outline_handles_survive_save(void)
{
    viewer_editor *editor = NULL;
    char path[] = "/tmp/vexel-editor-outline-save.XXXXXX";
    int fd = -1;
    size_t rectangle_offset = (size_t)(strstr((const char *)multi_source,
        "<rect") - (const char *)multi_source);
    char label[UI_TEXT_MAX_BYTES] = {0};
    document_source_span selection = {0};
    int result = 1;
    editor = viewer_editor_create_styled((base_bytes){multi_source,
        sizeof(multi_source) - 1U}, UI_THEME_DARK, 640U, 640U, true);
    CHECK(editor != NULL && viewer_editor_select_at_source_offset(editor,
        rectangle_offset));
    CHECK(editor->outline_count == 3U);
    CHECK(editor_format_outline(editor, 1U, label) > 0U);
    CHECK(strstr(label, "<rect>") != NULL && strstr(label, "#first") != NULL);
    size_t visibility_index = 1U;
    CHECK(editor_outline_toggle_visibility(editor, visibility_index));
    CHECK(!editor_outline_is_visible(editor, editor->outline[visibility_index]));
    fd = mkstemp(path);
    CHECK(fd >= 0 && close(fd) == 0);
    fd = -1;
    document_edit_save_result saved = viewer_editor_save(editor,
        (base_bytes){(const uint8_t *)path, strlen(path)});
    CHECK(saved.error == DOCUMENT_EDIT_OK && !viewer_editor_is_dirty(editor));
    memset(label, 0, sizeof(label));
    CHECK(editor_format_outline(editor, 1U, label) > 0U);
    CHECK(strstr(label, "<rect>") != NULL && strstr(label, "#first") != NULL);
    CHECK(viewer_editor_selection_span(editor, &selection) &&
        selection.offset == rectangle_offset);
    CHECK(!editor_outline_is_visible(editor, editor->outline[1U]));
    result = 0;
done:
    if (fd >= 0) (void)close(fd);
    (void)unlink(path);
    viewer_editor_destroy(editor);
    return result == 0;
}

static bool test_node_mode_requires_supported_path_and_protects_move_to(void)
{
    viewer_editor *editor = NULL;
    svg_shape_document document = {0};
    int result = 1;
    editor = viewer_editor_create_styled((base_bytes){multi_source,
        sizeof(multi_source) - 1U}, UI_THEME_DARK, 640U, 640U, false);
    CHECK(editor != NULL);
    CHECK((viewer_editor_key(editor, 0U, 0U, "n", 1U) &
        VIEWER_EDITOR_HANDLED) != 0U);
    CHECK(!editor->node_edit_active);
    viewer_editor_destroy(editor);
    editor = viewer_editor_create_styled((base_bytes){unsupported_path_source,
        sizeof(unsupported_path_source) - 1U}, UI_THEME_DARK, 640U, 640U, false);
    size_t path_offset = (size_t)(strstr((const char *)unsupported_path_source,
        "<path") - (const char *)unsupported_path_source);
    CHECK(editor != NULL && viewer_editor_select_at_source_offset(editor,
        path_offset));
    CHECK((viewer_editor_key(editor, 0U, 0U, "n", 1U) &
        VIEWER_EDITOR_HANDLED) != 0U);
    CHECK(!editor->node_edit_active);
    viewer_editor_destroy(editor);
    editor = viewer_editor_create_styled((base_bytes){cubic_source,
        sizeof(cubic_source) - 1U}, UI_THEME_DARK, 640U, 640U, false);
    CHECK(editor != NULL && decode(cubic_source, sizeof(cubic_source) - 1U,
        &document));
    path_offset = (size_t)(strstr((const char *)cubic_source, "<path") -
        (const char *)cubic_source);
    CHECK(viewer_editor_select_at_source_offset(editor, path_offset));
    CHECK((viewer_editor_key(editor, 0U, 0U, "n", 1U) &
        VIEWER_EDITOR_HANDLED) != 0U);
    CHECK(editor->node_edit_active);
    CHECK((viewer_editor_key(editor, 53U, 0U, NULL, 0U) &
        VIEWER_EDITOR_HANDLED) != 0U);
    CHECK(!editor->node_edit_active);
    CHECK((viewer_editor_key(editor, 36U, 0U, NULL, 0U) &
        VIEWER_EDITOR_HANDLED) != 0U);
    CHECK(editor->node_edit_active);
    CHECK(editor_canvas_path_node_begin(editor, &document,
        (geom_point){8.0, 40.0}, 1.0));
    uint32_t delete_result = viewer_editor_key(editor, 51U, 0U, NULL, 0U);
    CHECK((delete_result & VIEWER_EDITOR_DELETE_PATH_NODE) == 0U);
    CHECK((delete_result & VIEWER_EDITOR_HANDLED) != 0U);
    CHECK((editor_canvas_path_node_delete(editor, &document) &
        VIEWER_EDITOR_MUTATED) == 0U);
    result = 0;
done:
    svg_shape_document_destroy(&document);
    viewer_editor_destroy(editor);
    return result == 0;
}

static bool test_node_traversal_handles_nudges_and_undo(void)
{
    viewer_editor *editor = NULL;
    svg_shape_document document = {0}, changed_document = {0};
    uint8_t changed[8192], restored[8192], redone[8192];
    size_t changed_length = 0U, restored_length = 0U, redone_length = 0U;
    int result = 1;
    editor = viewer_editor_create_styled((base_bytes){node_command_source,
        sizeof(node_command_source) - 1U}, UI_THEME_LIGHT, 640U, 640U, false);
    CHECK(editor != NULL && decode(node_command_source,
        sizeof(node_command_source) - 1U, &document));
    size_t path_offset = (size_t)(strstr((const char *)node_command_source,
        "<path") - (const char *)node_command_source);
    CHECK(viewer_editor_select_at_source_offset(editor, path_offset));
    CHECK((viewer_editor_key(editor, 36U, 0U, NULL, 0U) &
        VIEWER_EDITOR_HANDLED) != 0U);
    CHECK(viewer_editor_node_edit_active(editor));
    viewer_editor_canvas_overlay_state overlay = {0};
    CHECK(viewer_editor_get_canvas_overlay(editor, &document, &overlay));
    CHECK(overlay.visible && overlay.path_nodes_only &&
        overlay.path_point_count == 7U);
    CHECK((viewer_editor_node_edit_key(editor, &document, 48U, 0U) &
        VIEWER_EDITOR_HANDLED) != 0U);
    CHECK(editor->path_selected_node_segment == 0U);
    CHECK(editor->path_node_is_initial);
    CHECK((viewer_editor_key(editor, 51U, 0U, NULL, 0U) &
        VIEWER_EDITOR_DELETE_PATH_NODE) == 0U);
    CHECK((viewer_editor_node_edit_key(editor, &document, 48U, 0U) &
        VIEWER_EDITOR_HANDLED) != 0U);
    CHECK(editor->path_selected_node_segment == 1U);
    CHECK(!editor->path_node_is_initial);
    CHECK(viewer_editor_get_canvas_overlay(editor, &document, &overlay));
    CHECK(overlay.path_point_states[1] == 2U &&
        overlay.path_has_handles[1] && fabs(overlay.path_handles[1].x - 12.0) < 1.0e-9 &&
        fabs(overlay.path_handles[1].y - 6.0) < 1.0e-9);
    CHECK((viewer_editor_node_edit_key(editor, &document, 48U,
            PLATFORM_MODIFIER_SHIFT) & VIEWER_EDITOR_HANDLED) != 0U);
    CHECK(editor->path_selected_node_segment == 0U);
    CHECK((viewer_editor_node_edit_key(editor, &document, 48U, 0U) &
        VIEWER_EDITOR_HANDLED) != 0U);
    CHECK(editor->path_selected_node_segment == 1U);
    for (size_t index = 2U; index < 7U; ++index)
        CHECK((viewer_editor_node_edit_key(editor, &document, 48U, 0U) &
            VIEWER_EDITOR_HANDLED) != 0U);
    CHECK(editor->path_selected_node_segment == 6U);
    CHECK((viewer_editor_node_edit_key(editor, &document, 124U,
            PLATFORM_MODIFIER_SHIFT) & VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(materialize(editor, changed, sizeof(changed), &changed_length));
    CHECK(decode(changed, changed_length, &changed_document));
    CHECK(source_contains(editor, "A 4 4 0 0 1 50 8"));
    CHECK((viewer_editor_key(editor, 0U, PLATFORM_MODIFIER_COMMAND, "z", 1U) &
        VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(materialize(editor, restored, sizeof(restored), &restored_length));
    CHECK(restored_length == sizeof(node_command_source) - 1U &&
        memcmp(restored, node_command_source, restored_length) == 0);
    CHECK((viewer_editor_key(editor, 0U, PLATFORM_MODIFIER_COMMAND |
            PLATFORM_MODIFIER_SHIFT, "z", 1U) & VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(materialize(editor, redone, sizeof(redone), &redone_length));
    CHECK(redone_length == changed_length &&
        memcmp(redone, changed, redone_length) == 0);
    CHECK((viewer_editor_node_edit_key(editor, &document, 48U, 0U) &
        VIEWER_EDITOR_HANDLED) != 0U);
    CHECK(!viewer_editor_node_edit_active(editor));
    CHECK(editor->focus_id != EDITOR_NODE_CANVAS);
    result = 0;
done:
    svg_shape_document_destroy(&changed_document);
    svg_shape_document_destroy(&document);
    viewer_editor_destroy(editor);
    return result == 0;
}

static bool test_node_curve_handles_hover_cancel_and_conversion(void)
{
    viewer_editor *editor = NULL;
    svg_shape_document document = {0}, converted = {0}, smoothed = {0};
    uint8_t original[8192], changed[8192], undone[8192];
    size_t original_length = 0U, changed_length = 0U, undone_length = 0U;
    int result = 1;
    editor = viewer_editor_create_styled((base_bytes){node_command_source,
        sizeof(node_command_source) - 1U}, UI_THEME_DARK, 640U, 640U, false);
    CHECK(editor != NULL && decode(node_command_source,
        sizeof(node_command_source) - 1U, &document));
    size_t path_offset = (size_t)(strstr((const char *)node_command_source,
        "<path") - (const char *)node_command_source);
    CHECK(viewer_editor_select_at_source_offset(editor, path_offset));
    CHECK((viewer_editor_key(editor, 0U, 0U, "n", 1U) &
        VIEWER_EDITOR_HANDLED) != 0U);
    CHECK(editor_canvas_path_node_select_index(editor, &document, 2U));
    CHECK(editor->path_node_corner_available && editor->path_node_smooth_available);
    CHECK(editor_canvas_path_node_begin(editor, &document,
        (geom_point){14.0, 10.0}, 0.5));
    CHECK(editor->path_node_handle_kind == 2U &&
        editor->path_selected_node_segment == 2U);
    CHECK((editor_canvas_path_node_pointer_move(editor, &document,
            (geom_point){15.0, 11.0}) & VIEWER_EDITOR_HANDLED) != 0U);
    CHECK((viewer_editor_key(editor, 53U, 0U, NULL, 0U) &
        VIEWER_EDITOR_HANDLED) != 0U);
    CHECK(materialize(editor, original, sizeof(original), &original_length));
    CHECK(original_length == sizeof(node_command_source) - 1U &&
        memcmp(original, node_command_source, original_length) == 0);
    CHECK(editor_canvas_path_node_begin(editor, &document,
        (geom_point){18.0, 6.0}, 0.5));
    CHECK(editor->path_node_handle_kind == 1U && editor->path_node_virtual_handle);
    CHECK((editor_canvas_path_node_pointer_move(editor, &document,
            (geom_point){19.0, 7.0}) & VIEWER_EDITOR_HANDLED) != 0U);
    CHECK((editor_canvas_path_node_pointer_up(editor, &document) &
        VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(source_contains(editor, "C 19 7 22 6 24 8"));
    CHECK((viewer_editor_key(editor, 0U, PLATFORM_MODIFIER_COMMAND, "z", 1U) &
        VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(materialize(editor, original, sizeof(original), &original_length));
    CHECK(original_length == sizeof(node_command_source) - 1U &&
        memcmp(original, node_command_source, original_length) == 0);
    CHECK(editor_canvas_path_node_begin(editor, &document,
        (geom_point){30.0, 10.0}, 0.5));
    CHECK((editor->path_node_handle_kind == 1U ||
        editor->path_node_handle_kind == 2U) && editor->path_node_virtual_handle);
    CHECK((editor_canvas_path_node_pointer_move(editor, &document,
            (geom_point){31.0, 11.0}) & VIEWER_EDITOR_HANDLED) != 0U);
    CHECK((editor_canvas_path_node_pointer_up(editor, &document) &
        VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(source_contains(editor, "Q 31 11 32 8"));
    CHECK((viewer_editor_key(editor, 0U, PLATFORM_MODIFIER_COMMAND, "z", 1U) &
        VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(materialize(editor, original, sizeof(original), &original_length));
    CHECK(original_length == sizeof(node_command_source) - 1U &&
        memcmp(original, node_command_source, original_length) == 0);
    CHECK(editor_canvas_path_node_select_index(editor, &document, 2U));
    CHECK((viewer_editor_node_edit_hover(editor, &document,
            (geom_point){7.0, 8.0}, 0.5) & VIEWER_EDITOR_HANDLED) != 0U);
    CHECK(editor->path_hovered_segment_valid);
    CHECK((viewer_editor_node_edit_hover(editor, &document,
            (geom_point){10.0, 8.0}, 0.5) & VIEWER_EDITOR_HANDLED) != 0U);
    viewer_editor_canvas_overlay_state overlay = {0};
    CHECK(viewer_editor_get_canvas_overlay(editor, &document, &overlay));
    CHECK(overlay.path_point_states[1] == 1U);
    uint32_t corner_result = viewer_editor_convert_selected_path_node(editor,
        &document, false);
    CHECK((corner_result & VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(materialize(editor, changed, sizeof(changed), &changed_length));
    CHECK(decode(changed, changed_length, &converted));
    CHECK(document_edges_near(&document, &converted, 0.01));
    CHECK(source_contains(editor, "C 18 6 22 6 24 8"));
    CHECK((viewer_editor_key(editor, 0U, PLATFORM_MODIFIER_COMMAND, "z", 1U) &
        VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(materialize(editor, undone, sizeof(undone), &undone_length));
    CHECK(undone_length == sizeof(node_command_source) - 1U &&
        memcmp(undone, node_command_source, undone_length) == 0);
    viewer_editor_destroy(editor);
    editor = NULL;
    svg_shape_document_destroy(&converted);
    svg_shape_document_destroy(&document);
    memset(&document, 0, sizeof(document));
    editor = viewer_editor_create_styled((base_bytes){smooth_node_source,
        sizeof(smooth_node_source) - 1U}, UI_THEME_LIGHT, 640U, 640U, false);
    CHECK(editor != NULL && decode(smooth_node_source,
        sizeof(smooth_node_source) - 1U, &document));
    path_offset = (size_t)(strstr((const char *)smooth_node_source,
        "<path") - (const char *)smooth_node_source);
    CHECK(viewer_editor_select_at_source_offset(editor, path_offset));
    CHECK((viewer_editor_key(editor, 0U, 0U, "n", 1U) &
        VIEWER_EDITOR_HANDLED) != 0U);
    CHECK(editor_canvas_path_node_select_index(editor, &document, 1U));
    CHECK(editor->path_node_smooth_available);
    CHECK((viewer_editor_convert_selected_path_node(editor, &document, true) &
        VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(source_contains(editor, "C 21.3851648 10"));
    CHECK(materialize(editor, changed, sizeof(changed), &changed_length));
    CHECK(decode(changed, changed_length, &smoothed));
    CHECK(smoothed.path_number_count == document.path_number_count);
    result = 0;
done:
    svg_shape_document_destroy(&smoothed);
    svg_shape_document_destroy(&converted);
    svg_shape_document_destroy(&document);
    viewer_editor_destroy(editor);
    return result == 0;
}

int main(void)
{
    if (!test_curve_node_round_trip()) return 1;
    if (!test_quadratic_and_arc_node_round_trip()) return 1;
    if (!test_align_and_distribute_shortcuts()) return 1;
    if (!test_multiselect_group_round_trip()) return 1;
    if (!test_multiselect_cap()) return 1;
    if (!test_multigroup_ungroup_round_trip()) return 1;
    if (!test_shift_nudge_shortcut()) return 1;
    if (!test_layer_visibility_is_preview_only()) return 1;
    if (!test_outline_handles_survive_save()) return 1;
    if (!test_node_mode_requires_supported_path_and_protects_move_to()) return 1;
    if (!test_node_traversal_handles_nudges_and_undo()) return 1;
    if (!test_node_curve_handles_hover_cancel_and_conversion()) return 1;
    (void)puts("editor follow-ups: curve nodes, arrangement, grouping, nudging, "
        "visibility, and save stability passed");
    return 0;
}
