#include "viewer_timeline.h"

#include "viewer_chrome.h"

#include <math.h>
#include <string.h>

static bool scaled_points(uint32_t points, double scale, uint32_t *out_pixels)
{
    if (out_pixels == NULL || !isfinite(scale) || scale < 0.5 || scale > 4.0)
        return false;
    double pixels = ceil((double)points * scale);
    if (!isfinite(pixels) || pixels > (double)UINT32_MAX) return false;
    *out_pixels = (uint32_t)pixels;
    return true;
}

static ui_rect rect_at(uint32_t x, uint32_t y, uint32_t width, uint32_t height)
{
    return (ui_rect){x, y, width, height};
}

static bool rect_contains(ui_rect rect, double x, double y)
{
    return rect.width > 0U && rect.height > 0U && isfinite(x) && isfinite(y) &&
        x >= (double)rect.x && y >= (double)rect.y &&
        x - (double)rect.x < (double)rect.width &&
        y - (double)rect.y < (double)rect.height;
}

bool viewer_timeline_layout_compute(const viewer_timeline *timeline,
                                    uint32_t width, uint32_t height,
                                    double backing_scale,
                                    viewer_timeline_layout *out_layout)
{
    if (timeline == NULL || out_layout == NULL || width == 0U || height == 0U ||
        width > 8192U || height > 8192U || !isfinite(backing_scale) ||
        backing_scale < 0.5 || backing_scale > 4.0 ||
        viewer_timeline_track_count(timeline) == 0U) return false;
    uint32_t minimum_width = 0U;
    if (!scaled_points(280U, backing_scale, &minimum_width) || width < minimum_width)
        return false;
    memset(out_layout, 0, sizeof(*out_layout));
    uint32_t requested_height = timeline->dock_open ? timeline->dock_height_points :
        VIEWER_TIMELINE_DOCK_COLLAPSED_POINTS;
    viewer_chrome_layout chrome;
    if (!viewer_chrome_layout_compute_docked(width, height, backing_scale,
            false, false, true, requested_height, &chrome) ||
        chrome.timeline.height == 0U) return false;
    out_layout->backing_scale = backing_scale;
    out_layout->panel = chrome.timeline;
    out_layout->track_count = viewer_timeline_track_count(timeline);
    out_layout->first_track = timeline->first_track;
    if (out_layout->first_track >= out_layout->track_count)
        out_layout->first_track = out_layout->track_count - 1U;
    uint32_t inset = 0U, title_height = 0U, controls_height = 0U;
    uint32_t ruler_height = 0U, row_height = 0U, row_start = 0U;
    if (!scaled_points(10U, backing_scale, &inset) ||
        !scaled_points(22U, backing_scale, &title_height) ||
        !scaled_points(26U, backing_scale, &controls_height) ||
        !scaled_points(22U, backing_scale, &ruler_height) ||
        !scaled_points(18U, backing_scale, &row_height) ||
        !scaled_points(84U, backing_scale, &row_start)) return false;
    uint32_t right = width > inset ? width - inset : 0U;
    uint32_t title_y = out_layout->panel.y + 2U * (uint32_t)ceil(backing_scale);
    out_layout->header = rect_at(inset, title_y, right > inset ? right - inset : 0U,
        title_height);
    uint32_t collapse_width = 68U;
    if (!scaled_points(collapse_width, backing_scale, &collapse_width)) return false;
    if (collapse_width > out_layout->header.width) collapse_width = out_layout->header.width;
    out_layout->collapse_toggle = rect_at(
        out_layout->panel.x + out_layout->panel.width - inset - collapse_width,
        title_y, collapse_width, title_height);
    uint32_t resize_height = 8U * (uint32_t)ceil(backing_scale);
    out_layout->resize_handle = rect_at(0U,
        out_layout->panel.y > resize_height / 2U ?
            out_layout->panel.y - resize_height / 2U : 0U,
        width, resize_height);
    uint32_t collapsed_minimum = 0U;
    if (!scaled_points(VIEWER_TIMELINE_DOCK_MIN_POINTS, backing_scale,
            &collapsed_minimum)) return false;
    out_layout->collapsed = !timeline->dock_open ||
        out_layout->panel.height < collapsed_minimum;
    if (out_layout->collapsed) return true;

    uint32_t gap = 0U, button_x = 0U, play_width = 0U, time_width = 0U;
    uint32_t mode_width = 0U, speed_width = 0U, fit_width = 0U;
    if (!scaled_points(6U, backing_scale, &gap) ||
        !scaled_points(10U, backing_scale, &button_x) ||
        !scaled_points(52U, backing_scale, &play_width) ||
        !scaled_points(102U, backing_scale, &time_width) ||
        !scaled_points(96U, backing_scale, &mode_width) ||
        !scaled_points(58U, backing_scale, &speed_width) ||
        !scaled_points(44U, backing_scale, &fit_width)) return false;
    uint32_t controls_y = out_layout->panel.y +
        (uint32_t)ceil(26.0 * backing_scale);
    if ((uint64_t)controls_y + controls_height > UINT32_MAX) return false;
    uint32_t available = out_layout->panel.width > button_x * 2U ?
        out_layout->panel.width - button_x * 2U : 0U;
    bool compact = (uint64_t)play_width + time_width + mode_width + speed_width +
        fit_width + gap * 4U > available;
    if (compact) {
        if (!scaled_points(78U, backing_scale, &time_width) ||
            !scaled_points(46U, backing_scale, &mode_width) ||
            !scaled_points(34U, backing_scale, &speed_width) ||
            !scaled_points(32U, backing_scale, &fit_width) ||
            !scaled_points(42U, backing_scale, &play_width)) return false;
    }
    out_layout->compact = compact;
    uint32_t cursor = button_x;
    out_layout->play_button = rect_at(cursor, controls_y, play_width, controls_height);
    cursor += play_width + gap;
    out_layout->time_readout = rect_at(cursor, controls_y, time_width, controls_height);
    cursor += time_width + gap;
    if (mode_width > 0U) {
        out_layout->mode_button = rect_at(cursor, controls_y, mode_width, controls_height);
        cursor += mode_width + gap;
    }
    if (speed_width > 0U) {
        out_layout->speed_button = rect_at(cursor, controls_y, speed_width, controls_height);
        cursor += speed_width + gap;
    }
    out_layout->fit_button = rect_at(cursor, controls_y, fit_width, controls_height);

    uint32_t ruler_y = out_layout->panel.y +
        (uint32_t)ceil(56.0 * backing_scale);
    uint32_t ruler_x = inset;
    uint32_t gutter_width = 190U;
    if (!scaled_points(gutter_width, backing_scale, &gutter_width)) return false;
    uint32_t one_third = out_layout->panel.width / 3U;
    if (gutter_width > one_third) gutter_width = one_third;
    uint32_t minimum_gutter = 104U;
    if (!scaled_points(minimum_gutter, backing_scale, &minimum_gutter)) return false;
    if (gutter_width < minimum_gutter) gutter_width = minimum_gutter;
    uint32_t content_width = out_layout->panel.width > inset * 2U ?
        out_layout->panel.width - inset * 2U : 0U;
    if (gutter_width >= content_width) gutter_width = content_width / 2U;
    uint32_t grid_x = ruler_x + gutter_width;
    uint32_t grid_width = content_width > gutter_width ? content_width - gutter_width : 0U;
    out_layout->ruler = rect_at(grid_x, ruler_y, grid_width, ruler_height);
    out_layout->scrubber = out_layout->ruler;
    uint32_t rows_y = out_layout->panel.y + row_start;
    uint32_t panel_bottom = out_layout->panel.y + out_layout->panel.height;
    uint32_t bottom_inset = inset;
    uint32_t rows_bottom = panel_bottom > bottom_inset ? panel_bottom - bottom_inset : 0U;
    uint32_t rows_height = rows_bottom > rows_y ? rows_bottom - rows_y : 0U;
    out_layout->track_gutter = rect_at(ruler_x, rows_y, gutter_width, rows_height);
    out_layout->time_grid = rect_at(grid_x, rows_y, grid_width, rows_height);
    if (rows_height == 0U || row_height == 0U) return true;
    size_t visible = rows_height / row_height;
    if (visible > VIEWER_TIMELINE_VISIBLE_TRACK_CAP)
        visible = VIEWER_TIMELINE_VISIBLE_TRACK_CAP;
    size_t remaining = out_layout->track_count - out_layout->first_track;
    if (visible > remaining) visible = remaining;
    for (size_t row = 0U; row < visible; ++row) {
        uint64_t y = (uint64_t)rows_y + (uint64_t)row * row_height;
        if (y > UINT32_MAX || y + row_height > panel_bottom) break;
        out_layout->track_rows[row] = rect_at(ruler_x, (uint32_t)y,
            content_width, row_height);
        out_layout->visible_track_count++;
    }
    return true;
}

