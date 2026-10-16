#include "viewer_timeline.h"

#include "svg/animation/authoring.h"
#include "viewer_anim_edit.h"

#include <math.h>
#include <stdio.h>
#include <string.h>

#define EASING_POINT_LIMIT 330U
#define EASING_HANDLE_COUNT 2U
#define EASING_CURVE_STEPS 48U

static const char *const handle_names[4] = {"x1", "y1", "x2", "y2"};
static const char *const mode_names[3] = {"Linear", "Discrete", "Spline"};
static const char *const preset_names[4] = {
    "Linear", "Ease In", "Ease Out", "Ease In Out"
};
static const double ease_in[4] = {0.42, 0.0, 1.0, 1.0};
static const double ease_out[4] = {0.0, 0.0, 0.58, 1.0};
static const double ease_in_out[4] = {0.42, 0.0, 0.58, 1.0};

static bool scale_points(double scale, uint32_t points, uint32_t *out)
{
    if (out == NULL || !isfinite(scale) || scale < 0.5 || scale > 4.0)
        return false;
    double pixels = ceil(scale * (double)points);
    if (!isfinite(pixels) || pixels > (double)UINT32_MAX) return false;
    *out = (uint32_t)pixels;
    return true;
}

static bool rect_contains(ui_rect rect, double x, double y)
{
    return isfinite(x) && isfinite(y) && x >= (double)rect.x &&
        y >= (double)rect.y && x - (double)rect.x < (double)rect.width &&
        y - (double)rect.y < (double)rect.height;
}

bool viewer_timeline_easing_layout_compute(const viewer_timeline_layout *layout,
    viewer_timeline_easing_layout *out_layout)
{
    if (layout == NULL || out_layout == NULL || layout->panel.width == 0U ||
        layout->panel.height == 0U || !isfinite(layout->backing_scale) ||
        layout->backing_scale < 0.5 || layout->backing_scale > 4.0) return false;
    *out_layout = (viewer_timeline_easing_layout){0};
    uint32_t inset, mode_y, graph_y, graph_width, graph_height, field_x, field_gap;
    uint32_t field_y, field_height, preset_y, control_height, mode_widths[3];
    uint32_t preset_height;
    if (!scale_points(layout->backing_scale, 12U, &inset) ||
        !scale_points(layout->backing_scale, 166U, &mode_y) ||
        !scale_points(layout->backing_scale, 198U, &graph_y) ||
        !scale_points(layout->backing_scale, 104U, &graph_width) ||
        !scale_points(layout->backing_scale, 56U, &graph_height) ||
        !scale_points(layout->backing_scale, 8U, &field_gap) ||
        !scale_points(layout->backing_scale, 194U, &field_y) ||
        !scale_points(layout->backing_scale, 28U, &field_height) ||
        !scale_points(layout->backing_scale, 260U, &preset_y) ||
        !scale_points(layout->backing_scale, 24U, &control_height) ||
        !scale_points(layout->backing_scale, 60U, &mode_widths[0]) ||
        !scale_points(layout->backing_scale, 76U, &mode_widths[1]) ||
        !scale_points(layout->backing_scale, 60U, &mode_widths[2]) ||
        !scale_points(layout->backing_scale, 24U, &preset_height)) return false;
    uint64_t graph_x = (uint64_t)layout->panel.x + inset;
    uint64_t graph_top = (uint64_t)layout->panel.y + graph_y;
    uint64_t mode_top = (uint64_t)layout->panel.y + mode_y;
    uint64_t field_top = (uint64_t)layout->panel.y + field_y;
    uint64_t preset_top = (uint64_t)layout->panel.y + preset_y;
    uint64_t panel_right = (uint64_t)layout->panel.x + layout->panel.width;
    uint64_t panel_bottom = (uint64_t)layout->panel.y + layout->panel.height;
    if (graph_x + graph_width + inset >= panel_right ||
        graph_top + graph_height + inset > panel_bottom ||
        mode_top + control_height > panel_bottom ||
        preset_top + preset_height > panel_bottom) return false;
    uint32_t mode_x = (uint32_t)graph_x;
    uint32_t mode_used = mode_widths[0] + mode_widths[1] + mode_widths[2] +
        field_gap * 2U;
    if ((uint64_t)mode_x + mode_used > panel_right - inset) return false;
    for (size_t index = 0U; index < 3U; ++index) {
        out_layout->mode_buttons[index] = (ui_rect){mode_x,
            (uint32_t)mode_top, mode_widths[index], control_height};
        mode_x += mode_widths[index] + field_gap;
    }
    out_layout->graph = (ui_rect){(uint32_t)graph_x, (uint32_t)graph_top,
        graph_width, graph_height};
    field_x = out_layout->graph.x + graph_width + field_gap;
    uint32_t field_right = (uint32_t)panel_right - inset;
    if (field_right <= field_x + field_gap) return false;
    uint32_t field_width = (field_right - field_x - field_gap) / 2U;
    uint32_t second_x = field_x + field_width + field_gap;
    out_layout->fields[0] = (ui_rect){field_x, (uint32_t)field_top,
        field_width, field_height};
    out_layout->fields[1] = (ui_rect){second_x, (uint32_t)field_top,
        field_width, field_height};
    out_layout->fields[2] = (ui_rect){field_x,
        (uint32_t)field_top + field_height + field_gap,
        field_width, field_height};
    out_layout->fields[3] = (ui_rect){second_x,
        (uint32_t)field_top + field_height + field_gap,
        field_width, field_height};
    uint32_t preset_x = (uint32_t)graph_x;
    uint32_t preset_width = (layout->panel.width - inset * 2U - field_gap * 3U) / 4U;
    for (size_t index = 0U; index < 4U; ++index) {
        out_layout->presets[index] = (ui_rect){preset_x, (uint32_t)preset_top,
            preset_width, preset_height};
        preset_x += preset_width + field_gap;
    }
    out_layout->visible = true;
    return true;
}

