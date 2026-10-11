#define _POSIX_C_SOURCE 200809L

#include "mcp/mcp_broker.h"
#include "mcp/mcp_json.h"
#include "mcp/mcp_owner_queue.h"
#include "platform/platform_metal.h"
#include "viewer_chrome.h"
#include "viewer_editor_internal.h"
#include "viewer_internal.h"
#include "viewer_mcp_host.h"
#include "platform/platform_window.h"

#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

bool platform_window_set_material_enabled(platform_window *window, bool enabled)
{
    (void)window;
    (void)enabled;
    return true;
}
#include <unistd.h>

#define CHECK(condition) do { if (!(condition)) { \
    (void)fprintf(stderr, "FAIL %s:%d: %s\n", __FILE__, __LINE__, #condition); \
    return false; } } while (0)

#define TEST_WIDTH 1000U
#define TEST_HEIGHT 700U
#define TEST_FRAME_BYTES ((size_t)TEST_WIDTH * TEST_HEIGHT * 4U)

static const uint8_t test_svg[] =
    "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"64\" height=\"64\" "
    "viewBox=\"0 0 64 64\"><rect id=\"box\" x=\"4\" y=\"4\" "
    "width=\"56\" height=\"56\" fill=\"#f00\"><animate "
    "attributeName=\"x\" from=\"4\" to=\"8\" dur=\"2s\" "
    "repeatCount=\"indefinite\"/></rect></svg>";

static uint8_t presented_frame[TEST_FRAME_BYTES];
static size_t presented_length;

bool viewer_bump_generation(viewer_app *app)
{
    if (app == NULL || app->generation == UINT64_MAX) return false;
    app->generation++;
    return true;
}

void viewer_refresh_sharp_plan(viewer_app *app)
{
    if (app != NULL) app->sharp_plan_ready = false;
}

bool viewer_set_window_title(viewer_app *app)
{
    return app != NULL;
}

void viewer_update_file_stamp(viewer_app *app)
{
    if (app != NULL) app->file_stamp.valid = false;
}

bool viewer_update_active_path(viewer_app *app, const char *path, size_t length)
{
    if (app == NULL || path == NULL || length == 0U || length >= sizeof(app->path) ||
        memchr(path, '\0', length) != NULL) return false;
    memcpy(app->path, path, length);
    app->path[length] = '\0';
    return true;
}

ui_viewer_navigation viewer_navigation_for_frame(const viewer_app *app)
{
    return app == NULL ? (ui_viewer_navigation){0} : app->navigation;
}

bool platform_metal_present(platform_window *window, const uint8_t *rgba,
    size_t byte_count, uint32_t width, uint32_t height,
    char *diagnostic, size_t diagnostic_capacity)
{
    (void)window;
    (void)diagnostic;
    (void)diagnostic_capacity;
    if (rgba == NULL || width != TEST_WIDTH || height != TEST_HEIGHT ||
        byte_count != TEST_FRAME_BYTES) return false;
    memcpy(presented_frame, rgba, byte_count);
    presented_length = byte_count;
    return true;
}

bool platform_window_set_option_state(platform_window *window,
    platform_viewer_action action, bool enabled)
{
    (void)window;
    (void)action;
    (void)enabled;
    return true;
}

static bool response_init(mcp_tool_response *response)
{
    memset(response, 0, sizeof(*response));
    return mcp_json_buffer_init(&response->structured_json, 128U,
            MCP_JSON_MAX_BYTES) &&
        mcp_json_buffer_init(&response->content_json, 128U, MCP_JSON_MAX_BYTES);
}

static void response_destroy(mcp_tool_response *response)
{
    mcp_json_buffer_destroy(&response->structured_json);
    mcp_json_buffer_destroy(&response->content_json);
    *response = (mcp_tool_response){0};
}

static bool call_tool(mcp_broker *broker, const char *name, const char *arguments,
                      mcp_tool_response *out)
{
    mcp_json_doc request = {0};
    size_t length = strlen(arguments);
    mcp_protocol_backend backend = mcp_broker_protocol_backend(broker);
    bool okay = response_init(out) &&
        mcp_json_parse((const uint8_t *)arguments, length, &request);
    if (okay) okay = backend.call_tool(backend.context, &request,
        request.root, name, out);
    mcp_json_doc_destroy(&request);
    return okay;
}

