#define _DARWIN_C_SOURCE
#define _POSIX_C_SOURCE 200809L

#include "mcp_broker_hook.h"

#include "viewer_editor_internal.h"
#include "viewer_internal.h"
#include "mcp_broker_io.h"
#include "mcp/mcp_broker.h"
#include "mcp/mcp_json.h"
#include "mcp/mcp_protocol.h"
#include "mcp/mcp_socket.h"

#include <errno.h>
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/stat.h>
#include <sys/un.h>
#include <unistd.h>

typedef struct mcp_hook_session {
    mcp_broker_io_owner owner;
    mcp_protocol_session protocol;
} mcp_hook_session;

typedef struct mcp_hook_state {
    viewer_app *app;
    int listener;
    bool owns_socket;
    bool start_attempted;
    bool reported_error;
    char socket_path[sizeof(((struct sockaddr_un *)0)->sun_path)];
    mcp_broker *broker;
    mcp_broker_io *io;
    mcp_hook_session sessions[MCP_BROKER_IO_CLIENTS];
} mcp_hook_state;

static mcp_hook_state hook = {.listener = -1};

static void host_agent_edited(void *context, const char *path);
static void host_agent_action_completed(void *context, const char *tool_name,
                                        bool succeeded);
static bool process_line(void *context, mcp_broker_io_owner owner,
                         const uint8_t *line, size_t length);
static void client_disconnected(void *context, mcp_broker_io_owner owner,
                                mcp_broker_io_reason reason);

static bool path_is_vexel_project(const char *path)
{
    if (path == NULL) return false;
    size_t length = strnlen(path, PLATFORM_PATH_CAP);
    static const char suffix[] = ".vexel";
    if (length >= PLATFORM_PATH_CAP || length <= sizeof(suffix) - 1U) return false;
    size_t start = length - (sizeof(suffix) - 1U);
    for (size_t index = 0U; index < sizeof(suffix) - 1U; ++index) {
        char value = path[start + index];
        if (value >= 'A' && value <= 'Z') value = (char)(value - 'A' + 'a');
        if (value != suffix[index]) return false;
    }
    return true;
}

static document_edit *host_current_edit(void *context)
{
    viewer_app *app = context;
    return app == NULL || app->editor == NULL ? NULL : app->editor->edit;
}

static project_timeline_session *host_current_timeline_session(void *context)
{
    viewer_app *app = context;
    return app == NULL || !app->document.native_project ||
        !path_is_vexel_project(app->path) ? NULL :
        app->document.timeline_session;
}

static const project_bundle_view *host_current_project_bundle(void *context)
{
    viewer_app *app = context;
    return app == NULL || !app->document.native_project ? NULL :
        &app->document.project;
}

static project_bundle_result host_add_native_drawing_asset(void *context,
    base_bytes name, base_bytes svg)
{
    viewer_app *app = context;
    if (app == NULL || !app->document.native_project ||
        app->document.timeline_session == NULL)
        return (project_bundle_result){PROJECT_BUNDLE_INVALID_ARGUMENT, 0U, 0U};
    project_bundle_result added = project_bundle_asset_overlay_add_svg(
        &app->document.asset_overlay, name, svg);
    if (added.error == PROJECT_BUNDLE_OK) {
        viewer_editor_set_status(app->editor, "Agent added a native drawing asset");
        app->frame_dirty = true;
        (void)viewer_set_window_title(app);
    }
    return added;
}

static void host_native_timeline_changed(void *context)
{
    viewer_app *app = context;
    if (app != NULL) {
        if (app->editor != NULL)
            viewer_editor_set_status(app->editor, "Agent edited animation timeline");
        bool refreshed = viewer_timeline_refresh_native(&app->timeline);
        if (refreshed && app->editor != NULL) {
            bool available = viewer_timeline_track_count(&app->timeline) > 0U;
            (void)viewer_editor_set_timeline_state(app->editor, available,
                app->timeline.dock_open, app->timeline.dock_height_points);
            (void)viewer_sync_canvas_geometry(app, false);
        }
        viewer_timeline_invalidate_preview(&app->timeline);
        app->frame_dirty = true;
        (void)viewer_set_window_title(app);
    }
}

