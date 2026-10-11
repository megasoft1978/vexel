#include "viewer_editor_internal.h"

#include "viewer_chrome.h"

#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define LARGE_SHAPE_FIXTURE_CAP (32U * 1024U)

#define CHECK(condition, message) do { \
    if (!(condition)) { \
        (void)fprintf(stderr, "FAIL: %s\n", message); \
        return 1; \
    } \
} while (0)

static const uint8_t source[] =
    "<svg xmlns='http://www.w3.org/2000/svg' width='100' height='100' "
    "viewBox='0 0 100 100'><rect id='selected' x='20' y='20' "
    "width='10' height='10' fill='#cc3344'/><rect id='neighbor' "
    "x='41' y='42' width='10' height='10' fill='#3366cc'/></svg>";

static const uint8_t zoom_source[] =
    "<svg xmlns='http://www.w3.org/2000/svg' width='400' height='400' "
    "viewBox='0 0 400 400'><rect id='selected' x='20' y='200' "
    "width='80' height='80' fill='#cc3344' stroke='#111111'/><rect "
    "id='neighbor' x='160' y='320' width='20' height='20' "
    "fill='#3366cc'/></svg>";

static const uint8_t action_source[] =
    "<svg xmlns='http://www.w3.org/2000/svg' width='250' height='250' "
    "viewBox='0 0 250 250'><path id='curve' "
    "d='M20 100 L100 100 L100 180 Z' fill='#aa3344' stroke='#111111' "
    "stroke-width='2'/><rect id='second' x='120' y='120' width='20' "
    "height='20' fill='#3366cc' stroke='#111111'/></svg>";

static bool decode_source(const uint8_t *bytes, size_t length,
                          svg_shape_document *document)
{
    svg_shape_diagnostic diagnostic = {0};
    return svg_shape_decode((base_bytes){bytes, length},
        document, &diagnostic) == SVG_SHAPE_ERROR_NONE;
}

static bool decode_document(svg_shape_document *document)
{
    return decode_source(source, sizeof(source) - 1U, document);
}

static viewer_editor *create_zoom_editor(void)
{
    return viewer_editor_create_styled(
        (base_bytes){zoom_source, sizeof(zoom_source) - 1U}, UI_THEME_LIGHT,
        1280U, 800U, true);
}

static bool select_first_zoom_shape(viewer_editor *editor)
{
    const char *rect = strstr((const char *)zoom_source, "<rect");
    return rect != NULL && viewer_editor_select_at_source_offset(editor,
        (size_t)(rect - (const char *)zoom_source));
}

static bool initialize_navigation(ui_viewer_navigation *navigation,
    double zoom, uint32_t size)
{
    if (!ui_viewer_navigation_init(navigation, size, size, 400U, 400U)) return false;
    navigation->scale = zoom;
    navigation->pan_x = 0.0;
    navigation->pan_y = 0.0;
    return true;
}

static bool make_many_shape_fixture(uint8_t **out_bytes, size_t *out_length)
{
    if (out_bytes == NULL || out_length == NULL) return false;
    uint8_t *bytes = malloc(LARGE_SHAPE_FIXTURE_CAP);
    if (bytes == NULL) return false;
    size_t cursor = 0U;
    int amount = snprintf((char *)bytes, LARGE_SHAPE_FIXTURE_CAP,
        "<svg xmlns='http://www.w3.org/2000/svg' width='400' height='200' "
        "viewBox='0 0 400 200'><rect id='selected' x='20' y='20' "
        "width='10' height='10' fill='#cc3344'/>");
    if (amount <= 0 || (size_t)amount >= LARGE_SHAPE_FIXTURE_CAP) {
        free(bytes);
        return false;
    }
    cursor = (size_t)amount;
    for (size_t index = 0U; index < 160U; ++index) {
        size_t x = 41U + index * 2U;
        amount = snprintf((char *)bytes + cursor,
            LARGE_SHAPE_FIXTURE_CAP - cursor,
            "<rect id='near%zu' x='%zu' y='80' width='10' height='10' "
            "fill='#3366cc'/>", index, x);
        if (amount <= 0 || (size_t)amount >= LARGE_SHAPE_FIXTURE_CAP - cursor) {
            free(bytes);
            return false;
        }
        cursor += (size_t)amount;
    }
    static const char ending[] = "</svg>";
    if (sizeof(ending) - 1U >= LARGE_SHAPE_FIXTURE_CAP - cursor) {
        free(bytes);
        return false;
    }
    memcpy(bytes + cursor, ending, sizeof(ending) - 1U);
    cursor += sizeof(ending) - 1U;
    *out_bytes = bytes;
    *out_length = cursor;
    return true;
}

