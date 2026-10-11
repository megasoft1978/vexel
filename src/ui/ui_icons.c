#include "ui_icons.h"

#include "ui_icon_paths.h"

#include <errno.h>
#include <math.h>
#include <stdlib.h>
#include <string.h>

#define UI_ICON_VIEWBOX 24U
#define UI_ICON_MAX_SCALE 3U
#define UI_ICON_MAX_AXIS (UI_ICON_VIEWBOX * UI_ICON_MAX_SCALE)
#define UI_ICON_MASK_BYTES (UI_ICON_MAX_AXIS * UI_ICON_MAX_AXIS)
#define UI_ICON_PATH_MAX_BYTES 2048U
#define UI_ICON_MAX_SEGMENTS 384U
#define UI_ICON_AA_SAMPLES 4U
#define UI_ICON_STYLE_CACHE_COUNT 128U
#define UI_ICON_PI 3.14159265358979323846

typedef struct ui_icon_segment { double x0, y0, x1, y1; } ui_icon_segment;
typedef struct ui_icon_style_entry {
    ui_icon_kind icon;
    ui_theme theme;
    bool increased_contrast;
    uint8_t scale;
    ui_control_state state;
    uint32_t color;
    uint64_t use;
    bool valid;
} ui_icon_style_entry;

static uint8_t icon_masks[UI_ICON_COUNT - 1U][UI_ICON_MAX_SCALE + 1U]
    [UI_ICON_MASK_BYTES];
static bool icon_mask_ready[UI_ICON_COUNT - 1U][UI_ICON_MAX_SCALE + 1U];
static ui_icon_style_entry icon_style_cache[UI_ICON_STYLE_CACHE_COUNT];
static uint64_t icon_style_clock;

static bool path_separator(char value)
{
    return value == ' ' || value == '\t' || value == '\r' || value == '\n' || value == ',';
}

static void skip_separators(const char *path, size_t length, size_t *offset)
{
    while (*offset < length && path_separator(path[*offset])) (*offset)++;
}

static bool parse_number(const char *path, size_t length, size_t *offset,
                         double *out_value)
{
    skip_separators(path, length, offset);
    size_t start = *offset;
    while (*offset < length) {
        char c = path[*offset];
        if ((c >= '0' && c <= '9') || c == '+' || c == '-' || c == '.' ||
            c == 'e' || c == 'E') (*offset)++;
        else break;
    }
    size_t token_length = *offset - start;
    if (token_length == 0U || token_length >= 48U) return false;
    char token[48];
    memcpy(token, path + start, token_length);
    token[token_length] = '\0';
    char *end = NULL;
    errno = 0;
    double value = strtod(token, &end);
    if (errno == ERANGE || end != token + token_length || !isfinite(value)) return false;
    *out_value = value;
    return true;
}

static bool add_segment(ui_icon_segment *segments, size_t *count,
                        double x0, double y0, double x1, double y1)
{
    if (*count >= UI_ICON_MAX_SEGMENTS || !isfinite(x0) || !isfinite(y0) ||
        !isfinite(x1) || !isfinite(y1)) return false;
    segments[*count] = (ui_icon_segment){x0, y0, x1, y1};
    (*count)++;
    return true;
}

static bool valid_coordinate(double value)
{
    return isfinite(value) && value >= 0.0 && value <= (double)UI_ICON_VIEWBOX;
}

static bool valid_point(double x, double y)
{
    return valid_coordinate(x) && valid_coordinate(y);
}

static void bounds_include(ui_icon_bounds *bounds, double x, double y)
{
    if (x < bounds->left) bounds->left = x;
    if (x > bounds->right) bounds->right = x;
    if (y < bounds->top) bounds->top = y;
    if (y > bounds->bottom) bounds->bottom = y;
}

static bool parse_pair(const char *path, size_t length, size_t *offset,
                       double *x, double *y, ui_icon_bounds *bounds)
{
    return parse_number(path, length, offset, x) &&
        parse_number(path, length, offset, y) && valid_point(*x, *y) &&
        (bounds_include(bounds, *x, *y), true);
}

