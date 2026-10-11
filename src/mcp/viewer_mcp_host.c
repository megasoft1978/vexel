#define _POSIX_C_SOURCE 200809L

#include "viewer_mcp_host.h"

#include "frame_compose.h"
#include "mcp/mcp_broker.h"
#include "mcp/mcp_broker_internal.h"
#include "platform/macos/platform_png.h"
#include "viewer_editor_internal.h"
#include "viewer_internal.h"

#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

#define VIEWER_MCP_COMMAND_CAP 48U
#define VIEWER_MCP_PAN_LIMIT 10000000.0

static bool app_live_control_enabled(void)
{
    const char *disabled = getenv("VEXEL_MCP_DISABLE_LIVE");
    return disabled == NULL || disabled[0] == '\0' ||
        (disabled[0] != '1' && disabled[0] != 't' && disabled[0] != 'T' &&
         disabled[0] != 'y' && disabled[0] != 'Y');
}

bool viewer_mcp_live_control_enabled(void *context)
{
    (void)context;
    return app_live_control_enabled();
}

static bool app_ready(const viewer_app *app)
{
    return app != NULL && app->editor != NULL && app->load_status == VIEWER_LOAD_OK &&
        app->document_visible && app->navigation_ready;
}

static mcp_ui_host_status app_get_view(void *context,
                                        mcp_ui_view_state *out_state)
{
    viewer_app *app = context;
    if (out_state == NULL) return MCP_UI_HOST_INVALID_ARGUMENT;
    *out_state = (mcp_ui_view_state){0};
    if (!app_live_control_enabled()) return MCP_UI_HOST_DISABLED;
    if (!app_ready(app)) return MCP_UI_HOST_UNAVAILABLE;
    out_state->zoom = app->navigation.scale;
    out_state->pan_x = app->navigation.pan_x;
    out_state->pan_y = app->navigation.pan_y;
    out_state->viewport_width = app->navigation.viewport_width;
    out_state->viewport_height = app->navigation.viewport_height;
    out_state->document_width = app->document.tile.width;
    out_state->document_height = app->document.tile.height;
    return MCP_UI_HOST_OK;
}

static void app_navigation_changed(viewer_app *app)
{
    if (!viewer_bump_generation(app)) {
        app->fatal_error = true;
        app->running = false;
        return;
    }
    viewer_navigation_clamp(&app->navigation);
    viewer_refresh_sharp_plan(app);
    app->frame_dirty = true;
    (void)viewer_set_window_title(app);
}

static mcp_ui_host_status app_set_view(void *context,
                                        const mcp_ui_view_request *request)
{
    viewer_app *app = context;
    if (request == NULL) return MCP_UI_HOST_INVALID_ARGUMENT;
    if (!app_live_control_enabled()) return MCP_UI_HOST_DISABLED;
    if (!app_ready(app)) return MCP_UI_HOST_UNAVAILABLE;
    if (viewer_editor_text_input_active(app->editor) ||
        app->editor->canvas_gesture != EDITOR_CANVAS_GESTURE_NONE ||
        app->timeline.dragging_scrubber) return MCP_UI_HOST_BUSY;
    ui_viewer_navigation before = app->navigation;
    bool changed = false;
    switch (request->action) {
        case MCP_UI_VIEW_FIT:
            changed = ui_viewer_navigation_fit(&app->navigation);
            if (changed) app->navigation_user_zoomed = false;
            break;
        case MCP_UI_VIEW_FIT_WIDTH:
            changed = ui_viewer_navigation_fit_width(&app->navigation);
            if (changed) app->navigation_user_zoomed = true;
            break;
        case MCP_UI_VIEW_ACTUAL_SIZE:
            changed = ui_viewer_navigation_actual_size(&app->navigation);
            if (changed) app->navigation_user_zoomed = true;
            break;
        case MCP_UI_VIEW_SET:
            if (!isfinite(request->zoom) || request->zoom < UI_VIEWER_MIN_SCALE ||
                request->zoom > UI_VIEWER_MAX_SCALE || !isfinite(request->pan_x) ||
                !isfinite(request->pan_y) || fabs(request->pan_x) > VIEWER_MCP_PAN_LIMIT ||
                fabs(request->pan_y) > VIEWER_MCP_PAN_LIMIT)
                return MCP_UI_HOST_INVALID_ARGUMENT;
            app->navigation.scale = request->zoom;
            app->navigation.pan_x = request->pan_x;
            app->navigation.pan_y = request->pan_y;
            app->navigation_user_zoomed = true;
            viewer_navigation_clamp(&app->navigation);
            changed = before.scale != app->navigation.scale ||
                before.pan_x != app->navigation.pan_x ||
                before.pan_y != app->navigation.pan_y;
            break;
        default:
            return MCP_UI_HOST_INVALID_ARGUMENT;
    }
    if (changed) app_navigation_changed(app);
    return MCP_UI_HOST_OK;
}