static bool mcp_path_has_extension(const char *path, size_t length,
                                   const char *extension)
{
    size_t extension_length = strlen(extension);
    if (path == NULL || extension_length == 0U || length <= extension_length)
        return false;
    size_t start = length - extension_length;
    if (path[start - 1U] != '.') return false;
    for (size_t index = 0U; index < extension_length; ++index) {
        char value = path[start + index];
        if (value >= 'A' && value <= 'Z') value = (char)(value - 'A' + 'a');
        if (value != extension[index]) return false;
    }
    return true;
}

static bool host_save_current_project(void *context, const char *path,
                                      size_t path_length, bool save_as)
{
    viewer_app *app = context;
    if (app == NULL || app->editor == NULL) return false;
    if (!save_as) return viewer_save_document(app, false);
    if (path == NULL || path_length == 0U || path_length >= PLATFORM_PATH_CAP ||
        memchr(path, '\0', path_length) != NULL) return false;
    size_t current_length = viewer_event_path_length(app->path,
        PLATFORM_PATH_CAP - 1U);
    if (app->path[0] != '\0' && path_length == current_length &&
        memcmp(path, app->path, path_length) == 0 &&
        viewer_file_changed_on_disk(app)) {
        viewer_editor_set_status(app->editor,
            "The selected file changed outside Vexel. Save As to a different path to keep both versions.");
        return false;
    }
    bool project_format = mcp_path_has_extension(path, path_length, "vexel");
    if (!project_format &&
        project_timeline_session_is_dirty(app->document.timeline_session)) {
        viewer_editor_set_status(app->editor,
            "Save the edited animation as a .vexel project to keep timeline changes");
        return false;
    }
    bool saved = false;
    if (project_format) {
        atomic_store_explicit(&app->project_save_cancel_requested, false,
            memory_order_release);
        saved = viewer_save_project_file(app, path, path_length);
    } else {
        document_edit_save_result result = viewer_editor_save(app->editor,
            (base_bytes){(const uint8_t *)path, path_length});
        saved = result.error == DOCUMENT_EDIT_OK;
    }
    char previous_path[PLATFORM_PATH_CAP];
    size_t previous_length = strnlen(app->path, sizeof(app->path));
    if (previous_length < sizeof(previous_path))
        memcpy(previous_path, app->path, previous_length + 1U);
    else previous_path[0] = '\0';
    bool updated = saved && viewer_update_active_path(app, path, path_length);
    if (updated && !project_format)
        (void)viewer_recovery_cleanup_after_save(app, previous_path, path);
    return updated;
}

static const char *host_current_path(void *context)
{
    viewer_app *app = context;
    return app == NULL || app->editor == NULL || app->load_status != VIEWER_LOAD_OK ? "" : app->path;
}

static bool host_current_selection(void *context, document_handle *out)
{
    viewer_app *app = context;
    if (app == NULL || app->editor == NULL || out == NULL) return false;
    *out = app->editor->selected;
    return true;
}

static bool host_replace_edit(void *context, document_edit *expected,
                              document_edit *replacement, document_handle selection)
{
    viewer_app *app = context;
    if (app == NULL || app->editor == NULL || app->editor->edit != expected || replacement == NULL ||
        !viewer_bump_generation(app)) return false;
    app->editor->edit = replacement;
    document_edit_destroy(expected);
    editor_select_handle(app->editor, selection);
    editor_refresh_outline(app->editor);
    viewer_editor_set_status(app->editor, "Agent edited document — transaction committed");
    app->editor_render_requested = true;
    app->frame_dirty = true;
    (void)viewer_set_window_title(app);
    return true;
}

