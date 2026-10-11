#include "viewer_editor_internal.h"

#include "viewer_chrome.h"

#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define CHECK(condition) do { if (!(condition)) { \
    (void)fprintf(stderr, "check failed at %s:%d: %s\n", \
        __FILE__, __LINE__, #condition); return 1; } } while (0)

static int test_bounded_storage(void)
{
    viewer_editor_guides guides;
    viewer_editor_guides_init(&guides);
    CHECK(guides.rulers_visible && guides.guides_visible && !guides.locked);
    CHECK(viewer_editor_guides_count(&guides, VIEWER_EDITOR_GUIDE_HORIZONTAL) == 0U);
    CHECK(viewer_editor_guides_add(&guides, VIEWER_EDITOR_GUIDE_HORIZONTAL, -20.5));
    for (size_t index = 1U; index < VIEWER_EDITOR_GUIDE_CAP_PER_AXIS; ++index)
        CHECK(viewer_editor_guides_add(&guides, VIEWER_EDITOR_GUIDE_HORIZONTAL,
            (double)index));
    CHECK(viewer_editor_guides_count(&guides, VIEWER_EDITOR_GUIDE_HORIZONTAL) ==
        VIEWER_EDITOR_GUIDE_CAP_PER_AXIS);
    CHECK(!viewer_editor_guides_add(&guides, VIEWER_EDITOR_GUIDE_HORIZONTAL, 400.0));
    CHECK(viewer_editor_guides_count(&guides, VIEWER_EDITOR_GUIDE_HORIZONTAL) ==
        VIEWER_EDITOR_GUIDE_CAP_PER_AXIS);
    CHECK(viewer_editor_guides_add(&guides, VIEWER_EDITOR_GUIDE_VERTICAL, 75.0));
    CHECK(viewer_editor_guides_count(&guides, VIEWER_EDITOR_GUIDE_VERTICAL) == 1U);
    CHECK(!viewer_editor_guides_add(&guides, VIEWER_EDITOR_GUIDE_HORIZONTAL, NAN));
    CHECK(!viewer_editor_guides_add(&guides, VIEWER_EDITOR_GUIDE_HORIZONTAL, INFINITY));
    CHECK(!viewer_editor_guides_add(&guides, VIEWER_EDITOR_GUIDE_HORIZONTAL, 1000001.0));
    return 0;
}

static int test_edit_and_selection(void)
{
    viewer_editor_guides guides;
    double value = 0.0;
    viewer_editor_guides_init(&guides);
    CHECK(viewer_editor_guides_add(&guides, VIEWER_EDITOR_GUIDE_VERTICAL, 12.0));
    CHECK(viewer_editor_guides_add(&guides, VIEWER_EDITOR_GUIDE_VERTICAL, 24.0));
    CHECK(viewer_editor_guides_get(&guides, VIEWER_EDITOR_GUIDE_VERTICAL, 1U, &value));
    CHECK(value == 24.0);
    CHECK(!viewer_editor_guides_move(&guides, VIEWER_EDITOR_GUIDE_VERTICAL, 2U, 42.0));
    CHECK(!viewer_editor_guides_move(&guides, VIEWER_EDITOR_GUIDE_VERTICAL, 0U, NAN));
    CHECK(viewer_editor_guides_move(&guides, VIEWER_EDITOR_GUIDE_VERTICAL, 1U, 32.0));
    CHECK(viewer_editor_guides_get(&guides, VIEWER_EDITOR_GUIDE_VERTICAL, 1U, &value));
    CHECK(value == 32.0);
    guides.selected = true;
    guides.selected_axis = VIEWER_EDITOR_GUIDE_VERTICAL;
    guides.selected_index = 0U;
    guides.locked = true;
    CHECK(!viewer_editor_guides_add(&guides, VIEWER_EDITOR_GUIDE_VERTICAL, 48.0));
    CHECK(!viewer_editor_guides_remove(&guides, VIEWER_EDITOR_GUIDE_VERTICAL, 0U));
    CHECK(!viewer_editor_guides_delete_selected(&guides));
    guides.locked = false;
    CHECK(viewer_editor_guides_delete_selected(&guides));
    CHECK(!guides.selected);
    CHECK(viewer_editor_guides_count(&guides, VIEWER_EDITOR_GUIDE_VERTICAL) == 1U);
    CHECK(viewer_editor_guides_clear(&guides, VIEWER_EDITOR_GUIDE_VERTICAL));
    CHECK(viewer_editor_guides_count(&guides, VIEWER_EDITOR_GUIDE_VERTICAL) == 0U);
    CHECK(!viewer_editor_guides_clear(&guides, VIEWER_EDITOR_GUIDE_VERTICAL));
    return 0;
}

static int test_nearest_and_ticks(void)
{
    viewer_editor_guides guides;
    double value = 0.0, step = 0.0;
    size_t index = 99U;
    viewer_editor_guides_init(&guides);
    CHECK(viewer_editor_guides_add(&guides, VIEWER_EDITOR_GUIDE_HORIZONTAL, 5.0));
    CHECK(viewer_editor_guides_add(&guides, VIEWER_EDITOR_GUIDE_HORIZONTAL, 15.0));
    CHECK(viewer_editor_guides_nearest(&guides, VIEWER_EDITOR_GUIDE_HORIZONTAL,
        10.0, 5.0, &value, &index));
    CHECK(value == 5.0 && index == 0U);
    CHECK(!viewer_editor_guides_nearest(&guides, VIEWER_EDITOR_GUIDE_HORIZONTAL,
        10.0, 4.9, &value, &index));
    CHECK(viewer_editor_guides_tick_step(1.0, 1.0, 64.0, &step));
    CHECK(step == 100.0);
    CHECK(viewer_editor_guides_tick_step(1.0, 2.0, 64.0, &step));
    CHECK(step == 200.0);
    CHECK(viewer_editor_guides_tick_step(1.0, 4.0, 64.0, &step));
    CHECK(step == 500.0);
    CHECK(viewer_editor_guides_tick_step(4.0, 2.0, 64.0, &step));
    CHECK(step == 50.0);
    CHECK(!viewer_editor_guides_tick_step(0.0, 1.0, 64.0, &step));
    CHECK(!viewer_editor_guides_tick_step(1.0, 8.0, 64.0, &step));
    return 0;
}

static uint32_t guide_fuzz_next(uint32_t *state)
{
    uint32_t value = *state;
    value ^= value << 13U;
    value ^= value >> 17U;
    value ^= value << 5U;
    *state = value;
    return value;
}

static int test_bounded_state_fuzz(void)
{
    viewer_editor_guides guides;
    viewer_editor_guides_init(&guides);
    uint32_t random_state = UINT32_C(0x14a7d311);
    for (size_t step = 0U; step < 20000U; ++step) {
        uint32_t bits = guide_fuzz_next(&random_state);
        viewer_editor_guide_axis_kind axis = (bits & 1U) == 0U ?
            VIEWER_EDITOR_GUIDE_HORIZONTAL : VIEWER_EDITOR_GUIDE_VERTICAL;
        size_t count = viewer_editor_guides_count(&guides, axis);
        size_t index = count == 0U ? 0U :
            (size_t)(guide_fuzz_next(&random_state) % (uint32_t)count);
        double value = (double)(int32_t)(guide_fuzz_next(&random_state) % 2000001U) -
            1000000.0;
        switch (bits % 7U) {
            case 0U: (void)viewer_editor_guides_add(&guides, axis, value); break;
            case 1U: (void)viewer_editor_guides_move(&guides, axis, index, value); break;
            case 2U: (void)viewer_editor_guides_remove(&guides, axis, index); break;
            case 3U: (void)viewer_editor_guides_clear(&guides, axis); break;
            case 4U: guides.locked = (bits & 2U) != 0U; break;
            case 5U: (void)viewer_editor_guides_delete_selected(&guides); break;
            default:
                (void)viewer_editor_guides_nearest(&guides, axis, value,
                    32.0, NULL, NULL);
                break;
        }
        const viewer_editor_guide_axis_kind axes[2] = {
            VIEWER_EDITOR_GUIDE_HORIZONTAL, VIEWER_EDITOR_GUIDE_VERTICAL};
        for (size_t axis_index = 0U; axis_index < 2U; ++axis_index) {
            count = viewer_editor_guides_count(&guides, axes[axis_index]);
            CHECK(count <= VIEWER_EDITOR_GUIDE_CAP_PER_AXIS);
            for (size_t guide = 0U; guide < count; ++guide) {
                double stored = 0.0;
                CHECK(viewer_editor_guides_get(&guides, axes[axis_index], guide,
                    &stored));
                CHECK(isfinite(stored) && fabs(stored) <= 1000000.0);
            }
        }
    }
    return 0;
}

static const ui_node *node_for_command(const viewer_editor *editor, uint32_t command)
{
    if (editor == NULL || !editor->tree_ready) return NULL;
    for (size_t index = 0U; index < editor->tree.count; ++index)
        if (editor->tree.nodes[index].spec.command_id == command)
            return &editor->tree.nodes[index];
    return NULL;
}

static int test_ruler_layout_and_zoom_hud(void)
{
    static const uint8_t source[] =
        "<svg xmlns='http://www.w3.org/2000/svg' width='100' height='100'>"
        "<rect x='10' y='10' width='30' height='20'/></svg>";
    viewer_editor *editor = viewer_editor_create_styled(
        (base_bytes){source, sizeof(source) - 1U}, UI_THEME_LIGHT, 1600U, 1200U, true);
    CHECK(editor != NULL);
    viewer_editor_resize(editor, 1600U, 1200U, 2.0);
    struct viewer_chrome_layout layout;
    ui_rect canvas;
    CHECK(viewer_editor_get_navigation_canvas_rect(editor, &layout, &canvas));
    CHECK(viewer_editor_rulers_visible(editor));
    CHECK(canvas.x == layout.canvas.x + 72U && canvas.y == layout.canvas.y + 72U);
    CHECK(canvas.width == layout.canvas.width - 72U &&
        canvas.height == layout.canvas.height - 72U);
    CHECK(viewer_editor_set_rulers_visible(editor, false));
    CHECK(viewer_editor_get_navigation_canvas_rect(editor, &layout, &canvas));
    CHECK(canvas.x == layout.canvas.x && canvas.y == layout.canvas.y &&
        canvas.width == layout.canvas.width && canvas.height == layout.canvas.height);
    CHECK(viewer_editor_set_rulers_visible(editor, true));

    CHECK(viewer_editor_guides_add(&editor->guides,
        VIEWER_EDITOR_GUIDE_HORIZONTAL, 30.0));
    CHECK(viewer_editor_set_guides_visible(editor, false));
    CHECK(!viewer_editor_guides_visible(editor));
    CHECK(!editor->guides.selected);
    CHECK(viewer_editor_guides_count(&editor->guides,
        VIEWER_EDITOR_GUIDE_HORIZONTAL) == 1U);
    CHECK(viewer_editor_set_guides_visible(editor, true));

    (void)viewer_editor_pointer_down(editor, UINT32_MAX, UINT32_MAX, 0U);
    const ui_node *hud = node_for_command(editor, EDITOR_BUTTON_ZOOM_HUD);
    CHECK(hud != NULL && hud->bounds.width > 0U && hud->bounds.height > 0U);
    uint32_t x = hud->bounds.x + hud->bounds.width / 2U;
    uint32_t y = hud->bounds.y + hud->bounds.height / 2U;
    CHECK((viewer_editor_pointer_down(editor, x, y, 0U) & VIEWER_EDITOR_HANDLED) != 0U);
    CHECK(editor->zoom_menu_open);
    editor_build_tree(editor);
    const ui_node *preset = node_for_command(editor,
        EDITOR_BUTTON_ZOOM_PRESET_BASE + 3U);
    CHECK(preset != NULL && preset->bounds.width > 0U && preset->bounds.height > 0U);
    x = preset->bounds.x + preset->bounds.width / 2U;
    y = preset->bounds.y + preset->bounds.height / 2U;
    uint32_t result = viewer_editor_pointer_down(editor, x, y, 0U);
    double scale = 0.0;
    CHECK((result & VIEWER_EDITOR_ZOOM_PRESET_REQUEST) != 0U);
    CHECK(!editor->zoom_menu_open);
    CHECK(viewer_editor_take_zoom_preset(editor, &scale) && scale == 2.0);
    CHECK(!viewer_editor_take_zoom_preset(editor, &scale));
    viewer_editor_destroy(editor);
    return 0;
}

static int test_editor_guide_gestures_and_snap(void)
{
    static const uint8_t source[] =
        "<svg xmlns='http://www.w3.org/2000/svg' width='100' height='100' "
        "viewBox='0 0 100 100'><rect x='0' y='0' width='10' height='10'/></svg>";
    viewer_editor *editor = viewer_editor_create_styled(
        (base_bytes){source, sizeof(source) - 1U}, UI_THEME_LIGHT, 400U, 300U, false);
    CHECK(editor != NULL);
    svg_shape_document document = {.width = 100U, .height = 100U,
        .view_x = 0.0, .view_y = 0.0, .view_width = 100.0, .view_height = 100.0,
        .preserve_aspect = {0.5, 0.5, false, false, true}};
    ui_viewer_navigation navigation;
    CHECK(ui_viewer_navigation_init(&navigation, 400.0, 300.0, 100.0, 100.0));
    CHECK(ui_viewer_navigation_fit(&navigation));
    ui_rect canvas = {60U, 50U, 400U, 300U};
    viewer_editor_guide_pointer_result down = viewer_editor_guide_pointer_down(
        editor, &document, &navigation, canvas, 80.0, 30.0, 0U);
    CHECK(down == VIEWER_EDITOR_GUIDE_POINTER_HANDLED);
    CHECK(viewer_editor_guide_pointer_move(editor, &document, &navigation,
        canvas, 80.0, 140.0));
    CHECK(viewer_editor_guide_pointer_up(editor));
    CHECK(viewer_editor_guides_count(&editor->guides,
        VIEWER_EDITOR_GUIDE_HORIZONTAL) == 1U);
    double value = 0.0;
    CHECK(viewer_editor_guides_get(&editor->guides,
        VIEWER_EDITOR_GUIDE_HORIZONTAL, 0U, &value));
    CHECK(fabs(value - 30.0) < 1e-9);
    CHECK(viewer_editor_set_sidebar_visibility(editor, false, false));
    CHECK(viewer_editor_guides_count(&editor->guides,
        VIEWER_EDITOR_GUIDE_HORIZONTAL) == 1U);
    CHECK(viewer_editor_guide_pointer_down(editor, &document, &navigation,
        canvas, 30.0, 140.0, 0U) == VIEWER_EDITOR_GUIDE_POINTER_HANDLED);
    CHECK(viewer_editor_guide_pointer_move(editor, &document, &navigation,
        canvas, 245.0, 140.0));
    CHECK(viewer_editor_guide_pointer_up(editor));
    CHECK(viewer_editor_guides_count(&editor->guides,
        VIEWER_EDITOR_GUIDE_VERTICAL) == 1U);
    CHECK(viewer_editor_guides_get(&editor->guides,
        VIEWER_EDITOR_GUIDE_VERTICAL, 0U, &value));
    CHECK(fabs(value - 45.0) < 1e-9);
    CHECK(viewer_editor_guide_pointer_down(editor, &document, &navigation,
        canvas, 80.0, 30.0, 1U) ==
        VIEWER_EDITOR_GUIDE_POINTER_CONTEXT_HORIZONTAL);
    CHECK(viewer_editor_guide_pointer_down(editor, &document, &navigation,
        canvas, 30.0, 140.0, 1U) ==
        VIEWER_EDITOR_GUIDE_POINTER_CONTEXT_VERTICAL);
    viewer_editor_canvas_overlay_state overlay;
    editor->canvas_gesture = EDITOR_CANVAS_GESTURE_DRAW;
    editor->canvas_preview_bounds = (svg_shape_rect){10.0, 10.0, 20.0, 20.0};
    CHECK(viewer_editor_get_canvas_overlay(editor, &document, &overlay));
    viewer_editor_set_navigation_overlays_hidden(editor, true);
    CHECK(!viewer_editor_get_canvas_overlay(editor, &document, &overlay));
    viewer_editor_set_navigation_overlays_hidden(editor, false);
    CHECK(viewer_editor_get_canvas_overlay(editor, &document, &overlay));
    editor->canvas_gesture = EDITOR_CANVAS_GESTURE_NONE;

    CHECK(viewer_editor_guide_pointer_down(editor, &document, &navigation,
        canvas, 80.0, 140.0, 0U) == VIEWER_EDITOR_GUIDE_POINTER_HANDLED);
    CHECK(viewer_editor_guide_pointer_move(editor, &document, &navigation,
        canvas, 80.0, 170.0));
    CHECK(viewer_editor_guide_pointer_up(editor));
    CHECK(viewer_editor_guides_get(&editor->guides,
        VIEWER_EDITOR_GUIDE_HORIZONTAL, 0U, &value));
    CHECK(fabs(value - 40.0) < 1e-9);
    CHECK(viewer_editor_set_rulers_visible(editor, false));
    CHECK(viewer_editor_guide_pointer_down(editor, &document, &navigation,
        canvas, 200.0, 170.0, 0U) == VIEWER_EDITOR_GUIDE_POINTER_HANDLED);
    CHECK(viewer_editor_guide_pointer_move(editor, &document, &navigation,
        canvas, 200.0, 185.0));
    CHECK(viewer_editor_guide_pointer_up(editor));
    CHECK(viewer_editor_guides_get(&editor->guides,
        VIEWER_EDITOR_GUIDE_HORIZONTAL, 0U, &value));
    CHECK(fabs(value - 45.0) < 1e-9);
    CHECK(viewer_editor_set_rulers_visible(editor, true));
    size_t source_length = viewer_editor_byte_length(editor);
    uint8_t *materialized = malloc(source_length);
    CHECK(materialized != NULL);
    size_t out_length = 0U;
    CHECK(viewer_editor_materialize(editor, materialized, source_length, &out_length));
    CHECK(out_length == sizeof(source) - 1U &&
        memcmp(materialized, source, out_length) == 0);
    free(materialized);

    CHECK(viewer_editor_set_guides_locked(editor, true));
    CHECK(viewer_editor_guide_pointer_down(editor, &document, &navigation,
        canvas, 80.0, 185.0, 0U) == VIEWER_EDITOR_GUIDE_POINTER_HANDLED);
    CHECK(viewer_editor_key(editor, 51U, 0U, NULL, 0U) == VIEWER_EDITOR_HANDLED);
    CHECK(viewer_editor_guides_count(&editor->guides,
        VIEWER_EDITOR_GUIDE_HORIZONTAL) == 1U);
    CHECK(viewer_editor_set_guides_locked(editor, false));
    CHECK(viewer_editor_key(editor, 51U, 0U, NULL, 0U) == VIEWER_EDITOR_HANDLED);
    CHECK(viewer_editor_guides_count(&editor->guides,
        VIEWER_EDITOR_GUIDE_HORIZONTAL) == 0U);

    CHECK(viewer_editor_guides_add(&editor->guides,
        VIEWER_EDITOR_GUIDE_VERTICAL, 10.55));
    geom_point snapped = editor_canvas_snap_point(editor, &document,
        (geom_point){10.6, 12.4}, 1.0);
    CHECK(fabs(snapped.x - 10.55) < 1e-9);
    viewer_editor_set_snap_bypass(editor, true);
    geom_point bypassed = editor_canvas_snap_point(editor, &document,
        (geom_point){10.6, 12.4}, 1.0);
    CHECK(bypassed.x == 10.6 && bypassed.y == 12.4);
    viewer_editor_set_snap_bypass(editor, false);
    CHECK(viewer_editor_clear_guides(editor, VIEWER_EDITOR_GUIDE_VERTICAL));
    CHECK(viewer_editor_guides_count(&editor->guides,
        VIEWER_EDITOR_GUIDE_VERTICAL) == 0U);
    viewer_editor_destroy(editor);
    return 0;
}

int main(void)
{
    if (test_bounded_storage() != 0 || test_edit_and_selection() != 0 ||
        test_nearest_and_ticks() != 0 || test_bounded_state_fuzz() != 0 ||
        test_ruler_layout_and_zoom_hud() != 0 ||
        test_editor_guide_gestures_and_snap() != 0) return 1;
    (void)puts("editor guide state passed");
    return 0;
}