static bool flatten_cubic(ui_icon_segment *segments, size_t *count,
                          double x0, double y0, double x1, double y1,
                          double x2, double y2, double x3, double y3)
{
    double px = x0, py = y0;
    for (unsigned step = 1U; step <= 16U; ++step) {
        double t = (double)step / 16.0, u = 1.0 - t;
        double x = u * u * u * x0 + 3.0 * u * u * t * x1 +
            3.0 * u * t * t * x2 + t * t * t * x3;
        double y = u * u * u * y0 + 3.0 * u * u * t * y1 +
            3.0 * u * t * t * y2 + t * t * t * y3;
        if (!add_segment(segments, count, px, py, x, y)) return false;
        px = x; py = y;
    }
    return true;
}

static bool flatten_quadratic(ui_icon_segment *segments, size_t *count,
                              double x0, double y0, double x1, double y1,
                              double x2, double y2)
{
    double px = x0, py = y0;
    for (unsigned step = 1U; step <= 12U; ++step) {
        double t = (double)step / 12.0, u = 1.0 - t;
        double x = u * u * x0 + 2.0 * u * t * x1 + t * t * x2;
        double y = u * u * y0 + 2.0 * u * t * y1 + t * t * y2;
        if (!add_segment(segments, count, px, py, x, y)) return false;
        px = x; py = y;
    }
    return true;
}

static bool flatten_arc(ui_icon_segment *segments, size_t *count,
                        double x1, double y1, double rx, double ry,
                        double rotation, bool large_arc, bool sweep,
                        double x2, double y2)
{
    /* Convert SVG endpoint parameters using the SVG elliptical-arc algorithm. */
    if (x1 == x2 && y1 == y2) return true;
    rx = fabs(rx); ry = fabs(ry);
    if (rx == 0.0 || ry == 0.0) return add_segment(segments, count, x1, y1, x2, y2);
    double phi = rotation * UI_ICON_PI / 180.0;
    double cosine = cos(phi), sine = sin(phi);
    double dx = (x1 - x2) * 0.5, dy = (y1 - y2) * 0.5;
    double xp = cosine * dx + sine * dy;
    double yp = -sine * dx + cosine * dy;
    double lambda = xp * xp / (rx * rx) + yp * yp / (ry * ry);
    if (lambda > 1.0) { double scale = sqrt(lambda); rx *= scale; ry *= scale; }
    double rx2 = rx * rx, ry2 = ry * ry;
    double denominator = rx2 * yp * yp + ry2 * xp * xp;
    double numerator = rx2 * ry2 - denominator;
    double factor = denominator == 0.0 ? 0.0 : sqrt(fmax(0.0, numerator / denominator));
    if (large_arc == sweep) factor = -factor;
    double cxp = factor * rx * yp / ry;
    double cyp = factor * -ry * xp / rx;
    double cx = cosine * cxp - sine * cyp + (x1 + x2) * 0.5;
    double cy = sine * cxp + cosine * cyp + (y1 + y2) * 0.5;
    double ux = (xp - cxp) / rx, uy = (yp - cyp) / ry;
    double vx = (-xp - cxp) / rx, vy = (-yp - cyp) / ry;
    double start = atan2(uy, ux);
    double delta = atan2(ux * vy - uy * vx, ux * vx + uy * vy);
    if (!sweep && delta > 0.0) delta -= 2.0 * UI_ICON_PI;
    if (sweep && delta < 0.0) delta += 2.0 * UI_ICON_PI;
    unsigned steps = (unsigned)ceil(fabs(delta) * fmax(rx, ry) / 0.65);
    if (steps < 4U) steps = 4U;
    if (steps > 64U) steps = 64U;
    double px = x1, py = y1;
    for (unsigned step = 1U; step <= steps; ++step) {
        double angle = start + delta * (double)step / (double)steps;
        double x = cx + cosine * rx * cos(angle) - sine * ry * sin(angle);
        double y = cy + sine * rx * cos(angle) + cosine * ry * sin(angle);
        if (step == steps) { x = x2; y = y2; }
        if (!add_segment(segments, count, px, py, x, y)) return false;
        px = x; py = y;
    }
    return true;
}