static int test_outline_selection_activates_canvas_feedback(void)
{
    viewer_editor *editor = viewer_editor_create_styled(
        (base_bytes){source, sizeof(source) - 1U}, UI_THEME_LIGHT,
        1280U, 800U, true);
    svg_shape_document document = {0};
    CHECK(editor != NULL && decode_document(&document), "create fixture editor");
    viewer_editor_resize(editor, 1280U, 800U, 1.0);
    const char *rect = strstr((const char *)source, "<rect");
    CHECK(rect != NULL && viewer_editor_select_at_source_offset(editor,
        (size_t)(rect - (const char *)source)), "select layer row object");
    viewer_editor_canvas_overlay_state overlay;
    CHECK(viewer_editor_get_canvas_overlay(editor, &document, &overlay) &&
        overlay.visible, "outline selection exposes the canvas selection outline");
    svg_shape_rect bounds = {0};
    CHECK(viewer_editor_get_selection_bounds(editor, &document, &bounds) &&
        bounds.x == 20.0 && bounds.y == 20.0 && bounds.width == 10.0 &&
        bounds.height == 10.0, "outline selection bounds match decoded canvas bounds");
    ui_viewer_navigation navigation = {0};
    CHECK(ui_viewer_navigation_init(&navigation, 400U, 400U,
        document.width, document.height) && ui_viewer_navigation_fit(&navigation),
        "initialize selection feedback canvas");
    CHECK((viewer_editor_canvas_pointer_down(editor, &document, &navigation,
        25.0 * navigation.scale, 25.0 * navigation.scale, 0U) &
        VIEWER_EDITOR_HANDLED) != 0U,
        "select the same element from the canvas");
    (void)viewer_editor_canvas_pointer_up(editor, &document);
    svg_shape_rect canvas_bounds = {0};
    CHECK(viewer_editor_get_selection_bounds(editor, &document, &canvas_bounds) &&
        canvas_bounds.x == bounds.x && canvas_bounds.y == bounds.y &&
        canvas_bounds.width == bounds.width && canvas_bounds.height == bounds.height,
        "canvas and Layers-pane selection expose matching bounds");
    svg_shape_document_destroy(&document);
    viewer_editor_destroy(editor);
    return 0;
}

static int test_drag_chooses_one_snap_relationship(void)
{
    viewer_editor *editor = viewer_editor_create_styled(
        (base_bytes){source, sizeof(source) - 1U}, UI_THEME_LIGHT,
        1280U, 800U, true);
    svg_shape_document document = {0};
    CHECK(editor != NULL && decode_document(&document), "create snap fixture editor");
    const char *rect = strstr((const char *)source, "<rect");
    CHECK(rect != NULL && viewer_editor_select_at_source_offset(editor,
        (size_t)(rect - (const char *)source)), "select snap fixture object");
    svg_shape_selection selection;
    CHECK(editor_canvas_selection_bounds(editor, &document, &selection),
        "resolve selected bounds");
    editor->canvas_selection = selection;
    editor->canvas_selection_valid = true;
    double dx = 10.5;
    double dy = 11.5;
    editor_canvas_snap_translation(editor, &document, 1.0, &dx, &dy);
    unsigned visible = (editor->snap_guide_x_visible ? 1U : 0U) +
        (editor->snap_guide_y_visible ? 1U : 0U);
    CHECK(visible <= 1U && editor->snap_label_visible,
        "a drag exposes one chosen snap relationship and one label");
    svg_shape_document_destroy(&document);
    viewer_editor_destroy(editor);
    return 0;
}

