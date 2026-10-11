#include "viewer_editor.h"

#include "svg/shape/shape.h"
#include "svg/xml/xml_tokenizer.h"

#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define CHECK(condition) do { if (!(condition)) { \
    (void)fprintf(stderr, "FAIL %s:%d: %s\n", __FILE__, __LINE__, #condition); \
    goto done; } } while (0)

static const uint8_t source[] =
    "<svg xmlns='http://www.w3.org/2000/svg' width='64' height='64' "
    "viewBox='0 0 64 64'><rect id='target' x='8' y='8' width='16' height='12' "
    "fill='#4f7cff'/></svg>";
static const uint8_t multi_source[] =
    "<svg xmlns='http://www.w3.org/2000/svg' width='64' height='64' "
    "viewBox='0 0 64 64'><rect id='first' x='8' y='8' width='16' height='12' "
    "fill='#4f7cff'/><ellipse id='second' cx='42' cy='42' rx='8' ry='8' "
    "fill='#ef5261'/></svg>";
static const uint8_t path_source[] =
    "<svg xmlns='http://www.w3.org/2000/svg' width='64' height='64' "
    "viewBox='0 0 64 64'><path id='curve' d='M 8 8 C 12 8 16 12 20 12' "
    "fill='none' stroke='#172033'/></svg>";
static const uint8_t line_path_source[] =
    "<svg xmlns='http://www.w3.org/2000/svg' width='64' height='64' "
    "viewBox='0 0 64 64'><path id='linepath' d='M 8 8 L 20 8' "
    "fill='none' stroke='#172033'/></svg>";

static bool source_bytes(viewer_editor *editor, uint8_t *bytes, size_t capacity,
                         size_t *length)
{
    return viewer_editor_materialize(editor, bytes, capacity, length);
}

static bool source_valid(viewer_editor *editor)
{
    uint8_t bytes[2048];
    size_t length = 0U;
    if (!source_bytes(editor, bytes, sizeof(bytes), &length)) return false;
    svg_xml_token tokens[128];
    svg_xml_result parsed = svg_xml_tokenize((base_bytes){bytes, length},
        svg_xml_default_options(), tokens, 128U);
    return parsed.diagnostic.category == SVG_XML_ERROR_NONE;
}

static bool source_is_original(viewer_editor *editor)
{
    uint8_t bytes[2048];
    size_t length = 0U;
    return source_bytes(editor, bytes, sizeof(bytes), &length) &&
        length == sizeof(source) - 1U && memcmp(bytes, source, length) == 0;
}

static bool source_has(viewer_editor *editor, const char *needle)
{
    uint8_t bytes[2048];
    size_t length = 0U, needle_length = strlen(needle);
    if (!source_bytes(editor, bytes, sizeof(bytes), &length) ||
        needle_length > length) return false;
    for (size_t index = 0U; index <= length - needle_length; ++index)
        if (memcmp(bytes + index, needle, needle_length) == 0) return true;
    return false;
}

static size_t source_count(viewer_editor *editor, const char *needle)
{
    uint8_t bytes[4096];
    size_t length = 0U, needle_length = strlen(needle), count = 0U;
    if (!source_bytes(editor, bytes, sizeof(bytes), &length) || needle_length > length)
        return 0U;
    for (size_t index = 0U; index <= length - needle_length; ++index)
        if (memcmp(bytes + index, needle, needle_length) == 0) ++count;
    return count;
}

static bool source_matches(viewer_editor *editor, const uint8_t *expected,
                           size_t expected_length)
{
    uint8_t bytes[4096];
    size_t length = 0U;
    return source_bytes(editor, bytes, sizeof(bytes), &length) &&
        length == expected_length && memcmp(bytes, expected, length) == 0;
}

static bool decode_bytes(svg_shape_document *document, const uint8_t *bytes,
                         size_t length)
{
    svg_shape_diagnostic diagnostic = {0};
    return svg_shape_decode((base_bytes){bytes, length}, document, &diagnostic) ==
        SVG_SHAPE_ERROR_NONE;
}

static bool decode_document(svg_shape_document *document)
{
    return decode_bytes(document, source, sizeof(source) - 1U);
}

static bool make_navigation(ui_viewer_navigation *navigation)
{
    return ui_viewer_navigation_init(navigation, 640.0, 640.0, 64.0, 64.0) &&
        ui_viewer_navigation_fit(navigation);
}

static geom_point screen_point(const ui_viewer_navigation *navigation,
                              double user_x, double user_y)
{
    return (geom_point){navigation->pan_x + user_x * navigation->scale,
        navigation->pan_y + user_y * navigation->scale};
}

static bool select_target(viewer_editor *editor,
                          const svg_shape_document *document,
                          const ui_viewer_navigation *navigation)
{
    geom_point point = screen_point(navigation, 16.0, 14.0);
    uint32_t down = viewer_editor_canvas_pointer_down(editor, document,
        navigation, point.x, point.y, 0U);
    uint32_t up = viewer_editor_canvas_pointer_up(editor, document);
    document_source_span span = {0};
    return (down & VIEWER_EDITOR_HANDLED) != 0U &&
        (up & VIEWER_EDITOR_HANDLED) != 0U &&
        viewer_editor_selection_span(editor, &span) &&
        span.offset < sizeof(source) - 1U;
}