bool viewer_timeline_easing_enabled(const viewer_timeline *timeline)
{
    if (timeline == NULL || timeline->native_project || timeline->document == NULL ||
        timeline->selected_track >= timeline->document->animation.track_count)
        return false;
    const svg_anim_track *track =
        &timeline->document->animation.tracks[timeline->selected_track];
    return track->key_count > 1U &&
        timeline->selected_key < track->key_count - 1U &&
        svg_anim_authoring_track_editable(&timeline->document->animation,
            timeline->selected_track);
}

static void set_field_exact(ui_widget_number_field *field, double value)
{
    if (field == NULL || !isfinite(value) || value < 0.0 || value > 1.0) return;
    char text[64];
    int amount = snprintf(text, sizeof(text), "%.17g", value);
    if (amount <= 0 || (size_t)amount >= sizeof(text)) return;
    if (ui_widget_number_field_set_draft(field, text, (size_t)amount)) {
        field->value = value;
        field->original = value;
        memcpy(field->edit.original, text, (size_t)amount + 1U);
        field->edit.original_length = (size_t)amount;
        field->edit.editing = false;
        field->edit.invalid = false;
    }
}

void viewer_timeline_easing_sync(viewer_timeline *timeline)
{
    if (timeline == NULL || timeline->easing_initialized) return;
    const double *values = ease_in_out;
    if (timeline->document != NULL &&
        timeline->selected_track < timeline->document->animation.track_count) {
        const svg_anim_track *track =
            &timeline->document->animation.tracks[timeline->selected_track];
        if (track->calc_mode == SVG_ANIM_CALC_SPLINE &&
            track->splines != NULL && timeline->selected_key < track->spline_count)
            values = track->splines[timeline->selected_key];
    }
    for (size_t index = 0U; index < 4U; ++index) {
        if (!ui_widget_number_field_init(&timeline->easing_fields[index],
                handle_names[index], strlen(handle_names[index]), values[index],
                0.0, 1.0, 0.01, NULL, 0U)) continue;
        set_field_exact(&timeline->easing_fields[index], values[index]);
    }
    timeline->easing_focused_field = 0U;
    timeline->easing_initialized = true;
}

static uint32_t widget_scale_for(uint32_t value)
{
    return value < 1U ? 1U : value > 3U ? 3U : value;
}

