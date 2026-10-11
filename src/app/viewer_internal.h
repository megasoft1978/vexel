#ifndef VEXEL_APP_VIEWER_INTERNAL_H
#define VEXEL_APP_VIEWER_INTERNAL_H

#include "platform/platform_task.h"
#include "platform/platform_window.h"
#include "ui/viewer_controls.h"
#include "viewer_editor.h"
#include "viewer_load.h"
#include "viewer_quality.h"
#include "viewer_timeline.h"
#include "viewer_home.h"

#include <stdatomic.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <sys/types.h>
#include <time.h>

#define VIEWER_TASK_ID UINT64_C(1)
#define VIEWER_GENERATION UINT64_C(1)
#define VIEWER_FRAME_MAX_AXIS 8192U
#define VIEWER_FRAME_MAX_BYTES (128U * 1024U * 1024U)
#define VIEWER_TITLE_CAP 1024U
#define VIEWER_STATUS_CAP 256U
#define VIEWER_AUTORELOAD_INTERVAL_MS 250.0

typedef struct viewer_app viewer_app;
typedef enum viewer_job_kind {
    VIEWER_JOB_LOAD, VIEWER_JOB_NEW_DOCUMENT, VIEWER_JOB_SHARP_TILE,
    VIEWER_JOB_EDIT_RENDER
} viewer_job_kind;

typedef struct viewer_job {
    viewer_app *app;
    viewer_job_kind kind;
    uint64_t generation;
    char path[PLATFORM_PATH_CAP];
    uint8_t *source_snapshot;
    size_t source_snapshot_length;
    viewer_loaded_document loaded;
    viewer_load_status load_status;
    viewer_sharp_tile sharp_tile;
    ui_viewer_navigation navigation;
    char diagnostic[192];
    double elapsed_ms;
    atomic_bool cleanup_complete;
} viewer_job;

typedef struct viewer_file_stamp {
    off_t size;
    struct timespec modified;
    bool valid;
} viewer_file_stamp;

struct viewer_app {
    platform_window *window;
    platform_task_queue *tasks;
    viewer_loaded_document document;
    viewer_editor *editor;
    viewer_timeline timeline;
    uint8_t *render_source;
    viewer_job *job;
    uint64_t generation;
    uint32_t width;
    uint32_t height;
    double backing_scale;
    ui_viewer_navigation navigation;
    ui_rect navigation_canvas;
    ui_viewer_progress progress;
    viewer_tile_cache sharp_cache;
    viewer_tile_plan sharp_plan;
    uint8_t *frame;
    size_t frame_bytes;
    char path[PLATFORM_PATH_CAP];
    char pending_path[PLATFORM_PATH_CAP];
    char failed_path[PLATFORM_PATH_CAP];
    char load_error[192];
    char display_name[256];
    char title[VIEWER_TITLE_CAP];
    char diagnostic[192];
    char editor_render_error[192];
    double load_ms;
    double render_ms;
    double test_layout_last_write_ms;
    double pointer_doc_x;
    double pointer_doc_y;
    struct timespec last_file_poll;
    viewer_file_stamp file_stamp;
    platform_recent_file recent_files[PLATFORM_RECENT_FILES_MAX];
    size_t recent_file_count;
    viewer_load_status load_status;
    ui_theme theme;
    uint8_t background_index;
    bool checkerboard;
    bool pixel_grid;
    bool modifier_scroll_zooms;
    bool pointer_valid;
    bool drop_hover;
    bool navigation_ready;
    bool navigation_user_zoomed;
    bool space_down;
    bool document_visible;
    bool frame_dirty;
    bool pending_open;
    bool pending_new_document;
    bool load_result_handled;
    bool sharp_plan_ready;
    bool close_requested;
    bool quit_all_requested;
    bool editor_render_requested;
    bool external_change_pending;
    bool running;
    bool fatal_error;
    bool smoke_mode;
    bool smoke_scripted;
    bool smoke_passed;
    bool test_layout_dump_failed;
    unsigned presented_frames;
};

const char *viewer_error_reason(const viewer_app *app);
size_t viewer_bounded_length(const char *text, size_t limit);
void viewer_copy_display_name(const char *path, char out[256]);
bool viewer_test_layout_dump(viewer_app *app);
bool viewer_set_window_title(viewer_app *app);
void viewer_copy_display_name(const char *path, char out[256]);
bool viewer_write_test_layout_dump(viewer_app *app);
void viewer_update_file_stamp(viewer_app *app);
bool viewer_file_changed_on_disk(viewer_app *app);
bool viewer_file_stamp_read(const char *path, viewer_file_stamp *out);
bool viewer_file_stamp_equal(const viewer_file_stamp *left,
                             const viewer_file_stamp *right);
bool viewer_allocate_frame(viewer_app *app, uint32_t width, uint32_t height);
bool viewer_bump_generation(viewer_app *app);
void viewer_refresh_sharp_plan(viewer_app *app);
bool viewer_request_open(viewer_app *app, const char *path, bool initial);
bool viewer_request_new_document(viewer_app *app);
bool viewer_update_active_path(viewer_app *app, const char *path, size_t path_length);
bool viewer_confirm_unsaved_changes(viewer_app *app, const char *title,
                                    const char *message);
size_t viewer_event_path_length(const char *path, size_t limit);
bool viewer_save_document(viewer_app *app, bool force_save_as);
void viewer_request_close(viewer_app *app);
void viewer_close_confirmed(viewer_app *app);
void viewer_apply_action(viewer_app *app, platform_viewer_action action);
void viewer_apply_navigation_key(viewer_app *app, ui_viewer_key key);
void viewer_toggle_checkerboard(viewer_app *app);
void viewer_cycle_background(viewer_app *app);
void viewer_toggle_pixel_grid(viewer_app *app);
void viewer_request_reload_or_open(viewer_app *app);
void viewer_apply_synthetic_key(viewer_app *app, uint16_t key_code,
    uint32_t modifiers, const char *text, size_t text_len);
void viewer_process_editor_result(viewer_app *app, uint32_t result);
void viewer_apply_command(viewer_app *app, viewer_command_id command);
void viewer_app_tick(viewer_app *app);
void viewer_app_destroy(viewer_app *app);
void navigation_changed(viewer_app *app, const ui_viewer_navigation *previous);
void apply_navigation_fit(viewer_app *app, platform_viewer_action action);
void viewer_process_event(viewer_app *app, const platform_event *event);
bool viewer_export_png(viewer_app *app, bool copy_to_clipboard);
bool viewer_compose_and_present(viewer_app *app);
void viewer_home_state_from_app(const viewer_app *app, viewer_home_state *out_state);
bool viewer_test_layout_dump(viewer_app *app);
bool viewer_sync_canvas_geometry(viewer_app *app, bool refit);
ui_viewer_navigation viewer_navigation_for_frame(const viewer_app *app);
void viewer_update_native_state(viewer_app *app);

#endif