static bool host_destroy_edit(void *context, document_edit *edit)
{
    viewer_app *app = context;
    if (app == NULL || app->editor == NULL || app->editor->edit != edit) return false;
    app->editor->edit = NULL;
    document_edit_destroy(edit);
    return true;
}

static bool host_set_path(void *context, const char *path)
{
    viewer_app *app = context;
    if (app == NULL || path == NULL || app->editor == NULL) return false;
    size_t length = strnlen(path, sizeof(app->path));
    if (length == 0U || length >= sizeof(app->path)) return false;
    memcpy(app->path, path, length + 1U);
    const char *base = path;
    for (const char *cursor = path; *cursor != '\0'; cursor++)
        if (*cursor == '/') base = cursor + 1;
    size_t base_length = strnlen(base, sizeof(app->display_name) - 1U);
    for (size_t index = 0U; index < base_length; index++) {
        unsigned char byte = (unsigned char)base[index];
        app->display_name[index] = byte >= 32U && byte < 127U ? (char)byte : '?';
    }
    app->display_name[base_length] = '\0';
    viewer_update_file_stamp(app);
    app->frame_dirty = true;
    return viewer_set_window_title(app);
}

static void host_agent_edited(void *context, const char *path)
{
    viewer_app *app = context;
    if (app == NULL || app->editor == NULL) return;
    const char *base = path == NULL ? "document" : path;
    if (path != NULL) {
        for (const char *cursor = path; *cursor != '\0'; cursor++)
            if (*cursor == '/') base = cursor + 1;
    }
    char status[192];
    int amount = snprintf(status, sizeof(status), "Agent edited — %.120s", base);
    if (amount > 0 && (size_t)amount < sizeof(status)) viewer_editor_set_status(app->editor, status);
    if (path != NULL && strcmp(path, app->path) == 0) {
        if (app->timeline.native_project) {
            app->timeline.native_context_valid = false;
            viewer_timeline_invalidate_preview(&app->timeline);
        }
        if (viewer_bump_generation(app)) app->editor_render_requested = true;
    }
    app->frame_dirty = true;
}

static void host_agent_action_completed(void *context, const char *tool_name,
                                        bool succeeded)
{
    viewer_app *app = context;
    if (app == NULL || app->editor == NULL || tool_name == NULL) return;
    char message[EDITOR_LOG_TEXT_CAP] = "MCP ";
    size_t used = sizeof("MCP ") - 1U;
    size_t name_length = strnlen(tool_name, 64U);
    for (size_t index = 0U; index < name_length && used < sizeof(message) - 1U;
         ++index) {
        unsigned char value = (unsigned char)tool_name[index];
        bool safe = (value >= 'a' && value <= 'z') ||
            (value >= 'A' && value <= 'Z') ||
            (value >= '0' && value <= '9') || value == '_';
        message[used++] = safe ? (char)value : '?';
    }
    const char *result = succeeded ? " completed" : " failed";
    size_t result_length = strlen(result);
    if (result_length > sizeof(message) - 1U - used)
        result_length = sizeof(message) - 1U - used;
    memcpy(message + used, result, result_length);
    used += result_length;
    message[used] = '\0';
    viewer_editor_log(app->editor, succeeded ? EDITOR_LOG_OPERATION :
        EDITOR_LOG_ERROR, message, used);
    viewer_editor_set_status(app->editor, message);
    app->frame_dirty = true;
}

static void host_selection_changed(void *context, document_handle selection)
{
    viewer_app *app = context;
    if (app == NULL || app->editor == NULL) return;
    editor_select_handle(app->editor, selection);
    editor_refresh_outline(app->editor);
    app->frame_dirty = true;
}

static bool parent_directory_is_private(const char *socket_path)
{
    char parent[sizeof(((struct sockaddr_un *)0)->sun_path)];
    size_t length = strnlen(socket_path, sizeof(parent));
    if (length == 0U || length >= sizeof(parent)) return false;
    memcpy(parent, socket_path, length + 1U);
    char *slash = strrchr(parent, '/');
    if (slash == NULL || slash == parent) return false;
    *slash = '\0';
    struct stat status;
    return stat(parent, &status) == 0 && S_ISDIR(status.st_mode) && status.st_uid == getuid() &&
        (status.st_mode & (S_IWGRP | S_IWOTH)) == 0;
}

