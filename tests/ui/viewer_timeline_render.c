#include "raster/shape_render.h"
#include "ui/ui_draw.h"
#include "viewer_chrome.h"
#include "viewer_editor.h"
#include "viewer_timeline.h"

#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define RENDER_MAX_AXIS 8192U
#define RENDER_MAX_BYTES (128U * 1024U * 1024U)

static const char sample_svg[] =
    "<svg xmlns='http://www.w3.org/2000/svg' width='64' height='64'>"
    "<rect id='logo' x='8' y='12' width='24' height='20' rx='3' fill='#6b9eff'>"
    "<animate id='anim-logo-position' attributeName='x' values='8;30;8' keyTimes='0;0.55;1' "
    "dur='4s' calcMode='linear'/></rect>"
    "<circle id='pulse' cx='45' cy='42' r='9' fill='#ef8354'>"
    "<animate id='anim-pulse-opacity' attributeName='opacity' values='1;0.25;1' dur='2s' "
    "repeatCount='indefinite' calcMode='discrete'/></circle></svg>";

static void fill_rgba(uint8_t *rgba, size_t length, uint32_t packed)
{
    uint8_t color[4] = {(uint8_t)(packed >> 24U), (uint8_t)(packed >> 16U),
        (uint8_t)(packed >> 8U), 255U};
    for (size_t offset = 0U; offset + 3U < length; offset += 4U)
        memcpy(rgba + offset, color, sizeof(color));
}

static bool fill_rect(uint8_t *rgba, uint32_t width, uint32_t height,
    size_t stride, ui_rect rect, uint32_t packed)
{
    if (rgba == NULL || rect.x > width || rect.y > height ||
        rect.width > width - rect.x || rect.height > height - rect.y) return false;
    uint8_t color[4] = {(uint8_t)(packed >> 24U), (uint8_t)(packed >> 16U),
        (uint8_t)(packed >> 8U), 255U};
    for (uint32_t y = rect.y; y < rect.y + rect.height; ++y)
        for (uint32_t x = rect.x; x < rect.x + rect.width; ++x)
            memcpy(rgba + (size_t)y * stride + (size_t)x * 4U, color, sizeof(color));
    return true;
}

static bool paint_tile(uint8_t *rgba, uint32_t width, uint32_t height,
    size_t stride, ui_rect canvas, const raster_tile *tile, uint32_t backing_scale)
{
    if (rgba == NULL || tile == NULL || tile->rgba == NULL || tile->width == 0U ||
        tile->height == 0U || backing_scale == 0U || canvas.width == 0U ||
        canvas.height == 0U) return false;
    uint32_t target_width = 360U * backing_scale;
    if (target_width > canvas.width / 2U) target_width = canvas.width / 2U;
    if (target_width > canvas.height / 2U) target_width = canvas.height / 2U;
    if (target_width == 0U) return false;
    double ratio = (double)tile->height / (double)tile->width;
    double target_height_value = (double)target_width * ratio;
    if (!isfinite(target_height_value) || target_height_value < 1.0 ||
        target_height_value > (double)canvas.height / 2.0) return false;
    uint32_t target_height = (uint32_t)lround(target_height_value);
    if (target_height == 0U) target_height = 1U;
    uint32_t origin_x = canvas.x + (canvas.width - target_width) / 2U;
    uint32_t origin_y = canvas.y + (canvas.height - target_height) / 2U;
    if (origin_x > width || origin_y > height || target_width > width - origin_x ||
        target_height > height - origin_y) return false;
    for (uint32_t y = 0U; y < target_height; ++y) {
        uint32_t source_y = (uint32_t)(((uint64_t)y * tile->height) / target_height);
        for (uint32_t x = 0U; x < target_width; ++x) {
            uint32_t source_x = (uint32_t)(((uint64_t)x * tile->width) / target_width);
            const uint8_t *source = tile->rgba +
                ((size_t)source_y * tile->width + source_x) * 4U;
            uint8_t *destination = rgba + (size_t)(origin_y + y) * stride +
                (size_t)(origin_x + x) * 4U;
            uint32_t alpha = source[3];
            if (alpha == 0U) continue;
            uint32_t inverse = 255U - alpha;
            for (size_t channel = 0U; channel < 3U; ++channel)
                destination[channel] = (uint8_t)(((uint32_t)source[channel] * alpha +
                    (uint32_t)destination[channel] * inverse + 127U) / 255U);
            destination[3] = 255U;
        }
    }
    return true;
}

