#include "viewer_timeline.h"
#include "platform/platform_text_ui.h"
#include "ui/ui_widgets.h"
#include "viewer_anim_edit.h"
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
bool viewer_editor_visibility_test_run(void);
#define CHECK(expression) do { \
    if (!(expression)) { \
        (void)fprintf(stderr, "%s:%d: check failed: %s\n", __FILE__, __LINE__, #expression); \
        return false; \
    } \
} while (0)
static ui_semantics_snapshot semantics_scratch;
static bool decode(const char *source, svg_shape_document *document)
{
    svg_shape_diagnostic diagnostic = {0};
    svg_shape_error error = svg_shape_decode(
        (base_bytes){(const uint8_t *)source, strlen(source)}, document, &diagnostic);
    if (error != SVG_SHAPE_ERROR_NONE)
        (void)fprintf(stderr, "viewer native timeline fixture decode failed: %d at %zu\n",
            (int)error, diagnostic.offset);
    return error == SVG_SHAPE_ERROR_NONE;
}
static bool snapshot_has_label(const ui_semantics_snapshot *snapshot,
    ui_role role, const char *expected)
{
    size_t expected_length = strlen(expected);
    for (size_t index = 0U; index < snapshot->count; ++index) {
        const ui_semantics_node *node = &snapshot->nodes[index];
        const char *label = NULL;
        size_t length = 0U;
        if (node->role == role && ui_semantics_text(snapshot, node, false,
                &label, &length) && length == expected_length &&
            memcmp(label, expected, length) == 0) return true;
    }
    return false;
}
static bool snapshot_has_value_fragment(const ui_semantics_snapshot *snapshot,
    ui_role role, const char *fragment, bool require_disabled)
{
    size_t fragment_length = strlen(fragment);
    for (size_t index = 0U; index < snapshot->count; ++index) {
        const ui_semantics_node *node = &snapshot->nodes[index];
        if (node->role != role ||
            (require_disabled && (node->state & UI_STATE_DISABLED) == 0U)) continue;
        const char *value = NULL;
        size_t value_length = 0U;
        if (!ui_semantics_text(snapshot, node, true, &value, &value_length) ||
            value_length < fragment_length) continue;
        for (size_t offset = 0U; offset <= value_length - fragment_length; ++offset) {
            if (memcmp(value + offset, fragment, fragment_length) == 0) return true;
        }
    }
    return false;
}
static bool check_transform_source(const char *source, bool expected,
    const double expected_matrix[6])
{
    viewer_editor *editor = viewer_editor_create(
        (base_bytes){(const uint8_t *)source, strlen(source)},
        UI_THEME_DARK, 640U, 480U);
    CHECK(editor != NULL);
    const char *element = strstr(source, "<g");
    CHECK(element != NULL && viewer_editor_select_at_source_offset(editor,
        (size_t)(element - source) + 1U));
    uint8_t id[32U]; size_t id_length = 0U; double matrix[6] = {0};
    bool actual = viewer_editor_selected_transform_target(editor, id, sizeof(id),
        &id_length, matrix);
    CHECK(actual == expected);
    if (actual && expected_matrix != NULL)
        for (size_t i = 0U; i < 6U; ++i)
            CHECK(fabs(matrix[i] - expected_matrix[i]) < 1.0e-12);
    viewer_editor_destroy(editor);
    return true;
}
static bool check_paint_source(const char *source, const char *property,
    bool expected, const double rgba_expected[4])
{
    viewer_editor *editor = viewer_editor_create(
        (base_bytes){(const uint8_t *)source, strlen(source)},
        UI_THEME_DARK, 640U, 480U);
    CHECK(editor != NULL);
    const char *element = strstr(source, "<g");
    CHECK(element != NULL && viewer_editor_select_at_source_offset(editor,
        (size_t)(element - source) + 1U));
    uint8_t id[32U]; size_t id_length = 0U; double rgba[4] = {0};
    bool actual = viewer_editor_selected_paint_target(editor, property, id,
        sizeof(id), &id_length, rgba);
    CHECK(actual == expected);
    if (actual) for (size_t i = 0U; i < 4U; ++i)
        CHECK(fabs(rgba[i] - rgba_expected[i]) < 1.0e-12);
    viewer_editor_destroy(editor);
    return true;
}
static bool test_transform_capture_parser(void)
{
    static const double identity[6] = {1, 0, 0, 1, 0, 0};
    static const double matrix[6] = {1, 0, 0, 1, 2.5, -3};
    CHECK(check_transform_source(
        "<svg width='8' height='8'><g id='x'/></svg>", true, identity));
    CHECK(check_transform_source(
        "<svg width='8' height='8'><g id='x' transform='matrix( +1e0, 0, 0 1 2.5 -3 )'/></svg>",
        true, matrix));
    CHECK(check_transform_source(
        "<svg width='8' height='8'><g id='x' transform='translate(1 2)'/></svg>",
        false, NULL));
    CHECK(check_transform_source(
        "<svg width='8' height='8'><g id='x' transform='matrix(1 0 0 1 0)'/></svg>",
        false, NULL));
    CHECK(check_transform_source(
        "<svg width='8' height='8'><g id='x' transform='matrix(1 0 0 1 NaN 0)'/></svg>",
        false, NULL));
    CHECK(check_transform_source(
        "<svg width='8' height='8'><g id='x' transform='matrix(1 0 0 1 1e999 0)'/></svg>",
        false, NULL));
    CHECK(check_transform_source(
        "<svg width='8' height='8'><g id='x' transform='matrix(1 0 0 1 1e10 0)'/></svg>",
        false, NULL));
    CHECK(check_transform_source(
        "<svg width='8' height='8'><g id='x' transform='matrix(0x1p0 0 0 1 0 0)'/></svg>",
        false, NULL));
    CHECK(check_transform_source(
        "<svg width='8' height='8'><g id='x' style='TRANSFORM:matrix(1 0 0 1 0 0)'/></svg>",
        false, NULL));
    CHECK(check_transform_source(
        "<svg width='8' height='8'><g id='x' style='transform:matrix(1 0 0 1 0 0)'/></svg>",
        false, NULL));
    CHECK(check_transform_source(
        "<svg width='8' height='8'><g id='x'/><g id='x'/></svg>", false, NULL));
    static const double alpha_hex[4] = {17.0 / 255.0, 34.0 / 255.0,
        51.0 / 255.0, 68.0 / 255.0};
    static const double named_black[4] = {0.0, 0.0, 0.0, 1.0};
    CHECK(check_paint_source(
        "<svg width='8' height='8'><g id='x' fill='#1234'/></svg>",
        "fill", true, alpha_hex));
    CHECK(check_paint_source(
        "<svg width='8' height='8'><g id='x' fill='black'/></svg>",
        "fill", true, named_black));
    CHECK(check_paint_source(
        "<svg width='8' height='8'><g id='x' fill='url(#paint)'/></svg>",
        "fill", false, NULL));
    CHECK(check_paint_source(
        "<svg width='8' height='8'><g id='x' fill='red' style='fill:blue'/></svg>",
        "fill", false, NULL));
    return true;
}
static bool test_native_timeline_playback_and_read_only_rows(void)
{
    static const char scene_source[] =
        "<svg width='2' height='2' viewBox='0 0 2 2'><g id='layer' opacity='0.75' "
        "transform='matrix(1 0 0 1 3 -2)' fill='#f00' stroke='blue'/></svg>";
    static const char timeline_source[] =
        "{\"schema\":\"vexel.animation\",\"version\":1,"
        "\"ticks_per_second\":24,\"duration_ticks\":24,"
        "\"fps\":{\"numerator\":2,\"denominator\":1},"
        "\"loop\":\"once\",\"onion\":{\"before\":0,\"after\":0,\"opacity\":0.5},"
        "\"tracks\":[{\"id\":\"pencil\",\"kind\":\"drawing\","
        "\"target\":\"layer\",\"name\":\"Pencil\",\"visible\":true,"
        "\"cels\":[{\"tick\":0,\"asset\":\"red\"},"
        "{\"tick\":12,\"asset\":\"green\"}]}]}";
    static const char red_svg[] =
        "<svg width='2' height='2' viewBox='0 0 2 2'>"
        "<rect width='1' height='2' fill='#ff0000'/></svg>";
    static const char green_svg[] =
        "<svg width='2' height='2' viewBox='0 0 2 2'>"
        "<rect x='1' width='1' height='2' fill='#00ff00'/></svg>";
    static const char mismatched_svg[] =
        "<svg width='3' height='2' viewBox='0 0 3 2'>"
        "<rect width='3' height='2' fill='#0000ff'/></svg>";
    static const uint8_t media[] = "image/svg+xml";
    static const uint8_t red_name[] = "red";
    static const uint8_t green_name[] = "green";
    static const uint8_t mismatched_name[] = "wide";
    project_bundle_asset assets[] = {
        {{red_name, sizeof(red_name) - 1U}, {media, sizeof(media) - 1U},
            {(const uint8_t *)red_svg, sizeof(red_svg) - 1U}},
        {{green_name, sizeof(green_name) - 1U}, {media, sizeof(media) - 1U},
            {(const uint8_t *)green_svg, sizeof(green_svg) - 1U}},
        {{mismatched_name, sizeof(mismatched_name) - 1U},
            {media, sizeof(media) - 1U},
            {(const uint8_t *)mismatched_svg, sizeof(mismatched_svg) - 1U}}
    };
    project_bundle_input input = {
        .manifest_json = {(const uint8_t *)"{}", 2U},
        .scene_svg = {(const uint8_t *)scene_source, sizeof(scene_source) - 1U},
        .timeline_json = {(const uint8_t *)timeline_source,
            sizeof(timeline_source) - 1U},
        .assets = assets, .asset_count = 3U
    };
    project_bundle_result measured = project_bundle_encoded_size(&input);
    CHECK(measured.error == PROJECT_BUNDLE_OK);
    uint8_t *encoded = malloc(measured.encoded_size);
    CHECK(encoded != NULL);
    CHECK(project_bundle_encode(&input,
        (base_mut_bytes){encoded, measured.encoded_size}).error == PROJECT_BUNDLE_OK);
    project_bundle_view bundle = {0};
    CHECK(project_bundle_decode((base_bytes){encoded, measured.encoded_size},
        &bundle).error == PROJECT_BUNDLE_OK);
    project_timeline_context_owner context = {0};
    CHECK(project_timeline_validate_bundle_owned(&bundle, bundle.scene_svg,
        NULL, NULL, NULL, &context).validation.error == PROJECT_TIMELINE_OK);
    project_timeline_session *session = NULL;
    CHECK(project_timeline_session_create(bundle.timeline_json, &context,
        project_timeline_edit_default_limits(), &session).error == PROJECT_TIMELINE_OK);
    svg_shape_document scene = {0};
    CHECK(decode(scene_source, &scene));
    viewer_timeline timeline;
    viewer_timeline_init(&timeline);
    CHECK(viewer_timeline_reset_native(&timeline, &scene, session, &bundle));
    CHECK(timeline.dock_open && viewer_timeline_track_count(&timeline) == 1U &&
        fabs(viewer_timeline_duration(&timeline) - 1.0) < 1.0e-9);
    char label[64] = {0};
    size_t label_length = 0U;
    CHECK(viewer_timeline_track_label(&timeline, 0U, label, sizeof(label),
        &label_length) && strcmp(label, "Pencil") == 0);
    char short_label[5] = {0};
    size_t short_length = 0U;
    CHECK(viewer_timeline_track_label(&timeline, 0U, short_label,
        sizeof(short_label), &short_length) && short_length == 4U &&
        strcmp(short_label, "Penc") == 0);
    uint32_t tick = UINT32_MAX;
    CHECK(viewer_timeline_seconds_to_native_tick(&timeline, 0.5, &tick) &&
        tick == 12U);
    CHECK(viewer_timeline_seconds_to_native_tick(&timeline, 1.0, &tick) &&
        tick == 23U);
    viewer_timeline tick_probe = timeline;
    tick_probe.native_ticks_per_second = 30U;
    tick_probe.native_duration_ticks = 180U;
    CHECK(viewer_timeline_seconds_to_native_tick(&tick_probe, 4.1, &tick) &&
        tick == 123U);
    CHECK(viewer_timeline_seconds_to_native_tick(&tick_probe,
        4.099999999, &tick) && tick == 122U);
    tick_probe.native_ticks_per_second = 60U;
    tick_probe.native_duration_ticks = 180U;
    CHECK(viewer_timeline_seconds_to_native_tick(&tick_probe, 2.05, &tick) &&
        tick == 123U);
    viewer_timeline_layout layout;
    CHECK(viewer_timeline_layout_compute(&timeline, 900U, 720U, 1.0, &layout));
    CHECK(layout.key_opacity_button.width > 0U &&
        layout.key_transform_button.width > 0U &&
        layout.key_fill_button.width > 0U &&
        layout.key_stroke_button.width > 0U &&
        layout.key_visibility_button.width > 0U);
    viewer_timeline_layout compact_layout;
    CHECK(viewer_timeline_layout_compute(&timeline, 450U, 720U, 1.0,
        &compact_layout));
    CHECK(compact_layout.compact && compact_layout.key_transform_button.width == 0U);
    CHECK(compact_layout.key_fill_button.width == 0U &&
        compact_layout.key_stroke_button.width == 0U &&
        compact_layout.key_visibility_button.width == 0U);
    CHECK(layout.drawing_asset_button.width > 0U && layout.add_cel_button.width > 0U);
    viewer_editor *editor = viewer_editor_create(
        (base_bytes){(const uint8_t *)scene_source, sizeof(scene_source) - 1U},
        UI_THEME_DARK, 900U, 720U);
    CHECK(editor != NULL);
    CHECK(viewer_timeline_pointer_ex(&timeline, &layout,
        VIEWER_TIMELINE_POINTER_DOWN,
        (double)layout.mode_button.x + 2.0,
        (double)layout.mode_button.y + 2.0, 0.0, 0.0, 0U, 0.0, editor));
    CHECK(timeline.mode == VIEWER_TIMELINE_MODE_LOOP &&
        viewer_timeline_take_native_mutation(&timeline) &&
        project_timeline_session_is_dirty(session));
    CHECK(project_timeline_session_undo(session).error == PROJECT_TIMELINE_OK &&
        viewer_timeline_refresh_native(&timeline) &&
        timeline.mode == VIEWER_TIMELINE_MODE_ONE_SHOT &&
        !project_timeline_session_is_dirty(session));
    size_t canvas_bytes = 900U * 720U * 4U;
    uint8_t *canvas = calloc(canvas_bytes, 1U);
    CHECK(canvas != NULL);
    CHECK(viewer_timeline_paint_scalar_appearance(&timeline, editor, canvas,
        canvas_bytes, 900U, 720U, 900U * 4U, 1.0, UI_THEME_DARK, false,
        false));
    free(canvas);
    CHECK(viewer_timeline_pointer_ex(&timeline, &layout,
        VIEWER_TIMELINE_POINTER_DOWN,
        (double)layout.track_rows[0].x + 2.0,
        (double)layout.track_rows[0].y + 2.0, 0.0, 0.0, 0U, 1.0,
        NULL));
    CHECK(timeline.selected_track == 0U && timeline.time_seconds == 0.0 &&
        !timeline.dragging_key && !timeline.edit_mutation_pending);
    double marker_y = (double)layout.track_rows[0].y +
        (double)layout.track_rows[0].height / 2.0;
    double source_marker_x = (double)layout.ruler.x + layout.ruler.width * 0.5;
    double target_marker_x = (double)layout.ruler.x +
        layout.ruler.width * (16.0 / 24.0);
    base_bytes before_drag = project_timeline_session_source(session);
    uint8_t before_drag_copy[4096U];
    CHECK(before_drag.len < sizeof(before_drag_copy));
    memcpy(before_drag_copy, before_drag.ptr, before_drag.len);
    CHECK(viewer_timeline_pointer_ex(&timeline, &layout,
        VIEWER_TIMELINE_POINTER_DOWN, source_marker_x, marker_y,
        0.0, 0.0, 0U, 1.0, editor) && timeline.native_dragging_item &&
        timeline.native_drag_from_tick == 12U);
    CHECK(viewer_timeline_pointer_ex(&timeline, &layout,
        VIEWER_TIMELINE_POINTER_MOVE, target_marker_x, marker_y,
        0.0, 0.0, 0U, 1.0, editor) &&
        timeline.native_drag_to_tick == 16U);
    base_bytes during_drag = project_timeline_session_source(session);
    CHECK(during_drag.len == before_drag.len &&
        memcmp(during_drag.ptr, before_drag_copy, during_drag.len) == 0 &&
        !viewer_timeline_take_native_mutation(&timeline));
    platform_event release = {.type = PLATFORM_EVENT_POINTER_UP,
        .logical_x = (int32_t)lround(target_marker_x),
        .logical_y = (int32_t)lround(marker_y), .backing_scale = 1.0};
    CHECK(viewer_timeline_process_event(&timeline, &release, editor,
        900U, 720U, 1.0) && !timeline.native_dragging_item &&
        viewer_timeline_take_native_mutation(&timeline));
    project_timeline_item_view moved_cel;
    CHECK(viewer_timeline_native_item_at(&timeline, 0U, 1U, &moved_cel) &&
        moved_cel.tick == 16U && project_timeline_session_is_dirty(session));
    CHECK(project_timeline_session_undo(session).error == PROJECT_TIMELINE_OK &&
        viewer_timeline_refresh_native(&timeline) &&
        viewer_timeline_native_item_at(&timeline, 0U, 1U, &moved_cel) &&
        moved_cel.tick == 12U);
    before_drag = project_timeline_session_source(session);
    CHECK(before_drag.len < sizeof(before_drag_copy));
    memcpy(before_drag_copy, before_drag.ptr, before_drag.len);
    double baseline_x = (double)layout.ruler.x;
    CHECK(viewer_timeline_pointer_ex(&timeline, &layout,
        VIEWER_TIMELINE_POINTER_DOWN, baseline_x, marker_y,
        0.0, 0.0, 0U, 1.0, editor) && timeline.native_dragging_item &&
        timeline.native_drag_from_tick == 0U);
    double invalid_x = (double)layout.ruler.x + layout.ruler.width * (8.0 / 24.0);
    CHECK(viewer_timeline_pointer_ex(&timeline, &layout,
        VIEWER_TIMELINE_POINTER_MOVE, invalid_x, marker_y,
        0.0, 0.0, 0U, 1.0, editor) && timeline.native_drag_to_tick == 8U);
    release.logical_x = (int32_t)lround(invalid_x);
    CHECK(viewer_timeline_process_event(&timeline, &release, editor,
        900U, 720U, 1.0) && !timeline.native_dragging_item &&
        !viewer_timeline_take_native_mutation(&timeline));
    base_bytes after_rejected_drag = project_timeline_session_source(session);
    CHECK(after_rejected_drag.len == before_drag.len &&
        memcmp(after_rejected_drag.ptr, before_drag_copy, before_drag.len) == 0 &&
        strstr(viewer_editor_status(editor), "tick zero") != NULL);
    double occupied_x = (double)layout.ruler.x;
    CHECK(viewer_timeline_pointer_ex(&timeline, &layout,
        VIEWER_TIMELINE_POINTER_DOWN, source_marker_x, marker_y,
        0.0, 0.0, 0U, 1.0, editor) && timeline.native_dragging_item);
    CHECK(viewer_timeline_pointer_ex(&timeline, &layout,
        VIEWER_TIMELINE_POINTER_MOVE, occupied_x, marker_y,
        0.0, 0.0, 0U, 1.0, editor));
    release.logical_x = (int32_t)lround(occupied_x);
    CHECK(viewer_timeline_process_event(&timeline, &release, editor,
        900U, 720U, 1.0) && !timeline.native_dragging_item &&
        !viewer_timeline_take_native_mutation(&timeline));
    after_rejected_drag = project_timeline_session_source(session);
    CHECK(after_rejected_drag.len == before_drag.len &&
        memcmp(after_rejected_drag.ptr, before_drag_copy, before_drag.len) == 0 &&
        strstr(viewer_editor_status(editor), "occupied") != NULL);
    CHECK(project_timeline_session_redo(session).error == PROJECT_TIMELINE_OK &&
        viewer_timeline_refresh_native(&timeline) &&
        viewer_timeline_native_item_at(&timeline, 0U, 1U, &moved_cel) &&
        moved_cel.tick == 16U);
    CHECK(project_timeline_session_undo(session).error == PROJECT_TIMELINE_OK &&
        viewer_timeline_refresh_native(&timeline) &&
        viewer_timeline_native_item_at(&timeline, 0U, 1U, &moved_cel) &&
        moved_cel.tick == 12U);
    CHECK(viewer_timeline_seek(&timeline, 0.5));
    const raster_tile *frame = viewer_timeline_prepare_native_frame(&timeline,
        bundle.scene_svg);
    CHECK(frame != NULL && frame->rgba[4] == 0U && frame->rgba[5] == 255U &&
        viewer_timeline_native_preview_is_current(&timeline));
    CHECK(layout.onion_button.width > 0U);
    CHECK(viewer_timeline_pointer_ex(&timeline, &layout,
        VIEWER_TIMELINE_POINTER_DOWN,
        (double)layout.onion_button.x + 2.0,
        (double)layout.onion_button.y + 2.0, 0.0, 0.0, 0U, 1.0, NULL));
    CHECK(viewer_timeline_take_native_mutation(&timeline));
    CHECK(timeline.native_onion_before == 1U && timeline.native_onion_after == 1U);
    CHECK(project_timeline_session_is_dirty(session) &&
        timeline.native_view.onion_opacity == 0.5);
    frame = viewer_timeline_prepare_native_frame(&timeline, bundle.scene_svg);
    CHECK(frame != NULL && viewer_timeline_native_preview_is_current(&timeline) &&
        timeline.native_onion_pixels <= VIEWER_TIMELINE_ONION_PIXEL_BUDGET);
    CHECK(!timeline.native_onion_limited);
    CHECK(frame->rgba[0] == 128U && frame->rgba[1] == 16U &&
        frame->rgba[2] == 16U && frame->rgba[3] == 128U);
    CHECK(viewer_timeline_toggle_play(&timeline, 10.0));
    CHECK(!viewer_timeline_native_preview_is_current(&timeline));
    frame = viewer_timeline_prepare_native_frame(&timeline, bundle.scene_svg);
    CHECK(frame != NULL && timeline.native_onion_pixels == 0U);
    CHECK(viewer_timeline_toggle_play(&timeline, 10.1));
    CHECK(viewer_timeline_seek(&timeline, 0.25));
    viewer_timeline_invalidate_preview(&timeline);
    frame = viewer_timeline_prepare_native_frame(&timeline, bundle.scene_svg);
    CHECK(frame != NULL && timeline.native_onion_pixels <=
        VIEWER_TIMELINE_ONION_PIXEL_BUDGET);
    CHECK(frame->rgba[4] == 0U && frame->rgba[5] == 112U &&
        frame->rgba[6] == 128U && frame->rgba[7] == 128U);
    CHECK(viewer_timeline_accessibility_snapshot(&timeline, &layout,
        UI_THEME_DARK, false, &semantics_scratch));
    CHECK(snapshot_has_label(&semantics_scratch, UI_ROLE_BUTTON,
        "Onion skin preview"));
    CHECK(snapshot_has_value_fragment(&semantics_scratch, UI_ROLE_BUTTON,
        "1 before, 1 after", false));
    CHECK(snapshot_has_label(&semantics_scratch, UI_ROLE_BUTTON,
        "Key selected object opacity at playhead"));
    timeline.native_onion_limited = true;
    CHECK(viewer_timeline_accessibility_snapshot(&timeline, &layout,
        UI_THEME_DARK, false, &semantics_scratch));
    CHECK(snapshot_has_value_fragment(&semantics_scratch, UI_ROLE_BUTTON,
        "onion preview limited", false));
    timeline.native_onion_limited = false;
    CHECK(snapshot_has_label(&semantics_scratch, UI_ROLE_ROW, "Pencil"));
    CHECK(snapshot_has_value_fragment(&semantics_scratch, UI_ROLE_ROW,
        "start ticks: 0, 12", false));
    CHECK(viewer_timeline_pointer_ex(&timeline, &layout,
        VIEWER_TIMELINE_POINTER_DOWN,
        (double)layout.drawing_asset_button.x + 2.0,
        (double)layout.drawing_asset_button.y + 2.0, 0.0, 0.0, 0U, 1.1,
        editor));
    project_bundle_asset selected_asset = {0};
    CHECK(viewer_timeline_native_selected_asset(&timeline, &selected_asset) &&
        selected_asset.name.len == sizeof(green_name) - 1U &&
        memcmp(selected_asset.name.ptr, green_name, sizeof(green_name) - 1U) == 0);
    timeline.native_selected_asset = 2U;
    CHECK(viewer_timeline_native_selected_asset(&timeline, &selected_asset) &&
        selected_asset.name.len == sizeof(mismatched_name) - 1U &&
        memcmp(selected_asset.name.ptr, mismatched_name,
            sizeof(mismatched_name) - 1U) == 0);
    CHECK(!viewer_timeline_add_native_cel(&timeline, editor));
    timeline.native_selected_asset = 1U;
    CHECK(viewer_timeline_cycle_native_asset(&timeline, editor));
    CHECK(viewer_timeline_native_selected_asset(&timeline, &selected_asset) &&
        selected_asset.name.len == sizeof(red_name) - 1U &&
        memcmp(selected_asset.name.ptr, red_name, sizeof(red_name) - 1U) == 0);
    CHECK(viewer_timeline_cycle_native_asset(&timeline, editor));
    CHECK(viewer_timeline_native_selected_asset(&timeline, &selected_asset) &&
        selected_asset.name.len == sizeof(green_name) - 1U &&
        memcmp(selected_asset.name.ptr, green_name, sizeof(green_name) - 1U) == 0);
    CHECK(viewer_timeline_accessibility_snapshot(&timeline, &layout,
        UI_THEME_DARK, false, &semantics_scratch));
    CHECK(snapshot_has_label(&semantics_scratch, UI_ROLE_BUTTON,
        "Choose drawing asset"));
    CHECK(snapshot_has_value_fragment(&semantics_scratch, UI_ROLE_BUTTON,
        "green", false));
    CHECK(snapshot_has_label(&semantics_scratch, UI_ROLE_BUTTON,
        "Add drawing cel at playhead"));
    CHECK(project_timeline_session_set_cel(session,
        (base_bytes){(const uint8_t *)"pencil", 6U}, 18U,
        (base_bytes){green_name, sizeof(green_name) - 1U},
        (base_bytes){(const uint8_t *)"late", 4U}).error == PROJECT_TIMELINE_OK);
    CHECK(viewer_timeline_refresh_native(&timeline));
    (void)viewer_timeline_seek(&timeline, 0.3);
    CHECK(viewer_timeline_seek(&timeline, 0.25));
    CHECK(viewer_timeline_pointer_ex(&timeline, &layout,
        VIEWER_TIMELINE_POINTER_DOWN,
        (double)layout.add_cel_button.x + 2.0,
        (double)layout.add_cel_button.y + 2.0, 0.0, 0.0, 0U, 1.15,
        editor));
    CHECK(viewer_timeline_take_native_mutation(&timeline));
    project_timeline_track_view cel_track;
    CHECK(viewer_timeline_native_track_at(&timeline, 0U, &cel_track) &&
        cel_track.item_count == 4U && !timeline.preview_valid);
    project_timeline_item_view added_cel;
    CHECK(viewer_timeline_native_item_at(&timeline, 0U, 1U, &added_cel) &&
        added_cel.tick == 6U && added_cel.asset.len == sizeof(green_name) - 1U &&
        memcmp(added_cel.asset.ptr, green_name, sizeof(green_name) - 1U) == 0);
    frame = viewer_timeline_prepare_native_frame(&timeline, bundle.scene_svg);
    CHECK(frame != NULL && frame->rgba[4] == 0U && frame->rgba[5] == 255U &&
        viewer_timeline_native_preview_is_current(&timeline));
    CHECK(project_timeline_session_undo(session).error == PROJECT_TIMELINE_OK);
    CHECK(viewer_timeline_refresh_native(&timeline) &&
        viewer_timeline_native_track_at(&timeline, 0U, &cel_track) &&
        cel_track.item_count == 3U);
    CHECK(project_timeline_session_redo(session).error == PROJECT_TIMELINE_OK);
    CHECK(viewer_timeline_refresh_native(&timeline) &&
        viewer_timeline_native_track_at(&timeline, 0U, &cel_track) &&
        cel_track.item_count == 4U);
    CHECK(viewer_timeline_seek(&timeline, 1.0) &&
        viewer_timeline_add_native_cel(&timeline, editor));
    CHECK(viewer_timeline_take_native_mutation(&timeline) &&
        viewer_timeline_native_track_at(&timeline, 0U, &cel_track) &&
        cel_track.item_count == 5U);
    CHECK(viewer_timeline_native_item_at(&timeline, 0U, 4U, &added_cel) &&
        added_cel.tick == 23U);
    CHECK(project_timeline_session_undo(session).error == PROJECT_TIMELINE_OK &&
        viewer_timeline_refresh_native(&timeline));
    CHECK(viewer_timeline_cycle_native_asset(&timeline, editor) &&
        viewer_timeline_seek(&timeline, 0.5) &&
        viewer_timeline_add_native_cel(&timeline, editor));
    CHECK(viewer_timeline_take_native_mutation(&timeline) &&
        viewer_timeline_native_track_at(&timeline, 0U, &cel_track) &&
        cel_track.item_count == 4U);
    CHECK(viewer_timeline_native_item_at(&timeline, 0U, 2U, &added_cel) &&
        added_cel.tick == 12U && added_cel.asset.len == sizeof(red_name) - 1U &&
        memcmp(added_cel.asset.ptr, red_name, sizeof(red_name) - 1U) == 0);
    CHECK(project_timeline_session_undo(session).error == PROJECT_TIMELINE_OK &&
        viewer_timeline_refresh_native(&timeline));
    CHECK(project_timeline_session_redo(session).error == PROJECT_TIMELINE_OK &&
        viewer_timeline_refresh_native(&timeline));
    double valid_playhead = timeline.time_seconds;
    timeline.time_seconds = NAN;
    CHECK(!viewer_timeline_add_native_cel(&timeline, editor));
    timeline.time_seconds = valid_playhead;
    timeline.selected_track = SIZE_MAX;
    CHECK(!viewer_timeline_add_native_cel(&timeline, editor));
    const char *selected_tag = strstr(scene_source, "<g id=");
    CHECK(selected_tag != NULL && viewer_editor_select_at_source_offset(editor,
        (size_t)(selected_tag - scene_source) + 1U));
    uint8_t transform_target[257U]; size_t transform_target_length = 0U;
    double captured_matrix[6] = {0};
    CHECK(viewer_editor_selected_transform_target(editor, transform_target,
        sizeof(transform_target), &transform_target_length, captured_matrix));
    CHECK(transform_target_length == 5U && memcmp(transform_target, "layer", 5U) == 0 &&
        captured_matrix[0] == 1.0 && captured_matrix[3] == 1.0 &&
        captured_matrix[4] == 3.0 && captured_matrix[5] == -2.0);
    (void)viewer_timeline_seek(&timeline, 0.6);
    CHECK(viewer_timeline_seek(&timeline, 0.5));
    CHECK(viewer_timeline_pointer_ex(&timeline, &layout,
        VIEWER_TIMELINE_POINTER_DOWN,
        (double)layout.key_opacity_button.x + 2.0,
        (double)layout.key_opacity_button.y + 2.0, 0.0, 0.0, 0U, 1.2,
        editor));
    CHECK(viewer_timeline_take_native_mutation(&timeline));
    CHECK(viewer_timeline_track_count(&timeline) == 2U &&
        timeline.selected_track == 1U && project_timeline_session_is_dirty(session));
    project_timeline_track_view opacity_track;
    CHECK(viewer_timeline_native_track_at(&timeline, 1U, &opacity_track) &&
        opacity_track.property.len == 7U &&
        memcmp(opacity_track.property.ptr, "opacity", 7U) == 0 &&
        opacity_track.item_count == 2U);
    project_timeline_item_view opacity_key;
    CHECK(viewer_timeline_native_item_at(&timeline, 1U, 0U, &opacity_key) &&
        opacity_key.tick == 0U && opacity_key.value_json.len == 4U &&
        memcmp(opacity_key.value_json.ptr, "0.75", 4U) == 0);
    CHECK(viewer_timeline_native_item_at(&timeline, 1U, 1U, &opacity_key) &&
        opacity_key.tick == 12U && opacity_key.value_json.len == 4U &&
        memcmp(opacity_key.value_json.ptr, "0.75", 4U) == 0);
    CHECK(project_timeline_session_undo(session).error == PROJECT_TIMELINE_OK);
    CHECK(viewer_timeline_refresh_native(&timeline) &&
        viewer_timeline_track_count(&timeline) == 1U);
    CHECK(project_timeline_session_redo(session).error == PROJECT_TIMELINE_OK);
    CHECK(viewer_timeline_refresh_native(&timeline) &&
        viewer_timeline_track_count(&timeline) == 2U);
    timeline.selected_track = 1U;
    CHECK(strcmp(viewer_timeline_native_drawing_action_label(&timeline),
        "Add Track") == 0);
    CHECK(viewer_timeline_accessibility_snapshot(&timeline, &layout,
        UI_THEME_DARK, false, &semantics_scratch));
    CHECK(snapshot_has_label(&semantics_scratch, UI_ROLE_BUTTON,
        "Create drawing track for selected SVG object"));
    CHECK(viewer_timeline_pointer_ex(&timeline, &layout,
        VIEWER_TIMELINE_POINTER_DOWN,
        (double)layout.add_cel_button.x + 2.0,
        (double)layout.add_cel_button.y + 2.0, 0.0, 0.0, 0U, 1.3,
        editor));
    CHECK(viewer_timeline_take_native_mutation(&timeline) &&
        viewer_timeline_track_count(&timeline) == 3U &&
        timeline.selected_track == 2U);
    project_timeline_track_view new_track;
    CHECK(viewer_timeline_native_track_at(&timeline, 2U, &new_track) &&
        new_track.kind.len == 7U && memcmp(new_track.kind.ptr, "drawing", 7U) == 0 &&
        new_track.item_count == 1U);
    uint8_t new_track_name[128U];
    size_t new_track_name_length = 0U;
    CHECK(project_timeline_string_copy(new_track.name,
        (base_mut_bytes){new_track_name, sizeof(new_track_name)},
        &new_track_name_length) && new_track_name_length == sizeof("Drawing layer") - 1U &&
        memcmp(new_track_name, "Drawing layer", sizeof("Drawing layer") - 1U) == 0);
    uint8_t new_track_id[129U];
    size_t new_track_id_length = 0U;
    CHECK(project_timeline_string_copy(new_track.id,
        (base_mut_bytes){new_track_id, sizeof(new_track_id)},
        &new_track_id_length));
    project_timeline_item_view initial_cel;
    CHECK(viewer_timeline_native_item_at(&timeline, 2U, 0U, &initial_cel) &&
        initial_cel.tick == 0U && initial_cel.asset.len == sizeof(red_name) - 1U &&
        memcmp(initial_cel.asset.ptr, red_name, sizeof(red_name) - 1U) == 0);
    (void)viewer_timeline_seek(&timeline, 0.0);
    frame = viewer_timeline_prepare_native_frame(&timeline, bundle.scene_svg);
    CHECK(frame != NULL && frame->rgba[0] == 255U && frame->rgba[3] == 255U);
    CHECK(strcmp(viewer_timeline_native_drawing_action_label(&timeline),
        "Add Cel") == 0);
    CHECK(project_timeline_session_undo(session).error == PROJECT_TIMELINE_OK &&
        viewer_timeline_refresh_native(&timeline) &&
        viewer_timeline_track_count(&timeline) == 2U);
    CHECK(project_timeline_session_redo(session).error == PROJECT_TIMELINE_OK &&
        viewer_timeline_refresh_native(&timeline) &&
        viewer_timeline_track_count(&timeline) == 3U);
    timeline.selected_track = 1U;
    CHECK(viewer_timeline_add_native_cel(&timeline, editor) &&
        viewer_timeline_take_native_mutation(&timeline) &&
        viewer_timeline_track_count(&timeline) == 4U &&
        timeline.selected_track == 3U);
    project_timeline_track_view second_new_track;
    uint8_t second_track_id[129U];
    size_t second_track_id_length = 0U;
    CHECK(viewer_timeline_native_track_at(&timeline, 3U, &second_new_track) &&
        project_timeline_string_copy(second_new_track.id,
            (base_mut_bytes){second_track_id, sizeof(second_track_id)},
            &second_track_id_length) &&
        (second_track_id_length != new_track_id_length ||
            memcmp(second_track_id, new_track_id, new_track_id_length) != 0));
    CHECK(project_timeline_session_undo(session).error == PROJECT_TIMELINE_OK &&
        viewer_timeline_refresh_native(&timeline) &&
        viewer_timeline_track_count(&timeline) == 3U);
    CHECK(viewer_timeline_seek(&timeline, 0.5));
    base_bytes before_rejected_key = project_timeline_session_source(session);
    uint8_t source_copy[4096U];
    CHECK(before_rejected_key.len < sizeof(source_copy));
    memcpy(source_copy, before_rejected_key.ptr, before_rejected_key.len);
    CHECK(!viewer_timeline_key_selected_transform(&timeline, editor) &&
        strstr(viewer_editor_status(editor), "tick zero") != NULL &&
        !viewer_timeline_take_native_mutation(&timeline) &&
        viewer_timeline_track_count(&timeline) == 3U);
    base_bytes after_rejected_key = project_timeline_session_source(session);
    CHECK(after_rejected_key.len == before_rejected_key.len &&
        memcmp(source_copy, after_rejected_key.ptr, after_rejected_key.len) == 0);
    CHECK(viewer_timeline_seek(&timeline, 0.0));
    uint8_t scene_before_key[4096U]; size_t scene_before_key_length = 0U;
    CHECK(viewer_editor_materialize(editor, scene_before_key,
        sizeof(scene_before_key), &scene_before_key_length));
    CHECK(viewer_timeline_pointer_ex(&timeline, &layout,
        VIEWER_TIMELINE_POINTER_DOWN,
        (double)layout.key_transform_button.x + 2.0,
        (double)layout.key_transform_button.y + 2.0, 0.0, 0.0, 0U, 1.4,
        editor));
    CHECK(viewer_timeline_take_native_mutation(&timeline) &&
        viewer_timeline_track_count(&timeline) == 4U && timeline.selected_track == 3U);
    project_timeline_track_view transform_track;
    CHECK(viewer_timeline_native_track_at(&timeline, 3U, &transform_track) &&
        transform_track.property.len == 9U &&
        memcmp(transform_track.property.ptr, "transform", 9U) == 0 &&
        transform_track.value_type.len == 11U &&
        memcmp(transform_track.value_type.ptr, "transform2d", 11U) == 0 &&
        transform_track.item_count == 1U);
    project_timeline_item_view transform_key;
    CHECK(viewer_timeline_native_item_at(&timeline, 3U, 0U, &transform_key) &&
        transform_key.tick == 0U && transform_key.value_json.len == 14U &&
        memcmp(transform_key.value_json.ptr, "[1,0,0,1,3,-2]", 14U) == 0);
    base_bytes baseline_source = project_timeline_session_source(session);
    uint8_t baseline_copy[4096U];
    CHECK(baseline_source.len < sizeof(baseline_copy));
    memcpy(baseline_copy, baseline_source.ptr, baseline_source.len);
    CHECK(viewer_timeline_seek(&timeline, 0.5) &&
        viewer_timeline_key_selected_transform(&timeline, editor) &&
        viewer_timeline_take_native_mutation(&timeline));
    CHECK(viewer_timeline_native_track_at(&timeline, 3U, &transform_track) &&
        transform_track.item_count == 2U &&
        viewer_timeline_native_item_at(&timeline, 3U, 1U, &transform_key) &&
        transform_key.tick == 12U);
    base_bytes animated_source = project_timeline_session_source(session);
    uint8_t animated_copy[4096U];
    CHECK(animated_source.len < sizeof(animated_copy));
    memcpy(animated_copy, animated_source.ptr, animated_source.len);
    base_bytes before_repeat = project_timeline_session_source(session);
    uint8_t repeat_copy[4096U];
    CHECK(before_repeat.len < sizeof(repeat_copy));
    memcpy(repeat_copy, before_repeat.ptr, before_repeat.len);
    CHECK(viewer_timeline_key_selected_transform(&timeline, editor) &&
        !viewer_timeline_take_native_mutation(&timeline));
    base_bytes after_repeat = project_timeline_session_source(session);
    CHECK(after_repeat.len == before_repeat.len &&
        memcmp(repeat_copy, after_repeat.ptr, after_repeat.len) == 0);
    CHECK(viewer_timeline_accessibility_snapshot(&timeline, &layout,
        UI_THEME_DARK, false, &semantics_scratch) &&
        snapshot_has_label(&semantics_scratch, UI_ROLE_BUTTON,
            "Key selected object transform at playhead"));
    CHECK(project_timeline_session_undo(session).error == PROJECT_TIMELINE_OK &&
        viewer_timeline_refresh_native(&timeline) &&
        viewer_timeline_native_track_at(&timeline, 3U, &transform_track) &&
        transform_track.item_count == 1U);
    base_bytes undone_source = project_timeline_session_source(session);
    CHECK(undone_source.len == baseline_source.len &&
        memcmp(undone_source.ptr, baseline_copy, undone_source.len) == 0);
    CHECK(project_timeline_session_redo(session).error == PROJECT_TIMELINE_OK &&
        viewer_timeline_refresh_native(&timeline) &&
        viewer_timeline_native_track_at(&timeline, 3U, &transform_track) &&
        transform_track.item_count == 2U);
    base_bytes redone_source = project_timeline_session_source(session);
    CHECK(redone_source.len == animated_source.len &&
        memcmp(redone_source.ptr, animated_copy, redone_source.len) == 0);
    uint8_t scene_after_key[4096U]; size_t scene_after_key_length = 0U;
    CHECK(viewer_editor_materialize(editor, scene_after_key,
        sizeof(scene_after_key), &scene_after_key_length) &&
        scene_before_key_length == scene_after_key_length &&
        memcmp(scene_before_key, scene_after_key, scene_after_key_length) == 0);
    uint8_t paint_target[257U]; size_t paint_target_length = 0U;
    double captured_paint[4] = {0};
    CHECK(viewer_editor_selected_paint_target(editor, "fill", paint_target,
        sizeof(paint_target), &paint_target_length, captured_paint) &&
        paint_target_length == 5U && memcmp(paint_target, "layer", 5U) == 0 &&
        captured_paint[0] == 1.0 && captured_paint[1] == 0.0 &&
        captured_paint[2] == 0.0 && captured_paint[3] == 1.0);
    CHECK(viewer_editor_selected_paint_target(editor, "stroke", paint_target,
        sizeof(paint_target), &paint_target_length, captured_paint) &&
        captured_paint[0] == 0.0 && captured_paint[1] == 0.0 &&
        captured_paint[2] == 1.0 && captured_paint[3] == 1.0);
    size_t scene_copy_length = 0U; uint8_t scene_copy[4096U];
    CHECK(viewer_editor_materialize(editor, scene_copy, sizeof(scene_copy),
        &scene_copy_length));
    CHECK(viewer_timeline_seek(&timeline, 0.0) &&
        viewer_timeline_key_selected_paint(&timeline, editor, "fill") &&
        viewer_timeline_take_native_mutation(&timeline));
    project_timeline_track_view fill_track;
    CHECK(viewer_timeline_native_track_at(&timeline, 4U, &fill_track) &&
        fill_track.property.len == 4U && memcmp(fill_track.property.ptr, "fill", 4U) == 0 &&
        fill_track.value_type.len == 4U && memcmp(fill_track.value_type.ptr, "rgba", 4U) == 0 &&
        fill_track.item_count == 1U);
    project_timeline_item_view fill_key;
    CHECK(viewer_timeline_native_item_at(&timeline, 4U, 0U, &fill_key) &&
        fill_key.tick == 0U && memcmp(fill_key.value_json.ptr, "[1,0,0,1]", 9U) == 0);
    base_bytes fill_source = project_timeline_session_source(session);
    uint8_t fill_source_copy[4096U]; CHECK(fill_source.len < sizeof(fill_source_copy));
    memcpy(fill_source_copy, fill_source.ptr, fill_source.len);
    CHECK(viewer_timeline_key_selected_paint(&timeline, editor, "fill") &&
        !viewer_timeline_take_native_mutation(&timeline));
    base_bytes repeated_fill = project_timeline_session_source(session);
    CHECK(repeated_fill.len == fill_source.len &&
        memcmp(repeated_fill.ptr, fill_source_copy, repeated_fill.len) == 0);
    CHECK(viewer_timeline_seek(&timeline, 0.5) &&
        viewer_timeline_key_selected_paint(&timeline, editor, "fill") &&
        viewer_timeline_take_native_mutation(&timeline));
    CHECK(viewer_timeline_native_track_at(&timeline, 4U, &fill_track) &&
        fill_track.item_count == 2U &&
        viewer_timeline_native_item_at(&timeline, 4U, 1U, &fill_key) &&
        fill_key.tick == 12U);
    CHECK(viewer_timeline_seek(&timeline, 0.0));
    CHECK(viewer_timeline_key_selected_paint(&timeline, editor, "stroke") &&
        viewer_timeline_take_native_mutation(&timeline));
    CHECK(viewer_timeline_track_count(&timeline) == 6U &&
        viewer_timeline_native_track_at(&timeline, 5U, &fill_track) &&
        fill_track.property.len == 6U && memcmp(fill_track.property.ptr, "stroke", 6U) == 0);
    CHECK(viewer_timeline_accessibility_snapshot(&timeline, &layout,
        UI_THEME_DARK, false, &semantics_scratch) &&
        snapshot_has_label(&semantics_scratch, UI_ROLE_BUTTON,
            "Key selected object fill color at playhead") &&
        snapshot_has_label(&semantics_scratch, UI_ROLE_BUTTON,
            "Key selected object stroke color at playhead"));
    CHECK(project_timeline_session_undo(session).error == PROJECT_TIMELINE_OK &&
        viewer_timeline_refresh_native(&timeline) && viewer_timeline_track_count(&timeline) == 5U);
    CHECK(project_timeline_session_redo(session).error == PROJECT_TIMELINE_OK &&
        viewer_timeline_refresh_native(&timeline) && viewer_timeline_track_count(&timeline) == 6U);
    uint8_t scene_after_paint[4096U]; size_t scene_after_paint_length = 0U;
    CHECK(viewer_editor_materialize(editor, scene_after_paint, sizeof(scene_after_paint),
        &scene_after_paint_length) && scene_after_paint_length == scene_copy_length &&
        memcmp(scene_after_paint, scene_copy, scene_copy_length) == 0);
    uint8_t visibility_id[257U]; size_t visibility_id_length = 0U;
    bool visible = false;
    CHECK(viewer_editor_selected_visibility_target(editor, visibility_id,
        sizeof(visibility_id), &visibility_id_length, &visible) && visible &&
        visibility_id_length == 5U && memcmp(visibility_id, "layer", 5U) == 0);
    (void)viewer_timeline_seek(&timeline, 0.0);
    bool visibility_keyed = viewer_timeline_key_selected_visibility(&timeline, editor);
    CHECK(visibility_keyed &&
        viewer_timeline_take_native_mutation(&timeline));
    project_timeline_track_view visibility_track; CHECK(viewer_timeline_native_track_at(&timeline, 6U, &visibility_track) &&
        visibility_track.property.len == 10U &&
        memcmp(visibility_track.property.ptr, "visibility", 10U) == 0 &&
        visibility_track.value_type.len == 7U &&
        memcmp(visibility_track.value_type.ptr, "boolean", 7U) == 0 &&
        visibility_track.item_count == 1U);
    CHECK(viewer_timeline_key_selected_visibility(&timeline, editor) &&
        !viewer_timeline_take_native_mutation(&timeline));
    CHECK(viewer_timeline_seek(&timeline, 0.5) &&
        viewer_timeline_key_selected_visibility(&timeline, editor) &&
        viewer_timeline_take_native_mutation(&timeline) &&
        viewer_timeline_native_track_at(&timeline, 6U, &visibility_track) &&
        visibility_track.item_count == 2U);
    project_timeline_item_view visibility_key; CHECK(viewer_timeline_native_item_at(&timeline, 6U, 1U, &visibility_key) &&
        visibility_key.tick == 12U && visibility_key.value_json.len == 4U &&
        memcmp(visibility_key.value_json.ptr, "true", 4U) == 0);
    CHECK(viewer_timeline_accessibility_snapshot(&timeline, &layout,
        UI_THEME_DARK, false, &semantics_scratch) &&
        snapshot_has_label(&semantics_scratch, UI_ROLE_BUTTON,
            "Key selected object visibility at playhead"));
    CHECK(project_timeline_session_undo(session).error == PROJECT_TIMELINE_OK &&
        viewer_timeline_refresh_native(&timeline) &&
        viewer_timeline_native_track_at(&timeline, 6U, &visibility_track) &&
        visibility_track.item_count == 1U);
    CHECK(project_timeline_session_redo(session).error == PROJECT_TIMELINE_OK &&
        viewer_timeline_refresh_native(&timeline) &&
        viewer_timeline_native_track_at(&timeline, 6U, &visibility_track) &&
        visibility_track.item_count == 2U);
    CHECK(viewer_timeline_pointer_ex(&timeline, &layout,
        VIEWER_TIMELINE_POINTER_DOWN,
        (double)layout.key_visibility_button.x + 2.0,
        (double)layout.key_visibility_button.y + 2.0, 0.0, 0.0, 0U, 1.5,
        editor) && !viewer_timeline_take_native_mutation(&timeline));
    viewer_editor_destroy(editor);
    viewer_timeline_destroy(&timeline);
    svg_shape_document_destroy(&scene);
    project_timeline_session_destroy(session);
    project_timeline_context_owner_destroy(&context);
    free(encoded);
    return true;
}
int main(void)
{
    if (!viewer_editor_visibility_test_run()) return 1;
    if (!test_transform_capture_parser()) return 1;
    if (!test_native_timeline_playback_and_read_only_rows()) return 1;
    (void)puts("viewer native timeline: passed");
    return 0;
}