static mcp_ui_tool app_tool(const viewer_editor *editor)
{
    if (editor->node_edit_active) return MCP_UI_TOOL_NODE;
    if (editor->draw_tool != VIEWER_EDITOR_TOOL_SELECT) {
        switch (editor->draw_tool) {
            case VIEWER_EDITOR_TOOL_RECT: return MCP_UI_TOOL_RECTANGLE;
            case VIEWER_EDITOR_TOOL_ELLIPSE: return MCP_UI_TOOL_ELLIPSE;
            case VIEWER_EDITOR_TOOL_LINE: return MCP_UI_TOOL_LINE;
            case VIEWER_EDITOR_TOOL_POLYGON: return MCP_UI_TOOL_POLYGON;
            case VIEWER_EDITOR_TOOL_POLYLINE: return MCP_UI_TOOL_POLYLINE;
            case VIEWER_EDITOR_TOOL_PEN: return MCP_UI_TOOL_PEN;
            case VIEWER_EDITOR_TOOL_TEXT: return MCP_UI_TOOL_TEXT;
            case VIEWER_EDITOR_TOOL_SELECT: break;
        }
    }
    switch (viewer_editor_get_tool(editor)) {
        case UI_ICON_PAN: return MCP_UI_TOOL_PAN;
        case UI_ICON_ZOOM: return MCP_UI_TOOL_ZOOM;
        case UI_ICON_SELECT: default: return MCP_UI_TOOL_SELECT;
    }
}

static mcp_ui_host_status app_get_tool(void *context, mcp_ui_tool *out_tool)
{
    viewer_app *app = context;
    if (out_tool == NULL) return MCP_UI_HOST_INVALID_ARGUMENT;
    *out_tool = MCP_UI_TOOL_SELECT;
    if (!app_live_control_enabled()) return MCP_UI_HOST_DISABLED;
    if (app == NULL || app->editor == NULL) return MCP_UI_HOST_UNAVAILABLE;
    *out_tool = app_tool(app->editor);
    return MCP_UI_HOST_OK;
}