int main(int argc, char **argv)
{
    if ((argc != 3 && argc != 4 && argc != 5 && argc != 6) || (strcmp(argv[1], "dark") != 0 &&
            strcmp(argv[1], "light") != 0)) {
        (void)fputs("usage: viewer_timeline_render dark|light 1x|2x [contrast|normal [hover|normal [loop]]]\n", stderr);
        return 2;
    }
    if (argc >= 4 && strcmp(argv[3], "contrast") != 0 &&
        strcmp(argv[3], "normal") != 0) return 2;
    if (argc == 5 && strcmp(argv[4], "hover") != 0) return 2;
    if (argc == 6 && ((strcmp(argv[4], "hover") != 0 &&
            strcmp(argv[4], "normal") != 0) || strcmp(argv[5], "loop") != 0)) return 2;
    bool contrast = argc >= 4 && strcmp(argv[3], "contrast") == 0;
    bool hover = (argc == 5 && strcmp(argv[4], "hover") == 0) ||
        (argc == 6 && strcmp(argv[4], "hover") == 0);
    uint32_t scale = strcmp(argv[2], "2x") == 0 ? 2U :
        strcmp(argv[2], "1x") == 0 ? 1U : 0U;
    if (scale == 0U) return 2;
    uint32_t width = 1280U * scale;
    uint32_t height = 800U * scale;
    if ((size_t)width > SIZE_MAX / 4U ||
        (size_t)height > SIZE_MAX / ((size_t)width * 4U)) return 2;
    size_t length = (size_t)width * height * 4U;
    if (length > RENDER_MAX_BYTES) return 2;
    ui_theme theme = strcmp(argv[1], "light") == 0 ? UI_THEME_LIGHT : UI_THEME_DARK;
    base_bytes source = {(const uint8_t *)sample_svg, sizeof(sample_svg) - 1U};
    svg_shape_document document = {0};
    svg_shape_diagnostic diagnostic = {0};
    if (svg_shape_decode(source, &document, &diagnostic) != SVG_SHAPE_ERROR_NONE)
        return 1;
    raster_tile artwork = {0};
    if (!raster_shape_render(&document, &artwork, &diagnostic)) {
        svg_shape_document_destroy(&document);
        return 1;
    }
    viewer_timeline timeline;
    viewer_timeline_init(&timeline);
    viewer_timeline_reset(&timeline, &document);
    if (argc == 6)
        (void)viewer_timeline_set_mode(&timeline, VIEWER_TIMELINE_MODE_LOOP);
    viewer_editor *editor = viewer_editor_create_styled(source, theme, width, height, true);
    uint8_t *rgba = calloc(1U, length);
    if (editor == NULL || rgba == NULL ||
        !viewer_editor_set_timeline_state(editor, true, true,
            VIEWER_TIMELINE_DOCK_DEFAULT_POINTS)) {
        free(rgba);
        viewer_editor_destroy(editor);
        viewer_timeline_destroy(&timeline);
        raster_tile_destroy(&artwork);
        svg_shape_document_destroy(&document);
        return 1;
    }
    (void)viewer_editor_set_appearance(editor, theme, contrast, false);
    viewer_editor_resize(editor, width, height, (double)scale);
    ui_tokens tokens = ui_theme_tokens(theme, contrast);
    fill_rgba(rgba, length, tokens.window);
    viewer_chrome_layout layout;
    bool okay = viewer_editor_get_chrome_layout(editor, &layout) &&
        fill_rect(rgba, width, height, (size_t)width * 4U,
            layout.canvas, tokens.canvas_well) &&
        paint_tile(rgba, width, height, (size_t)width * 4U,
            layout.canvas, &artwork, scale);
    viewer_editor_paint_scalar(editor, rgba, length, (size_t)width * 4U);
    viewer_timeline_layout timeline_layout;
    if (hover && viewer_timeline_layout_compute(&timeline, width, height,
            (double)scale, &timeline_layout)) {
        timeline.hover_valid = true;
        timeline.hover_x = timeline_layout.play_button.x + 1.0;
        timeline.hover_y = timeline_layout.play_button.y + 1.0;
    }
    okay = okay && viewer_timeline_paint_scalar_appearance(&timeline, editor,
        rgba, length, width, height, (size_t)width * 4U, (double)scale,
        theme, contrast, false);
    if (okay && hover) {
        static ui_semantics_snapshot snapshot;
        okay = viewer_timeline_accessibility_snapshot(&timeline, &timeline_layout,
            theme, contrast, &snapshot);
        if (okay) {
            (void)fputs("{\"nodes\":[", stderr);
            for (size_t index = 0U; index < snapshot.count; ++index) {
                const ui_semantics_node *node = &snapshot.nodes[index];
                const char *label = NULL, *value = NULL;
                size_t label_length = 0U, value_length = 0U;
                okay = ui_semantics_text(&snapshot, node, false, &label, &label_length) &&
                    ui_semantics_text(&snapshot, node, true, &value, &value_length);
                if (!okay) break;
                (void)fprintf(stderr,
                    "%s{\"label\":\"%.*s\",\"value\":\"%.*s\",\"state\":%u}",
                    index == 0U ? "" : ",", (int)label_length, label,
                    (int)value_length, value, node->state);
            }
            (void)fputs("]}\n", stderr);
        }
    }
    if (okay) okay = fwrite(rgba, 1U, length, stdout) == length;
    free(rgba);
    viewer_editor_destroy(editor);
    viewer_timeline_destroy(&timeline);
    raster_tile_destroy(&artwork);
    svg_shape_document_destroy(&document);
    return okay ? 0 : 1;
}