static int test_snap_tolerance_is_six_logical_points(void)
{
    const double zooms[] = {0.25, 1.0, 8.0};
    for (size_t zoom_index = 0U; zoom_index < sizeof(zooms) / sizeof(zooms[0]);
         ++zoom_index) {
        for (size_t distance_index = 0U; distance_index < 2U; ++distance_index) {
            viewer_editor *editor = create_zoom_editor();
            svg_shape_document document = {0};
            ui_viewer_navigation navigation = {0};
            CHECK(editor != NULL && decode_source(zoom_source,
                sizeof(zoom_source) - 1U, &document), "create zoom fixture");
            viewer_editor_resize(editor, 1280U, 800U, 1.0);
            CHECK(select_first_zoom_shape(editor) &&
                initialize_navigation(&navigation, zooms[zoom_index], 400U),
                "select zoom object and initialize navigation");
            editor->snap_grid_enabled = false;
            editor->snap_objects_enabled = true;
            editor->snap_guides_enabled = false;
            double logical_distance = distance_index == 0U ? 5.0 : 7.0;
            double start_x = 60.0, start_y = 240.0;
            double raw_dx = 60.0 - logical_distance / zooms[zoom_index];
            CHECK((viewer_editor_canvas_pointer_down(editor, &document,
                &navigation, start_x * zooms[zoom_index],
                start_y * zooms[zoom_index], 0U) & VIEWER_EDITOR_HANDLED) != 0U,
                "begin zoomed selection drag");
            CHECK((viewer_editor_canvas_pointer_move(editor, &document,
                &navigation, (start_x + raw_dx) * zooms[zoom_index],
                start_y * zooms[zoom_index]) & VIEWER_EDITOR_HANDLED) != 0U,
                "move selection near object edge");
            bool snapped = editor->snap_guide_x_visible || editor->snap_guide_y_visible;
            bool expected = distance_index == 0U;
            CHECK(snapped == expected && editor->snap_label_visible == expected,
                "snap activation stays within six logical points at 25%, 100%, and 800%");
            if (expected)
                CHECK(fabs(editor->gesture_current_x - 120.0) < 0.000001,
                    "in-tolerance move aligns the selected right edge");
            else
                CHECK(fabs(editor->gesture_current_x -
                    (120.0 - logical_distance / zooms[zoom_index])) < 0.000001,
                    "out-of-tolerance move keeps the unsnapped coordinate");
            (void)viewer_editor_cancel_canvas_gesture(editor);
            svg_shape_document_destroy(&document);
            viewer_editor_destroy(editor);
        }
    }
    return 0;
}

static int test_shift_bypass_and_cancel_preserve_source(void)
{
    viewer_editor *editor = create_zoom_editor();
    svg_shape_document document = {0};
    ui_viewer_navigation navigation = {0};
    CHECK(editor != NULL && decode_source(zoom_source,
        sizeof(zoom_source) - 1U, &document), "create Shift bypass fixture");
    viewer_editor_resize(editor, 1280U, 800U, 1.0);
    CHECK(select_first_zoom_shape(editor) &&
        initialize_navigation(&navigation, 1.0, 400U),
        "select Shift bypass object");
    editor->snap_grid_enabled = false;
    editor->snap_objects_enabled = true;
    editor->snap_guides_enabled = false;
    CHECK((viewer_editor_canvas_pointer_down(editor, &document, &navigation,
        60.0, 240.0, 0U) & VIEWER_EDITOR_HANDLED) != 0U,
        "begin Shift bypass drag");
    CHECK((viewer_editor_canvas_pointer_move_modifiers(editor, &document,
        &navigation, 115.0, 240.0, PLATFORM_MODIFIER_SHIFT) &
        VIEWER_EDITOR_HANDLED) != 0U, "move with Shift held");
    CHECK(fabs(editor->gesture_current_x - 115.0) < 0.000001 &&
        !editor->snap_label_visible && !editor->snap_guide_x_visible &&
        !editor->snap_guide_y_visible,
        "Shift bypasses every snap candidate for this pointer gesture");
    size_t before_length = 0U;
    uint8_t before[sizeof(zoom_source) + 64U];
    CHECK(viewer_editor_materialize(editor, before, sizeof(before), &before_length) &&
        before_length == sizeof(zoom_source) - 1U &&
        memcmp(before, zoom_source, before_length) == 0,
        "preview drag has not changed SVG source");
    size_t history_before = document_edit_history_count(editor->edit);
    uint32_t canceled = viewer_editor_key(editor, 53U, 0U, NULL, 0U);
    CHECK((canceled & VIEWER_EDITOR_HANDLED) != 0U &&
        !viewer_editor_is_dirty(editor) &&
        document_edit_history_count(editor->edit) == history_before,
        "Escape cancels the transform without source mutation or undo entry");
    size_t after_length = 0U;
    uint8_t after[sizeof(zoom_source) + 64U];
    CHECK(viewer_editor_materialize(editor, after, sizeof(after), &after_length) &&
        before_length == after_length && memcmp(before, after, after_length) == 0,
        "Escape leaves exact source bytes unchanged");
    CHECK((viewer_editor_canvas_pointer_down(editor, &document, &navigation,
        60.0, 240.0, 0U) & VIEWER_EDITOR_HANDLED) != 0U &&
        (viewer_editor_canvas_pointer_move(editor, &document, &navigation,
            115.0, 240.0) & VIEWER_EDITOR_HANDLED) != 0U,
        "begin a fresh transform for release history verification");
    uint32_t released = viewer_editor_canvas_pointer_up(editor, &document);
    CHECK((released & VIEWER_EDITOR_MUTATED) != 0U &&
        document_edit_history_count(editor->edit) == history_before + 1U &&
        viewer_editor_is_dirty(editor),
        "release commits the transform as one undo step");
    svg_shape_document_destroy(&document);
    viewer_editor_destroy(editor);
    return 0;
}