static mcp_ui_host_status app_set_tool(void *context, mcp_ui_tool tool)
{
    viewer_app *app = context;
    if (!app_live_control_enabled()) return MCP_UI_HOST_DISABLED;
    if (app == NULL || app->editor == NULL) return MCP_UI_HOST_UNAVAILABLE;
    if (viewer_mcp_owner_is_busy(app)) return MCP_UI_HOST_BUSY;
    bool changed = false;
    switch (tool) {
        case MCP_UI_TOOL_SELECT: changed = viewer_editor_set_tool(app->editor, UI_ICON_SELECT); break;
        case MCP_UI_TOOL_PAN: changed = viewer_editor_set_tool(app->editor, UI_ICON_PAN); break;
        case MCP_UI_TOOL_ZOOM: changed = viewer_editor_set_tool(app->editor, UI_ICON_ZOOM); break;
        case MCP_UI_TOOL_RECTANGLE: changed = viewer_editor_set_draw_tool(app->editor, VIEWER_EDITOR_TOOL_RECT); break;
        case MCP_UI_TOOL_ELLIPSE: changed = viewer_editor_set_draw_tool(app->editor, VIEWER_EDITOR_TOOL_ELLIPSE); break;
        case MCP_UI_TOOL_LINE: changed = viewer_editor_set_draw_tool(app->editor, VIEWER_EDITOR_TOOL_LINE); break;
        case MCP_UI_TOOL_POLYGON: changed = viewer_editor_set_draw_tool(app->editor, VIEWER_EDITOR_TOOL_POLYGON); break;
        case MCP_UI_TOOL_POLYLINE: changed = viewer_editor_set_draw_tool(app->editor, VIEWER_EDITOR_TOOL_POLYLINE); break;
        case MCP_UI_TOOL_PEN: changed = viewer_editor_set_draw_tool(app->editor, VIEWER_EDITOR_TOOL_PEN); break;
        case MCP_UI_TOOL_TEXT: changed = viewer_editor_set_draw_tool(app->editor, VIEWER_EDITOR_TOOL_TEXT); break;
        case MCP_UI_TOOL_NODE:
        {
            bool draw_tool_changed = false;
            if (app->editor->draw_tool != VIEWER_EDITOR_TOOL_PEN)
                draw_tool_changed = viewer_editor_set_draw_tool(app->editor,
                    VIEWER_EDITOR_TOOL_PEN);
            changed = draw_tool_changed || !app->editor->node_edit_active;
            app->editor->node_edit_active = true;
            app->editor->tree_ready = false;
            app->editor->redraw_requested = true;
            break;
        }
        default: return MCP_UI_HOST_INVALID_ARGUMENT;
    }
    if (changed) app->frame_dirty = true;
    return MCP_UI_HOST_OK;
}

static mcp_ui_host_status app_get_ui_state(void *context, mcp_ui_state *out_state)
{
    viewer_app *app = context;
    if (out_state == NULL) return MCP_UI_HOST_INVALID_ARGUMENT;
    *out_state = (mcp_ui_state){0};
    if (!app_live_control_enabled()) return MCP_UI_HOST_DISABLED;
    if (app == NULL || app->editor == NULL || app->editor->edit == NULL)
        return MCP_UI_HOST_UNAVAILABLE;
    viewer_editor *editor = app->editor;
    size_t cursor = document_edit_history_cursor(editor->edit);
    size_t history = document_edit_history_count(editor->edit);
    out_state->live_control_enabled = true;
    out_state->agent_connected = app->mcp_connected;
    out_state->dirty = viewer_editor_is_dirty(editor);
    out_state->can_undo = cursor > 0U;
    out_state->can_redo = cursor < history;
    out_state->has_selection = editor->selection_active;
    out_state->selected_count = editor->canvas_selection_count;
    out_state->structure_visible = editor->want_structure;
    out_state->inspector_visible = editor->want_inspector;
    if (editor->selected_has_id)
        (void)snprintf(out_state->selected_id, sizeof(out_state->selected_id),
            "%s", editor->selected_id);
    (void)snprintf(out_state->status, sizeof(out_state->status), "%s",
        viewer_editor_status(editor));
    return MCP_UI_HOST_OK;
}

static mcp_ui_host_status app_capture_window_png(void *context,
                                                  mcp_ui_png *out_png)
{
    viewer_app *app = context;
    if (out_png == NULL) return MCP_UI_HOST_INVALID_ARGUMENT;
    *out_png = (mcp_ui_png){0};
    if (!app_live_control_enabled()) return MCP_UI_HOST_DISABLED;
    if (app == NULL || app->frame == NULL || app->frame_bytes == 0U ||
        app->width == 0U || app->height == 0U) return MCP_UI_HOST_UNAVAILABLE;
    if (app->width > MCP_UI_CAPTURE_MAX_AXIS ||
        app->height > MCP_UI_CAPTURE_MAX_AXIS ||
        (uint64_t)app->width * app->height > MCP_UI_CAPTURE_MAX_PIXELS)
        return MCP_UI_HOST_LIMIT;
    platform_png_blob encoded = {0};
    if (!platform_png_encode_rgba8(app->frame, app->frame_bytes,
            (size_t)app->width * 4U, app->width, app->height, &encoded))
        return MCP_UI_HOST_FAILED;
    *out_png = (mcp_ui_png){encoded.bytes, encoded.length, app->width, app->height};
    return MCP_UI_HOST_OK;
}