static bool rect_to_widget(ui_rect pixels, uint32_t scale, ui_rect *out)
{
    if (out == NULL || scale == 0U || pixels.width == 0U || pixels.height == 0U)
        return false;
    *out = (ui_rect){pixels.x / scale, pixels.y / scale,
        pixels.width / scale + (pixels.width % scale != 0U ? 1U : 0U),
        pixels.height / scale + (pixels.height % scale != 0U ? 1U : 0U)};
    return out->width > 0U && out->height > 0U;
}

static bool paint_widget(ui_widget_canvas *canvas, ui_rect pixels,
    ui_widget_kind kind, ui_control_state state, const char *label,
    const char *value, double number, bool invalid)
{
    ui_rect bounds;
    if (!rect_to_widget(pixels, canvas->backing_scale, &bounds)) return false;
    ui_widget_draw_spec spec = {
        .kind = kind,
        .state = state,
        .bounds = bounds,
        .label = label,
        .label_length = label == NULL ? 0U : strlen(label),
        .value = value,
        .value_length = value == NULL ? 0U : strlen(value),
        .numeric_value = number,
        .invalid = invalid,
        .focus_visible = state == UI_CONTROL_FOCUS
    };
    return ui_widget_paint(canvas, &spec);
}

static void pixel(ui_widget_canvas *canvas, int32_t x, int32_t y, uint32_t color)
{
    if (canvas == NULL || canvas->rgba == NULL || x < 0 || y < 0 ||
        (uint32_t)x >= canvas->width || (uint32_t)y >= canvas->height) return;
    size_t row = (size_t)(uint32_t)y * canvas->stride;
    size_t column = (size_t)(uint32_t)x * 4U;
    if (row > canvas->capacity || column > canvas->capacity - row ||
        canvas->capacity - row - column < 4U) return;
    uint8_t *destination = canvas->rgba + row + column;
    uint32_t alpha = color & 255U;
    uint32_t inverse = 255U - alpha;
    for (size_t channel = 0U; channel < 3U; ++channel) {
        uint32_t shift = 24U - (uint32_t)channel * 8U;
        uint32_t source = (color >> shift) & 255U;
        destination[channel] = (uint8_t)((source * alpha +
            (uint32_t)destination[channel] * inverse + 127U) / 255U);
    }
    destination[3] = 255U;
}

static void line(ui_widget_canvas *canvas, double x0, double y0,
    double x1, double y1, uint32_t color)
{
    double span_x = x1 - x0, span_y = y1 - y0;
    double magnitude_x = fabs(span_x), magnitude_y = fabs(span_y);
    double steps_value = ceil(magnitude_x > magnitude_y ? magnitude_x : magnitude_y);
    if (!isfinite(steps_value) || steps_value > (double)EASING_POINT_LIMIT * 4.0)
        return;
    uint32_t steps = steps_value < 1.0 ? 1U : (uint32_t)steps_value;
    for (uint32_t index = 0U; index <= steps; ++index) {
        double t = (double)index / (double)steps;
        double x = x0 + span_x * t, y = y0 + span_y * t;
        if (!isfinite(x) || !isfinite(y) || x < 0.0 || y < 0.0 ||
            x > (double)INT32_MAX || y > (double)INT32_MAX) continue;
        pixel(canvas, (int32_t)lround(x), (int32_t)lround(y), color);
    }
}

static void draw_circle(ui_widget_canvas *canvas, double center_x,
                        double center_y, int32_t radius, uint32_t color)
{
    for (int32_t y = -radius; y <= radius; ++y) {
        for (int32_t x = -radius; x <= radius; ++x) {
            if (x * x + y * y <= radius * radius)
                pixel(canvas, (int32_t)lround(center_x) + x,
                    (int32_t)lround(center_y) + y, color);
        }
    }
}

static void curve_point(ui_rect graph, double x, double y,
    const double values[4], double *out_x, double *out_y)
{
    double inverse = 1.0 - x;
    double curve_x = 3.0 * inverse * inverse * x * values[0] +
        3.0 * inverse * x * x * values[2] + x * x * x;
    double curve_y = 3.0 * inverse * inverse * x * values[1] +
        3.0 * inverse * x * x * values[3] + x * x * x;
    *out_x = (double)graph.x + 7.0 +
        curve_x * ((double)graph.width - 14.0);
    *out_y = (double)graph.y + (double)graph.height - 7.0 -
        curve_y * ((double)graph.height - 14.0);
    (void)y;
}

