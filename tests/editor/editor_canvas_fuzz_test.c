#include "viewer_editor.h"

#include "svg/shape/shape.h"
#include "svg/xml/xml_tokenizer.h"

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

static bool source_valid(viewer_editor *editor)
{
    uint8_t bytes[2048];
    size_t length = 0U;
    if (!viewer_editor_materialize(editor, bytes, sizeof(bytes), &length)) return false;
    svg_xml_token tokens[128];
    svg_xml_result parsed = svg_xml_tokenize((base_bytes){bytes, length},
        svg_xml_default_options(), tokens, 128U);
    return parsed.diagnostic.category == SVG_XML_ERROR_NONE;
}

static bool source_is_original(viewer_editor *editor)
{
    uint8_t bytes[2048];
    size_t length = 0U;
    return viewer_editor_materialize(editor, bytes, sizeof(bytes), &length) &&
        length == sizeof(source) - 1U && memcmp(bytes, source, length) == 0;
}

static bool selection_valid(viewer_editor *editor)
{
    uint8_t bytes[2048];
    size_t length = 0U;
    document_source_span span = {0};
    return viewer_editor_materialize(editor, bytes, sizeof(bytes), &length) &&
        viewer_editor_selection_span(editor, &span) && span.offset <= length &&
        span.length <= length - span.offset;
}

static bool decode_document(svg_shape_document *document)
{
    svg_shape_diagnostic diagnostic = {0};
    return svg_shape_decode((base_bytes){source, sizeof(source) - 1U}, document,
        &diagnostic) == SVG_SHAPE_ERROR_NONE;
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
        if (cycle % 1000U == 999U) {
            /* Aim the mutating cycle's drag at the rectangle's interior. */
            start_x = 16.0;
            start_y = 14.0;
        }
        geom_point start = screen_point(&navigation, start_x, start_y);
        CHECK((viewer_editor_canvas_pointer_down(editor, &document, &navigation,
            start.x, start.y, 0U) & VIEWER_EDITOR_HANDLED) != 0U);
        CHECK(source_valid(editor) && selection_valid(editor));
        random = next_random(&random_state);
        double end_x = start_x + (double)((int32_t)(random % 5001U) - 2500) / 1000.0;
        random = next_random(&random_state);
        double end_y = start_y + (double)((int32_t)(random % 5001U) - 2500) / 1000.0;
        if (cycle % 1000U == 999U) {
            /* The mutating cycle starts its drag inside the rectangle, because a
             * gesture resolves its target at pointer_down. A drag from empty
             * space leaves the selection on the document root, and transforming
             * the root is refused: it used to rewrite the whole document. */
            start_x = 16.0;
            start_y = 14.0;
            end_x = start_x + 3.0;
            end_y = start_y + 2.0;
        }
        geom_point finish = screen_point(&navigation, end_x, end_y);
        CHECK((viewer_editor_canvas_pointer_move(editor, &document, &navigation,
            finish.x, finish.y) & VIEWER_EDITOR_HANDLED) != 0U);
        CHECK(source_valid(editor) && selection_valid(editor));
        if (cycle % 1000U == 999U) {
            CHECK((viewer_editor_canvas_pointer_up(editor, &document) &
                VIEWER_EDITOR_MUTATED) != 0U);
            CHECK(source_valid(editor) && selection_valid(editor));
            CHECK((viewer_editor_key(editor, 0U, 0U, "v", 1U) &
                VIEWER_EDITOR_HANDLED) != 0U);
        } else {
            CHECK((viewer_editor_key(editor, 53U, 0U, NULL, 0U) &
                VIEWER_EDITOR_HANDLED) != 0U);
            CHECK(source_valid(editor) && selection_valid(editor));
            CHECK((viewer_editor_canvas_pointer_up(editor, &document) &
                VIEWER_EDITOR_HANDLED) != 0U);
            CHECK(source_valid(editor) && selection_valid(editor));
        }
    }
    size_t undo_count = 0U;
    while ((viewer_editor_key(editor, 0U, PLATFORM_MODIFIER_COMMAND, "z", 1U) &
            VIEWER_EDITOR_MUTATED) != 0U) {
        ++undo_count;
        CHECK(source_valid(editor) && selection_valid(editor));
    }
    CHECK(undo_count == 50U && source_is_original(editor));
    result = 0;
done:
    svg_shape_document_destroy(&document);
    viewer_editor_destroy(editor);
    if (result == 0) {
        (void)puts("editor canvas fuzz: 200000 randomized events, source/selection invariants, undo-all passed");
    }
    return result == 0;
}

int main(void)
{
    return test_randomized_gesture_fuzz() ? 0 : 1;
}