static mcp_ui_host_status app_timeline_get(void *context,
    mcp_ui_timeline_state *out_state)
{
    viewer_app *app = context;
    if (out_state == NULL) return MCP_UI_HOST_INVALID_ARGUMENT;
    *out_state = (mcp_ui_timeline_state){0};
    if (!app_live_control_enabled()) return MCP_UI_HOST_DISABLED;
    if (app == NULL || app->editor == NULL) return MCP_UI_HOST_UNAVAILABLE;
    out_state->time_seconds = app->timeline.time_seconds;
    out_state->duration_seconds = viewer_timeline_duration(&app->timeline);
    out_state->track_count = viewer_timeline_track_count(&app->timeline);
    out_state->selected_track = app->timeline.selected_track;
    out_state->playing = app->timeline.playing;
    out_state->loop = app->timeline.loop;
    return MCP_UI_HOST_OK;
}

static double monotonic_seconds(void)
{
    struct timespec now;
    if (clock_gettime(CLOCK_MONOTONIC, &now) != 0) return 0.0;
    return (double)now.tv_sec + (double)now.tv_nsec / 1000000000.0;
}

static mcp_ui_host_status app_timeline_set(void *context,
    const mcp_ui_timeline_update *update)
{
    viewer_app *app = context;
    if (update == NULL) return MCP_UI_HOST_INVALID_ARGUMENT;
    if (!app_live_control_enabled()) return MCP_UI_HOST_DISABLED;
    if (app == NULL || app->editor == NULL) return MCP_UI_HOST_UNAVAILABLE;
    if (viewer_mcp_owner_is_busy(app)) return MCP_UI_HOST_BUSY;
    if (update->set_time && (!isfinite(update->time_seconds) ||
        update->time_seconds < 0.0 || update->time_seconds > 3600.0))
        return MCP_UI_HOST_INVALID_ARGUMENT;
    if (update->set_selected_track &&
        update->selected_track >= viewer_timeline_track_count(&app->timeline))
        return MCP_UI_HOST_INVALID_ARGUMENT;
    if (update->set_time && !viewer_timeline_seek(&app->timeline,
            update->time_seconds)) return MCP_UI_HOST_FAILED;
    if (update->set_loop && !viewer_timeline_set_loop(&app->timeline, update->loop))
        return MCP_UI_HOST_FAILED;
    if (update->set_selected_track)
        app->timeline.selected_track = update->selected_track;
    if (update->set_playing && app->timeline.playing != update->playing &&
        !viewer_timeline_toggle_play(&app->timeline, monotonic_seconds()))
        return MCP_UI_HOST_FAILED;
    app->frame_dirty = true;
    return MCP_UI_HOST_OK;
}