static void draw_graph(ui_widget_canvas *canvas, ui_rect graph,
    const double values[4], bool enabled)
{
    if (canvas == NULL || graph.width < 24U || graph.height < 24U) return;
    uint32_t background = canvas->tokens.panel_raised;
    uint32_t border = enabled ? canvas->tokens.control_edge : canvas->tokens.separator;
    uint32_t accent = enabled ? canvas->tokens.accent : canvas->tokens.disabled_text;
    uint32_t handle = enabled ? canvas->tokens.selection_edge : canvas->tokens.text_tertiary;
    for (uint32_t y = graph.y; y < graph.y + graph.height; ++y)
        for (uint32_t x = graph.x; x < graph.x + graph.width; ++x)
            pixel(canvas, (int32_t)x, (int32_t)y, background);
    line(canvas, graph.x, graph.y, graph.x + graph.width - 1U, graph.y, border);
    line(canvas, graph.x, graph.y + graph.height - 1U,
        graph.x + graph.width - 1U, graph.y + graph.height - 1U, border);
    line(canvas, graph.x, graph.y, graph.x, graph.y + graph.height - 1U, border);
    line(canvas, graph.x + graph.width - 1U, graph.y,
        graph.x + graph.width - 1U, graph.y + graph.height - 1U, border);
    double x0 = (double)graph.x + 7.0;
    double y0 = (double)graph.y + (double)graph.height - 7.0;
    double x1 = (double)graph.x + (double)graph.width - 7.0;
    double y1 = (double)graph.y + 7.0;
    for (uint32_t dash = 0U; dash < EASING_CURVE_STEPS; dash += 4U) {
        double start = (double)dash / (double)EASING_CURVE_STEPS;
        double end = start + 2.0 / (double)EASING_CURVE_STEPS;
        line(canvas, x0 + (x1 - x0) * start, y0 + (y1 - y0) * start,
            x0 + (x1 - x0) * end, y0 + (y1 - y0) * end,
            canvas->tokens.guide);
    }
    double handle_x[2] = {
        x0 + values[0] * (x1 - x0), x0 + values[2] * (x1 - x0)};
    double handle_y[2] = {
        y0 - values[1] * (y0 - y1), y0 - values[3] * (y0 - y1)};
    line(canvas, x0, y0, handle_x[0], handle_y[0], border);
    line(canvas, handle_x[1], handle_y[1], x1, y1, border);
    double prior_x = x0, prior_y = y0;
    for (uint32_t step = 1U; step <= EASING_CURVE_STEPS; ++step) {
        double t = (double)step / (double)EASING_CURVE_STEPS;
        double next_x, next_y;
        curve_point(graph, t, 0.0, values, &next_x, &next_y);
        line(canvas, prior_x, prior_y, next_x, next_y, accent);
        prior_x = next_x;
        prior_y = next_y;
    }
    draw_circle(canvas, handle_x[0], handle_y[0], 3, handle);
    draw_circle(canvas, handle_x[1], handle_y[1], 3, handle);
    draw_circle(canvas, handle_x[0], handle_y[0], 1, accent);
    draw_circle(canvas, handle_x[1], handle_y[1], 1, accent);
}

static void field_values(const viewer_timeline *timeline, double values[4])
{
    for (size_t index = 0U; index < 4U; ++index)
        values[index] = timeline->easing_fields[index].value;
}

static svg_anim_calc_mode displayed_mode(const viewer_timeline *timeline)
{
    if (timeline->easing_mode_override) return timeline->easing_mode;
    if (timeline->document != NULL &&
        timeline->selected_track < timeline->document->animation.track_count)
        return timeline->document->animation.tracks[timeline->selected_track].calc_mode;
    return SVG_ANIM_CALC_LINEAR;
}

static void set_pending_value(viewer_timeline *timeline, size_t index, double value)
{
    if (timeline == NULL || index >= 4U) return;
    set_field_exact(&timeline->easing_fields[index], value);
    timeline->easing_pending = true;
}