static int test_resize_handles_and_cursor_zones_at_all_zooms(void)
{
    const double zooms[] = {0.25, 1.0, 8.0};
    const platform_cursor_kind expected[] = {
        PLATFORM_CURSOR_RESIZE_NORTHWEST_SOUTHEAST,
        PLATFORM_CURSOR_RESIZE_NORTH_SOUTH,
        PLATFORM_CURSOR_RESIZE_NORTHEAST_SOUTHWEST,
        PLATFORM_CURSOR_RESIZE_EAST_WEST,
        PLATFORM_CURSOR_RESIZE_NORTHWEST_SOUTHEAST,
        PLATFORM_CURSOR_RESIZE_NORTH_SOUTH,
        PLATFORM_CURSOR_RESIZE_NORTHEAST_SOUTHWEST,
        PLATFORM_CURSOR_RESIZE_EAST_WEST,
        PLATFORM_CURSOR_ROTATE
    };
    viewer_editor *editor = create_zoom_editor();
    svg_shape_document document = {0};
    CHECK(editor != NULL && decode_source(zoom_source,
        sizeof(zoom_source) - 1U, &document), "create handle fixture");
    viewer_editor_resize(editor, 1280U, 800U, 1.0);
    CHECK(select_first_zoom_shape(editor), "select handle fixture object");
    svg_shape_selection selection;
    CHECK(editor_canvas_selection_bounds(editor, &document, &selection),
        "resolve handle fixture bounds");
    for (size_t zoom_index = 0U; zoom_index < sizeof(zooms) / sizeof(zooms[0]);
         ++zoom_index) {
        ui_viewer_navigation navigation = {0};
        CHECK(initialize_navigation(&navigation, zooms[zoom_index], 400U),
            "initialize handle navigation");
        double left = selection.device_bounds.x * zooms[zoom_index];
        double top = selection.device_bounds.y * zooms[zoom_index];
        double right = left + selection.device_bounds.width * zooms[zoom_index];
        double bottom = top + selection.device_bounds.height * zooms[zoom_index];
        geom_point handles[9] = {
            {left, top}, {(left + right) * 0.5, top}, {right, top},
            {right, (top + bottom) * 0.5}, {right, bottom},
            {(left + right) * 0.5, bottom}, {left, bottom},
            {left, (top + bottom) * 0.5},
            {(left + right) * 0.5, top - 22.0}
        };
        for (size_t handle = 0U; handle < 9U; ++handle) {
            CHECK(viewer_editor_canvas_cursor(editor, &document, &navigation,
                handles[handle].x, handles[handle].y) == expected[handle],
                "cursor orientation matches each resize or rotate handle");
            CHECK((viewer_editor_canvas_pointer_down(editor, &document,
                &navigation, handles[handle].x, handles[handle].y, 0U) &
                VIEWER_EDITOR_HANDLED) != 0U && editor->resize_handle == handle,
                "each constant-size handle remains separately hit-testable");
            (void)viewer_editor_cancel_canvas_gesture(editor);
        }
        CHECK(viewer_editor_canvas_cursor(editor, &document, &navigation,
            60.0 * zooms[zoom_index], 240.0 * zooms[zoom_index]) ==
                PLATFORM_CURSOR_OPEN_HAND,
            "selected artwork shows the move cursor at every zoom");
    }
    svg_shape_document_destroy(&document);
    viewer_editor_destroy(editor);
    return 0;
}

