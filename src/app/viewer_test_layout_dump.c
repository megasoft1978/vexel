#define _POSIX_C_SOURCE 200809L

#include "viewer_internal.h"

#include "viewer_editor_test_layout.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static size_t layout_path_bounded_length(const char *text, size_t limit)
{
    size_t length = 0U;
    if (text == NULL) return 0U;
    while (length <= limit && text[length] != '\0') length++;
    return length;
}

static double layout_dump_monotonic_ms(void)
{
    struct timespec now;
    if (clock_gettime(CLOCK_MONOTONIC, &now) != 0) return 0.0;
    return (double)now.tv_sec * 1000.0 + (double)now.tv_nsec / 1000000.0;
}

bool viewer_test_layout_dump(viewer_app *app)
{
    const char *path = getenv("VEXEL_TEST_LAYOUT_DUMP");
    if (path == NULL || path[0] == '\0' || app == NULL || app->editor == NULL ||
        !app->navigation_ready || !app->document_visible) return true;
    double now = layout_dump_monotonic_ms();
    if (now - app->test_layout_last_write_ms < 100.0) return true;
    app->test_layout_last_write_ms = now;

    size_t path_length = layout_path_bounded_length(path, PLATFORM_PATH_CAP - 1U);
    double origin_x = 0.0, origin_y = 0.0;
    char temporary[PLATFORM_PATH_CAP + 8U];
    if (path_length == 0U || path_length >= PLATFORM_PATH_CAP ||
        path_length + sizeof(".tmp") > sizeof(temporary) ||
        !platform_window_content_screen_origin(app->window, &origin_x, &origin_y)) {
        if (!app->test_layout_dump_failed)
            (void)fputs("viewer: invalid VEXEL_TEST_LAYOUT_DUMP target or window origin\n",
                stderr);
        app->test_layout_dump_failed = true;
        return false;
    }
    memcpy(temporary, path, path_length);
    memcpy(temporary + path_length, ".tmp", sizeof(".tmp"));
    FILE *stream = fopen(temporary, "wb");
    if (stream == NULL) goto failed;
    double translate_x = (double)app->navigation_canvas.x + app->navigation.pan_x;
    double translate_y = (double)app->navigation_canvas.y + app->navigation.pan_y;
    bool okay = fprintf(stream,
        "{\"window_content_origin\":{\"x\":%.9g,\"y\":%.9g},"
        "\"backing_scale\":%.9g,\"window\":{\"width\":%u,\"height\":%u},"
        "\"canvas\":{\"x\":%u,\"y\":%u,\"width\":%u,\"height\":%u},"
        "\"document\":{\"width\":%u,\"height\":%u},"
        "\"view\":{\"scale\":%.9g,\"translate_x\":%.9g,\"translate_y\":%.9g},"
        "\"nodes\":",
        origin_x, origin_y, app->backing_scale, app->width, app->height,
        app->navigation_canvas.x, app->navigation_canvas.y,
        app->navigation_canvas.width, app->navigation_canvas.height,
        app->document.tile.width, app->document.tile.height,
        app->navigation.scale, translate_x, translate_y) >= 0 &&
        viewer_editor_test_layout_write_nodes(app->editor, stream) &&
        fputs("}\n", stream) != EOF;
    if (fclose(stream) != 0) okay = false;
    if (!okay || rename(temporary, path) != 0) goto failed;
    return true;

failed:
    if (!app->test_layout_dump_failed)
        (void)fprintf(stderr, "viewer: could not write VEXEL_TEST_LAYOUT_DUMP: %s\n",
            path);
    app->test_layout_dump_failed = true;
    (void)remove(temporary);
    return false;
}