static mcp_ui_host_status app_run_command(void *context, const char *command_id)
{
    viewer_app *app = context;
    if (command_id == NULL || strnlen(command_id, VIEWER_MCP_COMMAND_CAP) >=
        VIEWER_MCP_COMMAND_CAP) return MCP_UI_HOST_INVALID_ARGUMENT;
    if (!app_live_control_enabled()) return MCP_UI_HOST_DISABLED;
    if (app == NULL || app->editor == NULL) return MCP_UI_HOST_UNAVAILABLE;
    if (viewer_mcp_owner_is_busy(app)) return MCP_UI_HOST_BUSY;
    if (strcmp(command_id, "fit") == 0 || strcmp(command_id, "fit-window") == 0 ||
        strcmp(command_id, "fit-width") == 0 || strcmp(command_id, "actual-size") == 0) {
        bool fit_window = strcmp(command_id, "fit") == 0 ||
            strcmp(command_id, "fit-window") == 0;
        mcp_ui_view_request request = {.action = fit_window ? MCP_UI_VIEW_FIT :
            strcmp(command_id, "fit-width") == 0 ? MCP_UI_VIEW_FIT_WIDTH :
            MCP_UI_VIEW_ACTUAL_SIZE};
        return app_set_view(app, &request);
    }
    if (strcmp(command_id, "undo") == 0 || strcmp(command_id, "redo") == 0) {
        bool redo = strcmp(command_id, "redo") == 0;
        document_edit_error error = redo ? document_edit_redo(app->editor->edit) :
            document_edit_undo(app->editor->edit);
        if (error != DOCUMENT_EDIT_OK) return MCP_UI_HOST_UNAVAILABLE;
        char document_id[32];
        int amount = snprintf(document_id, sizeof(document_id), "window:%u",
            app->mcp_window_number);
        if (amount <= 0 || (size_t)amount >= sizeof(document_id))
            return MCP_UI_HOST_FAILED;
        if (app->mcp_broker != NULL)
            mcp_broker_commit_mutation(app->mcp_broker, document_id,
                redo ? MCP_BROKER_MUTATION_REDO : MCP_BROKER_MUTATION_UNDO);
        else viewer_mcp_commit_mutation(app, app,
            redo ? MCP_BROKER_MUTATION_REDO : MCP_BROKER_MUTATION_UNDO,
            (document_handle){0}, false);
        return MCP_UI_HOST_OK;
    }
    if (strcmp(command_id, "toggle-structure") == 0 ||
        strcmp(command_id, "toggle-inspector") == 0) {
        bool structure = app->editor->want_structure;
        bool inspector = app->editor->want_inspector;
        if (strcmp(command_id, "toggle-structure") == 0) structure = !structure;
        else inspector = !inspector;
        (void)viewer_editor_set_sidebar_visibility(app->editor, structure, inspector);
        app->frame_dirty = true;
        return MCP_UI_HOST_OK;
    }
    if (strcmp(command_id, "toggle-checkerboard") == 0) {
        app->checkerboard = !app->checkerboard;
        (void)platform_window_set_option_state(app->window,
            PLATFORM_ACTION_TOGGLE_CHECKERBOARD, app->checkerboard);
        app->frame_dirty = true;
        return MCP_UI_HOST_OK;
    }
    if (strcmp(command_id, "cycle-background") == 0) {
        app->background_index = (uint8_t)((app->background_index + 1U) % 3U);
        if (app->checkerboard) {
            app->checkerboard = false;
            (void)platform_window_set_option_state(app->window,
                PLATFORM_ACTION_TOGGLE_CHECKERBOARD, false);
        }
        app->frame_dirty = true;
        return MCP_UI_HOST_OK;
    }
    if (strcmp(command_id, "toggle-pixel-grid") == 0) {
        app->pixel_grid = !app->pixel_grid;
        (void)platform_window_set_option_state(app->window,
            PLATFORM_ACTION_TOGGLE_PIXEL_GRID, app->pixel_grid);
        app->frame_dirty = true;
        return MCP_UI_HOST_OK;
    }
    if (strcmp(command_id, "save") == 0) {
        if (app->path[0] == '\0') return MCP_UI_HOST_UNAVAILABLE;
        document_edit_save_result saved = viewer_editor_save(app->editor,
            (base_bytes){(const uint8_t *)app->path, strlen(app->path)});
        if (saved.error != DOCUMENT_EDIT_OK) return MCP_UI_HOST_FAILED;
        viewer_update_file_stamp(app);
        char document_id[32];
        int amount = snprintf(document_id, sizeof(document_id), "window:%u",
            app->mcp_window_number);
        if (amount <= 0 || (size_t)amount >= sizeof(document_id))
            return MCP_UI_HOST_FAILED;
        if (app->mcp_broker != NULL)
            mcp_broker_commit_mutation(app->mcp_broker, document_id,
                MCP_BROKER_MUTATION_SAVE);
        else viewer_mcp_commit_mutation(app, app, MCP_BROKER_MUTATION_SAVE,
            (document_handle){0}, false);
        return MCP_UI_HOST_OK;
    }
    return MCP_UI_HOST_UNAVAILABLE;
}