/* Reverts uncommitted drafts so only a field's own Return commits its value. */
static void discard_drafts(viewer_timeline *timeline, size_t keep)
{
    for (size_t index = 0U; index < 4U; ++index) {
        if (index == keep || !timeline->easing_fields[index].edit.editing) continue;
        (void)ui_widget_number_field_key(&timeline->easing_fields[index],
            UI_KEY_ESCAPE, 0U);
    }
}

static uint32_t apply_spline(viewer_timeline *timeline, viewer_editor *editor)
{
    if (!viewer_timeline_easing_enabled(timeline)) return VIEWER_EDITOR_HANDLED;
    double values[4];
    field_values(timeline, values);
    uint32_t result = viewer_editor_animation_set_spline(editor,
        &timeline->document->animation, timeline->selected_track,
        timeline->selected_key, values);
    if ((result & VIEWER_EDITOR_MUTATED) != 0U) {
        timeline->edit_mutation_pending = true;
        timeline->easing_mode = SVG_ANIM_CALC_SPLINE;
        timeline->easing_mode_override = true;
    }
    timeline->easing_pending = false;
    return result;
}

static uint32_t apply_mode(viewer_timeline *timeline, viewer_editor *editor,
                           svg_anim_calc_mode mode)
{
    if (!viewer_timeline_easing_enabled(timeline)) return VIEWER_EDITOR_HANDLED;
    uint32_t result;
    if (mode == SVG_ANIM_CALC_SPLINE) {
        double values[4];
        field_values(timeline, values);
        result = viewer_editor_animation_set_spline(editor,
            &timeline->document->animation, timeline->selected_track,
            timeline->selected_key, values);
    } else result = viewer_editor_animation_set_mode(editor,
        &timeline->document->animation, timeline->selected_track, mode);
    if ((result & VIEWER_EDITOR_MUTATED) != 0U) {
        timeline->edit_mutation_pending = true;
        timeline->easing_mode = mode;
        timeline->easing_mode_override = true;
    }
    return result;
}

static void point_to_value(const viewer_timeline_easing_layout *layout,
    double x, double y, double *out_x, double *out_y)
{
    double inner_width = (double)layout->graph.width - 14.0;
    double inner_height = (double)layout->graph.height - 14.0;
    double mapped_x = (x - (double)layout->graph.x - 7.0) / inner_width;
    double mapped_y = ((double)layout->graph.y + (double)layout->graph.height -
        7.0 - y) / inner_height;
    if (mapped_x < 0.0) mapped_x = 0.0;
    if (mapped_x > 1.0) mapped_x = 1.0;
    if (mapped_y < 0.0) mapped_y = 0.0;
    if (mapped_y > 1.0) mapped_y = 1.0;
    *out_x = mapped_x;
    *out_y = mapped_y;
}

static bool event_point(const platform_event *event, double scale,
                        double *out_x, double *out_y)
{
    if (event == NULL || out_x == NULL || out_y == NULL ||
        !isfinite(scale) || scale <= 0.0) return false;
    double x = (double)event->logical_x * scale;
    double y = (double)event->logical_y * scale;
    if (!isfinite(x) || !isfinite(y) || x > (double)INT32_MAX ||
        y > (double)INT32_MAX) return false;
    *out_x = x;
    *out_y = y;
    return true;
}

static void commit_graph_drag(viewer_timeline *timeline, viewer_editor *editor)
{
    if (timeline == NULL || editor == NULL || !timeline->easing_graph_dragging) return;
    timeline->easing_graph_dragging = false;
    (void)apply_spline(timeline, editor);
}

static bool graph_handle_at(const viewer_timeline_easing_layout *layout,
    const double values[4], double x, double y, uint32_t scale, size_t *out_handle)
{
    if (layout == NULL || out_handle == NULL) return false;
    double left = (double)layout->graph.x + 7.0;
    double bottom = (double)layout->graph.y + (double)layout->graph.height - 7.0;
    double width = (double)layout->graph.width - 14.0;
    double height = (double)layout->graph.height - 14.0;
    double radius = 9.0 * (double)scale;
    double best = radius * radius;
    bool found = false;
    for (size_t index = 0U; index < EASING_HANDLE_COUNT; ++index) {
        double hx = left + values[index * 2U] * width;
        double hy = bottom - values[index * 2U + 1U] * height;
        double dx = x - hx, dy = y - hy;
        double distance = dx * dx + dy * dy;
        if (distance <= best) {
            best = distance;
            *out_handle = index;
            found = true;
        }
    }
    return found;
}