static bool write_test_file(char path[64])
{
    memcpy(path, "/tmp/vexel-live-XXXXXX", sizeof("/tmp/vexel-live-XXXXXX"));
    int descriptor = mkstemp(path);
    if (descriptor < 0) return false;
    size_t cursor = 0U;
    while (cursor < sizeof(test_svg) - 1U) {
        ssize_t amount = write(descriptor, test_svg + cursor,
            sizeof(test_svg) - 1U - cursor);
        if (amount <= 0) { (void)close(descriptor); (void)unlink(path); return false; }
        cursor += (size_t)amount;
    }
    return close(descriptor) == 0;
}

static bool app_create(viewer_app *app, const char *path, uint32_t window_number,
                       bool render_document)
{
    if (app == NULL || path == NULL) return false;
    *app = (viewer_app){0};
    app->mcp_window_number = window_number;
    app->generation = 1U;
    app->running = true;
    app->backing_scale = 1.0;
    app->theme = UI_THEME_LIGHT;
    app->load_status = VIEWER_LOAD_OK;
    app->document_visible = true;
    app->load_result_handled = true;
    app->checkerboard = true;
    size_t path_length = strlen(path);
    if (path_length >= sizeof(app->path)) return false;
    memcpy(app->path, path, path_length + 1U);
    app->editor = viewer_editor_create_styled((base_bytes){test_svg,
        sizeof(test_svg) - 1U}, UI_THEME_LIGHT, TEST_WIDTH, TEST_HEIGHT, false);
    if (app->editor == NULL) return false;
    viewer_editor_resize(app->editor, TEST_WIDTH, TEST_HEIGHT, 1.0);
    viewer_timeline_init(&app->timeline);
    viewer_chrome_layout layout = {0};
    if (!viewer_editor_get_chrome_layout(app->editor, &layout)) return false;
    app->navigation_canvas = layout.canvas;
    if (!ui_viewer_navigation_init(&app->navigation,
            (double)layout.canvas.width, (double)layout.canvas.height, 64.0, 64.0) ||
        !ui_viewer_navigation_fit(&app->navigation)) return false;
    app->navigation_ready = true;
    if (!render_document) return true;
    app->width = TEST_WIDTH;
    app->height = TEST_HEIGHT;
    app->frame_bytes = TEST_FRAME_BYTES;
    app->frame = calloc(1U, app->frame_bytes);
    if (app->frame == NULL) return false;
    uint8_t *source = malloc(sizeof(test_svg) - 1U);
    if (source == NULL) return false;
    memcpy(source, test_svg, sizeof(test_svg) - 1U);
    viewer_loaded_document loaded = {0};
    if (viewer_decode_owned_source(source, sizeof(test_svg) - 1U, NULL,
            &loaded) != VIEWER_LOAD_OK) {
        viewer_loaded_document_destroy(&loaded);
        return false;
    }
    app->render_source = loaded.source;
    app->document.shape = loaded.shape;
    app->document.tile = loaded.tile;
    loaded = (viewer_loaded_document){0};
    viewer_timeline_reset(&app->timeline, &app->document.shape);
    app->frame_dirty = true;
    return viewer_compose_and_present(app);
}

static void app_destroy(viewer_app *app)
{
    if (app == NULL) return;
    viewer_timeline_destroy(&app->timeline);
    svg_shape_document_destroy(&app->document.shape);
    raster_tile_destroy(&app->document.tile);
    free(app->render_source);
    free(app->frame);
    viewer_editor_destroy(app->editor);
    *app = (viewer_app){0};
}