static int test_snap_popover_independent_toggles_and_master_shortcut(void)
{
    viewer_editor *editor = create_zoom_editor();
    CHECK(editor != NULL, "create snapping control fixture");
    ui_semantics_snapshot snapshot;
    CHECK(viewer_editor_semantics(editor, &snapshot), "build snapping options row");
    const ui_node *trigger = NULL;
    for (size_t index = 0U; index < editor->tree.count; ++index)
        if (editor->tree.nodes[index].spec.id == EDITOR_BUTTON_SNAPPING)
            trigger = &editor->tree.nodes[index];
    CHECK(trigger != NULL, "snapping popover trigger is present");
    uint32_t trigger_x = trigger->bounds.x + trigger->bounds.width / 2U;
    uint32_t trigger_y = trigger->bounds.y + trigger->bounds.height / 2U;
    CHECK((viewer_editor_pointer_down(editor, trigger_x, trigger_y, 0U) &
        VIEWER_EDITOR_HANDLED) != 0U && editor->snap_popover_open,
        "Snapping button opens the category popover");
    viewer_editor_pointer_up(editor);
    uint32_t panel_x = trigger->bounds.x;
    uint32_t panel_y = trigger->bounds.y + trigger->bounds.height + 4U;
    uint32_t row_x = panel_x + 16U;
    uint32_t grid_y = panel_y + 28U + 15U;
    uint32_t objects_y = grid_y + 30U;
    editor_snap_popover_pointer_move(editor, row_x, grid_y);
    CHECK(editor->snap_popover_hovered &&
        editor->snap_popover_hovered_row == 0U,
        "Grid row receives its own popover hover state");
    editor_snap_popover_pointer_move(editor, row_x, objects_y);
    CHECK(editor->snap_popover_hovered_row == 1U,
        "hover state follows the active popover row");
    CHECK(editor_snap_popover_pointer_down(editor, row_x, grid_y) &&
        !editor->snap_grid_enabled && editor->snap_objects_enabled &&
        editor->snap_guides_enabled,
        "Grid toggle changes without changing Objects or Guides");
    CHECK(editor_snap_popover_pointer_down(editor, row_x, objects_y) &&
        !editor->snap_grid_enabled && !editor->snap_objects_enabled &&
        editor->snap_guides_enabled,
        "Objects toggle changes independently of Grid and Guides");
    uint32_t guides_y = objects_y + 30U;
    CHECK(editor_snap_popover_pointer_down(editor, row_x, guides_y) &&
        !editor->snap_grid_enabled && !editor->snap_objects_enabled &&
        !editor->snap_guides_enabled,
        "Guides toggle changes independently of Grid and Objects");
    CHECK((viewer_editor_key(editor, 53U, 0U, NULL, 0U) &
        VIEWER_EDITOR_HANDLED) != 0U && !editor->snap_popover_open &&
        editor->snap_popover_hovered_row == UINT8_MAX,
        "Escape closes the snapping settings popover");
    CHECK((viewer_editor_key(editor, 39U, PLATFORM_MODIFIER_COMMAND,
        NULL, 0U) & VIEWER_EDITOR_HANDLED) != 0U && !editor->snap_enabled,
        "existing master snapping shortcut still toggles all snap sources");
    viewer_editor_destroy(editor);
    return 0;
}

