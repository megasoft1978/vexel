#include "viewer_home.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define TEST_WIDTH 1280U
#define TEST_HEIGHT 800U
#define TEST_STRIDE ((size_t)TEST_WIDTH * 4U)
#define TEST_BYTES (TEST_STRIDE * TEST_HEIGHT)

static int fail(const char *message)
{
    (void)fprintf(stderr, "viewer home test: %s\n", message);
    return 1;
}

static bool rect_contains(ui_rect rect, uint32_t x, uint32_t y)
{
    return x >= rect.x && y >= rect.y && x - rect.x < rect.width &&
        y - rect.y < rect.height;
}

int main(void)
{
    viewer_home_state state = {0};
    viewer_home_layout layout = {0};
    if (!viewer_home_layout_get(&state, TEST_WIDTH, TEST_HEIGHT, 1.0, &layout) ||
        layout.open_button.width == 0U || layout.drop_zone.width == 0U)
        return fail("empty state must expose Open SVG and a drop target");
    viewer_home_action action = viewer_home_pointer_down(&state, TEST_WIDTH,
        TEST_HEIGHT, 1.0, layout.open_button.x + layout.open_button.width / 2U,
        layout.open_button.y + layout.open_button.height / 2U);
    if (action.kind != VIEWER_HOME_ACTION_OPEN)
        return fail("primary action must open a file panel");

    state.recent_labels[0] = "Shapes.svg";
    state.recent_labels[1] = "Icons.svg";
    state.recent_count = 2U;
    if (!viewer_home_layout_get(&state, TEST_WIDTH, TEST_HEIGHT, 1.0, &layout) ||
        layout.recent_count != 2U || layout.recent_rows[1].height == 0U)
        return fail("available recent files must be visible");
    action = viewer_home_pointer_down(&state, TEST_WIDTH, TEST_HEIGHT, 1.0,
        layout.recent_rows[1].x + 4U,
        layout.recent_rows[1].y + layout.recent_rows[1].height / 2U);
    if (action.kind != VIEWER_HOME_ACTION_RECENT || action.recent_index != 1U)
        return fail("recent file rows must select their own document");

    state.load_failed = true;
    state.load_error = "Invalid SVG";
    if (!viewer_home_layout_get(&state, TEST_WIDTH, TEST_HEIGHT, 1.0, &layout) ||
        layout.retry_button.width == 0U || layout.open_button.width == 0U)
        return fail("load errors must offer both Open and Retry");
    action = viewer_home_pointer_down(&state, TEST_WIDTH, TEST_HEIGHT, 1.0,
        layout.retry_button.x + layout.retry_button.width / 2U,
        layout.retry_button.y + layout.retry_button.height / 2U);
    if (action.kind != VIEWER_HOME_ACTION_RETRY)
        return fail("Retry must be an actionable error recovery control");
    if (!viewer_home_layout_get(&state, 360U, 400U, 1.0, &layout) ||
        layout.open_button.width == 0U || layout.retry_button.width == 0U ||
        layout.open_button.x + layout.open_button.width > 360U ||
        layout.retry_button.x + layout.retry_button.width > 360U ||
        layout.open_button.y + layout.open_button.height > 400U ||
        layout.retry_button.y + layout.retry_button.height > 400U)
        return fail("compact windows must retain visible error recovery controls");

    state.has_document = true;
    state.loading = true;
    state.load_failed = false;
    state.drop_hover = true;
    state.canvas = (ui_rect){80U, 40U, 1120U, 700U};
    if (!viewer_home_layout_get(&state, TEST_WIDTH, TEST_HEIGHT, 1.0, &layout) ||
        layout.open_button.width != 0U || layout.drop_zone.width == 0U)
        return fail("loading an occupied window must retain the document and highlight drops");
    action = viewer_home_pointer_down(&state, TEST_WIDTH, TEST_HEIGHT, 1.0, 120U, 100U);
    if (action.kind != VIEWER_HOME_ACTION_NONE)
        return fail("loading feedback must not replace document interaction");

    uint8_t *light = calloc(TEST_BYTES, 1U);
    uint8_t *dark = calloc(TEST_BYTES, 1U);
    if (light == NULL || dark == NULL) {
        free(light);
        free(dark);
        return fail("allocate headless frames");
    }
    state = (viewer_home_state){0};
    viewer_home_paint(&state, UI_THEME_LIGHT, light, TEST_BYTES, TEST_STRIDE,
        TEST_WIDTH, TEST_HEIGHT, 1.0);
    viewer_home_paint(&state, UI_THEME_DARK, dark, TEST_BYTES, TEST_STRIDE,
        TEST_WIDTH, TEST_HEIGHT, 1.0);
    bool different = false;
    for (size_t index = 0U; index < TEST_BYTES; index++) {
        if (light[index] != dark[index]) {
            different = true;
            break;
        }
    }
    free(light);
    free(dark);
    if (!different) return fail("light and dark home screens must render distinctly");
    if (viewer_home_layout_get(&state, 0U, TEST_HEIGHT, 1.0, &layout))
        return fail("zero-sized frames must be rejected");
    (void)rect_contains;
    return EXIT_SUCCESS;
}
