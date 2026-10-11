#include "viewer_chrome.h"

#include <math.h>
#include <stdint.h>
#include <stdio.h>

#define FUZZ_CASES 200000U

static uint32_t random_next(uint32_t *state)
{
    uint32_t value = *state;
    value ^= value << 13U;
    value ^= value >> 17U;
    value ^= value << 5U;
    *state = value;
    return value;
}

static bool rect_inside(ui_rect rect, uint32_t width, uint32_t height)
{
    return rect.x <= width && rect.y <= height && rect.width <= width - rect.x &&
        rect.height <= height - rect.y;
}

static bool validate(uint32_t width, uint32_t height, double scale, double ui_scale,
    uint16_t structure_width, uint16_t inspector_width,
    bool want_structure, bool want_inspector)
{
    viewer_chrome_layout layout;
    if (!viewer_chrome_layout_compute_sized(width, height, scale, ui_scale,
            structure_width, inspector_width, want_structure, want_inspector,
            &layout)) return false;
    if (sizeof(layout) > 256U || !rect_inside(layout.toolbar, width, height) ||
        !rect_inside(layout.options, width, height) ||
        !rect_inside(layout.workspace, width, height) ||
        !rect_inside(layout.status, width, height) ||
        !rect_inside(layout.rail, width, height) ||
        !rect_inside(layout.canvas, width, height)) return false;
    if (layout.structure_visible && !rect_inside(layout.structure, width, height)) return false;
    if (layout.inspector_visible && !rect_inside(layout.inspector, width, height)) return false;
    if (layout.structure_visible && !rect_inside(layout.structure_splitter, width, height)) return false;
    if (layout.inspector_visible && !rect_inside(layout.inspector_splitter, width, height)) return false;
    if (layout.toolbar.y != 0U || layout.options.y != layout.toolbar.height ||
        layout.workspace.y != layout.options.y + layout.options.height ||
        layout.status.y != layout.workspace.y + layout.workspace.height ||
        layout.status.y + layout.status.height != height) return false;
    uint32_t cursor = layout.rail.width;
    if (layout.structure_visible) {
        if (layout.structure.x != cursor || layout.structure.y != layout.workspace.y ||
            layout.structure.height != layout.workspace.height) return false;
        cursor += layout.structure.width;
        if (layout.structure_splitter.x != cursor ||
            layout.structure_splitter.y != layout.workspace.y ||
            layout.structure_splitter.height != layout.workspace.height) return false;
        cursor += layout.structure_splitter.width;
    }
    if (layout.canvas.x != cursor || layout.canvas.y != layout.workspace.y ||
        layout.canvas.height != layout.workspace.height) return false;
    cursor += layout.canvas.width;
    if (layout.inspector_visible) {
        if (layout.inspector_splitter.x != cursor ||
            layout.inspector_splitter.y != layout.workspace.y ||
            layout.inspector_splitter.height != layout.workspace.height) return false;
        cursor += layout.inspector_splitter.width;
        if (layout.inspector.x != cursor || layout.inspector.y != layout.workspace.y ||
            layout.inspector.height != layout.workspace.height) return false;
        cursor += layout.inspector.width;
    }
    return cursor == width;
}

int main(void)
{
    uint32_t state = UINT32_C(0x6d2b79f5);
    for (uint32_t index = 0U; index < FUZZ_CASES; ++index) {
        uint32_t width = 200U + random_next(&state) % 7801U;
        uint32_t height = 200U + random_next(&state) % 7801U;
        double scale = 0.5 + (double)(random_next(&state) % 351U) / 100.0;
        double ui_scale = 1.0 + (double)(random_next(&state) % 3U) / 10.0;
        uint16_t structure_width = (uint16_t)(184U + random_next(&state) % 137U);
        uint16_t inspector_width = (uint16_t)(240U + random_next(&state) % 201U);
        bool structure = (random_next(&state) & 1U) != 0U;
        bool inspector = (random_next(&state) & 1U) != 0U;
        if (!validate(width, height, scale, ui_scale, structure_width,
                inspector_width, structure, inspector)) {
            (void)fprintf(stderr, "layout fuzz invariant failed at case %u (%u×%u, %.2f)\n",
                index, width, height, scale);
            return 1;
        }
    }
    viewer_chrome_layout ignored;
    if (viewer_chrome_layout_compute(0U, 200U, 1.0, true, true, &ignored) ||
        viewer_chrome_layout_compute(200U, 200U, NAN, true, true, &ignored) ||
        viewer_chrome_layout_compute(200U, 200U, 4.1, true, true, &ignored)) return 2;
    (void)printf("layout fuzz passed: %u cases, seed 0x6d2b79f5\n", FUZZ_CASES);
    return 0;
}