static bool path_parse(const char *path, size_t length, ui_icon_segment *segments,
                       size_t *out_count, ui_icon_bounds *out_bounds)
{
    if (path == NULL || length == 0U || length > UI_ICON_PATH_MAX_BYTES ||
        segments == NULL || out_count == NULL || out_bounds == NULL) return false;
    *out_count = 0U;
    *out_bounds = (ui_icon_bounds){24.0, 24.0, 0.0, 0.0};
    size_t offset = 0U;
    double x = 0.0, y = 0.0, start_x = 0.0, start_y = 0.0;
    bool has_current = false;
    while (offset < length) {
        skip_separators(path, length, &offset);
        if (offset == length) break;
        char command = path[offset++];
        if (command == 'Z') {
            if (!has_current) return false;
            if (!add_segment(segments, out_count, x, y, start_x, start_y)) return false;
            x = start_x; y = start_y;
            continue;
        }
        if (command != 'M' && command != 'L' && command != 'C' &&
            command != 'Q' && command != 'A') return false;
        if (command != 'M' && !has_current) return false;
        if (command == 'M') {
            if (!parse_pair(path, length, &offset, &x, &y, out_bounds)) return false;
            start_x = x; start_y = y; has_current = true;
            continue;
        }
        if (command == 'L') {
            double next_x = 0.0, next_y = 0.0;
            if (!parse_pair(path, length, &offset, &next_x, &next_y, out_bounds) ||
                !add_segment(segments, out_count, x, y, next_x, next_y)) return false;
            x = next_x; y = next_y;
            continue;
        }
        if (command == 'C') {
            double x1 = 0.0, y1 = 0.0, x2 = 0.0, y2 = 0.0, nx = 0.0, ny = 0.0;
            if (!parse_pair(path, length, &offset, &x1, &y1, out_bounds) ||
                !parse_pair(path, length, &offset, &x2, &y2, out_bounds) ||
                !parse_pair(path, length, &offset, &nx, &ny, out_bounds) ||
                !flatten_cubic(segments, out_count, x, y, x1, y1, x2, y2, nx, ny)) return false;
            x = nx; y = ny;
            continue;
        }
        if (command == 'Q') {
            double x1 = 0.0, y1 = 0.0, nx = 0.0, ny = 0.0;
            if (!parse_pair(path, length, &offset, &x1, &y1, out_bounds) ||
                !parse_pair(path, length, &offset, &nx, &ny, out_bounds) ||
                !flatten_quadratic(segments, out_count, x, y, x1, y1, nx, ny)) return false;
            x = nx; y = ny;
            continue;
        }
        double rx = 0.0, ry = 0.0, rotation = 0.0, large = 0.0, sweep = 0.0;
        double nx = 0.0, ny = 0.0;
        if (!parse_number(path, length, &offset, &rx) ||
            !parse_number(path, length, &offset, &ry) ||
            !parse_number(path, length, &offset, &rotation) ||
            !parse_number(path, length, &offset, &large) ||
            !parse_number(path, length, &offset, &sweep) ||
            !parse_pair(path, length, &offset, &nx, &ny, out_bounds) ||
            !valid_coordinate(rx) || !valid_coordinate(ry) || fabs(rotation) > 360.0 ||
            (large != 0.0 && large != 1.0) || (sweep != 0.0 && sweep != 1.0) ||
            !flatten_arc(segments, out_count, x, y, rx, ry, rotation,
                large == 1.0, sweep == 1.0, nx, ny)) return false;
        x = nx; y = ny;
    }
    if (*out_count == 0U) return false;
    for (size_t index = 0U; index < *out_count; ++index) {
        if (!valid_point(segments[index].x0, segments[index].y0) ||
            !valid_point(segments[index].x1, segments[index].y1)) return false;
        bounds_include(out_bounds, segments[index].x0, segments[index].y0);
        bounds_include(out_bounds, segments[index].x1, segments[index].y1);
    }
    return out_bounds->left <= out_bounds->right && out_bounds->top <= out_bounds->bottom;
}

size_t ui_icon_path_count(void)
{
    size_t count = 0U;
    (void)ui_icon_definitions(&count);
    return count;
}