bool viewer_timeline_easing_process_event(viewer_timeline *timeline,
    viewer_editor *editor, const platform_event *event,
    const viewer_timeline_layout *layout)
{
    if (timeline == NULL || editor == NULL || event == NULL || layout == NULL)
        return false;
    viewer_timeline_easing_layout easing;
    if (!viewer_timeline_easing_layout_compute(layout, &easing)) return false;
    viewer_timeline_easing_sync(timeline);
    bool enabled = viewer_timeline_easing_enabled(timeline);
    double x = 0.0, y = 0.0;
    bool has_point = event_point(event, layout->backing_scale, &x, &y);
    if (event->type == PLATFORM_EVENT_POINTER_DOWN && has_point) {
        for (size_t index = 0U; index < 3U; ++index) {
            if (!rect_contains(easing.mode_buttons[index], x, y)) continue;
            discard_drafts(timeline, 4U);
            if (enabled) (void)apply_mode(timeline, editor,
                index == 0U ? SVG_ANIM_CALC_LINEAR :
                index == 1U ? SVG_ANIM_CALC_DISCRETE : SVG_ANIM_CALC_SPLINE);
            return true;
        }
        for (size_t index = 0U; index < 4U; ++index) {
            if (!rect_contains(easing.presets[index], x, y)) continue;
            if (!enabled) return true;
            if (index == 0U) (void)apply_mode(timeline, editor, SVG_ANIM_CALC_LINEAR);
            else {
                const double *preset = index == 1U ? ease_in :
                    index == 2U ? ease_out : ease_in_out;
                for (size_t handle = 0U; handle < 4U; ++handle)
                    set_pending_value(timeline, handle, preset[handle]);
                (void)apply_spline(timeline, editor);
            }
            return true;
        }
        for (size_t index = 0U; index < 4U; ++index) {
            if (!rect_contains(easing.fields[index], x, y)) continue;
            if (!enabled) return true;
            discard_drafts(timeline, index);
            timeline->easing_focused_field = index;
            timeline->easing_has_focus = true;
            timeline->easing_graph_focused = false;
            timeline->easing_field_dragging = true;
            timeline->easing_drag_field = index;
            (void)ui_widget_number_field_pointer(
                &timeline->easing_fields[index], UI_WIDGET_POINTER_DOWN,
                (int32_t)x, 0U);
            return true;
        }
        if (rect_contains(easing.graph, x, y)) {
            if (!enabled) return true;
            discard_drafts(timeline, 4U);
            double values[4];
            size_t handle = 0U;
            field_values(timeline, values);
            if (!graph_handle_at(&easing, values, x, y,
                    widget_scale_for((uint32_t)ceil(layout->backing_scale)), &handle))
                handle = x < (double)easing.graph.x + (double)easing.graph.width / 2.0 ?
                    0U : 1U;
            timeline->easing_drag_handle = handle;
            timeline->easing_focused_field = handle * 2U;
            timeline->easing_has_focus = true;
            timeline->easing_graph_focused = true;
            timeline->easing_graph_dragging = true;
            double value_x, value_y;
            point_to_value(&easing, x, y, &value_x, &value_y);
            set_pending_value(timeline, handle * 2U, value_x);
            set_pending_value(timeline, handle * 2U + 1U, value_y);
            return true;
        }
        return false;
    }
    if (event->type == PLATFORM_EVENT_POINTER_MOVE && has_point) {
        if (timeline->easing_graph_dragging) {
            double value_x, value_y;
            size_t handle = timeline->easing_drag_handle;
            point_to_value(&easing, x, y, &value_x, &value_y);
            set_pending_value(timeline, handle * 2U, value_x);
            set_pending_value(timeline, handle * 2U + 1U, value_y);
            return true;
        }
        if (timeline->easing_field_dragging) {
            size_t index = timeline->easing_drag_field;
            (void)ui_widget_number_field_pointer(&timeline->easing_fields[index],
                UI_WIDGET_POINTER_MOVE, (int32_t)x, 0U);
            timeline->easing_pending = true;
            return true;
        }
    }
    if (event->type == PLATFORM_EVENT_POINTER_UP) {
        if (timeline->easing_graph_dragging) {
            commit_graph_drag(timeline, editor);
            return true;
        }
        if (timeline->easing_field_dragging) {
            size_t index = timeline->easing_drag_field;
            timeline->easing_field_dragging = false;
            (void)ui_widget_number_field_pointer(&timeline->easing_fields[index],
                UI_WIDGET_POINTER_UP_INSIDE, has_point ? (int32_t)x : 0, 0U);
            (void)apply_spline(timeline, editor);
            return true;
        }
    }
    if (event->type != PLATFORM_EVENT_KEY_DOWN || !timeline->easing_has_focus ||
        !enabled) return false;
    if (event->key_code == 48U) {
        timeline->easing_focused_field =
            (timeline->easing_focused_field + 1U) % 4U;
        discard_drafts(timeline, timeline->easing_focused_field);
        timeline->easing_graph_focused = false;
        timeline->easing_graph_dragging = false;
        timeline->easing_pending = false;
        return true;
    }
    if (timeline->easing_graph_focused) {
        double values[4];
        field_values(timeline, values);
        size_t field = timeline->easing_focused_field;
        if (event->key_code == 36U) {
            (void)apply_spline(timeline, editor);
            return true;
        }
        double amount = (event->modifiers & PLATFORM_MODIFIER_SHIFT) != 0U ?
            0.1 : (event->modifiers & PLATFORM_MODIFIER_OPTION) != 0U ? 0.001 : 0.01;
        if (event->key_code == 123U || event->key_code == 124U) {
            values[field] += event->key_code == 124U ? amount : -amount;
            if (values[field] < 0.0) values[field] = 0.0;
            if (values[field] > 1.0) values[field] = 1.0;
            set_pending_value(timeline, field, values[field]);
            return true;
        }
        if (event->key_code == 125U || event->key_code == 126U) {
            field = timeline->easing_drag_handle * 2U + 1U;
            values[field] += event->key_code == 125U ? amount : -amount;
            if (values[field] < 0.0) values[field] = 0.0;
            if (values[field] > 1.0) values[field] = 1.0;
            set_pending_value(timeline, field, values[field]);
            return true;
        }
    }
    size_t active = timeline->easing_focused_field;
    uint32_t key = event->key_code == 126U ? UI_KEY_UP :
        event->key_code == 125U ? UI_KEY_DOWN :
        event->key_code == 36U ? UI_KEY_RETURN :
        event->key_code == 53U ? UI_KEY_ESCAPE :
        event->key_code == 51U ? UI_KEY_BACKSPACE :
        event->key_code == 117U ? UI_KEY_DELETE :
        event->key_code == 123U ? UI_KEY_LEFT :
        event->key_code == 124U ? UI_KEY_RIGHT :
        event->text_len == 1U ? (uint8_t)event->text[0] : 0U;
    uint32_t modifiers =
        ((event->modifiers & PLATFORM_MODIFIER_SHIFT) != 0U ? UI_MOD_SHIFT : 0U) |
        ((event->modifiers & PLATFORM_MODIFIER_OPTION) != 0U ? UI_MOD_OPTION : 0U) |
        ((event->modifiers & PLATFORM_MODIFIER_COMMAND) != 0U ? UI_MOD_COMMAND : 0U);
    ui_widget_input_result result = ui_widget_number_field_key(
        &timeline->easing_fields[active], key, modifiers);
    if (result == UI_WIDGET_INPUT_CHANGED) {
        timeline->easing_pending = true;
        return true;
    }
    if (result == UI_WIDGET_INPUT_COMMITTED) {
        (void)apply_spline(timeline, editor);
        return true;
    }
    if (result == UI_WIDGET_INPUT_INVALID) {
        discard_drafts(timeline, 4U);
        timeline->easing_pending = false;
        return true;
    }
    if (result == UI_WIDGET_INPUT_CANCELLED) {
        timeline->easing_pending = false;
        timeline->easing_initialized = false;
        viewer_timeline_easing_sync(timeline);
        return true;
    }
    return result != UI_WIDGET_INPUT_UNHANDLED;
}