static int connect_existing_socket(const char *path)
{
    struct sockaddr_un address = {0};
    size_t length = strlen(path);
    address.sun_family = AF_UNIX;
    address.sun_len = (uint8_t)(offsetof(struct sockaddr_un, sun_path) + length + 1U);
    if (length >= sizeof(address.sun_path)) return -1;
    memcpy(address.sun_path, path, length + 1U);
    int descriptor = socket(AF_UNIX, SOCK_STREAM, 0);
    if (descriptor < 0) return -1;
    if (connect(descriptor, (struct sockaddr *)&address, sizeof(address)) != 0) {
        (void)close(descriptor);
        return -1;
    }
    (void)close(descriptor);
    return 0;
}

static bool start_listener(mcp_hook_state *state)
{
    if (!mcp_socket_path(state->socket_path, sizeof(state->socket_path)) ||
        !parent_directory_is_private(state->socket_path)) return false;
    struct stat existing;
    if (lstat(state->socket_path, &existing) == 0) {
        if (!S_ISSOCK(existing.st_mode) || existing.st_uid != getuid() ||
            (existing.st_mode & (S_IRWXG | S_IRWXO)) != 0) return false;
        if (connect_existing_socket(state->socket_path) == 0) return false;
        if (unlink(state->socket_path) != 0) return false;
    }
    int descriptor = socket(AF_UNIX, SOCK_STREAM, 0);
    if (descriptor < 0) return false;
    struct sockaddr_un address = {0};
    size_t length = strlen(state->socket_path);
    address.sun_family = AF_UNIX;
    address.sun_len = (uint8_t)(offsetof(struct sockaddr_un, sun_path) + length + 1U);
    memcpy(address.sun_path, state->socket_path, length + 1U);
    if (bind(descriptor, (struct sockaddr *)&address, sizeof(address)) != 0 ||
        chmod(state->socket_path, S_IRUSR | S_IWUSR) != 0 ||
        listen(descriptor, (int)MCP_BROKER_IO_CLIENTS) != 0) {
        (void)close(descriptor);
        (void)unlink(state->socket_path);
        return false;
    }
    int flags = fcntl(descriptor, F_GETFL, 0);
    if (flags < 0 || fcntl(descriptor, F_SETFL, flags | O_NONBLOCK) != 0) {
        (void)close(descriptor);
        (void)unlink(state->socket_path);
        return false;
    }
    state->listener = descriptor;
    state->owns_socket = true;
    return true;
}

static bool ensure_hook(viewer_app *app)
{
    if (hook.app == app && hook.broker != NULL) return true;
    if (hook.app == app && hook.start_attempted) return false;
    if (hook.app != NULL) mcp_broker_hook_shutdown(hook.app);
    hook = (mcp_hook_state){.listener = -1, .app = app, .start_attempted = true};
    mcp_broker_host host = {0};
    host.context = app;
    host.current_edit = host_current_edit;
    host.current_path = host_current_path;
    host.current_selection = host_current_selection;
    host.replace_current_edit = host_replace_edit;
    host.destroy_current_edit = host_destroy_edit;
    host.set_current_path = host_set_path;
    host.agent_edited = host_agent_edited;
    host.agent_action_completed = host_agent_action_completed;
    host.selection_changed = host_selection_changed;
    host.current_timeline_session = host_current_timeline_session;
    host.current_project_bundle = host_current_project_bundle;
    host.native_timeline_changed = host_native_timeline_changed;
    host.add_native_drawing_asset = host_add_native_drawing_asset;
    host.save_current_project = host_save_current_project;
    if (!mcp_broker_create(&host, &hook.broker)) return false;
    mcp_broker_io_callbacks callbacks = {process_line, client_disconnected, NULL};
    if (!mcp_broker_io_create(&callbacks, &hook.io)) {
        mcp_broker_destroy(hook.broker);
        hook.broker = NULL;
        return false;
    }
    if (!start_listener(&hook)) {
        mcp_broker_io_destroy(hook.io);
        hook.io = NULL;
        mcp_broker_destroy(hook.broker);
        hook.broker = NULL;
        return false;
    }
    return true;
}