bool ui_icon_path_at(size_t index, const char **out_name, const char **out_path,
                     size_t *out_path_length)
{
    if (out_name == NULL || out_path == NULL || out_path_length == NULL) return false;
    size_t count = 0U;
    const ui_icon_definition *items = ui_icon_definitions(&count);
    if (index >= count) return false;
    *out_name = items[index].name;
    *out_path = items[index].path;
    *out_path_length = items[index].path_length;
    return true;
}

bool ui_icon_path_validate(const char *path, size_t path_length,
                           ui_icon_bounds *out_bounds)
{
    ui_icon_segment segments[UI_ICON_MAX_SEGMENTS];
    size_t segment_count = 0U;
    ui_icon_bounds bounds = {0};
    if (!path_parse(path, path_length, segments, &segment_count, &bounds)) return false;
    if (out_bounds != NULL) *out_bounds = bounds;
    return true;
}

bool ui_icon_validate_all(void)
{
    size_t count = 0U;
    const ui_icon_definition *items = ui_icon_definitions(&count);
    if (count != (size_t)UI_ICON_COUNT - 1U) return false;
    for (size_t index = 0U; index < count; ++index) {
        if ((uint32_t)items[index].icon != index + 1U || items[index].name == NULL ||
            items[index].path == NULL || !ui_icon_path_validate(items[index].path,
                items[index].path_length, NULL)) return false;
        for (size_t prior = 0U; prior < index; ++prior) {
            if (items[prior].icon == items[index].icon ||
                strcmp(items[prior].name, items[index].name) == 0) return false;
        }
    }
    return true;
}

static double distance_squared(double px, double py, const ui_icon_segment *segment)
{
    double dx = segment->x1 - segment->x0, dy = segment->y1 - segment->y0;
    double length_squared = dx * dx + dy * dy;
    double t = length_squared == 0.0 ? 0.0 :
        ((px - segment->x0) * dx + (py - segment->y0) * dy) / length_squared;
    if (t < 0.0) t = 0.0;
    if (t > 1.0) t = 1.0;
    double offset_x = px - (segment->x0 + t * dx);
    double offset_y = py - (segment->y0 + t * dy);
    return offset_x * offset_x + offset_y * offset_y;
}

static bool build_icon_mask(ui_icon_kind icon, uint32_t scale,
                            uint8_t **out_mask, uint32_t *out_axis)
{
    if (icon <= UI_ICON_NONE || icon >= UI_ICON_COUNT || scale == 0U ||
        scale > UI_ICON_MAX_SCALE || out_mask == NULL || out_axis == NULL) return false;
    uint32_t index = (uint32_t)icon - 1U;
    if (!icon_mask_ready[index][scale]) {
        const ui_icon_definition *definitions = ui_icon_definitions(NULL);
        ui_icon_segment segments[UI_ICON_MAX_SEGMENTS];
        size_t segment_count = 0U;
        ui_icon_bounds parsed_bounds = {0};
        const ui_icon_definition *definition = &definitions[index];
        if (!path_parse(definition->path, definition->path_length, segments,
                &segment_count, &parsed_bounds)) return false;
        uint8_t *mask = icon_masks[index][scale];
        memset(mask, 0, UI_ICON_MASK_BYTES);
        uint32_t axis = UI_ICON_VIEWBOX * scale;
        const double radius_squared = 0.75 * 0.75;
        for (uint32_t py = 0U; py < axis; ++py) {
            for (uint32_t px = 0U; px < axis; ++px) {
                unsigned covered = 0U;
                for (unsigned sy = 0U; sy < UI_ICON_AA_SAMPLES; ++sy) {
                    for (unsigned sx = 0U; sx < UI_ICON_AA_SAMPLES; ++sx) {
                        double x = ((double)px + ((double)sx + 0.5) /
                            (double)UI_ICON_AA_SAMPLES) / (double)scale;
                        double y = ((double)py + ((double)sy + 0.5) /
                            (double)UI_ICON_AA_SAMPLES) / (double)scale;
                        for (size_t segment = 0U; segment < segment_count; ++segment) {
                            double min_x = fmin(segments[segment].x0, segments[segment].x1) - 0.8;
                            double max_x = fmax(segments[segment].x0, segments[segment].x1) + 0.8;
                            double min_y = fmin(segments[segment].y0, segments[segment].y1) - 0.8;
                            double max_y = fmax(segments[segment].y0, segments[segment].y1) + 0.8;
                            if (x < min_x || x > max_x || y < min_y || y > max_y) continue;
                            if (distance_squared(x, y, &segments[segment]) <= radius_squared) {
                                covered++;
                                break;
                            }
                        }
                    }
                }
                mask[(size_t)py * axis + px] = (uint8_t)((covered * 255U + 8U) / 16U);
            }
        }
        icon_mask_ready[index][scale] = true;
    }
    *out_axis = UI_ICON_VIEWBOX * scale;
    *out_mask = icon_masks[index][scale];
    return true;
}

