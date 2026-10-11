#include "app/frame_compose.h"

#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

static bool near(double left, double right)
{
    return fabs(left - right) < 1e-9;
}

static bool check_fit_and_cursor_zoom(void)
{
    ui_viewer_navigation navigation;
    if (!ui_viewer_navigation_init(&navigation, 4.0, 4.0, 2.0, 1.0) ||
        !ui_viewer_navigation_fit(&navigation) || !near(navigation.scale, 2.0) ||
        !near(navigation.pan_x, 0.0) || !near(navigation.pan_y, 1.0)) return false;
    double world_x = (1.75 - navigation.pan_x) / navigation.scale;
    double world_y = (2.25 - navigation.pan_y) / navigation.scale;
    double before_x = 0.0, before_y = 0.0, after_x = 0.0, after_y = 0.0;
    if (!ui_viewer_navigation_project(&navigation, world_x, world_y, &before_x, &before_y) ||
        !ui_viewer_navigation_zoom(&navigation, 1.5, 1.75, 2.25) ||
        !ui_viewer_navigation_project(&navigation, world_x, world_y, &after_x, &after_y) ||
        !near(before_x, after_x) || !near(before_y, after_y)) return false;
    return true;
}

static bool check_pan_clamp(void)
{
    ui_viewer_navigation navigation;
    if (!ui_viewer_navigation_init(&navigation, 100.0, 100.0, 100.0, 100.0)) return false;
    navigation.scale = 2.0;
    navigation.pan_x = 100000.0;
    navigation.pan_y = -100000.0;
    viewer_navigation_clamp(&navigation);
    return near(navigation.pan_x, 90.0) && near(navigation.pan_y, -190.0);
}

static bool check_sampling_and_checkerboard(void)
{
    uint8_t colors[8] = {255U, 0U, 0U, 255U, 0U, 0U, 255U, 255U};
    raster_tile tile = {2U, 1U, sizeof(colors), colors};
    ui_viewer_navigation navigation;
    uint8_t frame[4U * 2U * 4U] = {0};
    if (!ui_viewer_navigation_init(&navigation, 4.0, 2.0, 2.0, 1.0) ||
        !ui_viewer_navigation_fit(&navigation) || navigation.scale != 2.0 ||
        !viewer_compose_frame(&tile, &navigation, NULL, VIEWER_FRAME_DOCUMENT, NULL,
                              frame, sizeof(frame), 4U, 2U)) return false;
    size_t middle = 1U * 4U;
    if (frame[middle] != 191U || frame[middle + 1U] != 0U ||
        frame[middle + 2U] != 64U || frame[middle + 3U] != 255U) return false;

    navigation.scale = 5.0;
    navigation.pan_x = 0.0;
    navigation.pan_y = 0.0;
    navigation.viewport_width = 10.0;
    navigation.viewport_height = 1.0;
    uint8_t nearest_frame[10U * 4U] = {0};
    if (!viewer_compose_frame(&tile, &navigation, NULL, VIEWER_FRAME_DOCUMENT, NULL,
                              nearest_frame, sizeof(nearest_frame), 10U, 1U) ||
        nearest_frame[4U * 4U] != 255U || nearest_frame[4U * 4U + 2U] != 0U ||
        nearest_frame[5U * 4U] != 0U || nearest_frame[5U * 4U + 2U] != 255U) return false;

    uint8_t transparent_rgba[4] = {0U, 0U, 0U, 0U};
    raster_tile transparent = {1U, 1U, sizeof(transparent_rgba), transparent_rgba};
    if (!ui_viewer_navigation_init(&navigation, 17.0, 17.0, 1.0, 1.0) ||
        !ui_viewer_navigation_fit(&navigation)) return false;
    uint8_t checker[17U * 17U * 4U] = {0};
    if (!viewer_compose_frame(&transparent, &navigation, NULL, VIEWER_FRAME_DOCUMENT, NULL,
                              checker, sizeof(checker), 17U, 17U)) return false;
    size_t light = 0U;
    size_t dark = (size_t)16U * 4U;
    return checker[light] == 224U && checker[light + 1U] == 226U &&
        checker[light + 2U] == 230U && checker[dark] == 204U &&
        checker[dark + 1U] == 206U && checker[dark + 2U] == 210U;
}

static bool check_pixel_grid_threshold_and_opacity(void)
{
    uint8_t transparent_pixel[4] = {0U, 0U, 0U, 0U};
    raster_tile transparent = {1U, 1U, sizeof(transparent_pixel), transparent_pixel};
    ui_viewer_navigation navigation;
    viewer_frame_options options = {.background_index = 1U, .pixel_grid = true};
    uint8_t frame[16U * 16U * 4U] = {0U};
    if (!ui_viewer_navigation_init(&navigation, 16.0, 16.0, 1.0, 1.0)) return false;
    navigation.scale = 8.0;
    navigation.pan_x = 1.0;
    navigation.pan_y = 1.0;
    if (!viewer_compose_frame_ex(&transparent, &navigation, NULL,
            VIEWER_FRAME_DOCUMENT, NULL, &options, frame, sizeof(frame), 16U, 16U) ||
        frame[((size_t)2U * 16U + 1U) * 4U] != 250U) return false;

    navigation.scale = 9.0;
    if (!viewer_compose_frame_ex(&transparent, &navigation, NULL,
            VIEWER_FRAME_DOCUMENT, NULL, &options, frame, sizeof(frame), 16U, 16U) ||
        frame[((size_t)2U * 16U + 1U) * 4U] != 234U ||
        frame[((size_t)2U * 16U + 2U) * 4U] != 250U) return false;

    options.background_index = 2U;
    if (!viewer_compose_frame_ex(&transparent, &navigation, NULL,
            VIEWER_FRAME_DOCUMENT, NULL, &options, frame, sizeof(frame), 16U, 16U) ||
        frame[((size_t)2U * 16U + 1U) * 4U] <= 24U ||
        frame[((size_t)2U * 16U + 1U) * 4U] >= 64U ||
        frame[((size_t)2U * 16U + 2U) * 4U] != 24U) return false;

    options.pixel_grid = false;
    return viewer_compose_frame_ex(&transparent, &navigation, NULL,
            VIEWER_FRAME_DOCUMENT, NULL, &options, frame, sizeof(frame), 16U, 16U) &&
        frame[((size_t)2U * 16U + 1U) * 4U] == 24U;
}