static const mcp_ui_host_vtable app_vtable = {
    app_get_view,
    app_set_view,
    app_get_tool,
    app_set_tool,
    app_get_ui_state,
    app_capture_window_png,
    app_run_command,
    app_timeline_get,
    app_timeline_set
};

void viewer_mcp_ui_host_init(viewer_app *app, mcp_ui_host *out_host)
{
    if (out_host != NULL) *out_host = (mcp_ui_host){&app_vtable, app};
}

void viewer_mcp_set_broker(viewer_session_collection *sessions,
                           struct mcp_broker *broker)
{
    if (sessions == NULL) return;
    for (size_t index = 0U; index < sessions->count &&
        index < VIEWER_SESSION_MAX; index++)
        if (sessions->items[index] != NULL)
            sessions->items[index]->mcp_broker = broker;
}

void viewer_mcp_gui_mutation(viewer_app *app, mcp_broker_mutation_kind kind)
{
    if (app == NULL || app->mcp_broker == NULL || app->mcp_window_number == 0U)
        return;
    char document_id[32];
    int amount = snprintf(document_id, sizeof(document_id), "window:%u",
        app->mcp_window_number);
    if (amount <= 0 || (size_t)amount >= sizeof(document_id)) return;
    mcp_broker_commit_mutation(app->mcp_broker, document_id, kind);
}

size_t viewer_mcp_live_document_count(void *context)
{
    viewer_session_collection *sessions = context;
    if (!app_live_control_enabled() || sessions == NULL) return 0U;
    size_t live = 0U;
    for (size_t index = 0U; index < sessions->count &&
        index < VIEWER_SESSION_MAX; index++) {
        viewer_app *app = sessions->items[index];
        if (app != NULL && app->editor != NULL && app->editor->edit != NULL &&
            app->load_status == VIEWER_LOAD_OK && app->path[0] != '\0') live++;
    }
    return live;
}

bool viewer_mcp_live_document_at(void *context, size_t index,
                                 mcp_broker_live_document *out_document)
{
    viewer_session_collection *sessions = context;
    if (out_document == NULL || sessions == NULL || !app_live_control_enabled())
        return false;
    size_t live_index = 0U;
    for (size_t slot = 0U; slot < sessions->count &&
        slot < VIEWER_SESSION_MAX; slot++) {
        viewer_app *app = sessions->items[slot];
        if (app == NULL || app->editor == NULL || app->editor->edit == NULL ||
            app->load_status != VIEWER_LOAD_OK || app->path[0] == '\0') continue;
        if (live_index++ != index) continue;
        *out_document = (mcp_broker_live_document){
            .identity = app,
            .edit = app->editor->edit,
            .path = app->path,
            .window_number = app->mcp_window_number,
            .selection = app->editor->selected,
            .selection_valid = app->editor->selection_active,
            .active = sessions->active == app
        };
        viewer_mcp_ui_host_init(app, &out_document->ui_host);
        return true;
    }
    return false;
}

bool viewer_mcp_current_selection(void *context, void *identity,
                                   document_handle *out_selection)
{
    (void)context;
    viewer_app *app = identity;
    if (app == NULL || app->editor == NULL || out_selection == NULL ||
        !app->editor->selection_active) return false;
    *out_selection = app->editor->selected;
    return true;
}

