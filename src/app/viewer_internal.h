#ifndef VEXEL_APP_VIEWER_INTERNAL_H
#define VEXEL_APP_VIEWER_INTERNAL_H

#include "platform/platform_task.h"
#include "platform/platform_window.h"
#include "ui/viewer_controls.h"
#include "viewer_editor.h"
#include "viewer_load.h"
#include "viewer_quality.h"

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
    VIEWER_JOB_LOAD, VIEWER_JOB_SHARP_TILE, VIEWER_JOB_EDIT_RENDER
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
    char display_name[256];
    char title[VIEWER_TITLE_CAP];
    char diagnostic[192];
    char editor_render_error[192];
    double load_ms;
    double render_ms;
    double pointer_doc_x;
    double pointer_doc_y;
    struct timespec last_file_poll;
    viewer_file_stamp file_stamp;
    viewer_load_status load_status;
    uint8_t background_index;
    bool checkerboard;
    bool pixel_grid;
    bool pointer_valid;
    bool navigation_ready;
    bool navigation_user_zoomed;
    bool space_down;
    bool document_visible;
    bool frame_dirty;
    bool pending_open;
    bool load_result_handled;
    bool sharp_plan_ready;
    bool close_requested;
    bool editor_render_requested;
    bool external_change_pending;
    bool running;
    bool fatal_error;
    bool smoke_mode;
    bool smoke_scripted;
    bool smoke_passed;
    unsigned presented_frames;
};

const char *viewer_error_reason(const viewer_app *app);
bool viewer_set_window_title(viewer_app *app);
void viewer_update_file_stamp(viewer_app *app);
bool viewer_file_changed_on_disk(viewer_app *app);
bool viewer_allocate_frame(viewer_app *app, uint32_t width, uint32_t height);
bool viewer_bump_generation(viewer_app *app);
void viewer_refresh_sharp_plan(viewer_app *app);
bool viewer_request_open(viewer_app *app, const char *path, bool initial);
bool viewer_update_active_path(viewer_app *app, const char *path, size_t path_length);
bool viewer_confirm_unsaved_changes(viewer_app *app, const char *title,
                                    const char *message);
void viewer_request_close(viewer_app *app);
void viewer_process_event(viewer_app *app, const platform_event *event);
bool viewer_export_png(viewer_app *app, bool copy_to_clipboard);
bool viewer_compose_and_present(viewer_app *app);
bool viewer_sync_canvas_geometry(viewer_app *app, bool refit);
ui_viewer_navigation viewer_navigation_for_frame(const viewer_app *app);

#endif