static bool rerender_edit(viewer_app *app)
{
    size_t length = viewer_editor_byte_length(app->editor);
    uint8_t *source = malloc(length + VIEWER_EDITOR_PREVIEW_EXTRA_BYTES);
    if (source == NULL) return false;
    size_t materialized = 0U;
    if (!viewer_editor_materialize_preview(app->editor, source,
            length + VIEWER_EDITOR_PREVIEW_EXTRA_BYTES, &materialized)) {
        free(source);
        return false;
    }
    viewer_loaded_document next = {0};
    if (viewer_decode_owned_source(source, materialized, NULL, &next) !=
        VIEWER_LOAD_OK) {
        viewer_loaded_document_destroy(&next);
        return false;
    }
    viewer_timeline_reset(&app->timeline, NULL);
    svg_shape_document_destroy(&app->document.shape);
    raster_tile_destroy(&app->document.tile);
    free(app->render_source);
    app->render_source = next.source;
    app->document.shape = next.shape;
    app->document.tile = next.tile;
    next = (viewer_loaded_document){0};
    viewer_timeline_reset(&app->timeline, &app->document.shape);
    app->frame_dirty = true;
    app->editor_render_requested = false;
    return viewer_compose_and_present(app);
}

static bool file_has_expected_fill(const char *path, const char *fill)
{
    FILE *file = fopen(path, "rb");
    if (file == NULL) return false;
    char bytes[2048];
    size_t length = fread(bytes, 1U, sizeof(bytes) - 1U, file);
    bool okay = !ferror(file) && fclose(file) == 0;
    bytes[length] = '\0';
    return okay && strstr(bytes, fill) != NULL;
}

static bool element_has_fill(document_edit *edit, const char *element_id,
                             const char *expected_fill)
{
    document_handle handle;
    if (edit == NULL || element_id == NULL || expected_fill == NULL ||
        !document_edit_find_id(edit, (base_bytes){(const uint8_t *)element_id,
            strlen(element_id)}, &handle)) return false;
    size_t count = document_edit_attribute_count(edit, handle);
    for (size_t index = 0U; index < count; index++) {
        document_edit_attribute attribute;
        base_bytes name = {0};
        if (!document_edit_attribute_at(edit, handle, index, &attribute) ||
            !document_edit_span_bytes(edit, attribute.name, &name) ||
            name.len != 4U || memcmp(name.ptr, "fill", 4U) != 0) continue;
        char value[64];
        size_t length = 0U;
        return document_edit_decode_span(edit, attribute.value,
            (uint8_t *)value, sizeof(value) - 1U, &length) &&
            length < sizeof(value) && (value[length] = '\0',
                strcmp(value, expected_fill) == 0);
    }
    return false;
}