static int test_guide_cap_and_large_shape_candidate_bound(void)
{
    viewer_editor *editor = create_zoom_editor();
    svg_shape_document document = {0};
    CHECK(editor != NULL && decode_source(zoom_source,
        sizeof(zoom_source) - 1U, &document), "create guide cap fixture");
    CHECK(select_first_zoom_shape(editor), "select guide cap object");
    svg_shape_selection selection;
    CHECK(editor_canvas_selection_bounds(editor, &document, &selection),
        "resolve guide cap bounds");
    editor->canvas_selection = selection;
    editor->canvas_selection_valid = true;
    for (size_t index = 0U; index < EDITOR_SNAP_GUIDE_CAP; ++index)
        CHECK(editor_canvas_snap_guide_add(editor, true, (double)index),
            "store bounded snap guide");
    CHECK(editor->snap_guide_count == EDITOR_SNAP_GUIDE_CAP &&
        editor_canvas_snap_guide_add(editor, true, 100.0) &&
        editor->snap_guide_count == EDITOR_SNAP_GUIDE_CAP &&
        !editor_canvas_snap_guide_add(editor, false, NAN),
        "guide storage caps at 128 unique finite guide candidates");
    editor->snap_grid_enabled = false;
    editor->snap_objects_enabled = false;
    editor->snap_guides_enabled = true;
    double dx = 69.5, dy = 0.0;
    editor_canvas_snap_translation(editor, &document, 1.0, &dx, &dy);
    CHECK(editor->snap_guide_x_visible && !editor->snap_guide_y_visible &&
        editor->snap_label_visible && fabs(dx - 69.0) < 0.000001,
        "capped guide candidates choose one deterministic nearest guide");
    editor_canvas_snap_guide_clear(editor);
    CHECK(editor->snap_guide_count == 0U, "guide candidate reset is bounded");
    svg_shape_document_destroy(&document);
    viewer_editor_destroy(editor);
    return 0;
}

static int test_large_shape_snap_scan_is_bounded(void)
{
    uint8_t *bytes = NULL;
    size_t length = 0U;
    CHECK(make_many_shape_fixture(&bytes, &length),
        "create bounded 160-shape snapping fixture");
    viewer_editor *editor = viewer_editor_create_styled(
        (base_bytes){bytes, length}, UI_THEME_LIGHT, 900U, 700U, true);
    svg_shape_document document = {0};
    CHECK(editor != NULL && decode_source(bytes, length, &document),
        "decode many-shape snapping fixture");
    const char *selected = strstr((const char *)bytes, "<rect");
    CHECK(selected != NULL && viewer_editor_select_at_source_offset(editor,
        (size_t)(selected - (const char *)bytes)), "select first nearby shape");
    svg_shape_selection selection;
    CHECK(editor_canvas_selection_bounds(editor, &document, &selection),
        "resolve many-shape selection");
    editor->canvas_selection = selection;
    editor->canvas_selection_valid = true;
    editor->snap_grid_enabled = false;
    editor->snap_objects_enabled = true;
    editor->snap_guides_enabled = false;
    CHECK(document.shape_count == 161U,
        "large fixture contains more shapes than the fixed candidate window");
    for (size_t iteration = 0U; iteration < 32U; ++iteration) {
        double dx = 10.5, dy = 0.0;
        editor_canvas_snap_translation(editor, &document, 1.0, &dx, &dy);
        CHECK(editor->snap_guide_x_visible && !editor->snap_guide_y_visible &&
            editor->snap_label_visible && fabs(dx - 11.0) < 0.000001,
            "large nearby shape set resolves one stable snap without per-event scratch allocation");
    }
    for (size_t guide = 0U; guide < EDITOR_SNAP_GUIDE_CAP; ++guide)
        CHECK(editor_canvas_snap_guide_add(editor, true, 30.0 + (double)guide),
            "add guide candidates to the large document case");
    CHECK(editor->snap_guide_count == EDITOR_SNAP_GUIDE_CAP &&
        !editor_canvas_snap_guide_add(editor, true, 1000.0) &&
        editor->snap_guide_count == EDITOR_SNAP_GUIDE_CAP,
        "large document keeps the guide candidate store capped at 128");
    editor->snap_objects_enabled = false;
    editor->snap_guides_enabled = true;
    for (size_t iteration = 0U; iteration < 32U; ++iteration) {
        double dx = 10.5, dy = 0.0;
        editor_canvas_snap_translation(editor, &document, 1.0, &dx, &dy);
        CHECK(editor->snap_guide_x_visible && !editor->snap_guide_y_visible &&
            editor->snap_label_visible && fabs(dx - 10.0) < 0.000001,
            "large document resolves a stable capped guide candidate without allocation");
    }
    svg_shape_document_destroy(&document);
    viewer_editor_destroy(editor);
    free(bytes);
    return 0;
}

static bool rect_intersects(ui_rect left, ui_rect right);