void viewer_mcp_selection_changed(void *context, void *identity,
                                  document_handle selection)
{
    (void)context;
    viewer_app *app = identity;
    if (app == NULL || app->editor == NULL ||
        app->editor->edit == NULL || !document_edit_source_span(app->editor->edit,
            selection, &(document_source_span){0})) return;
    editor_select_handle(app->editor, selection);
    app->frame_dirty = true;
}

void viewer_mcp_commit_mutation(void *context, void *identity,
                                mcp_broker_mutation_kind kind,
                                document_handle selection,
                                bool selection_valid)
{
    (void)context;
    viewer_app *app = identity;
    if (app == NULL || app->editor == NULL || app->editor->edit == NULL) return;
    viewer_editor *editor = app->editor;
    if (kind != MCP_BROKER_MUTATION_SAVE) {
        size_t old_offset = editor->selected_offset;
        char old_id[EDITOR_NAME_CAP];
        editor_set_text(old_id, sizeof(old_id),
            editor->selected_has_id ? editor->selected_id : "");
        document_handle preferred = selection_valid ? selection : (document_handle){0};
        editor_rebind_after_change(editor, old_offset, old_id, preferred);
        editor->tree_ready = false;
        editor->redraw_requested = true;
        viewer_timeline_reset(&app->timeline, NULL);
        if (!viewer_bump_generation(app)) {
            app->fatal_error = true;
            app->running = false;
            return;
        }
        app->editor_render_requested = true;
    } else {
        editor->tree_ready = false;
        editor->redraw_requested = true;
    }
    app->frame_dirty = true;
    (void)viewer_set_window_title(app);
}

bool viewer_mcp_set_document_path(void *context, void *identity,
                                  const char *path)
{
    (void)context;
    viewer_app *app = identity;
    if (app == NULL || path == NULL) return false;
    size_t length = strnlen(path, sizeof(app->path));
    if (length == 0U || length >= sizeof(app->path)) return false;
    return viewer_update_active_path(app, path, length);
}

bool viewer_mcp_owner_is_busy(void *context)
{
    const viewer_app *app = context;
    if (app == NULL || app->editor == NULL) return false;
    const viewer_editor *editor = app->editor;
    return editor->canvas_gesture != EDITOR_CANVAS_GESTURE_NONE ||
        editor->field_kind != EDITOR_FIELD_NONE || editor->text_draft_active ||
        editor->pressed_id != 0U || app->timeline.dragging_scrubber;
}

bool viewer_mcp_sessions_are_busy(void *context)
{
    const viewer_session_collection *sessions = context;
    if (sessions == NULL) return false;
    for (size_t index = 0U; index < sessions->count &&
        index < VIEWER_SESSION_MAX; index++)
        if (viewer_mcp_owner_is_busy(sessions->items[index])) return true;
    return false;
}

void viewer_mcp_connection_changed(viewer_session_collection *sessions,
                                   bool connected)
{
    if (sessions == NULL) return;
    for (size_t index = 0U; index < sessions->count &&
        index < VIEWER_SESSION_MAX; index++) {
        viewer_app *app = sessions->items[index];
        if (app == NULL) continue;
        app->mcp_connected = connected;
        app->mcp_live_control_enabled = connected && app_live_control_enabled();
        if (app->editor != NULL)
            viewer_editor_set_status(app->editor, connected ?
                app->mcp_live_control_enabled ?
                    "MCP agent connected · live control enabled" :
                    "MCP agent connected · live control disabled" :
                "MCP agent disconnected");
        app->frame_dirty = true;
        (void)viewer_set_window_title(app);
    }
}

bool viewer_mcp_connection_needs_sync(const viewer_session_collection *sessions,
                                      bool connected)
{
    if (sessions == NULL) return false;
    bool live_enabled = connected && app_live_control_enabled();
    for (size_t index = 0U; index < sessions->count &&
        index < VIEWER_SESSION_MAX; index++) {
        const viewer_app *app = sessions->items[index];
        if (app != NULL && (app->mcp_connected != connected ||
            app->mcp_live_control_enabled != live_enabled)) return true;
    }
    return false;
}