static double time_from_x(const viewer_timeline *timeline,
                          const viewer_timeline_layout *layout, double x)
{
    double span = viewer_timeline_view_duration(timeline);
    if (!(span > 0.0) || layout->ruler.width == 0U) return 0.0;
    double fraction = (x - (double)layout->ruler.x) /
        (double)layout->ruler.width;
    if (!isfinite(fraction) || fraction < 0.0) fraction = 0.0;
    if (fraction > 1.0) fraction = 1.0;
    double value = timeline->view_start_seconds + fraction * span;
    if (!isfinite(value) || value < 0.0) value = 0.0;
    double duration = viewer_timeline_duration(timeline);
    if (value > duration) value = duration;
    return value;
}

static double track_start_time(const svg_anim_track *track)
{
    if (track == NULL || track->has_begin_sync) return 0.0;
    return isfinite(track->begin_offset) && track->begin_offset > 0.0 ?
        track->begin_offset : 0.0;
}

static bool key_near_pointer(const viewer_timeline *timeline,
    const viewer_timeline_layout *layout, size_t track_index, size_t row,
    double x, double y, double *out_time, size_t *out_key)
{
    if (timeline == NULL || layout == NULL || out_time == NULL || out_key == NULL ||
        track_index >= viewer_timeline_track_count(timeline)) return false;
    const svg_anim_track *track = &timeline->document->animation.tracks[track_index];
    if (!track->supported || track->keys == NULL || track->key_count == 0U) return false;
    double duration = track->duration_seconds;
    if (!isfinite(duration) || duration <= 0.0) return false;
    double start = track_start_time(track);
    uint32_t radius = viewer_timeline_key_hit_radius(layout->backing_scale);
    double center_y = (double)layout->track_rows[row].y +
        (double)layout->track_rows[row].height / 2.0;
    if (fabs(y - center_y) > (double)radius) return false;
    for (size_t key = 0U; key < track->key_count && key < SVG_ANIM_MAX_VALUES; ++key) {
        double key_time = start + track->keys[key].time * duration;
        double span = viewer_timeline_view_duration(timeline);
        if (!isfinite(key_time) || key_time < timeline->view_start_seconds ||
            key_time > timeline->view_start_seconds + span || !(span > 0.0)) continue;
        double key_x = (double)layout->ruler.x +
            ((key_time - timeline->view_start_seconds) / span) * layout->ruler.width;
        if (fabs(x - key_x) <= (double)radius) {
            *out_time = key_time;
            *out_key = key;
            return true;
        }
    }
    return false;
}