static bool test_move_undo_and_nudge(void)
{
    viewer_editor *editor = NULL;
    svg_shape_document document = {0};
    ui_viewer_navigation navigation = {0};
    uint8_t before[2048];
    size_t before_length = 0U;
    int result = 1;
    editor = viewer_editor_create_styled((base_bytes){source, sizeof(source) - 1U},
        UI_THEME_DARK, 640U, 640U, false);
    CHECK(editor != NULL && decode_document(&document) && make_navigation(&navigation));
    geom_point start = screen_point(&navigation, 16.0, 14.0);
    geom_point finish = screen_point(&navigation, 19.0, 17.0);
    CHECK((viewer_editor_canvas_pointer_down(editor, &document, &navigation,
        start.x, start.y, 0U) & VIEWER_EDITOR_HANDLED) != 0U);
    CHECK(viewer_editor_selection_span(editor, &(document_source_span){0}));
    CHECK((viewer_editor_canvas_pointer_move(editor, &document, &navigation,
        finish.x, finish.y) & VIEWER_EDITOR_HANDLED) != 0U);
    CHECK(source_bytes(editor, before, sizeof(before), &before_length));
    CHECK(before_length == sizeof(source) - 1U &&
        memcmp(before, source, before_length) == 0);
    uint32_t committed = viewer_editor_canvas_pointer_up(editor, &document);
    CHECK((committed & VIEWER_EDITOR_MUTATED) != 0U);
    static const uint8_t moved_expected[] =
        "<svg xmlns='http://www.w3.org/2000/svg' width='64' height='64' "
        "viewBox='0 0 64 64'><rect id='target' x='8' y='8' width='16' height='12' "
        "fill='#4f7cff' transform=\"matrix(1 0 0 1 3 3)\"/></svg>";
    CHECK(source_bytes(editor, before, sizeof(before), &before_length));
    CHECK(before_length == sizeof(moved_expected) - 1U &&
        memcmp(before, moved_expected, before_length) == 0);
    CHECK(source_valid(editor));
    CHECK((viewer_editor_key(editor, 0U, PLATFORM_MODIFIER_COMMAND, "z", 1U) &
        VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(source_is_original(editor));
    CHECK(select_target(editor, &document, &navigation));
    CHECK((viewer_editor_nudge_selection(editor, 1.0, 0.0) &
        VIEWER_EDITOR_MUTATED) != 0U);
    static const uint8_t nudged_expected[] =
        "<svg xmlns='http://www.w3.org/2000/svg' width='64' height='64' "
        "viewBox='0 0 64 64'><rect id='target' x='8' y='8' width='16' height='12' "
        "fill='#4f7cff' transform=\"matrix(1 0 0 1 1 0)\"/></svg>";
    CHECK(source_bytes(editor, before, sizeof(before), &before_length));
    CHECK(before_length == sizeof(nudged_expected) - 1U &&
        memcmp(before, nudged_expected, before_length) == 0);
    CHECK((viewer_editor_key(editor, 0U, PLATFORM_MODIFIER_COMMAND, "z", 1U) &
        VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(source_is_original(editor));
    CHECK(select_target(editor, &document, &navigation));
    CHECK((viewer_editor_key(editor, 123U, 0U, NULL, 0U) &
        VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(source_has(editor, "transform=\"matrix(1 0 0 1 -1 0)\""));
    CHECK((viewer_editor_key(editor, 0U, PLATFORM_MODIFIER_COMMAND, "z", 1U) &
        VIEWER_EDITOR_MUTATED) != 0U);
    CHECK((viewer_editor_key(editor, 124U, PLATFORM_MODIFIER_SHIFT, NULL, 0U) &
        VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(source_has(editor, "transform=\"matrix(1 0 0 1 10 0)\""));
    CHECK((viewer_editor_key(editor, 0U, PLATFORM_MODIFIER_COMMAND, "z", 1U) &
        VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(source_is_original(editor));
    result = 0;
done:
    svg_shape_document_destroy(&document);
    viewer_editor_destroy(editor);
    return result;
}

static bool test_resize_and_rotate(void)
{
    viewer_editor *editor = NULL;
    svg_shape_document document = {0};
    ui_viewer_navigation navigation = {0};
    int result = 1;
    editor = viewer_editor_create_styled((base_bytes){source, sizeof(source) - 1U},
        UI_THEME_DARK, 640U, 640U, false);
    CHECK(editor != NULL && decode_document(&document) && make_navigation(&navigation));
    CHECK(select_target(editor, &document, &navigation));
    geom_point corner = screen_point(&navigation, 24.0, 20.0);
    geom_point resized = screen_point(&navigation, 32.0, 25.0);
    CHECK((viewer_editor_canvas_pointer_down(editor, &document, &navigation,
        corner.x, corner.y, 0U) & VIEWER_EDITOR_HANDLED) != 0U);
    CHECK((viewer_editor_canvas_pointer_move(editor, &document, &navigation,
        resized.x, resized.y) & VIEWER_EDITOR_HANDLED) != 0U);
    CHECK(source_is_original(editor));
    CHECK((viewer_editor_canvas_pointer_up(editor, &document) &
        VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(source_has(editor, "matrix(1.5 0 0 1.41666667 -4 -3.33333333)"));
    CHECK(source_valid(editor));
    CHECK((viewer_editor_key(editor, 0U, PLATFORM_MODIFIER_COMMAND, "z", 1U) &
        VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(source_is_original(editor));

    CHECK(select_target(editor, &document, &navigation));
    geom_point rotation_start = screen_point(&navigation, 16.0, 5.8);
    geom_point rotation_end = screen_point(&navigation, 19.93, 6.13);
    CHECK((viewer_editor_canvas_pointer_down(editor, &document, &navigation,
        rotation_start.x, rotation_start.y, 0U) & VIEWER_EDITOR_HANDLED) != 0U);
    CHECK((viewer_editor_canvas_pointer_move(editor, &document, &navigation,
        rotation_end.x, rotation_end.y) & VIEWER_EDITOR_HANDLED) != 0U);
    CHECK(source_is_original(editor));
    CHECK((viewer_editor_canvas_pointer_up(editor, &document) &
        VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(source_has(editor, "matrix("));
    CHECK((viewer_editor_key(editor, 0U, PLATFORM_MODIFIER_COMMAND, "z", 1U) &
        VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(source_is_original(editor));
    result = 0;
done:
    svg_shape_document_destroy(&document);
    viewer_editor_destroy(editor);
    return result;
}

static bool test_move_then_resize(void)
{
    viewer_editor *editor = NULL;
    svg_shape_document document = {0};
    ui_viewer_navigation navigation = {0};
    uint8_t moved_source[2048];
    size_t moved_length = 0U;
    int result = 1;
    editor = viewer_editor_create_styled((base_bytes){source, sizeof(source) - 1U},
        UI_THEME_DARK, 640U, 640U, false);
    CHECK(editor != NULL && decode_document(&document) && make_navigation(&navigation));
    geom_point start = screen_point(&navigation, 16.0, 14.0);
    geom_point finish = screen_point(&navigation, 19.0, 17.0);
    CHECK((viewer_editor_canvas_pointer_down(editor, &document, &navigation,
        start.x, start.y, 0U) & VIEWER_EDITOR_HANDLED) != 0U);
    CHECK((viewer_editor_canvas_pointer_move(editor, &document, &navigation,
        finish.x, finish.y) & VIEWER_EDITOR_HANDLED) != 0U);
    CHECK((viewer_editor_canvas_pointer_up(editor, &document) &
        VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(source_bytes(editor, moved_source, sizeof(moved_source), &moved_length));
    svg_shape_document_destroy(&document);
    CHECK(decode_bytes(&document, moved_source, moved_length));
    geom_point corner = screen_point(&navigation, 27.0, 23.0);
    geom_point resized = screen_point(&navigation, 30.0, 28.0);
    CHECK((viewer_editor_canvas_pointer_down(editor, &document, &navigation,
        corner.x, corner.y, 0U) & VIEWER_EDITOR_HANDLED) != 0U);
    CHECK((viewer_editor_canvas_pointer_move(editor, &document, &navigation,
        resized.x, resized.y) & VIEWER_EDITOR_HANDLED) != 0U);
    uint8_t preview[2048];
    size_t preview_length = 0U;
    CHECK(source_bytes(editor, preview, sizeof(preview), &preview_length));
    CHECK(preview_length == moved_length &&
        memcmp(preview, moved_source, moved_length) == 0);
    CHECK((viewer_editor_canvas_pointer_up(editor, &document) &
        VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(source_has(editor, "matrix(") && source_count(editor, "matrix(") == 1U);
    CHECK(source_valid(editor));
    CHECK((viewer_editor_key(editor, 0U, PLATFORM_MODIFIER_COMMAND, "z", 1U) &
        VIEWER_EDITOR_MUTATED) != 0U);
    uint8_t after_undo[2048];
    size_t after_undo_length = 0U;
    CHECK(source_bytes(editor, after_undo, sizeof(after_undo), &after_undo_length));
    CHECK(after_undo_length == moved_length &&
        memcmp(after_undo, moved_source, moved_length) == 0);
    CHECK((viewer_editor_key(editor, 0U, PLATFORM_MODIFIER_COMMAND, "z", 1U) &
        VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(source_is_original(editor));
    result = 0;
done:
    svg_shape_document_destroy(&document);
    viewer_editor_destroy(editor);
    return result;
}

static bool test_multi_selection_edits(void)
{
    viewer_editor *editor = NULL;
    svg_shape_document document = {0};
    ui_viewer_navigation navigation = {0};
    int result = 1;
    editor = viewer_editor_create_styled((base_bytes){multi_source,
        sizeof(multi_source) - 1U}, UI_THEME_DARK, 640U, 640U, false);
    CHECK(editor != NULL && decode_bytes(&document, multi_source,
        sizeof(multi_source) - 1U) && make_navigation(&navigation));

    geom_point first = screen_point(&navigation, 16.0, 14.0);
    geom_point second = screen_point(&navigation, 42.0, 42.0);
    CHECK((viewer_editor_canvas_pointer_down(editor, &document, &navigation,
        first.x, first.y, PLATFORM_MODIFIER_SHIFT) & VIEWER_EDITOR_HANDLED) != 0U);
    CHECK((viewer_editor_canvas_pointer_down(editor, &document, &navigation,
        second.x, second.y, PLATFORM_MODIFIER_SHIFT) & VIEWER_EDITOR_HANDLED) != 0U);
    viewer_editor_canvas_overlay_state overlay = {0};
    CHECK(viewer_editor_get_canvas_overlay(editor, &document, &overlay));
    CHECK(overlay.visible && fabs(overlay.bounds.x - 8.0) < 0.01 &&
        fabs(overlay.bounds.y - 8.0) < 0.01 &&
        fabs(overlay.bounds.width - 42.0) < 0.01 &&
        fabs(overlay.bounds.height - 42.0) < 0.1);
    CHECK((viewer_editor_nudge_selection(editor, 2.0, 3.0) &
        VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(source_count(editor, "transform=\"translate(2 3)\"") == 2U);
    CHECK(source_valid(editor));
    CHECK((viewer_editor_key(editor, 0U, PLATFORM_MODIFIER_COMMAND, "z", 1U) &
        VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(source_matches(editor, multi_source, sizeof(multi_source) - 1U));

    CHECK(viewer_editor_select_all(editor, &document));
    CHECK((viewer_editor_key(editor, 0U, PLATFORM_MODIFIER_COMMAND, "d", 1U) &
        VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(source_count(editor, "id='first'") == 2U &&
        source_count(editor, "id='second'") == 2U);
    CHECK(source_valid(editor));
    CHECK((viewer_editor_key(editor, 0U, PLATFORM_MODIFIER_COMMAND, "z", 1U) &
        VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(source_matches(editor, multi_source, sizeof(multi_source) - 1U));

    CHECK(viewer_editor_select_all(editor, &document));
    CHECK((viewer_editor_key(editor, 51U, 0U, NULL, 0U) &
        VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(!source_has(editor, "<rect") && !source_has(editor, "<ellipse"));
    CHECK(source_valid(editor));
    CHECK((viewer_editor_key(editor, 0U, PLATFORM_MODIFIER_COMMAND, "z", 1U) &
        VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(source_matches(editor, multi_source, sizeof(multi_source) - 1U));

    geom_point marquee_start = screen_point(&navigation, 3.0, 3.0);
    geom_point marquee_end = screen_point(&navigation, 54.0, 54.0);
    CHECK((viewer_editor_canvas_pointer_down(editor, &document, &navigation,
        marquee_start.x, marquee_start.y, 0U) & VIEWER_EDITOR_HANDLED) != 0U);
    CHECK((viewer_editor_canvas_pointer_move(editor, &document, &navigation,
        marquee_end.x, marquee_end.y) & VIEWER_EDITOR_HANDLED) != 0U);
    CHECK((viewer_editor_canvas_pointer_up(editor, &document) &
        VIEWER_EDITOR_HANDLED) != 0U);
    CHECK(viewer_editor_get_canvas_overlay(editor, &document, &overlay));
    CHECK(overlay.visible && fabs(overlay.bounds.width - 42.0) < 0.01);
    CHECK(source_matches(editor, multi_source, sizeof(multi_source) - 1U));
    result = 0;
done:
    svg_shape_document_destroy(&document);
    viewer_editor_destroy(editor);
    return result == 0;
}

static bool test_canvas_snapping(void)
{
    viewer_editor *editor = NULL;
    svg_shape_document document = {0};
    ui_viewer_navigation navigation = {0};
    int result = 1;
    editor = viewer_editor_create_styled((base_bytes){multi_source,
        sizeof(multi_source) - 1U}, UI_THEME_DARK, 640U, 640U, false);
    CHECK(editor != NULL && decode_bytes(&document, multi_source,
        sizeof(multi_source) - 1U) && make_navigation(&navigation));
    geom_point start = screen_point(&navigation, 16.0, 14.0);
    double target_center = 42.115;
    geom_point near_center = screen_point(&navigation, target_center, 14.0);
    CHECK((viewer_editor_canvas_pointer_down(editor, &document, &navigation,
        start.x, start.y, 0U) & VIEWER_EDITOR_HANDLED) != 0U);
    CHECK((viewer_editor_canvas_pointer_move(editor, &document, &navigation,
        near_center.x, near_center.y) & VIEWER_EDITOR_HANDLED) != 0U);
    viewer_editor_canvas_overlay_state overlay = {0};
    CHECK(viewer_editor_get_canvas_overlay(editor, &document, &overlay));
    CHECK(overlay.snap_guide_x_visible &&
        fabs(overlay.snap_guide_x - target_center) < 0.02 &&
        fabs(overlay.bounds.x - 34.115) < 0.02);
    CHECK((viewer_editor_key(editor, 53U, 0U, NULL, 0U) &
        VIEWER_EDITOR_HANDLED) != 0U);

    CHECK((viewer_editor_key(editor, 39U, PLATFORM_MODIFIER_COMMAND, "'", 1U) &
        VIEWER_EDITOR_HANDLED) != 0U);
    CHECK((viewer_editor_canvas_pointer_down(editor, &document, &navigation,
        start.x, start.y, 0U) & VIEWER_EDITOR_HANDLED) != 0U);
    CHECK((viewer_editor_canvas_pointer_move(editor, &document, &navigation,
        near_center.x, near_center.y) & VIEWER_EDITOR_HANDLED) != 0U);
    CHECK(viewer_editor_get_canvas_overlay(editor, &document, &overlay));
    CHECK(!overlay.snap_guide_x_visible && fabs(overlay.bounds.x - 34.115) < 0.02);
    CHECK((viewer_editor_key(editor, 53U, 0U, NULL, 0U) &
        VIEWER_EDITOR_HANDLED) != 0U);
    CHECK(source_matches(editor, multi_source, sizeof(multi_source) - 1U));
    result = 0;
done:
    svg_shape_document_destroy(&document);
    viewer_editor_destroy(editor);
    return result == 0;
}

static bool test_clipboard_fragments(void)
{
    viewer_editor *editor = NULL;
    svg_shape_document document = {0};
    ui_viewer_navigation navigation = {0};
    uint8_t *clipboard = NULL;
    int result = 1;
    editor = viewer_editor_create_styled((base_bytes){source, sizeof(source) - 1U},
        UI_THEME_DARK, 640U, 640U, false);
    clipboard = malloc(64U * 1024U);
    CHECK(editor != NULL && clipboard != NULL && decode_document(&document) &&
        make_navigation(&navigation));
    size_t offset = (size_t)(strstr((const char *)source, "<rect") -
        (const char *)source);
    CHECK(viewer_editor_select_at_source_offset(editor, offset));
    size_t copied_length = 0U;
    CHECK(viewer_editor_copy_selection(editor, clipboard, 64U * 1024U,
        &copied_length));
    CHECK(copied_length > 0U && copied_length < 1024U &&
        memcmp(clipboard, "<rect id='target'", 16U) == 0);
    CHECK((viewer_editor_paste_svg(editor, clipboard, copied_length, false) &
        VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(source_count(editor, "id='target'") == 2U && source_valid(editor));
    CHECK((viewer_editor_key(editor, 0U, PLATFORM_MODIFIER_COMMAND, "z", 1U) &
        VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(source_matches(editor, source, sizeof(source) - 1U));
    static const uint8_t invalid[] = "<circle><rect></circle>";
    CHECK((viewer_editor_paste_svg(editor, invalid, sizeof(invalid) - 1U, false) &
        VIEWER_EDITOR_MUTATED) == 0U);
    CHECK(source_matches(editor, source, sizeof(source) - 1U));
    viewer_editor_destroy(editor);
    svg_shape_document_destroy(&document);
    editor = viewer_editor_create_styled((base_bytes){multi_source,
        sizeof(multi_source) - 1U}, UI_THEME_DARK, 640U, 640U, false);
    CHECK(editor != NULL && decode_bytes(&document, multi_source,
        sizeof(multi_source) - 1U) && viewer_editor_select_all(editor, &document));
    CHECK(viewer_editor_copy_selection(editor, clipboard, 64U * 1024U,
        &copied_length));
    CHECK(memcmp(clipboard, "<g xmlns=", 9U) == 0 &&
        source_count(editor, "id='first'") == 1U);
    CHECK((viewer_editor_paste_svg(editor, clipboard, copied_length, true) &
        VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(source_count(editor, "id='first'") == 2U &&
        source_count(editor, "id='second'") == 2U && source_valid(editor));
    CHECK((viewer_editor_key(editor, 0U, PLATFORM_MODIFIER_COMMAND, "z", 1U) &
        VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(source_matches(editor, multi_source, sizeof(multi_source) - 1U));
    result = 0;
done:
    free(clipboard);
    svg_shape_document_destroy(&document);
    viewer_editor_destroy(editor);
    return result == 0;
}

static bool test_draw_tool(viewer_editor_draw_tool tool, const char *tag)
{
    viewer_editor *editor = NULL;
    svg_shape_document document = {0};
    ui_viewer_navigation navigation = {0};
    int result = 1;
    editor = viewer_editor_create_styled((base_bytes){source, sizeof(source) - 1U},
        UI_THEME_LIGHT, 640U, 640U, false);
    CHECK(editor != NULL && decode_document(&document) && make_navigation(&navigation));
    CHECK(viewer_editor_set_draw_tool(editor, tool));
    geom_point start = screen_point(&navigation, 30.0, 30.0);
    geom_point finish = screen_point(&navigation, 40.0, 36.0);
    CHECK((viewer_editor_canvas_pointer_down(editor, &document, &navigation,
        start.x, start.y, 0U) & VIEWER_EDITOR_HANDLED) != 0U);
    CHECK((viewer_editor_canvas_pointer_move(editor, &document, &navigation,
        finish.x, finish.y) & VIEWER_EDITOR_HANDLED) != 0U);
    viewer_editor_canvas_overlay_state overlay = {0};
    CHECK(viewer_editor_get_canvas_overlay(editor, &document, &overlay));
    CHECK(overlay.visible && overlay.drawing_preview);
    CHECK(fabs(overlay.bounds.width - 10.0) < 0.001);
    CHECK(fabs(overlay.bounds.height - 6.0) < 0.001);
    CHECK(source_is_original(editor));
    CHECK((viewer_editor_canvas_pointer_up(editor, &document) &
        VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(source_has(editor, tag) && source_valid(editor));
    CHECK(source_has(editor, "fill=\"#4f7cff\"") ||
        source_has(editor, "fill='#4f7cff'"));
    CHECK((viewer_editor_key(editor, 0U, PLATFORM_MODIFIER_COMMAND, "z", 1U) &
        VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(source_is_original(editor));
    result = 0;
done:
    svg_shape_document_destroy(&document);
    viewer_editor_destroy(editor);
    return result;
}

static bool test_draw_tool_shortcuts(void)
{
    viewer_editor *editor = viewer_editor_create_styled(
        (base_bytes){source, sizeof(source) - 1U}, UI_THEME_DARK,
        640U, 640U, false);
    if (editor == NULL) return 1;
    bool okay = (viewer_editor_key(editor, 0U, 0U, "r", 1U) &
            VIEWER_EDITOR_HANDLED) != 0U &&
        viewer_editor_get_draw_tool(editor) == VIEWER_EDITOR_TOOL_RECT &&
        (viewer_editor_key(editor, 0U, 0U, "o", 1U) &
            VIEWER_EDITOR_HANDLED) != 0U &&
        viewer_editor_get_draw_tool(editor) == VIEWER_EDITOR_TOOL_ELLIPSE &&
        (viewer_editor_key(editor, 0U, 0U, "l", 1U) &
            VIEWER_EDITOR_HANDLED) != 0U &&
        viewer_editor_get_draw_tool(editor) == VIEWER_EDITOR_TOOL_LINE &&
        (viewer_editor_key(editor, 0U, 0U, "v", 1U) &
            VIEWER_EDITOR_HANDLED) != 0U &&
        viewer_editor_get_draw_tool(editor) == VIEWER_EDITOR_TOOL_SELECT &&
        (viewer_editor_key(editor, 0U, 0U, "t", 1U) &
            VIEWER_EDITOR_HANDLED) != 0U &&
        viewer_editor_get_draw_tool(editor) == VIEWER_EDITOR_TOOL_TEXT;
    viewer_editor_destroy(editor);
    return okay ? 0 : 1;
}

static bool test_text_tool(void)
{
    viewer_editor *editor = NULL;
    svg_shape_document document = {0};
    ui_viewer_navigation navigation = {0};
    int result = 1;
    editor = viewer_editor_create_styled((base_bytes){source, sizeof(source) - 1U},
        UI_THEME_DARK, 640U, 640U, false);
    CHECK(editor != NULL && decode_document(&document) && make_navigation(&navigation));
    CHECK(viewer_editor_set_draw_tool(editor, VIEWER_EDITOR_TOOL_TEXT));
    geom_point point = screen_point(&navigation, 30.0, 32.0);
    CHECK((viewer_editor_canvas_pointer_down(editor, &document, &navigation,
        point.x, point.y, 0U) & VIEWER_EDITOR_HANDLED) != 0U);
    CHECK(viewer_editor_text_input_active(editor) && source_is_original(editor));
    CHECK((viewer_editor_key(editor, 0U, 0U, "A<&>", 4U) &
        VIEWER_EDITOR_HANDLED) != 0U);
    CHECK(source_is_original(editor));
    CHECK((viewer_editor_key(editor, 36U, 0U, NULL, 0U) &
        VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(source_has(editor, "<text x=\"30\" y=\"32\" font-size=\"16\" "
        "fill=\"#172033\">A&lt;&amp;&gt;</text>"));
    CHECK(source_valid(editor));
    CHECK((viewer_editor_key(editor, 0U, PLATFORM_MODIFIER_COMMAND, "z", 1U) &
        VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(source_is_original(editor));

    point = screen_point(&navigation, 36.0, 38.0);
    CHECK((viewer_editor_canvas_pointer_down(editor, &document, &navigation,
        point.x, point.y, 0U) & VIEWER_EDITOR_HANDLED) != 0U);
    CHECK((viewer_editor_key(editor, 53U, 0U, NULL, 0U) &
        VIEWER_EDITOR_HANDLED) != 0U);
    CHECK(source_is_original(editor) && !viewer_editor_text_input_active(editor));
    result = 0;
done:
    svg_shape_document_destroy(&document);
    viewer_editor_destroy(editor);
    return result == 0;
}

static bool path_click(viewer_editor *editor, const svg_shape_document *document,
    const ui_viewer_navigation *navigation, double x, double y, uint8_t clicks)
{
    geom_point point = screen_point(navigation, x, y);
    uint32_t down = viewer_editor_canvas_pointer_down_click_count(editor,
        document, navigation, point.x, point.y, 0U, clicks);
    uint32_t up = viewer_editor_canvas_pointer_up(editor, document);
    return (down & VIEWER_EDITOR_HANDLED) != 0U &&
        (up & VIEWER_EDITOR_HANDLED) != 0U;
}

static bool test_path_tools(void)
{
    viewer_editor *editor = NULL;
    svg_shape_document document = {0};
    ui_viewer_navigation navigation = {0};
    int result = 1;
    editor = viewer_editor_create_styled((base_bytes){source, sizeof(source) - 1U},
        UI_THEME_DARK, 640U, 640U, false);
    CHECK(editor != NULL && decode_document(&document) && make_navigation(&navigation));

    CHECK(viewer_editor_set_draw_tool(editor, VIEWER_EDITOR_TOOL_POLYGON));
    CHECK(path_click(editor, &document, &navigation, 30.0, 30.0, 1U));
    CHECK(path_click(editor, &document, &navigation, 46.0, 30.0, 1U));
    CHECK(path_click(editor, &document, &navigation, 38.0, 46.0, 1U));
    viewer_editor_canvas_overlay_state overlay = {0};
    CHECK(viewer_editor_get_canvas_overlay(editor, &document, &overlay));
    CHECK(overlay.visible && overlay.drawing_preview && overlay.path_point_count == 3U);
    CHECK(source_is_original(editor));
    CHECK((viewer_editor_key(editor, 36U, 0U, NULL, 0U) &
        VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(source_has(editor, "<polygon points=\"30,30 46,30 38,46\""));
    CHECK(source_valid(editor));
    CHECK((viewer_editor_key(editor, 0U, PLATFORM_MODIFIER_COMMAND, "z", 1U) &
        VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(source_is_original(editor));

    CHECK(viewer_editor_set_draw_tool(editor, VIEWER_EDITOR_TOOL_POLYLINE));
    CHECK(path_click(editor, &document, &navigation, 30.0, 48.0, 1U));
    CHECK(path_click(editor, &document, &navigation, 42.0, 48.0, 1U));
    CHECK(path_click(editor, &document, &navigation, 50.0, 48.0, 2U));
    CHECK(source_has(editor, "<polyline points=\"30,48 42,48 50,48\""));
    CHECK(source_valid(editor));
    CHECK((viewer_editor_key(editor, 0U, PLATFORM_MODIFIER_COMMAND, "z", 1U) &
        VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(source_is_original(editor));

    CHECK(viewer_editor_set_draw_tool(editor, VIEWER_EDITOR_TOOL_PEN));
    geom_point first = screen_point(&navigation, 30.0, 30.0);
    geom_point handle = screen_point(&navigation, 34.0, 26.0);
    CHECK((viewer_editor_canvas_pointer_down(editor, &document, &navigation,
        first.x, first.y, 0U) & VIEWER_EDITOR_HANDLED) != 0U);
    CHECK((viewer_editor_canvas_pointer_move(editor, &document, &navigation,
        handle.x, handle.y) & VIEWER_EDITOR_HANDLED) != 0U);
    CHECK(source_is_original(editor));
    CHECK((viewer_editor_canvas_pointer_up(editor, &document) &
        VIEWER_EDITOR_HANDLED) != 0U);
    CHECK(path_click(editor, &document, &navigation, 48.0, 42.0, 1U));
    CHECK((viewer_editor_key(editor, 36U, 0U, NULL, 0U) &
        VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(source_has(editor, "<path d=\"M 30 30 C"));
    CHECK(source_valid(editor));
    CHECK((viewer_editor_key(editor, 0U, PLATFORM_MODIFIER_COMMAND, "z", 1U) &
        VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(source_is_original(editor));

    CHECK(path_click(editor, &document, &navigation, 28.0, 28.0, 1U));
    CHECK((viewer_editor_key(editor, 53U, 0U, NULL, 0U) &
        VIEWER_EDITOR_HANDLED) != 0U);
    CHECK(source_is_original(editor));
    result = 0;
done:
    svg_shape_document_destroy(&document);
    viewer_editor_destroy(editor);
    return result == 0;
}

static bool test_path_node_editing(void)
{
    viewer_editor *editor = NULL;
    svg_shape_document document = {0};
    ui_viewer_navigation navigation = {0};
    int result = 1;
    editor = viewer_editor_create_styled((base_bytes){path_source,
        sizeof(path_source) - 1U}, UI_THEME_DARK, 640U, 640U, false);
    CHECK(editor != NULL && decode_bytes(&document, path_source,
        sizeof(path_source) - 1U) && make_navigation(&navigation));
    size_t path_offset = (size_t)(strstr((const char *)path_source, "<path") -
        (const char *)path_source);
    CHECK(viewer_editor_select_at_source_offset(editor, path_offset));
    CHECK((viewer_editor_key(editor, 0U, 0U, "n", 1U) &
        VIEWER_EDITOR_HANDLED) != 0U);
    viewer_editor_canvas_overlay_state overlay = {0};
    CHECK(viewer_editor_get_canvas_overlay(editor, &document, &overlay));
    CHECK(overlay.visible && overlay.path_nodes_only &&
        overlay.path_point_count == 2U && overlay.path_has_handles[0] &&
        overlay.path_has_in_handles[1]);

    geom_point start = screen_point(&navigation, 8.0, 8.0);
    geom_point finish = screen_point(&navigation, 10.0, 10.0);
    CHECK((viewer_editor_canvas_pointer_down(editor, &document, &navigation,
        start.x, start.y, 0U) & VIEWER_EDITOR_HANDLED) != 0U);
    CHECK((viewer_editor_canvas_pointer_move(editor, &document, &navigation,
        finish.x, finish.y) & VIEWER_EDITOR_HANDLED) != 0U);
    CHECK(source_matches(editor, path_source, sizeof(path_source) - 1U));
    CHECK((viewer_editor_canvas_pointer_up(editor, &document) &
        VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(source_has(editor, "d='M 10 10 C 12 8 16 12 20 12'"));
    CHECK(source_valid(editor));
    CHECK((viewer_editor_key(editor, 0U, PLATFORM_MODIFIER_COMMAND, "z", 1U) &
        VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(source_matches(editor, path_source, sizeof(path_source) - 1U));

    geom_point handle = screen_point(&navigation, 12.0, 8.0);
    geom_point moved_handle = screen_point(&navigation, 13.0, 9.0);
    CHECK((viewer_editor_canvas_pointer_down(editor, &document, &navigation,
        handle.x, handle.y, 0U) & VIEWER_EDITOR_HANDLED) != 0U);
    CHECK((viewer_editor_canvas_pointer_move(editor, &document, &navigation,
        moved_handle.x, moved_handle.y) & VIEWER_EDITOR_HANDLED) != 0U);
    CHECK((viewer_editor_canvas_pointer_up(editor, &document) &
        VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(source_has(editor, "d='M 8 8 C 13 9 16 12 20 12'"));
    CHECK((viewer_editor_key(editor, 0U, PLATFORM_MODIFIER_COMMAND, "z", 1U) &
        VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(source_matches(editor, path_source, sizeof(path_source) - 1U));

    geom_point end = screen_point(&navigation, 20.0, 12.0);
    CHECK((viewer_editor_canvas_pointer_down(editor, &document, &navigation,
        end.x, end.y, 0U) & VIEWER_EDITOR_HANDLED) != 0U);
    CHECK((viewer_editor_canvas_pointer_up(editor, &document) &
        VIEWER_EDITOR_HANDLED) != 0U);
    uint32_t delete_result = viewer_editor_key(editor, 51U, 0U, NULL, 0U);
    CHECK((delete_result & VIEWER_EDITOR_DELETE_PATH_NODE) != 0U);
    CHECK((viewer_editor_delete_selected_path_node(editor, &document) &
        VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(source_has(editor, "d='M 8 8 '"));
    CHECK(source_valid(editor));
    CHECK((viewer_editor_key(editor, 0U, PLATFORM_MODIFIER_COMMAND, "z", 1U) &
        VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(source_matches(editor, path_source, sizeof(path_source) - 1U));
    svg_shape_document_destroy(&document);
    viewer_editor_destroy(editor);
    editor = NULL;

    editor = viewer_editor_create_styled((base_bytes){line_path_source,
        sizeof(line_path_source) - 1U}, UI_THEME_DARK, 640U, 640U, false);
    CHECK(editor != NULL && decode_bytes(&document, line_path_source,
        sizeof(line_path_source) - 1U));
    path_offset = (size_t)(strstr((const char *)line_path_source, "<path") -
        (const char *)line_path_source);
    CHECK(viewer_editor_select_at_source_offset(editor, path_offset));
    CHECK((viewer_editor_key(editor, 0U, 0U, "n", 1U) &
        VIEWER_EDITOR_HANDLED) != 0U);
    geom_point midpoint = screen_point(&navigation, 14.0, 8.0);
    CHECK((viewer_editor_canvas_pointer_down_click_count(editor, &document,
        &navigation, midpoint.x, midpoint.y, 0U, 2U) &
        VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(source_has(editor, "d='M 8 8 L 14 8 L 20 8'"));
    CHECK(source_valid(editor));
    CHECK((viewer_editor_key(editor, 0U, PLATFORM_MODIFIER_COMMAND, "z", 1U) &
        VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(source_matches(editor, line_path_source, sizeof(line_path_source) - 1U));
    result = 0;
done:
    svg_shape_document_destroy(&document);
    viewer_editor_destroy(editor);
    return result == 0;
}

static bool test_overlay_paint_and_clip(void)
{
    viewer_editor *editor = NULL;
    svg_shape_document document = {0};
    ui_viewer_navigation navigation = {0};
    uint8_t *pixels = NULL;
    const uint32_t width = 800U, height = 800U;
    const size_t stride = (size_t)width * 4U;
    int result = 1;
    editor = viewer_editor_create_styled((base_bytes){source, sizeof(source) - 1U},
        UI_THEME_DARK, width, height, false);
    CHECK(editor != NULL && decode_document(&document) && make_navigation(&navigation));
    CHECK(select_target(editor, &document, &navigation));
    pixels = malloc(stride * height);
    CHECK(pixels != NULL);
    for (size_t offset = 0U; offset < stride * height; offset += 4U) {
        pixels[offset] = 7U; pixels[offset + 1U] = 8U;
        pixels[offset + 2U] = 9U; pixels[offset + 3U] = 255U;
    }
    ui_rect canvas = {20U, 30U, 640U, 640U};
    viewer_editor_paint_canvas_overlay(editor, &document, &navigation,
        canvas, pixels, width, height, stride);
    size_t border = (size_t)(30U + 80U) * stride + (size_t)(20U + 120U) * 4U;
    CHECK(pixels[border] == 36U && pixels[border + 1U] == 107U &&
        pixels[border + 2U] == 218U);
    size_t outside = (size_t)29U * stride + (size_t)20U * 4U;
    CHECK(pixels[outside] == 7U && pixels[outside + 1U] == 8U &&
        pixels[outside + 2U] == 9U);
    result = 0;
done:
    free(pixels);
    svg_shape_document_destroy(&document);
    viewer_editor_destroy(editor);
    return result;
}

int main(void)
{
    if (test_move_undo_and_nudge() != 0) return 1;
    if (test_resize_and_rotate() != 0) return 1;
    if (test_move_then_resize() != 0) return 1;
    if (!test_multi_selection_edits()) return 1;
    if (!test_canvas_snapping()) return 1;
    if (!test_clipboard_fragments()) return 1;
    if (test_draw_tool_shortcuts() != 0) return 1;
    if (!test_text_tool()) return 1;
    if (!test_path_tools()) return 1;
    if (!test_path_node_editing()) return 1;
    if (test_draw_tool(VIEWER_EDITOR_TOOL_RECT, "<rect") != 0) return 1;
    if (test_draw_tool(VIEWER_EDITOR_TOOL_ELLIPSE, "<ellipse") != 0) return 1;
    if (test_draw_tool(VIEWER_EDITOR_TOOL_LINE, "<line") != 0) return 1;
    if (test_overlay_paint_and_clip() != 0) return 1;
    (void)puts("editor canvas: selection, previews, transforms, shapes, and clipping passed");
    return 0;
}