void viewer_timeline_easing_paint_key_markers(const viewer_timeline *timeline,
    const viewer_timeline_layout *layout, size_t track_index, size_t row,
    ui_widget_canvas *canvas)
{
    if (timeline == NULL || layout == NULL || canvas == NULL ||
        timeline->document == NULL ||
        track_index >= timeline->document->animation.track_count ||
        row >= layout->visible_track_count) return;
    const svg_anim_track *track =
        &timeline->document->animation.tracks[track_index];
    double duration = viewer_timeline_duration(timeline);
    double inset = ceil(8.0 * layout->backing_scale);
    double reserve = ceil(64.0 * layout->backing_scale);
    double width = (double)layout->scrubber.width -
        (layout->scrubber.width > reserve ? reserve : 0.0);
    if (!(duration > 0.0) || !(width > 0.0)) return;
    double start_x = (double)layout->scrubber.x +
        (layout->scrubber.width > reserve ? inset : 0.0);
    double begin = track->has_begin_sync ? 0.0 : track->begin_offset;
    double top = (double)layout->track_rows[row].y +
        (double)layout->track_rows[row].height / 2.0;
    uint32_t selected_color = canvas->tokens.accent;
    uint32_t other_color = canvas->tokens.text_secondary;
    for (size_t index = 0U; index < track->key_count; ++index) {
        double time = begin + track->keys[index].time * track->duration_seconds;
        double x = start_x + time / duration * width;
        if (!isfinite(x) || x < 0.0 || x > (double)INT32_MAX) continue;
        uint32_t color = track_index == timeline->selected_track &&
            index == timeline->selected_key ? selected_color : other_color;
        int32_t center = (int32_t)lround(x);
        int32_t mid = (int32_t)lround(top);
        for (int32_t offset = -3; offset <= 3; ++offset)
            pixel(canvas, center, mid + offset, color);
    }
}

