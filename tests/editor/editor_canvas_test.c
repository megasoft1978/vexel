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

static uint32_t next_random(uint32_t *state)
{
    *state ^= *state << 13U;
    *state ^= *state >> 17U;
    *state ^= *state << 5U;
    return *state;
}

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
        "fill='#4f7cff' transform=\"translate(3 3)\"/></svg>";
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
        "fill='#4f7cff' transform=\"translate(1 0)\"/></svg>";
    CHECK(source_bytes(editor, before, sizeof(before), &before_length));
    CHECK(before_length == sizeof(nudged_expected) - 1U &&
        memcmp(before, nudged_expected, before_length) == 0);
    CHECK((viewer_editor_key(editor, 0U, PLATFORM_MODIFIER_COMMAND, "z", 1U) &
        VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(source_is_original(editor));
    CHECK(select_target(editor, &document, &navigation));
    CHECK((viewer_editor_key(editor, 123U, 0U, NULL, 0U) &
        VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(source_has(editor, "transform=\"translate(-1 0)\""));
    CHECK((viewer_editor_key(editor, 0U, PLATFORM_MODIFIER_COMMAND, "z", 1U) &
        VIEWER_EDITOR_MUTATED) != 0U);
    CHECK((viewer_editor_key(editor, 124U, PLATFORM_MODIFIER_SHIFT, NULL, 0U) &
        VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(source_has(editor, "transform=\"translate(10 0)\""));
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
    CHECK(source_has(editor, "scale(1.5 1.41666667)"));
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
    CHECK(source_has(editor, "rotate("));
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
    CHECK(source_has(editor, "scale("));
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
        viewer_editor_get_draw_tool(editor) == VIEWER_EDITOR_TOOL_SELECT;
    viewer_editor_destroy(editor);
    return okay ? 0 : 1;
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

static bool test_randomized_gesture_fuzz(void)
{
    viewer_editor *editor = NULL;
    svg_shape_document document = {0};
    ui_viewer_navigation navigation = {0};
    uint32_t random_state = UINT32_C(0x57e11e5);
    int result = 1;
    editor = viewer_editor_create_styled((base_bytes){source, sizeof(source) - 1U},
        UI_THEME_DARK, 640U, 640U, false);
    CHECK(editor != NULL && decode_document(&document) && make_navigation(&navigation));
    for (size_t cycle = 0U; cycle < 50000U; ++cycle) {
        uint32_t random = next_random(&random_state);
        double start_x = 9.0 + (double)(random % 14000U) / 1000.0;
        random = next_random(&random_state);
        double start_y = 9.0 + (double)(random % 10000U) / 1000.0;
        geom_point start = screen_point(&navigation, start_x, start_y);
        CHECK((viewer_editor_canvas_pointer_down(editor, &document, &navigation,
            start.x, start.y, 0U) & VIEWER_EDITOR_HANDLED) != 0U);
        CHECK(source_valid(editor));
        document_source_span span = {0};
        CHECK(viewer_editor_selection_span(editor, &span));
        CHECK(span.offset < sizeof(source) - 1U);
        random = next_random(&random_state);
        double end_x = start_x + (double)((int32_t)(random % 5001U) - 2500) / 1000.0;
        random = next_random(&random_state);
        double end_y = start_y + (double)((int32_t)(random % 5001U) - 2500) / 1000.0;
        if (cycle % 1000U == 999U) {
            end_x = start_x + 3.0;
            end_y = start_y + 2.0;
        }
        geom_point finish = screen_point(&navigation, end_x, end_y);
        CHECK((viewer_editor_canvas_pointer_move(editor, &document, &navigation,
            finish.x, finish.y) & VIEWER_EDITOR_HANDLED) != 0U);
        CHECK(source_valid(editor));
        if (cycle % 1000U == 999U) {
            CHECK((viewer_editor_canvas_pointer_up(editor, &document) &
                VIEWER_EDITOR_MUTATED) != 0U);
            CHECK(source_valid(editor));
        } else {
            CHECK((viewer_editor_key(editor, 53U, 0U, NULL, 0U) &
                VIEWER_EDITOR_HANDLED) != 0U);
            CHECK(source_valid(editor));
            CHECK((viewer_editor_canvas_pointer_up(editor, &document) &
                VIEWER_EDITOR_HANDLED) != 0U);
            CHECK(source_valid(editor));
        }
    }
    while ((viewer_editor_key(editor, 0U, PLATFORM_MODIFIER_COMMAND, "z", 1U) &
            VIEWER_EDITOR_MUTATED) != 0U) CHECK(source_valid(editor));
    CHECK(source_is_original(editor));
    result = 0;
done:
    svg_shape_document_destroy(&document);
    viewer_editor_destroy(editor);
    return result;
}

int main(void)
{
    if (test_move_undo_and_nudge() != 0) return 1;
    if (test_resize_and_rotate() != 0) return 1;
    if (test_move_then_resize() != 0) return 1;
    if (test_draw_tool_shortcuts() != 0) return 1;
    if (test_draw_tool(VIEWER_EDITOR_TOOL_RECT, "<rect") != 0) return 1;
    if (test_draw_tool(VIEWER_EDITOR_TOOL_ELLIPSE, "<ellipse") != 0) return 1;
    if (test_draw_tool(VIEWER_EDITOR_TOOL_LINE, "<line") != 0) return 1;
    if (test_overlay_paint_and_clip() != 0) return 1;
    if (test_randomized_gesture_fuzz() != 0) return 1;
    (void)puts("editor canvas: selection, previews, transforms, shapes, clipping; 200000 fuzz events passed");
    return 0;
}
