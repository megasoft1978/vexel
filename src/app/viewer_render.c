#include "viewer_internal.h"

#include "frame_compose.h"
#include "platform/platform_metal.h"

#include <stdio.h>
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
    if (!app->frame_dirty || app->frame == NULL || app->width == 0U || app->height == 0U)
        return true;
    viewer_frame_kind kind = app->path[0] == '\0' && !app->pending_open && app->job == NULL ?
        VIEWER_FRAME_EMPTY : VIEWER_FRAME_LOADING;
    const raster_tile *tile = NULL;
    const ui_viewer_navigation *navigation = NULL;
    const char *message = NULL;
    if (app->load_result_handled && app->load_status != VIEWER_LOAD_OK) {
        kind = VIEWER_FRAME_ERROR;
        message = viewer_error_reason(app);
    } else if (app->document_visible && app->navigation_ready) {
        kind = VIEWER_FRAME_DOCUMENT;
        tile = &app->document.tile;
        navigation = &app->navigation;
    }
    viewer_sharp_tile frame_tiles[VIEWER_SHARP_CACHE_ENTRIES];
    size_t tile_count = 0U;
    for (size_t index = 0U; index < VIEWER_SHARP_CACHE_ENTRIES; index++) {
        const viewer_sharp_tile *cached = &app->sharp_cache.entries[index].tile;
        if (cached->rgba != NULL && cached->region.generation == app->generation)
            frame_tiles[tile_count++] = *cached;
    }
    char status[VIEWER_STATUS_CAP] = {0};
    bool has_status = make_status_text(app, status);
    viewer_frame_options options = {
        .checkerboard = app->checkerboard,
        .background_index = app->background_index,
        .pixel_grid = app->pixel_grid,
        .error_hint = true,
        .sharp_tiles = frame_tiles,
        .sharp_tile_count = tile_count,
        .status_text = has_status ? status : NULL
    };
    if (!viewer_compose_frame_ex(tile, navigation, &app->progress, kind, message,
            &options, app->frame, app->frame_bytes, app->width, app->height)) {
        (void)fputs("viewer: frame composition failed\n", stderr);
        return false;
    }
    app->frame_dirty = false;
    if (!platform_metal_present(app->window, app->frame, app->frame_bytes,
            app->width, app->height, app->diagnostic, sizeof(app->diagnostic))) {
        (void)fprintf(stderr, "viewer: Metal presentation failed: %s\n",
            app->diagnostic[0] == '\0' ? "unknown error" : app->diagnostic);
        return false;
    }
    app->presented_frames++;
    return true;
}

