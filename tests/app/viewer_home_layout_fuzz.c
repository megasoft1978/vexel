#include "viewer_home.h"

#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#define FUZZ_WIDTH 64U
#define FUZZ_HEIGHT 64U
#define FUZZ_STRIDE ((size_t)FUZZ_WIDTH * 4U)
#define FUZZ_BYTES (FUZZ_STRIDE * FUZZ_HEIGHT)
#define FUZZ_LAYOUT_CASES 100000U
#define FUZZ_PAINT_CASES 5000U

static uint32_t random_state = UINT32_C(0x57494e31);

static uint32_t next_random(void)
{
    uint32_t value = random_state;
    value ^= value << 13U;
    value ^= value >> 17U;
    value ^= value << 5U;
    random_state = value;
    return value;
}

static bool rect_bounded(ui_rect rect, uint32_t width, uint32_t height)
{
    return rect.x <= width && rect.y <= height &&
        rect.width <= width - rect.x && rect.height <= height - rect.y;
}

static double random_scale(uint32_t value)
{
    switch (value % 16U) {
        case 0U: return NAN;
        case 1U: return INFINITY;
        case 2U: return -INFINITY;
        default: return (double)(value % 81U) / 20.0;
    }
}

static viewer_home_state random_state_value(void)
{
    viewer_home_state state = {0};
    uint32_t value = next_random();
    state.has_document = (value & 1U) != 0U;
    state.loading = (value & 2U) != 0U;
    state.load_failed = (value & 4U) != 0U;
    state.drop_hover = (value & 8U) != 0U;
    state.load_error = (value & 16U) != 0U ? "Invalid SVG input" : NULL;
    state.recent_count = (size_t)(next_random() % (VIEWER_HOME_RECENT_MAX + 4U));
    for (size_t index = 0U; index < VIEWER_HOME_RECENT_MAX; ++index)
        state.recent_labels[index] = "Recent SVG document";
    state.canvas = (ui_rect){next_random(), next_random(), next_random(), next_random()};
    return state;
}

static bool layout_is_bounded(const viewer_home_layout *layout,
                              uint32_t width, uint32_t height)
{
    if (layout->recent_count > VIEWER_HOME_RECENT_MAX ||
        !rect_bounded(layout->notice, width, height) ||
        !rect_bounded(layout->open_button, width, height) ||
        !rect_bounded(layout->retry_button, width, height) ||
        !rect_bounded(layout->drop_zone, width, height)) return false;
    for (size_t index = 0U; index < layout->recent_count; ++index)
        if (!rect_bounded(layout->recent_rows[index], width, height)) return false;
    return true;
}

int main(void)
{
    for (uint32_t index = 0U; index < FUZZ_LAYOUT_CASES; ++index) {
        viewer_home_state state = random_state_value();
        uint32_t width = next_random() % 10001U;
        uint32_t height = next_random() % 10001U;
        double scale = random_scale(next_random());
        viewer_home_layout layout = {0};
        if (viewer_home_layout_get(&state, width, height, scale, &layout) &&
            !layout_is_bounded(&layout, width, height)) {
            (void)fprintf(stderr,
                "home layout fuzz: case=%u size=%ux%u scale=%.4g doc=%d load=%d failed=%d recent=%zu\n",
                index, width, height, scale, state.has_document, state.loading,
                state.load_failed, state.recent_count);
            ui_rect rects[] = {layout.notice, layout.open_button, layout.retry_button,
                layout.drop_zone};
            for (size_t rect = 0U; rect < sizeof(rects) / sizeof(rects[0]); ++rect)
                if (!rect_bounded(rects[rect], width, height))
                    (void)fprintf(stderr, "  rect %zu: %u,%u %ux%u\n", rect,
                        rects[rect].x, rects[rect].y, rects[rect].width,
                        rects[rect].height);
            return 1;
        }
    }

    uint8_t frame[FUZZ_BYTES];
    for (uint32_t index = 0U; index < FUZZ_PAINT_CASES; ++index) {
        viewer_home_state state = random_state_value();
        uint32_t width = next_random() % 10001U;
        uint32_t height = next_random() % 10001U;
        uint32_t theme_value = next_random() % 4U;
        ui_theme theme = theme_value == 0U ? UI_THEME_LIGHT :
            theme_value == 1U ? UI_THEME_DARK : (ui_theme)theme_value;
        size_t stride = (size_t)(next_random() % (FUZZ_STRIDE + 16U));
        size_t capacity = (size_t)(next_random() % (FUZZ_BYTES + 1U));
        memset(frame, 0xa5, sizeof(frame));
        viewer_home_paint(&state, theme, frame, capacity, stride, width, height,
            random_scale(next_random()));
    }
    (void)puts("viewer home layout fuzz: 100000 layouts, 5000 bounded frames passed");
    return 0;
}
