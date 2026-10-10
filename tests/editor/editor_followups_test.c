#include "viewer_editor.h"
#include "viewer_editor_internal.h"

#include "svg/shape/shape.h"

#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

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

int main(void)
{
    if (!test_curve_node_round_trip()) return 1;
    if (!test_quadratic_and_arc_node_round_trip()) return 1;
    if (!test_align_and_distribute_shortcuts()) return 1;
    if (!test_multiselect_group_round_trip()) return 1;
    if (!test_multiselect_cap()) return 1;
    if (!test_multigroup_ungroup_round_trip()) return 1;
    if (!test_shift_nudge_shortcut()) return 1;
    (void)puts("editor follow-ups: curve nodes, arrangement, grouping, and nudging passed");
    return 0;
}