static bool check_sharp_tile_overlays_preview(void)
{
    uint8_t preview_pixels[4U * 4U * 4U];
    for (size_t index = 0U; index < sizeof(preview_pixels); index += 4U) {
        preview_pixels[index] = 255U;
        preview_pixels[index + 1U] = 0U;
        preview_pixels[index + 2U] = 0U;
        preview_pixels[index + 3U] = 255U;
    }
    raster_tile preview = {4U, 4U, sizeof(preview_pixels), preview_pixels};
    ui_viewer_navigation navigation;
    if (!ui_viewer_navigation_init(&navigation, 4.0, 4.0, 4.0, 4.0) ||
        !ui_viewer_navigation_actual_size(&navigation)) return false;
    uint8_t sharp_pixels[4U] = {0U, 255U, 0U, 255U};
    viewer_sharp_tile sharp = {{7U, 0U, 0U, 1U, 1U, 1U, 1U},
                               sizeof(sharp_pixels), sharp_pixels};
    viewer_frame_options options = {.sharp_tiles = &sharp, .sharp_tile_count = 1U};
    uint8_t frame[4U * 4U * 4U] = {0U};
    if (!viewer_compose_frame_ex(&preview, &navigation, NULL, VIEWER_FRAME_DOCUMENT,
            NULL, &options, frame, sizeof(frame), 4U, 4U)) return false;
    size_t pixel = (1U * 4U + 1U) * 4U;
    return frame[pixel] == 0U && frame[pixel + 1U] == 255U &&
        frame[pixel + 2U] == 0U && frame[pixel + 3U] == 255U &&
        frame[0] == 255U && frame[1] == 0U && frame[2] == 0U;
}

static bool check_empty_window_prompt(void)
{
    uint8_t frame[320U * 80U * 4U] = {0U};
    if (!viewer_compose_frame(NULL, NULL, NULL, VIEWER_FRAME_EMPTY, NULL,
                              frame, sizeof(frame), 320U, 80U)) return false;
    size_t changed = 0U;
    for (size_t index = 0U; index < sizeof(frame); index += 4U) {
        if (frame[index] != 28U || frame[index + 1U] != 34U || frame[index + 2U] != 46U)
            changed++;
    }
    return changed > 0U;
}

static bool check_bad_dimensions(void)
{
    uint8_t pixel[3] = {0U};
    if (viewer_compose_frame(NULL, NULL, NULL, VIEWER_FRAME_LOADING, NULL,
                             pixel, sizeof(pixel), 0U, 1U) ||
        viewer_compose_frame(NULL, NULL, NULL, VIEWER_FRAME_LOADING, NULL,
                             pixel, sizeof(pixel), UINT32_MAX, UINT32_MAX) ||
        viewer_compose_frame(NULL, NULL, NULL, VIEWER_FRAME_LOADING, NULL,
                             pixel, sizeof(pixel), 1U, 1U)) return false;
    return true;
}

static bool check_error_frame(void)
{
    uint8_t frame[64U * 48U * 4U] = {0};
    if (!viewer_compose_frame(NULL, NULL, NULL, VIEWER_FRAME_ERROR, "unsupported",
                              frame, sizeof(frame), 64U, 48U)) return false;
    size_t background = 0U;
    size_t changed = 0U;
    for (size_t index = 0U; index < sizeof(frame); index += 4U) {
        if (frame[index] == 67U && frame[index + 1U] == 21U && frame[index + 2U] == 36U) {
            background++;
        } else {
            changed++;
        }
    }
    return background > 0U && changed > 0U;
}

static bool check_interactive_error_hint(void)
{
    viewer_frame_options options = {.error_hint = true};
    uint8_t frame[320U * 80U * 4U] = {0U};
    if (!viewer_compose_frame_ex(NULL, NULL, NULL, VIEWER_FRAME_ERROR, "cannot open file",
            &options, frame, sizeof(frame), 320U, 80U)) return false;
    for (size_t offset = 0U; offset < sizeof(frame); offset += 4U) {
        if (frame[offset] == 226U && frame[offset + 1U] == 205U &&
            frame[offset + 2U] == 210U) return true;
    }
    return false;
}

int main(void)
{
    if (!check_fit_and_cursor_zoom()) return 1;
    if (!check_pan_clamp()) return 2;
    if (!check_sampling_and_checkerboard()) return 3;
    if (!check_pixel_grid_threshold_and_opacity()) return 9;
    if (!check_sharp_tile_overlays_preview()) return 4;
    if (!check_empty_window_prompt()) return 5;
    if (!check_bad_dimensions()) return 6;
    if (!check_error_frame()) return 7;
    if (!check_interactive_error_hint()) return 8;
    (void)puts("viewer frame composition: fit, zoom, pan clamp, sharp overlays, empty prompt, transparency, limits pass");
    return 0;
}
