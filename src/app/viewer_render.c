#include "viewer_internal.h"

#include "frame_compose.h"
#include "platform/platform_metal.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static bool make_status_text(const viewer_app *app, char out[VIEWER_STATUS_CAP])
{
    if (!app->document_visible || !app->navigation_ready) return false;
    int amount;
    if (app->pointer_valid) {
        amount = snprintf(out, VIEWER_STATUS_CAP,
            "Z %.0f%% X %.1f Y %.1f DOC %ux%u LOAD %.0fMS RENDER %.0fMS",
            app->navigation.scale * 100.0, app->pointer_doc_x, app->pointer_doc_y,
            app->document.tile.width, app->document.tile.height, app->load_ms, app->render_ms);
    } else {
        amount = snprintf(out, VIEWER_STATUS_CAP,
            "Z %.0f%% X -- Y -- DOC %ux%u LOAD %.0fMS RENDER %.0fMS",
            app->navigation.scale * 100.0, app->document.tile.width,
            app->document.tile.height, app->load_ms, app->render_ms);
    }
    return amount >= 0 && (size_t)amount < VIEWER_STATUS_CAP;
}

bool viewer_compose_and_present(viewer_app *app)
{
    if (app->editor != NULL && (app->timeline.native_project ||
            viewer_timeline_track_count(&app->timeline) > 0U)) {
        bool requested_open = viewer_editor_timeline_visible(app->editor);
        if (requested_open != app->timeline.dock_open)
            (void)viewer_timeline_dock_set_open(&app->timeline, requested_open);
        viewer_timeline_set_reduced_motion(&app->timeline,
            viewer_editor_reduced_motion(app->editor));
        viewer_timeline_sync_selection(&app->timeline, app->editor);
    }
    if (app->document_visible && viewer_timeline_tick(&app->timeline))
        app->frame_dirty = true;
    if (!app->frame_dirty || app->frame == NULL || app->width == 0U || app->height == 0U)
        return true;
    viewer_frame_kind kind = VIEWER_FRAME_EMPTY;
    const raster_tile *tile = NULL;
    const ui_viewer_navigation *navigation = NULL;
    const char *message = NULL;
    if (app->document_visible && app->navigation_ready) {
        kind = VIEWER_FRAME_DOCUMENT;
        tile = &app->document.tile;
        navigation = &app->navigation;
    }
    bool animation_tracks = kind == VIEWER_FRAME_DOCUMENT &&
        viewer_timeline_track_count(&app->timeline) > 0U;
    if (animation_tracks) {
        const raster_tile *animated = NULL;
        if (app->timeline.native_project) {
            bool already_failed = app->timeline.preview_failed;
            if (viewer_timeline_native_preview_is_current(&app->timeline)) {
                animated = &app->timeline.preview;
            } else {
                size_t length = viewer_editor_byte_length(app->editor);
                size_t capacity = length <= SIZE_MAX - VIEWER_EDITOR_PREVIEW_EXTRA_BYTES ?
                    length + VIEWER_EDITOR_PREVIEW_EXTRA_BYTES : 0U;
                uint8_t *snapshot = capacity == 0U ? NULL : malloc(capacity);
                size_t materialized = 0U;
                if (snapshot != NULL && viewer_editor_materialize_preview(app->editor,
                        snapshot, capacity, &materialized))
                    animated = viewer_timeline_prepare_native_frame(&app->timeline,
                        (base_bytes){snapshot, materialized});
                free(snapshot);
            }
            if (animated == NULL) app->timeline.preview_failed = true;
            if (animated == NULL && !already_failed)
                viewer_editor_set_status(app->editor,
                    "Native animation preview unavailable: current scene snapshot could not be rendered");
        } else animated = viewer_timeline_prepare_frame(&app->timeline);
        if (animated != NULL) tile = animated;
        else if (app->timeline.native_project) {
            kind = VIEWER_FRAME_ERROR;
            tile = NULL;
            message = "Native animation preview unavailable";
        } else animation_tracks = false;
    }
    viewer_sharp_tile frame_tiles[VIEWER_SHARP_CACHE_ENTRIES];
    size_t tile_count = 0U;
    if (!animation_tracks) {
        for (size_t index = 0U; index < VIEWER_SHARP_CACHE_ENTRIES; index++) {
            const viewer_sharp_tile *cached = &app->sharp_cache.entries[index].tile;
            if (cached->rgba != NULL && cached->region.generation == app->generation)
                frame_tiles[tile_count++] = *cached;
        }
    }
    char status[VIEWER_STATUS_CAP] = {0};
    bool has_status = make_status_text(app, status);
    ui_viewer_navigation frame_navigation = viewer_navigation_for_frame(app);
    size_t total_tiles = app->sharp_plan.columns > 0U && app->sharp_plan.rows > 0U ?
        (size_t)app->sharp_plan.columns * app->sharp_plan.rows : 0U;
    size_t done_tiles = app->sharp_cache.count;
    if (total_tiles > 0U && done_tiles > total_tiles) done_tiles = total_tiles;
    bool tiles_active = app->sharp_plan_ready ||
        (app->job != NULL && app->job->kind == VIEWER_JOB_SHARP_TILE);
    viewer_editor_set_canvas_status(app->editor, app->navigation.scale,
        app->pointer_valid, app->pointer_doc_x, app->pointer_doc_y,
        app->document.tile.width, app->document.tile.height,
        done_tiles, total_tiles, tiles_active);
    viewer_frame_options options = {
        .checkerboard = app->checkerboard,
        .background_index = app->background_index,
        .pixel_grid = app->pixel_grid,
        .error_hint = true,
        .sharp_tiles = frame_tiles,
        .sharp_tile_count = tile_count,
        .status_text = has_status ? status : NULL
    };
    const ui_viewer_navigation *paint_navigation = kind == VIEWER_FRAME_DOCUMENT ?
        &frame_navigation : navigation;
    if (!viewer_compose_frame_ex(tile, paint_navigation, &app->progress, kind, message,
            &options, app->frame, app->frame_bytes, app->width, app->height)) {
        (void)fputs("viewer: frame composition failed\n", stderr);
        return false;
    }
    viewer_editor_paint(app->editor, app->frame, app->frame_bytes,
        (size_t)app->width * 4U);
    if (app->editor != NULL && kind == VIEWER_FRAME_DOCUMENT)
        viewer_editor_paint_canvas_overlay(app->editor, &app->document.shape,
            &app->navigation, app->navigation_canvas, app->frame,
            app->width, app->height, (size_t)app->width * 4U);
    if (app->editor != NULL && app->document_visible)
        (void)viewer_timeline_paint(&app->timeline, app->editor, app->frame,
            app->frame_bytes, app->width, app->height,
            (size_t)app->width * 4U, app->backing_scale);
    viewer_home_state home_state;
    viewer_home_state_from_app(app, &home_state);
    viewer_home_paint(&home_state, app->theme, app->frame, app->frame_bytes,
        (size_t)app->width * 4U, app->width, app->height, app->backing_scale);
    app->frame_dirty = false;
    (void)platform_window_set_material_enabled(app->window,
        viewer_editor_native_material_enabled(app->editor));
    if (!platform_metal_present(app->window, app->frame, app->frame_bytes,
            app->width, app->height, app->diagnostic, sizeof(app->diagnostic))) {
        (void)fprintf(stderr, "viewer: Metal presentation failed: %s\n",
            app->diagnostic[0] == '\0' ? "unknown error" : app->diagnostic);
        return false;
    }
    app->presented_frames++;
    return true;
}