static int test_context_actions_clipped_and_avoid_pointer(void)
{
    viewer_editor *editor = viewer_editor_create_styled(
        (base_bytes){action_source, sizeof(action_source) - 1U},
        UI_THEME_LIGHT, 1280U, 800U, true);
    svg_shape_document document = {0};
    CHECK(editor != NULL && decode_source(action_source,
        sizeof(action_source) - 1U, &document), "create contextual action fixture");
    viewer_editor_resize(editor, 1280U, 800U, 1.0);
    const char *path = strstr((const char *)action_source, "<path");
    CHECK(path != NULL && viewer_editor_select_at_source_offset(editor,
        (size_t)(path - (const char *)action_source)), "select path for contextual actions");
    viewer_chrome_layout layout;
    CHECK(viewer_editor_get_chrome_layout(editor, &layout), "resolve canvas layout");
    ui_viewer_navigation navigation = {0};
    CHECK(ui_viewer_navigation_init(&navigation, layout.canvas.width,
        layout.canvas.height, document.width, document.height) &&
        ui_viewer_navigation_fit(&navigation), "fit contextual action canvas");
    uint32_t width = 1280U, height = 800U;
    size_t stride = (size_t)width * 4U;
    uint8_t *rgba = calloc((size_t)height, stride);
    CHECK(rgba != NULL, "allocate headless action canvas");
    viewer_editor_paint_canvas_overlay(editor, &document, &navigation,
        layout.canvas, rgba, width, height, stride);
    CHECK(editor->context_action_count == 3U &&
        editor->context_actions[0].id == EDITOR_CONTEXT_EDIT_NODES &&
        editor->context_actions[1].id == EDITOR_CONTEXT_FILL &&
        editor->context_actions[2].id == EDITOR_CONTEXT_STROKE &&
        !editor->context_actions_fallback,
        "a selected path exposes its three supported immediate actions");
    svg_shape_selection selection;
    CHECK(editor_canvas_selection_bounds(editor, &document, &selection),
        "resolve contextual selection bounds");
    ui_rect selected = {
        layout.canvas.x + (uint32_t)(navigation.pan_x +
            selection.device_bounds.x * navigation.scale),
        layout.canvas.y + (uint32_t)(navigation.pan_y +
            selection.device_bounds.y * navigation.scale),
        (uint32_t)(selection.device_bounds.width * navigation.scale),
        (uint32_t)(selection.device_bounds.height * navigation.scale)};
    for (size_t index = 0U; index < editor->context_action_count; ++index)
        CHECK(!rect_intersects(selected, editor->context_actions[index].bounds),
            "context action buttons do not cover the selected bounds");
    ui_rect first = editor->context_actions[0].bounds;
    uint32_t pointer_x = first.x + first.width / 2U;
    uint32_t pointer_y = first.y + first.height / 2U;
    viewer_editor_pointer_move(editor, pointer_x, pointer_y);
    CHECK(editor->context_action_hovered == EDITOR_CONTEXT_EDIT_NODES,
        "context action pointer receives a visible hover state");
    CHECK((viewer_editor_canvas_pointer_move_modifiers(editor, &document,
        &navigation, pointer_x - layout.canvas.x, pointer_y - layout.canvas.y,
        0U) & VIEWER_EDITOR_HANDLED) == 0U,
        "hovering a contextual action does not begin a gesture");
    viewer_editor_paint_canvas_overlay(editor, &document, &navigation,
        layout.canvas, rgba, width, height, stride);
    bool pointer_covered = false;
    for (size_t index = 0U; index < editor->context_action_count; ++index) {
        ui_rect bounds = editor->context_actions[index].bounds;
        if (pointer_x >= bounds.x && pointer_y >= bounds.y &&
            pointer_x - bounds.x < bounds.width &&
            pointer_y - bounds.y < bounds.height) pointer_covered = true;
    }
    CHECK(!pointer_covered, "context strip moves away from the pointer");
    uint32_t fill_x = editor->context_actions[1].bounds.x +
        editor->context_actions[1].bounds.width / 2U;
    uint32_t fill_y = editor->context_actions[1].bounds.y +
        editor->context_actions[1].bounds.height / 2U;
    CHECK(viewer_editor_context_action_at(editor, fill_x, fill_y) ==
        EDITOR_CONTEXT_FILL,
        "context action hit target is independent of canvas shape hit testing");
    CHECK((viewer_editor_context_action_run(editor, &document,
        EDITOR_CONTEXT_FILL) & VIEWER_EDITOR_HANDLED) != 0U &&
        editor->field_kind == EDITOR_FIELD_ATTRIBUTE &&
        strcmp(editor->selected_attribute_name, "fill") == 0,
        "Fill action opens the selected fill attribute in Inspector");
    free(rgba);
    svg_shape_document_destroy(&document);
    viewer_editor_destroy(editor);
    return 0;
}