static mcp_hook_session *find_session(mcp_broker_io_owner owner)
{
    for (size_t index = 0U; index < MCP_BROKER_IO_CLIENTS; ++index)
        if (hook.sessions[index].owner == owner) return &hook.sessions[index];
    return NULL;
}

static void client_disconnected(void *context, mcp_broker_io_owner owner,
                                mcp_broker_io_reason reason)
{
    (void)context;
    (void)reason;
    mcp_hook_session *session = find_session(owner);
    if (session != NULL) *session = (mcp_hook_session){0};
}

static bool process_line(void *context, mcp_broker_io_owner owner,
                         const uint8_t *line, size_t length)
{
    (void)context;
    mcp_hook_session *session = find_session(owner);
    if (session == NULL) return false;
    mcp_json_buffer response = {0};
    if (!mcp_json_buffer_init(&response, 256U, MCP_JSON_MAX_BYTES)) return false;
    mcp_protocol_backend backend = mcp_broker_protocol_backend(hook.broker);
    bool okay = mcp_protocol_handle(&session->protocol, line, length, &backend, &response);
    if (okay && response.length > 0U) {
        mcp_broker_io_enqueue_result result = mcp_broker_io_enqueue(hook.io,
            owner, (const uint8_t *)response.data, response.length,
            MCP_BROKER_IO_TERMINAL, NULL, 0U);
        okay = result == MCP_BROKER_IO_QUEUED || result == MCP_BROKER_IO_CLOSED;
    }
    mcp_json_buffer_destroy(&response);
    return okay;
}

static void accept_clients(mcp_hook_state *state)
{
    for (size_t accepted = 0U; accepted < MCP_BROKER_IO_CLIENTS; accepted++) {
        int descriptor = accept(state->listener, NULL, NULL);
        if (descriptor < 0) {
            if (errno != EAGAIN && errno != EWOULDBLOCK && errno != EINTR && !state->reported_error) {
                (void)fputs("vexel MCP broker: accept failed\n", stderr);
                state->reported_error = true;
            }
            return;
        }
        uid_t peer_uid;
        gid_t peer_gid;
        if (getpeereid(descriptor, &peer_uid, &peer_gid) != 0 || peer_uid != getuid()) {
            (void)close(descriptor);
            continue;
        }
        size_t index = 0U;
        while (index < MCP_BROKER_IO_CLIENTS && state->sessions[index].owner != 0U) index++;
        mcp_broker_io_owner owner = 0U;
        if (index == MCP_BROKER_IO_CLIENTS ||
            !mcp_broker_io_attach(state->io, descriptor, &owner)) {
            (void)close(descriptor);
            continue;
        }
        state->sessions[index] = (mcp_hook_session){.owner = owner};
        mcp_protocol_session_reset(&state->sessions[index].protocol);
    }
}

void mcp_broker_hook_poll(viewer_app *app)
{
    if (app == NULL || app->smoke_mode || !ensure_hook(app)) return;
    mcp_broker_bind_current(hook.broker);
    accept_clients(&hook);
    (void)mcp_broker_io_poll(hook.io);
}

void mcp_broker_hook_shutdown(viewer_app *app)
{
    if (hook.app == NULL || (app != NULL && hook.app != app)) return;
    mcp_broker_io_destroy(hook.io);
    hook.io = NULL;
    if (hook.listener >= 0) (void)close(hook.listener);
    if (hook.owns_socket) (void)unlink(hook.socket_path);
    mcp_broker_destroy(hook.broker);
    hook = (mcp_hook_state){.listener = -1};
}