static void blend_mask_pixel(uint8_t *destination, uint32_t color, uint8_t coverage)
{
    uint32_t alpha = ((color & 255U) * coverage + 127U) / 255U;
    uint32_t inverse = 255U - alpha;
    uint32_t destination_alpha = destination[3];
    uint32_t out_alpha = alpha + (destination_alpha * inverse + 127U) / 255U;
    if (out_alpha == 0U) return;
    for (size_t channel = 0U; channel < 3U; ++channel) {
        uint32_t source = (color >> (24U - (uint32_t)(channel * 8U))) & 255U;
        uint32_t numerator = source * alpha * 255U +
            (uint32_t)destination[channel] * destination_alpha * inverse;
        destination[channel] = (uint8_t)((numerator + out_alpha * 127U) /
            (out_alpha * 255U));
    }
    destination[3] = (uint8_t)out_alpha;
}

static bool icon_paint_cached(ui_icon_kind icon, ui_theme theme, uint32_t scale,
    ui_rect bounds, uint32_t color, uint8_t *rgba, uint32_t width,
    uint32_t height, size_t stride)
{
    if (icon == UI_ICON_NONE) return true;
    if (theme < UI_THEME_DARK || theme > UI_THEME_LIGHT || rgba == NULL ||
        width == 0U || height == 0U ||
        (size_t)width > SIZE_MAX / 4U || stride < (size_t)width * 4U ||
        (size_t)height > SIZE_MAX / stride) return false;
    uint8_t *mask = NULL;
    uint32_t icon_axis = 0U;
    if (!build_icon_mask(icon, scale, &mask, &icon_axis)) return false;
    uint64_t origin_x64 = (uint64_t)bounds.x +
        (bounds.width > icon_axis ? (bounds.width - icon_axis) / 2U : 0U);
    uint64_t origin_y64 = (uint64_t)bounds.y +
        (bounds.height > icon_axis ? (bounds.height - icon_axis) / 2U : 0U);
    if (origin_x64 > UINT32_MAX || origin_y64 > UINT32_MAX) return false;
    uint32_t origin_x = (uint32_t)origin_x64, origin_y = (uint32_t)origin_y64;
    uint32_t draw_width = bounds.width < icon_axis ? bounds.width : icon_axis;
    uint32_t draw_height = bounds.height < icon_axis ? bounds.height : icon_axis;
    if (origin_x > width || draw_width > width - origin_x ||
        origin_y > height || draw_height > height - origin_y) return false;
    for (uint32_t y = 0U; y < draw_height; ++y) {
        for (uint32_t x = 0U; x < draw_width; ++x) {
            uint8_t coverage = mask[(size_t)y * icon_axis + x];
            if (coverage != 0U) blend_mask_pixel(rgba +
                (size_t)(origin_y + y) * stride + (size_t)(origin_x + x) * 4U,
                color, coverage);
        }
    }
    return true;
}

bool ui_icon_paint_into(ui_icon_kind icon, ui_theme theme, double backing_scale,
                        ui_rect bounds, uint32_t color, uint8_t *rgba,
                        uint32_t width, uint32_t height, size_t stride)
{
    if (!isfinite(backing_scale) || backing_scale < 0.5 ||
        backing_scale > (double)UI_ICON_MAX_SCALE) return false;
    uint32_t scale = (uint32_t)ceil(backing_scale);
    return icon_paint_cached(icon, theme, scale, bounds, color, rgba,
        width, height, stride);
}