void viewer_timeline_easing_paint(viewer_timeline *timeline,
    const viewer_timeline_layout *timeline_layout,
    const viewer_timeline_easing_layout *easing_layout,
    ui_widget_canvas *canvas)
{
    if (timeline == NULL || timeline_layout == NULL || easing_layout == NULL ||
        canvas == NULL || !easing_layout->visible) return;
    viewer_timeline_easing_sync(timeline);
    bool enabled = viewer_timeline_easing_enabled(timeline);
    svg_anim_calc_mode mode = displayed_mode(timeline);
    for (size_t index = 0U; index < 3U; ++index) {
        svg_anim_calc_mode selected = index == 0U ? SVG_ANIM_CALC_LINEAR :
            index == 1U ? SVG_ANIM_CALC_DISCRETE : SVG_ANIM_CALC_SPLINE;
        ui_control_state state = !enabled ? UI_CONTROL_DISABLED :
            mode == selected ? UI_CONTROL_SELECTED : UI_CONTROL_REST;
        (void)paint_widget(canvas, easing_layout->mode_buttons[index],
            UI_WIDGET_BUTTON_SECONDARY, state, mode_names[index], NULL, 0.0, false);
    }
    double values[4];
    field_values(timeline, values);
    draw_graph(canvas, easing_layout->graph, values, enabled);
    for (size_t index = 0U; index < 4U; ++index) {
        char number[64];
        int amount = snprintf(number, sizeof(number), "%.15g",
            timeline->easing_fields[index].value);
        if (amount <= 0 || (size_t)amount >= sizeof(number)) continue;
        ui_control_state state = !enabled ? UI_CONTROL_DISABLED :
            timeline->easing_has_focus &&
                timeline->easing_focused_field == index ?
                UI_CONTROL_FOCUS : UI_CONTROL_REST;
        (void)paint_widget(canvas, easing_layout->fields[index],
            UI_WIDGET_NUMBER_FIELD, state, handle_names[index], number,
            timeline->easing_fields[index].value,
            timeline->easing_fields[index].edit.invalid);
    }
    for (size_t index = 0U; index < 4U; ++index) {
        ui_control_state state = enabled ? UI_CONTROL_REST : UI_CONTROL_DISABLED;
        (void)paint_widget(canvas, easing_layout->presets[index],
            UI_WIDGET_BUTTON_SECONDARY, state, preset_names[index], NULL, 0.0, false);
    }
    (void)timeline_layout;
}