static bool rect_intersects(ui_rect left, ui_rect right)
{
    return left.x < right.x + right.width && right.x < left.x + left.width &&
        left.y < right.y + right.height && right.y < left.y + left.height;
}

static int test_multi_action_strip_falls_back_to_options_row(void)
{
    viewer_editor *editor = viewer_editor_create_styled(
        (base_bytes){action_source, sizeof(action_source) - 1U},
        UI_THEME_DARK, 350U, 130U, true);
    svg_shape_document document = {0};
    CHECK(editor != NULL && decode_source(action_source,
        sizeof(action_source) - 1U, &document), "create narrow contextual fixture");
    viewer_editor_resize(editor, 350U, 130U, 1.0);
    const char *path = strstr((const char *)action_source, "<path");
    const char *rect = strstr((const char *)action_source, "<rect");
    CHECK(path != NULL && rect != NULL && viewer_editor_select_at_source_offset(editor,
        (size_t)(path - (const char *)action_source)) &&
        editor_canvas_selection_toggle(editor,
            (size_t)(rect - (const char *)action_source)),
        "select two siblings for multi-selection actions");
    viewer_chrome_layout layout;
    CHECK(viewer_editor_get_chrome_layout(editor, &layout),
        "resolve narrow canvas layout");
    ui_viewer_navigation navigation = {0};
    CHECK(ui_viewer_navigation_init(&navigation, layout.canvas.width,
        layout.canvas.height, document.width, document.height),
        "initialize narrow selection viewport");
    uint32_t width = 350U, height = 130U;
    size_t stride = (size_t)width * 4U;
    uint8_t *rgba = calloc((size_t)height, stride);
    CHECK(rgba != NULL, "allocate narrow headless frame");
    viewer_editor_paint_canvas_overlay(editor, &document, &navigation,
        layout.canvas, rgba, width, height, stride);
    CHECK(editor->context_actions_fallback && editor->context_action_count == 4U,
        "multi-selection actions fall back when the canvas is too short");
    for (size_t index = 0U; index < editor->context_action_count; ++index) {
        ui_rect bounds = editor->context_actions[index].bounds;
        CHECK(bounds.y >= layout.options.y &&
            (uint64_t)bounds.y + bounds.height <=
                (uint64_t)layout.options.y + layout.options.height,
            "fallback actions remain clipped to the options row");
        CHECK(viewer_editor_context_action_at(editor,
            bounds.x + bounds.width / 2U, bounds.y + bounds.height / 2U) ==
            editor->context_actions[index].id,
            "fallback action has an independent pointer hit target");
    }
    CHECK(editor->context_actions[0].id == EDITOR_CONTEXT_ALIGN &&
        editor->context_actions[1].id == EDITOR_CONTEXT_GROUP &&
        editor->context_actions[2].id == EDITOR_CONTEXT_DUPLICATE &&
        editor->context_actions[3].id == EDITOR_CONTEXT_DELETE,
        "multi-selection strip exposes Align, Group, Duplicate, and Delete");
    free(rgba);
    svg_shape_document_destroy(&document);
    viewer_editor_destroy(editor);
    return 0;
}

int main(void)
{
    int failed = 0;
    failed |= test_outline_selection_activates_canvas_feedback();
    failed |= test_drag_chooses_one_snap_relationship();
    failed |= test_snap_tolerance_is_six_logical_points();
    failed |= test_shift_bypass_and_cancel_preserve_source();
    failed |= test_resize_handles_and_cursor_zones_at_all_zooms();
    failed |= test_snap_popover_independent_toggles_and_master_shortcut();
    failed |= test_guide_cap_and_large_shape_candidate_bound();
    failed |= test_large_shape_snap_scan_is_bounded();
    failed |= test_context_actions_clipped_and_avoid_pointer();
    failed |= test_multi_action_strip_falls_back_to_options_row();
    if (failed != 0) return 1;
    (void)puts("editor UX selection and snap regressions pass");
    return 0;
}