static uint32_t state_color(const ui_tokens *tokens, ui_control_state state)
{
    switch (state) {
        case UI_CONTROL_DISABLED: return tokens->disabled_text;
        case UI_CONTROL_PRESSED:
        case UI_CONTROL_SELECTED: return tokens->accent;
        case UI_CONTROL_FOCUS: return tokens->focus_ring;
        case UI_CONTROL_INVALID: return tokens->error;
        case UI_CONTROL_REST:
        case UI_CONTROL_HOVER: default: return tokens->text_primary;
    }
}

static uint32_t style_cache_color(ui_icon_kind icon, ui_theme theme, bool increased,
    uint32_t scale, ui_control_state state, uint32_t color)
{
    size_t slot = 0U;
    for (size_t index = 0U; index < UI_ICON_STYLE_CACHE_COUNT; ++index) {
        ui_icon_style_entry *entry = &icon_style_cache[index];
        if (entry->valid && entry->icon == icon && entry->theme == theme &&
            entry->increased_contrast == increased && entry->scale == scale &&
            entry->state == state && entry->color == color) {
            entry->use = ++icon_style_clock;
            return entry->color;
        }
        if (entry->valid && entry->icon == icon && entry->theme == theme &&
            entry->increased_contrast == increased && entry->scale == scale &&
            entry->state == state) {
            entry->color = color;
            entry->use = ++icon_style_clock;
            return entry->color;
        }
        if (!entry->valid) { slot = index; goto selected; }
        if (entry->use < icon_style_cache[slot].use) slot = index;
    }
selected:
    icon_style_cache[slot] = (ui_icon_style_entry){.icon = icon, .theme = theme,
        .increased_contrast = increased, .scale = (uint8_t)scale,
        .state = state, .color = color, .use = ++icon_style_clock, .valid = true};
    return color;
}

bool ui_icon_paint_state_into(ui_icon_kind icon, ui_theme theme,
    bool increased_contrast, double backing_scale, ui_rect bounds,
    ui_control_state state, const ui_tokens *tokens, uint8_t *rgba,
    uint32_t width, uint32_t height, size_t stride)
{
    if (tokens == NULL || state < UI_CONTROL_REST || state > UI_CONTROL_INVALID ||
        theme < UI_THEME_DARK || theme > UI_THEME_LIGHT ||
        !isfinite(backing_scale) || backing_scale < 0.5 ||
        backing_scale > (double)UI_ICON_MAX_SCALE) return false;
    uint32_t scale = (uint32_t)ceil(backing_scale);
    uint32_t color = style_cache_color(icon, theme, increased_contrast, scale,
        state, state_color(tokens, state));
    return icon_paint_cached(icon, theme, scale, bounds, color, rgba,
        width, height, stride);
}

uint64_t ui_icon_asset_hash(ui_theme theme, uint32_t integer_scale)
{
    if (theme < UI_THEME_DARK || theme > UI_THEME_LIGHT || integer_scale == 0U ||
        integer_scale > UI_ICON_MAX_SCALE) return 0U;
    uint64_t hash = UINT64_C(1469598103934665603);
    for (uint32_t icon = UI_ICON_SELECT; icon < UI_ICON_COUNT; ++icon) {
        uint8_t *mask = NULL;
        uint32_t axis = 0U;
        if (!build_icon_mask((ui_icon_kind)icon, integer_scale, &mask, &axis)) return 0U;
        size_t length = (size_t)axis * axis;
        for (size_t index = 0U; index < length; ++index) {
            hash ^= mask[index];
            hash *= UINT64_C(1099511628211);
        }
    }
    return hash;
}

void ui_icon_cache_clear(void)
{
    memset(icon_mask_ready, 0, sizeof(icon_mask_ready));
    memset(icon_style_cache, 0, sizeof(icon_style_cache));
    icon_style_clock = 0U;
}