static void sync_editor_dock(viewer_timeline *timeline, viewer_editor *editor)
{
    if (timeline == NULL || editor == NULL) return;
    (void)viewer_editor_set_timeline_state(editor, true, timeline->dock_open,
        timeline->dock_height_points);
}

static bool select_track(viewer_timeline *timeline, viewer_editor *editor,
                         size_t track_index)
{
    if (track_index >= viewer_timeline_track_count(timeline)) return false;
    const svg_anim_track *track = &timeline->document->animation.tracks[track_index];
    if (!track->supported) {
        if (editor != NULL) viewer_editor_set_status(editor,
            "Unsupported animation track · preview only");
        return true;
    }
    timeline->selected_track = track_index;
    double start = track_start_time(track);
    (void)viewer_timeline_seek(timeline, start);
    timeline->selected_track = track_index;
    if (editor != NULL)
        (void)viewer_editor_select_at_source_offset(editor, track->target_offset);
    return true;
}

bool viewer_timeline_pointer_ex(viewer_timeline *timeline,
    const viewer_timeline_layout *layout, viewer_timeline_pointer_kind kind,
    double x, double y, double delta_x, double delta_y, uint32_t modifiers,
    double now_seconds, viewer_editor *editor)
{
    if (timeline == NULL || layout == NULL ||
        viewer_timeline_track_count(timeline) == 0U || !isfinite(x) ||
        !isfinite(y) || !isfinite(delta_x) || !isfinite(delta_y)) return false;
    if (kind == VIEWER_TIMELINE_POINTER_UP &&
        (timeline->dragging_scrubber || timeline->dragging_resize)) {
        timeline->dragging_scrubber = false;
        timeline->dragging_resize = false;
        return true;
    }
    if (kind == VIEWER_TIMELINE_POINTER_MOVE && timeline->dragging_scrubber) {
        (void)viewer_timeline_seek(timeline, time_from_x(timeline, layout, x));
        timeline->dragging_scrubber = true;
        return true;
    }
    if (kind == VIEWER_TIMELINE_POINTER_MOVE && timeline->dragging_key) {
        double next = time_from_x(timeline, layout, x);
        if (timeline->dragging_key_track < viewer_timeline_track_count(timeline)) {
            const svg_anim_track *track = &timeline->document->animation.tracks[
                timeline->dragging_key_track];
            size_t key = timeline->dragging_key_index;
            if (track->keys != NULL && key > 0U && key + 1U < track->key_count) {
                double start = track_start_time(track);
                double lower = start + track->keys[key - 1U].time *
                    track->duration_seconds;
                double upper = start + track->keys[key + 1U].time *
                    track->duration_seconds;
                if (isfinite(lower) && isfinite(upper) && lower < upper) {
                    lower = nextafter(lower, INFINITY);
                    upper = nextafter(upper, -INFINITY);
                    if (next < lower) next = lower;
                    if (next > upper) next = upper;
                }
            }
        }
        timeline->time_seconds = next;
        timeline->playing = false;
        timeline->has_tick_time = false;
        timeline->preview_failed = false;
        return true;
    }
    if (kind == VIEWER_TIMELINE_POINTER_MOVE && timeline->dragging_resize) {
        double panel_bottom = (double)layout->panel.y + layout->panel.height;
        double points = (panel_bottom - y) / layout->backing_scale;
        if (!isfinite(points)) return true;
        if (points < VIEWER_TIMELINE_DOCK_MIN_POINTS)
            points = VIEWER_TIMELINE_DOCK_MIN_POINTS;
        if (points > VIEWER_TIMELINE_DOCK_MAX_POINTS)
            points = VIEWER_TIMELINE_DOCK_MAX_POINTS;
        timeline->dock_height_points = (uint16_t)lround(points);
        timeline->dock_open = true;
        timeline->dock_state_explicit = true;
        sync_editor_dock(timeline, editor);
        return true;
    }
    if (kind == VIEWER_TIMELINE_POINTER_DOWN &&
        rect_contains(layout->resize_handle, x, y) && !layout->collapsed) {
        timeline->dragging_resize = true;
        return true;
    }
    if (!rect_contains(layout->panel, x, y)) return false;
    if (kind == VIEWER_TIMELINE_POINTER_SCROLL) {
        if (layout->collapsed) return true;
        if (rect_contains(layout->time_grid, x, y) ||
            rect_contains(layout->ruler, x, y)) {
            if ((modifiers & PLATFORM_MODIFIER_COMMAND) != 0U) {
                double anchor = time_from_x(timeline, layout, x);
                double factor = delta_y < 0.0 ? 1.15 : delta_y > 0.0 ? 1.0 / 1.15 : 1.0;
                (void)viewer_timeline_zoom_at(timeline, anchor, factor);
            } else if (delta_x != 0.0 && layout->ruler.width > 0U) {
                double seconds = delta_x *
                    viewer_timeline_view_duration(timeline) / layout->ruler.width;
                (void)viewer_timeline_pan(timeline, seconds);
            }
            return true;
        }
        if (layout->visible_track_count > 0U && delta_y != 0.0 &&
            y >= (double)layout->track_rows[0].y) {
            size_t step = fabs(delta_y) >= 3.0 ? 3U : 1U;
            size_t maximum = layout->track_count > layout->visible_track_count ?
                layout->track_count - layout->visible_track_count : 0U;
            if (delta_y < 0.0)
                timeline->first_track = timeline->first_track + step > maximum ?
                    maximum : timeline->first_track + step;
            else timeline->first_track = timeline->first_track > step ?
                timeline->first_track - step : 0U;
        }
        return true;
    }
    if (kind == VIEWER_TIMELINE_POINTER_DOWN) {
        if (layout->collapsed && rect_contains(layout->collapse_toggle, x, y)) {
            (void)viewer_timeline_dock_set_open(timeline, !timeline->dock_open);
            sync_editor_dock(timeline, editor);
            return true;
        }
        if (!layout->collapsed && rect_contains(layout->collapse_toggle, x, y)) {
            (void)viewer_timeline_dock_set_open(timeline, false);
            sync_editor_dock(timeline, editor);
            return true;
        }
        if (layout->collapsed) return true;
        if (rect_contains(layout->play_button, x, y)) {
            (void)viewer_timeline_toggle_play(timeline, now_seconds);
            return true;
        }
        if (rect_contains(layout->mode_button, x, y)) {
            viewer_timeline_mode next = timeline->mode == VIEWER_TIMELINE_MODE_ONE_SHOT ?
                VIEWER_TIMELINE_MODE_LOOP :
                timeline->mode == VIEWER_TIMELINE_MODE_LOOP ?
                    VIEWER_TIMELINE_MODE_PING_PONG : VIEWER_TIMELINE_MODE_ONE_SHOT;
            (void)viewer_timeline_set_mode(timeline, next);
            return true;
        }
        if (rect_contains(layout->speed_button, x, y)) {
            double next = timeline->playback_speed == 0.25 ? 0.5 :
                timeline->playback_speed == 0.5 ? 1.0 :
                timeline->playback_speed == 1.0 ? 2.0 : 0.25;
            (void)viewer_timeline_set_speed(timeline, next);
            return true;
        }
        if (rect_contains(layout->fit_button, x, y)) {
            (void)viewer_timeline_fit(timeline);
            return true;
        }
        if (rect_contains(layout->ruler, x, y)) {
            timeline->dragging_scrubber = true;
            (void)viewer_timeline_seek(timeline, time_from_x(timeline, layout, x));
            timeline->dragging_scrubber = true;
            return true;
        }
        for (size_t row = 0U; row < layout->visible_track_count; ++row) {
            if (!rect_contains(layout->track_rows[row], x, y)) continue;
            size_t index = layout->first_track + row;
            double key_time = 0.0;
            size_t key_index = 0U;
            bool on_key = rect_contains(layout->time_grid, x, y) &&
                key_near_pointer(timeline, layout, index, row, x, y,
                    &key_time, &key_index);
            if (!select_track(timeline, editor, index)) return false;
            timeline->selected_key = on_key ? key_index : 0U;
            timeline->easing_initialized = false;
            if (on_key) {
                (void)viewer_timeline_seek(timeline, key_time);
                const svg_anim_track *track =
                    &timeline->document->animation.tracks[index];
                timeline->dragging_key = track->supported &&
                    track->calc_mode != SVG_ANIM_CALC_SPLINE &&
                    key_index > 0U && key_index + 1U < track->key_count;
                timeline->dragging_key_track = index;
                timeline->dragging_key_index = key_index;
            }
            return true;
        }
        return true;
    }
    return kind == VIEWER_TIMELINE_POINTER_MOVE || kind == VIEWER_TIMELINE_POINTER_UP;
}

bool viewer_timeline_pointer(viewer_timeline *timeline,
    const viewer_timeline_layout *layout, viewer_timeline_pointer_kind kind,
    double x, double y, double delta_y, double now_seconds)
{
    return viewer_timeline_pointer_ex(timeline, layout, kind, x, y, 0.0,
        delta_y, 0U, now_seconds, NULL);
}