static bool test_live_edit_and_undo(void)
{
    char path[64];
    CHECK(write_test_file(path));
    viewer_app first = {0}, second = {0};
    CHECK(app_create(&first, path, 1U, true));
    CHECK(app_create(&second, "/tmp/vexel-second.svg", 2U, true));
    viewer_session_collection sessions;
    viewer_sessions_init(&sessions);
    CHECK(viewer_sessions_add(&sessions, &first));
    CHECK(viewer_sessions_add(&sessions, &second));
    CHECK(viewer_sessions_activate(&sessions, &first));

    mcp_broker_host host = {
        .context = &sessions,
        .live_document_count = viewer_mcp_live_document_count,
        .live_document_at = viewer_mcp_live_document_at,
        .live_control_enabled = viewer_mcp_live_control_enabled,
        .current_selection_for_document = viewer_mcp_current_selection,
        .selection_changed_for_document = viewer_mcp_selection_changed,
        .commit_mutation = viewer_mcp_commit_mutation,
        .set_document_path = viewer_mcp_set_document_path
    };
    mcp_broker *broker = NULL;
    CHECK(mcp_broker_create(&host, &broker));
    viewer_mcp_set_broker(&sessions, broker);
    mcp_broker_bind_live_documents(broker);
    CHECK(viewer_mcp_connection_needs_sync(&sessions, true));
    viewer_mcp_connection_changed(&sessions, true);
    CHECK(!viewer_mcp_connection_needs_sync(&sessions, true));
    CHECK(first.mcp_connected && first.mcp_live_control_enabled &&
        second.mcp_connected && second.mcp_live_control_enabled);
    CHECK(strstr(viewer_editor_status(first.editor),
        "MCP agent connected · live control enabled") != NULL &&
        strstr(viewer_editor_status(second.editor), "live control enabled") != NULL);

    mcp_tool_response response = {0};
    CHECK(call_tool(broker, "document_list", "{}", &response));
    CHECK(strstr(response.structured_json.data, "\"documentId\":\"active\"") != NULL);
    CHECK(strstr(response.structured_json.data, "\"windowId\":\"window:1\"") != NULL);
    CHECK(strstr(response.structured_json.data, "\"documentId\":\"window:2\"") != NULL);
    CHECK(strstr(response.structured_json.data, "\"live\":true") != NULL);
    response_destroy(&response);

    document_edit *shared_edit = first.editor->edit;
    size_t history_before = document_edit_history_cursor(shared_edit);
    CHECK(element_has_fill(shared_edit, "box", "#f00"));
    uint8_t *frame_before = malloc(TEST_FRAME_BYTES);
    CHECK(frame_before != NULL && presented_length == TEST_FRAME_BYTES);
    memcpy(frame_before, presented_frame, TEST_FRAME_BYTES);
    mcp_ui_host ui_host;
    viewer_mcp_ui_host_init(&first, &ui_host);
    CHECK(call_tool(broker, "selection_set",
        "{\"documentId\":\"active\",\"elementId\":\"box\"}",
        &response));
    CHECK(!response.is_error);
    response_destroy(&response);
    CHECK(call_tool(broker, "selection_get", "{\"documentId\":\"active\"}",
        &response));
    CHECK(!response.is_error && strstr(response.structured_json.data,
        "\"elementId\":\"box\"") != NULL);
    response_destroy(&response);
    CHECK(call_tool(broker, "render_png", "{\"documentId\":\"active\"}",
        &response));
    CHECK(!response.is_error && strstr(response.content_json.data,
        "image/png") != NULL);
    char *initial_render = strdup(response.content_json.data);
    CHECK(initial_render != NULL);
    response_destroy(&response);

    static const char queued_edit[] =
        "{\"documentId\":\"active\",\"elementId\":\"box\",\"name\":\"fill\",\"value\":\"#00f\"}";
    mcp_owner_queue queue;
    mcp_owner_queue_init(&queue);
    first.editor->canvas_gesture = EDITOR_CANVAS_GESTURE_MOVE;
    CHECK(mcp_owner_queue_enqueue(&queue, 0U, 1U,
        (const uint8_t *)queued_edit, sizeof(queued_edit), 100U, 2000U) ==
        MCP_OWNER_QUEUE_ENQUEUED);
    mcp_owner_queue_item queued = {0};
    CHECK(mcp_owner_queue_take(&queue, viewer_mcp_owner_is_busy(&first),
        101U, &queued) == MCP_OWNER_QUEUE_WAITING);
    CHECK(document_edit_history_cursor(shared_edit) == history_before);
    CHECK(!viewer_editor_is_dirty(first.editor));
    CHECK(memcmp(frame_before, presented_frame, TEST_FRAME_BYTES) == 0);
    first.editor->canvas_gesture = EDITOR_CANVAS_GESTURE_NONE;
    CHECK(mcp_owner_queue_take(&queue, viewer_mcp_owner_is_busy(&first),
        102U, &queued) == MCP_OWNER_QUEUE_READY);
    CHECK(queued.owner_index == 0U && queued.owner_generation == 1U &&
        queued.length == sizeof(queued_edit) &&
        memcmp(queued.bytes, queued_edit, sizeof(queued_edit)) == 0);
    CHECK(call_tool(broker, "attribute_set", (const char *)queued.bytes,
        &response));
    mcp_owner_queue_item_destroy(&queued);
    CHECK(!response.is_error);
    CHECK(first.editor->edit == shared_edit);
    CHECK(document_edit_history_cursor(shared_edit) == history_before + 1U);
    CHECK(viewer_editor_is_dirty(first.editor));
    CHECK(element_has_fill(shared_edit, "box", "#00f"));
    CHECK(first.editor->selected_has_id && strcmp(first.editor->selected_id, "box") == 0);
    CHECK(first.editor->tree_ready == false);
    CHECK(first.editor->redraw_requested);
    CHECK(first.editor_render_requested);
    response_destroy(&response);
    CHECK(call_tool(broker, "element_query",
        "{\"documentId\":\"active\",\"id\":\"box\"}", &response));
    CHECK(!response.is_error && strstr(response.structured_json.data,
        "\"elementId\":\"box\"") != NULL);
    response_destroy(&response);
    CHECK(call_tool(broker, "render_png", "{\"documentId\":\"active\"}",
        &response));
    CHECK(!response.is_error && strcmp(initial_render,
        response.content_json.data) != 0);
    response_destroy(&response);
    free(initial_render);
    CHECK(rerender_edit(&first));
    CHECK(memcmp(frame_before, presented_frame, TEST_FRAME_BYTES) != 0);

    size_t edited_tile_bytes = (size_t)first.document.tile.width *
        first.document.tile.height * 4U;
    uint8_t *edited_tile = malloc(edited_tile_bytes);
    CHECK(edited_tile != NULL && edited_tile_bytes > 0U);
    memcpy(edited_tile, first.document.tile.rgba, edited_tile_bytes);
    uint32_t undo_result = editor_history(first.editor, false);
    CHECK((undo_result & VIEWER_EDITOR_MUTATED) != 0U);
    viewer_mcp_gui_mutation(&first, MCP_BROKER_MUTATION_UNDO);
    CHECK(first.editor->edit == shared_edit);
    CHECK(document_edit_history_cursor(shared_edit) == history_before);
    CHECK(!viewer_editor_is_dirty(first.editor));
    CHECK(element_has_fill(shared_edit, "box", "#f00"));
    CHECK(rerender_edit(&first));
    CHECK(memcmp(edited_tile, first.document.tile.rgba, edited_tile_bytes) != 0);

    CHECK(call_tool(broker, "redo", "{\"documentId\":\"active\"}", &response));
    CHECK(!response.is_error && first.editor->edit == shared_edit);
    CHECK(document_edit_history_cursor(shared_edit) == history_before + 1U);
    CHECK(viewer_editor_is_dirty(first.editor));
    CHECK(element_has_fill(shared_edit, "box", "#00f"));
    response_destroy(&response);
    CHECK(rerender_edit(&first));
    CHECK(memcmp(edited_tile, first.document.tile.rgba, edited_tile_bytes) == 0);

    mcp_ui_view_state view = {0};
    CHECK(mcp_ui_host_get_view(&ui_host, &view) == MCP_UI_HOST_OK);
    CHECK(view.zoom > 0.0 && view.viewport_width > 0.0);
    mcp_ui_view_request fit_width = {.action = MCP_UI_VIEW_FIT_WIDTH};
    CHECK(mcp_ui_host_set_view(&ui_host, &fit_width) == MCP_UI_HOST_OK);
    mcp_ui_state ui_state = {0};
    CHECK(mcp_ui_host_get_ui_state(&ui_host, &ui_state) == MCP_UI_HOST_OK);
    CHECK(ui_state.live_control_enabled && ui_state.agent_connected &&
        ui_state.dirty && ui_state.can_undo &&
        ui_state.has_selection && strcmp(ui_state.selected_id, "box") == 0);
    CHECK(mcp_ui_host_run_command(&ui_host, "toggle-inspector") == MCP_UI_HOST_OK);
    CHECK(!first.editor->want_inspector);
    CHECK(mcp_ui_host_run_command(&ui_host, "fit-window") == MCP_UI_HOST_OK);
    bool checkerboard_before = first.checkerboard;
    CHECK(mcp_ui_host_run_command(&ui_host, "toggle-checkerboard") == MCP_UI_HOST_OK);
    CHECK(first.checkerboard != checkerboard_before);
    bool grid_before = first.pixel_grid;
    CHECK(mcp_ui_host_run_command(&ui_host, "toggle-pixel-grid") == MCP_UI_HOST_OK);
    CHECK(first.pixel_grid != grid_before);

    mcp_ui_timeline_state timeline = {0};
    CHECK(mcp_ui_host_timeline_get(&ui_host, &timeline) == MCP_UI_HOST_OK);
    CHECK(timeline.track_count == 1U && timeline.duration_seconds > 0.0);
    first.frame_dirty = true;
    CHECK(viewer_compose_and_present(&first));
    uint8_t *time_zero_frame = malloc(TEST_FRAME_BYTES);
    CHECK(time_zero_frame != NULL);
    memcpy(time_zero_frame, presented_frame, TEST_FRAME_BYTES);
    mcp_ui_timeline_update timeline_update = {
        .set_time = true,
        .time_seconds = 1.0,
        .set_loop = true,
        .loop = true,
        .set_selected_track = true,
        .selected_track = 0U
    };
    CHECK(mcp_ui_host_timeline_set(&ui_host, &timeline_update) == MCP_UI_HOST_OK);
    CHECK(mcp_ui_host_timeline_get(&ui_host, &timeline) == MCP_UI_HOST_OK);
    CHECK(timeline.time_seconds == 1.0 && timeline.loop &&
        timeline.selected_track == 0U);
    CHECK(viewer_compose_and_present(&first));
    CHECK(memcmp(time_zero_frame, presented_frame, TEST_FRAME_BYTES) != 0);
    free(time_zero_frame);
    CHECK(mcp_ui_host_run_command(&ui_host, "undo") == MCP_UI_HOST_OK);
    CHECK(document_edit_history_cursor(shared_edit) == history_before);
    CHECK(mcp_ui_host_run_command(&ui_host, "redo") == MCP_UI_HOST_OK);
    CHECK(document_edit_history_cursor(shared_edit) == history_before + 1U);

    uint32_t gui_edit = editor_apply_button(first.editor,
        EDITOR_BUTTON_DUPLICATE);
    CHECK((gui_edit & VIEWER_EDITOR_MUTATED) != 0U);
    viewer_mcp_gui_mutation(&first, MCP_BROKER_MUTATION_EDIT);
    CHECK(document_edit_history_cursor(shared_edit) == history_before + 2U);
    CHECK(call_tool(broker, "undo", "{\"documentId\":\"active\"}",
        &response));
    CHECK(!response.is_error &&
        document_edit_history_cursor(shared_edit) == history_before + 1U);
    CHECK(element_has_fill(shared_edit, "box", "#00f"));
    response_destroy(&response);

    document_edit *second_edit = second.editor->edit;
    size_t second_history = document_edit_history_cursor(second_edit);
    size_t second_tile_bytes = (size_t)second.document.tile.width *
        second.document.tile.height * 4U;
    uint8_t *second_tile_before = malloc(second_tile_bytes);
    CHECK(second_tile_before != NULL && second_tile_bytes > 0U);
    memcpy(second_tile_before, second.document.tile.rgba, second_tile_bytes);
    CHECK(call_tool(broker, "transaction",
        "{\"documentId\":\"window:2\",\"edits\":[{\"op\":\"attribute_set\",\"elementId\":\"box\",\"name\":\"fill\",\"value\":\"#0f0\"}]}",
        &response));
    CHECK(!response.is_error && second.editor->edit == second_edit &&
        document_edit_history_cursor(second_edit) == second_history + 1U &&
        element_has_fill(second_edit, "box", "#0f0"));
    response_destroy(&response);
    CHECK(viewer_editor_is_dirty(second.editor));
    CHECK(rerender_edit(&second));
    CHECK(memcmp(second_tile_before, second.document.tile.rgba,
        second_tile_bytes) != 0);
    CHECK((editor_history(second.editor, false) & VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(second.editor->edit == second_edit &&
        document_edit_history_cursor(second_edit) == second_history &&
        element_has_fill(second_edit, "box", "#f00"));
    CHECK(!viewer_editor_is_dirty(second.editor));
    CHECK(rerender_edit(&second));
    CHECK(memcmp(second_tile_before, second.document.tile.rgba,
        second_tile_bytes) == 0);
    free(second_tile_before);
    CHECK(rerender_edit(&first));
    CHECK(viewer_compose_and_present(&first));
    mcp_ui_png png = {0};
    CHECK(mcp_ui_host_capture_window_png(&ui_host, &png) == MCP_UI_HOST_OK);
    CHECK(png.length > 32U && png.bytes[0] == 137U && png.bytes[1] == 80U &&
        png.bytes[2] == 78U && png.bytes[3] == 71U);
    CHECK(png.width == TEST_WIDTH && png.height == TEST_HEIGHT);
    CHECK(first.frame != NULL && memcmp(first.frame, presented_frame,
        TEST_FRAME_BYTES) == 0);
    mcp_ui_png_destroy(&png);

    CHECK(viewer_mcp_owner_is_busy(&first) == false);
    first.editor->canvas_gesture = EDITOR_CANVAS_GESTURE_MOVE;
    CHECK(viewer_mcp_owner_is_busy(&first));
    mcp_ui_view_request fit = {.action = MCP_UI_VIEW_FIT};
    CHECK(mcp_ui_host_set_view(&ui_host, &fit) == MCP_UI_HOST_BUSY);
    first.editor->canvas_gesture = EDITOR_CANVAS_GESTURE_NONE;
    first.editor->field_kind = EDITOR_FIELD_ATTRIBUTE;
    CHECK(viewer_mcp_owner_is_busy(&first));
    first.editor->field_kind = EDITOR_FIELD_NONE;

    mcp_ui_tool tool = MCP_UI_TOOL_SELECT;
    CHECK(mcp_ui_host_set_tool(&ui_host, MCP_UI_TOOL_PEN) == MCP_UI_HOST_OK);
    CHECK(mcp_ui_host_get_tool(&ui_host, &tool) == MCP_UI_HOST_OK);
    CHECK(tool == MCP_UI_TOOL_PEN);
    mcp_ui_view_request invalid = {.action = MCP_UI_VIEW_SET,
        .zoom = NAN, .pan_x = 0.0, .pan_y = 0.0};
    CHECK(mcp_ui_host_set_view(&ui_host, &invalid) == MCP_UI_HOST_INVALID_ARGUMENT);

    first.editor->canvas_gesture = EDITOR_CANVAS_GESTURE_MOVE;
    CHECK(mcp_owner_queue_enqueue(&queue, 0U, 1U,
        (const uint8_t *)queued_edit, sizeof(queued_edit), 200U, 2U) ==
        MCP_OWNER_QUEUE_ENQUEUED);
    CHECK(mcp_owner_queue_take(&queue, viewer_mcp_owner_is_busy(&first),
        202U, &queued) == MCP_OWNER_QUEUE_EXPIRED);
    mcp_owner_queue_item_destroy(&queued);
    first.editor->canvas_gesture = EDITOR_CANVAS_GESTURE_NONE;
    for (size_t index = 0U; index < MCP_OWNER_QUEUE_CAPACITY; index++)
        CHECK(mcp_owner_queue_enqueue(&queue, 0U, 1U,
            (const uint8_t *)queued_edit, sizeof(queued_edit), 300U, 2000U) ==
            MCP_OWNER_QUEUE_ENQUEUED);
    CHECK(mcp_owner_queue_enqueue(&queue, 0U, 1U,
        (const uint8_t *)queued_edit, sizeof(queued_edit), 300U, 2000U) ==
        MCP_OWNER_QUEUE_FULL);
    while (mcp_owner_queue_take(&queue, false, 300U, &queued) ==
        MCP_OWNER_QUEUE_READY) mcp_owner_queue_item_destroy(&queued);

    CHECK(setenv("VEXEL_MCP_DISABLE_LIVE", "1", 1) == 0);
    CHECK(!viewer_mcp_live_control_enabled(&sessions));
    CHECK(mcp_ui_host_get_view(&ui_host, &view) == MCP_UI_HOST_DISABLED);
    CHECK(viewer_mcp_live_document_count(&sessions) == 0U);
    CHECK(unsetenv("VEXEL_MCP_DISABLE_LIVE") == 0);

    CHECK(call_tool(broker, "save", "{\"documentId\":\"active\"}", &response));
    CHECK(!response.is_error && first.editor->edit == shared_edit);
    CHECK(!viewer_editor_is_dirty(first.editor));
    CHECK(file_has_expected_fill(path, "#00f"));
    response_destroy(&response);

    mcp_broker_destroy(broker);
    free(edited_tile);
    free(frame_before);
    app_destroy(&second);
    app_destroy(&first);
    (void)unlink(path);
    return true;
}

static uint32_t fuzz_next(uint32_t *state)
{
    *state ^= *state << 13U;
    *state ^= *state >> 17U;
    *state ^= *state << 5U;
    return *state;
}

static bool parse_fuzz_number(const char *text, uint32_t *out)
{
    if (text == NULL || out == NULL || text[0] == '\0') return false;
    char *end = NULL;
    unsigned long value = strtoul(text, &end, 10);
    if (end == text || *end != '\0' || value > UINT32_MAX) return false;
    *out = (uint32_t)value;
    return true;
}

static bool fuzz_host(uint32_t cases, uint32_t seed)
{
    viewer_app app = {0};
    CHECK(app_create(&app, "/tmp/vexel-host-fuzz.svg", 99U, false));
    mcp_ui_host host;
    viewer_mcp_ui_host_init(&app, &host);
    uint32_t state = seed == 0U ? 1U : seed;
    for (uint32_t index = 0U; index < cases; index++) {
        uint32_t value = fuzz_next(&state);
        mcp_ui_view_request view = {
            .action = (mcp_ui_view_action)(value % 7U),
            .zoom = (double)(int32_t)fuzz_next(&state) / 1000.0,
            .pan_x = (double)(int32_t)fuzz_next(&state),
            .pan_y = (double)(int32_t)fuzz_next(&state)
        };
        mcp_ui_host_status status = mcp_ui_host_set_view(&host, &view);
        CHECK(status >= MCP_UI_HOST_OK && status <= MCP_UI_HOST_FAILED);
        mcp_ui_view_state current = {0};
        if (mcp_ui_host_get_view(&host, &current) == MCP_UI_HOST_OK)
            CHECK(isfinite(current.zoom) && current.zoom >= UI_VIEWER_MIN_SCALE &&
                current.zoom <= UI_VIEWER_MAX_SCALE && isfinite(current.pan_x) &&
                isfinite(current.pan_y));
        mcp_ui_tool tool = (mcp_ui_tool)(fuzz_next(&state) % 18U);
        status = mcp_ui_host_set_tool(&host, tool);
        CHECK(status >= MCP_UI_HOST_OK && status <= MCP_UI_HOST_FAILED);
        mcp_ui_timeline_update timeline = {
            .set_time = (fuzz_next(&state) & 1U) != 0U,
            .time_seconds = (double)(int32_t)fuzz_next(&state) / 1000.0,
            .set_playing = (fuzz_next(&state) & 1U) != 0U,
            .playing = (fuzz_next(&state) & 1U) != 0U,
            .set_loop = (fuzz_next(&state) & 1U) != 0U,
            .loop = (fuzz_next(&state) & 1U) != 0U,
            .set_selected_track = (fuzz_next(&state) & 1U) != 0U,
            .selected_track = fuzz_next(&state) % 8U
        };
        status = mcp_ui_host_timeline_set(&host, &timeline);
        CHECK(status >= MCP_UI_HOST_OK && status <= MCP_UI_HOST_FAILED);
        static const char *commands[] = {"fit", "fit-width", "actual-size",
            "undo", "redo", "toggle-structure", "toggle-inspector", "save",
            "invalid", "toggle-checkerboard"};
        const char *command = commands[fuzz_next(&state) %
            (uint32_t)(sizeof(commands) / sizeof(commands[0]))];
        status = mcp_ui_host_run_command(&host, command);
        CHECK(status >= MCP_UI_HOST_OK && status <= MCP_UI_HOST_FAILED);
    }
    app_destroy(&app);
    return true;
}

int main(int argc, char **argv)
{
    if (argc == 4 && strcmp(argv[1], "--fuzz") == 0) {
        uint32_t cases = 0U, seed = 0U;
        if (!parse_fuzz_number(argv[2], &cases) ||
            !parse_fuzz_number(argv[3], &seed) || cases == 0U) return EXIT_FAILURE;
        (void)setenv("VEXEL_MCP_DISABLE_LIVE", "0", 1);
        if (!fuzz_host(cases, seed)) return EXIT_FAILURE;
        (void)printf("viewer MCP host fuzz: %u cases, seed %u, pass\n", cases, seed);
        return EXIT_SUCCESS;
    }
    if (argc != 1) return EXIT_FAILURE;
    if (!test_live_edit_and_undo()) return EXIT_FAILURE;
    (void)puts("viewer MCP live plumbing: pass");
    return EXIT_SUCCESS;
}
