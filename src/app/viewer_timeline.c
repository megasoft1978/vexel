#define _POSIX_C_SOURCE 200809L

#include "viewer_timeline.h"

#include "raster/shape_render.h"
#include "viewer_editor.h"
#include "viewer_chrome.h"

#include <math.h>
#include <stdio.h>
#include <string.h>
#include <time.h>

#define VIEWER_TIMELINE_FALLBACK_SECONDS 5.0
#define VIEWER_TIMELINE_FRAME_INTERVAL (1.0 / 30.0)
#define VIEWER_TIMELINE_PANEL_POINTS 178U
#define VIEWER_TIMELINE_MIN_WIDTH_POINTS 300U
#define VIEWER_TIMELINE_SOURCE_SCAN_CAP 4096U
#define VIEWER_TIMELINE_TAG_CAP 32U
#define VIEWER_TIMELINE_ID_CAP 48U

static double timeline_monotonic_seconds(void)
{
    struct timespec now;
    if (clock_gettime(CLOCK_MONOTONIC, &now) != 0) return 0.0;
    return (double)now.tv_sec + (double)now.tv_nsec / 1000000000.0;
}

static double min_double(double left, double right)
{
    return left < right ? left : right;
}

static size_t timeline_track_count_for_document(const svg_shape_document *document)
{
    if (document == NULL || document->animation.tracks == NULL) return 0U;
    size_t count = document->animation.track_count;
    return count > SVG_ANIM_MAX_TRACKS ? SVG_ANIM_MAX_TRACKS : count;
}

static double track_display_end(const svg_anim_track *track)
{
    if (track == NULL || !track->supported || track->begin_indefinite) return 0.0;
    double start = track->has_begin_sync ? 0.0 : track->begin_offset;
    if (!isfinite(start) || start < 0.0) start = 0.0;
    double span = 0.0;
    if (track->duration_indefinite || track->repeat_indefinite ||
        track->repeat_duration_indefinite || track->kind == SVG_ANIM_KIND_SET) {
        span = VIEWER_TIMELINE_FALLBACK_SECONDS;
    } else {
        double repeat_count = track->repeat_count;
        if (!isfinite(repeat_count) || repeat_count < 0.0) return 0.0;
        if (repeat_count == 0.0) repeat_count = 1.0;
        span = track->duration_seconds * repeat_count;
        if (!isfinite(span) || span < 0.0) span = VIEWER_TIMELINE_FALLBACK_SECONDS;
        if (!track->repeat_duration_indefinite && isfinite(track->repeat_duration) &&
            track->repeat_duration >= 0.0)
            span = min_double(span, track->repeat_duration);
    }
    double end = start + span;
    if (track->has_end && !track->end_is_sync && isfinite(track->end_seconds) &&
        track->end_seconds >= start) end = min_double(end, track->end_seconds);
    if (!isfinite(end) || end > SVG_ANIM_MAX_TIMELINE_SECONDS)
        end = SVG_ANIM_MAX_TIMELINE_SECONDS;
    return end;
}

static double document_display_duration(const svg_shape_document *document)
{
    size_t count = timeline_track_count_for_document(document);
    double duration = 0.0;
    for (size_t index = 0U; index < count; ++index) {
        double end = track_display_end(&document->animation.tracks[index]);
        if (end > duration) duration = end;
    }
    return duration;
}

void viewer_timeline_init(viewer_timeline *timeline)
{
    if (timeline == NULL) return;
    *timeline = (viewer_timeline){0};
}

void viewer_timeline_reset(viewer_timeline *timeline,
                           const svg_shape_document *document)
{
    if (timeline == NULL) return;
    raster_tile_destroy(&timeline->preview);
    *timeline = (viewer_timeline){0};
    timeline->document = document;
    timeline->duration_seconds = document_display_duration(document);
}

void viewer_timeline_destroy(viewer_timeline *timeline)
{
    if (timeline == NULL) return;
    raster_tile_destroy(&timeline->preview);
    *timeline = (viewer_timeline){0};
}

size_t viewer_timeline_track_count(const viewer_timeline *timeline)
{
    return timeline == NULL ? 0U :
        timeline_track_count_for_document(timeline->document);
}

double viewer_timeline_duration(const viewer_timeline *timeline)
{
    return timeline == NULL || !isfinite(timeline->duration_seconds) ||
        timeline->duration_seconds < 0.0 ? 0.0 : timeline->duration_seconds;
}

static bool source_name_byte(uint8_t byte)
{
    return (byte >= 'a' && byte <= 'z') || (byte >= 'A' && byte <= 'Z') ||
        (byte >= '0' && byte <= '9') || byte == '_' || byte == '-' ||
        byte == ':' || byte == '.';
}

static bool source_space(uint8_t byte)
{
    return byte == ' ' || byte == '\t' || byte == '\r' || byte == '\n';
}

static bool read_target_name(const viewer_timeline *timeline,
                             const svg_anim_track *track,
                             char tag[VIEWER_TIMELINE_TAG_CAP],
                             char id[VIEWER_TIMELINE_ID_CAP])
{
    if (timeline == NULL || timeline->document == NULL || track == NULL) return false;
    base_bytes source = timeline->document->animation.source;
    if (source.ptr == NULL || track->target_offset > source.len ||
        track->target_length > source.len - track->target_offset) return false;
    size_t end = track->target_offset + track->target_length;
    size_t scan_limit = source.len - track->target_offset;
    if (scan_limit > VIEWER_TIMELINE_SOURCE_SCAN_CAP)
        scan_limit = VIEWER_TIMELINE_SOURCE_SCAN_CAP;
    if (scan_limit > track->target_length) scan_limit = track->target_length;
    end = track->target_offset + scan_limit;
    size_t cursor = track->target_offset;
    while (cursor < end && source_space(source.ptr[cursor])) cursor++;
    if (cursor >= end || source.ptr[cursor] != '<') return false;
    cursor++;
    if (cursor < end && source.ptr[cursor] == '/') return false;
    size_t name_start = cursor;
    while (cursor < end && source_name_byte(source.ptr[cursor])) cursor++;
    if (cursor == name_start) return false;
    size_t name_length = cursor - name_start;
    size_t tag_copy = name_length < VIEWER_TIMELINE_TAG_CAP - 1U ?
        name_length : VIEWER_TIMELINE_TAG_CAP - 1U;
    for (size_t index = 0U; index < tag_copy; ++index)
        tag[index] = (char)source.ptr[name_start + index];
    tag[tag_copy] = '\0';
    id[0] = '\0';

    while (cursor < end) {
        while (cursor < end && (source_space(source.ptr[cursor]) ||
                source.ptr[cursor] == '/')) cursor++;
        if (cursor >= end || source.ptr[cursor] == '>') break;
        size_t attribute_start = cursor;
        while (cursor < end && source_name_byte(source.ptr[cursor])) cursor++;
        if (cursor == attribute_start) {
            cursor++;
            continue;
        }
        size_t attribute_length = cursor - attribute_start;
        while (cursor < end && source_space(source.ptr[cursor])) cursor++;
        if (cursor >= end || source.ptr[cursor] != '=') continue;
        cursor++;
        while (cursor < end && source_space(source.ptr[cursor])) cursor++;
        if (cursor >= end) break;
        uint8_t quote = 0U;
        if (source.ptr[cursor] == '\'' || source.ptr[cursor] == '"')
            quote = source.ptr[cursor++];
        size_t value_start = cursor;
        if (quote != 0U) {
            while (cursor < end && source.ptr[cursor] != quote) cursor++;
        } else {
            while (cursor < end && !source_space(source.ptr[cursor]) &&
                   source.ptr[cursor] != '>') cursor++;
        }
        size_t value_length = cursor - value_start;
        if (quote != 0U && cursor < end) cursor++;
        if (attribute_length == 2U &&
            source.ptr[attribute_start] == 'i' && source.ptr[attribute_start + 1U] == 'd') {
            size_t copied = value_length < VIEWER_TIMELINE_ID_CAP - 1U ?
                value_length : VIEWER_TIMELINE_ID_CAP - 1U;
            for (size_t index = 0U; index < copied; ++index) {
                uint8_t value = source.ptr[value_start + index];
                id[index] = source_name_byte(value) ? (char)value : '?';
            }
            if (copied < value_length && copied >= 3U) {
                id[copied - 3U] = '.';
                id[copied - 2U] = '.';
                id[copied - 1U] = '.';
            }
            id[copied] = '\0';
        }
    }
    return tag[0] != '\0';
}

bool viewer_timeline_track_label(const viewer_timeline *timeline, size_t track_index,
                                 char *buffer, size_t capacity, size_t *out_length)
{
    if (out_length != NULL) *out_length = 0U;
    if (timeline == NULL || buffer == NULL || capacity == 0U ||
        track_index >= viewer_timeline_track_count(timeline)) return false;
    const svg_anim_track *track = &timeline->document->animation.tracks[track_index];
    char tag[VIEWER_TIMELINE_TAG_CAP] = "element";
    char id[VIEWER_TIMELINE_ID_CAP] = {0};
    (void)read_target_name(timeline, track, tag, id);
    const char *attribute = svg_anim_attribute_name(track->attribute);
    if (attribute == NULL || attribute[0] == '\0') attribute = "animation";
    int amount = id[0] != '\0' ?
        snprintf(buffer, capacity, "%s#%s / %s", tag, id, attribute) :
        snprintf(buffer, capacity, "%s / %s", tag, attribute);
    if (amount < 0 || (size_t)amount >= capacity) {
        buffer[0] = '\0';
        return false;
    }
    if (out_length != NULL) *out_length = (size_t)amount;
    return true;
}

bool viewer_timeline_seek(viewer_timeline *timeline, double time_seconds)
{
    if (timeline == NULL || !isfinite(time_seconds) ||
        viewer_timeline_track_count(timeline) == 0U) return false;
    double duration = viewer_timeline_duration(timeline);
    if (time_seconds < 0.0) time_seconds = 0.0;
    if (time_seconds > duration) time_seconds = duration;
    bool changed = timeline->time_seconds != time_seconds || timeline->playing ||
        timeline->dragging_scrubber;
    timeline->time_seconds = time_seconds;
    timeline->playing = false;
    timeline->dragging_scrubber = false;
    timeline->has_tick_time = false;
    timeline->preview_failed = false;
    return changed;
}

bool viewer_timeline_set_loop(viewer_timeline *timeline, bool loop)
{
    if (timeline == NULL || viewer_timeline_track_count(timeline) == 0U ||
        timeline->loop == loop) return false;
    timeline->loop = loop;
    return true;
}

bool viewer_timeline_toggle_play(viewer_timeline *timeline, double now_seconds)
{
    if (timeline == NULL || viewer_timeline_track_count(timeline) == 0U ||
        !(viewer_timeline_duration(timeline) > 0.0) || !isfinite(now_seconds)) return false;
    if (!timeline->playing && timeline->time_seconds >= timeline->duration_seconds &&
        !timeline->loop) timeline->time_seconds = 0.0;
    timeline->playing = !timeline->playing;
    timeline->dragging_scrubber = false;
    timeline->last_tick_seconds = now_seconds;
    timeline->last_frame_tick_seconds = now_seconds;
    timeline->has_tick_time = true;
    timeline->preview_failed = false;
    return true;
}

bool viewer_timeline_advance(viewer_timeline *timeline, double now_seconds)
{
    if (timeline == NULL || !timeline->playing || !isfinite(now_seconds)) return false;
    if (!timeline->has_tick_time) {
        timeline->last_tick_seconds = now_seconds;
        timeline->last_frame_tick_seconds = now_seconds;
        timeline->has_tick_time = true;
        return false;
    }
    double delta = now_seconds - timeline->last_tick_seconds;
    if (!isfinite(delta) || delta < 0.0 ||
        delta > SVG_ANIM_MAX_TIMELINE_SECONDS) {
        timeline->last_tick_seconds = now_seconds;
        timeline->last_frame_tick_seconds = now_seconds;
        return false;
    }
    if (delta == 0.0) return false;
    if (now_seconds - timeline->last_frame_tick_seconds <
        VIEWER_TIMELINE_FRAME_INTERVAL) return false;
    timeline->last_tick_seconds = now_seconds;
    double duration = viewer_timeline_duration(timeline);
    double next = timeline->time_seconds + delta;
    if (!isfinite(next)) next = duration;
    bool reached_end = next >= duration;
    if (reached_end && timeline->loop && duration > 0.0) {
        next = fmod(next, duration);
        if (!isfinite(next) || next < 0.0) next = 0.0;
    } else if (reached_end) {
        next = duration;
        timeline->playing = false;
    }
    bool changed = next != timeline->time_seconds;
    timeline->time_seconds = next;
    if (!changed) return false;
    timeline->last_frame_tick_seconds = now_seconds;
    timeline->preview_failed = false;
    return true;
}

bool viewer_timeline_tick(viewer_timeline *timeline)
{
    if (timeline == NULL || !timeline->playing) return false;
    return viewer_timeline_advance(timeline, timeline_monotonic_seconds());
}

bool viewer_timeline_handle_key(viewer_timeline *timeline, uint16_t key_code,
                                const char *text, size_t text_length,
                                double now_seconds)
{
    if (timeline == NULL || viewer_timeline_track_count(timeline) == 0U ||
        (text_length > 0U && text == NULL)) return false;
    if (key_code == 49U || (text_length == 1U && text[0] == ' '))
        return viewer_timeline_toggle_play(timeline, now_seconds);
    if (text_length != 1U || (text[0] != '[' && text[0] != ']')) return false;
    double step = 1.0 / 30.0;
    double next = text[0] == '[' ? timeline->time_seconds - step :
        timeline->time_seconds + step;
    if (next < 0.0) next = 0.0;
    if (next > timeline->duration_seconds) next = timeline->duration_seconds;
    return viewer_timeline_seek(timeline, next);
}

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

bool viewer_timeline_layout_compute(const viewer_timeline *timeline,
                                    uint32_t width, uint32_t height,
                                    double backing_scale,
                                    viewer_timeline_layout *out_layout)
{
    if (timeline == NULL || out_layout == NULL || width == 0U || height == 0U ||
        width > 8192U || height > 8192U || !isfinite(backing_scale) ||
        backing_scale < 0.5 || backing_scale > 4.0 ||
        viewer_timeline_track_count(timeline) == 0U) return false;
    uint32_t panel_height, status_height, min_width;
    if (!scaled_points(VIEWER_TIMELINE_PANEL_POINTS, backing_scale, &panel_height) ||
        !scaled_points(VIEWER_CHROME_STATUS_POINTS, backing_scale, &status_height) ||
        !scaled_points(VIEWER_TIMELINE_MIN_WIDTH_POINTS, backing_scale, &min_width) ||
        width < min_width || height <= status_height) return false;
    memset(out_layout, 0, sizeof(*out_layout));
    out_layout->backing_scale = backing_scale;
    uint32_t panel_bottom = height - status_height;
    if (panel_height > panel_bottom) panel_height = panel_bottom;
    if (panel_height < status_height * 3U) return false;
    uint32_t panel_top = panel_bottom - panel_height;
    out_layout->panel = rect_at(0U, panel_top, width, panel_height);
    out_layout->track_count = viewer_timeline_track_count(timeline);

    uint32_t inset, play_x, play_width, control_y, control_height;
    uint32_t time_x, time_width, loop_x, loop_width, scrub_y, scrub_height;
    uint32_t row_start_y, row_height, row_inset;
    if (!scaled_points(12U, backing_scale, &inset) ||
        !scaled_points(12U, backing_scale, &play_x) ||
        !scaled_points(64U, backing_scale, &play_width) ||
        !scaled_points(24U, backing_scale, &control_y) ||
        !scaled_points(26U, backing_scale, &control_height) ||
        !scaled_points(84U, backing_scale, &time_x) ||
        !scaled_points(104U, backing_scale, &time_width) ||
        !scaled_points(196U, backing_scale, &loop_x) ||
        !scaled_points(92U, backing_scale, &loop_width) ||
        !scaled_points(52U, backing_scale, &scrub_y) ||
        !scaled_points(22U, backing_scale, &scrub_height) ||
        !scaled_points(78U, backing_scale, &row_start_y) ||
        !scaled_points(17U, backing_scale, &row_height) ||
        !scaled_points(8U, backing_scale, &row_inset)) return false;
    if ((uint64_t)panel_top + control_y > UINT32_MAX ||
        (uint64_t)panel_top + scrub_y > UINT32_MAX ||
        (uint64_t)panel_top + row_start_y > UINT32_MAX ||
        (uint64_t)inset + play_width > width ||
        (uint64_t)time_x + time_width + inset > width ||
        (uint64_t)loop_x + loop_width + inset > width || width <= inset * 2U)
        return false;
    out_layout->play_button = rect_at(play_x, panel_top + control_y,
        play_width, control_height);
    out_layout->time_readout = rect_at(time_x, panel_top + control_y,
        time_width, control_height);
    out_layout->loop_toggle = rect_at(loop_x, panel_top + control_y,
        loop_width, control_height);
    out_layout->scrubber = rect_at(inset, panel_top + scrub_y,
        width - inset * 2U, scrub_height);

    uint32_t row_y = panel_top + row_start_y;
    uint32_t row_bottom = panel_bottom > row_inset ? panel_bottom - row_inset : 0U;
    if (row_y >= row_bottom || row_height == 0U) return true;
    size_t visible = (size_t)((row_bottom - row_y) / row_height);
    if (visible > VIEWER_TIMELINE_VISIBLE_TRACK_CAP)
        visible = VIEWER_TIMELINE_VISIBLE_TRACK_CAP;
    size_t track_count = out_layout->track_count;
    if (visible > track_count) visible = track_count;
    uint32_t row_width = width - inset * 2U;
    for (size_t row = 0U; row < visible; ++row) {
        uint64_t y = (uint64_t)row_y + (uint64_t)row * row_height;
        if (y > UINT32_MAX || y + row_height > panel_bottom) break;
        out_layout->track_rows[row] = rect_at(inset, (uint32_t)y, row_width, row_height);
        out_layout->visible_track_count++;
    }
    return true;
}

static bool rect_contains(ui_rect rect, double x, double y)
{
    return isfinite(x) && isfinite(y) && x >= (double)rect.x &&
        y >= (double)rect.y && x - (double)rect.x < (double)rect.width &&
        y - (double)rect.y < (double)rect.height;
}

static double seek_time_from_x(const viewer_timeline_layout *layout, double x,
                               double duration)
{
    uint32_t track_inset = 8U, value_reserve = 64U;
    double scaled_inset = ceil((double)track_inset * layout->backing_scale);
    double scaled_reserve = ceil((double)value_reserve * layout->backing_scale);
    if (!isfinite(scaled_inset) || !isfinite(scaled_reserve) ||
        scaled_inset > (double)UINT32_MAX || scaled_reserve > (double)UINT32_MAX)
        return 0.0;
    track_inset = (uint32_t)scaled_inset;
    value_reserve = (uint32_t)scaled_reserve;
    if (layout->scrubber.width <= value_reserve) track_inset = 0U;
    double start = (double)layout->scrubber.x + (double)track_inset;
    double track_width = layout->scrubber.width > value_reserve ?
        (double)(layout->scrubber.width - value_reserve) :
        (double)layout->scrubber.width;
    if (!(track_width > 0.0)) return 0.0;
    double progress = (x - start) / track_width;
    if (!isfinite(progress) || progress < 0.0) progress = 0.0;
    if (progress > 1.0) progress = 1.0;
    return progress * duration;
}

bool viewer_timeline_pointer(viewer_timeline *timeline,
                             const viewer_timeline_layout *layout,
                             viewer_timeline_pointer_kind kind,
                             double x, double y, double delta_y,
                             double now_seconds)
{
    if (timeline == NULL || layout == NULL ||
        viewer_timeline_track_count(timeline) == 0U) return false;
    if (kind == VIEWER_TIMELINE_POINTER_UP && timeline->dragging_scrubber) {
        timeline->dragging_scrubber = false;
        return true;
    }
    if (kind == VIEWER_TIMELINE_POINTER_MOVE && timeline->dragging_scrubber) {
        double next = seek_time_from_x(layout, x, timeline->duration_seconds);
        timeline->time_seconds = next;
        timeline->playing = false;
        timeline->has_tick_time = false;
        timeline->preview_failed = false;
        return true;
    }
    if (!rect_contains(layout->panel, x, y)) return false;
    if (kind == VIEWER_TIMELINE_POINTER_SCROLL) {
        if (layout->visible_track_count > 0U &&
            y >= (double)layout->track_rows[0].y && delta_y != 0.0) {
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
        if (rect_contains(layout->play_button, x, y)) {
            (void)viewer_timeline_toggle_play(timeline, now_seconds);
            return true;
        }
        if (rect_contains(layout->loop_toggle, x, y)) {
            (void)viewer_timeline_set_loop(timeline, !timeline->loop);
            return true;
        }
        if (rect_contains(layout->scrubber, x, y)) {
            timeline->dragging_scrubber = true;
            double next = seek_time_from_x(layout, x, timeline->duration_seconds);
            (void)viewer_timeline_seek(timeline, next);
            timeline->dragging_scrubber = true;
            return true;
        }
        for (size_t row = 0U; row < layout->visible_track_count; ++row) {
            if (!rect_contains(layout->track_rows[row], x, y)) continue;
            size_t index = timeline->first_track + row;
            if (index >= viewer_timeline_track_count(timeline)) return true;
            timeline->selected_track = index;
            double start = timeline->document->animation.tracks[index].has_begin_sync ?
                0.0 : timeline->document->animation.tracks[index].begin_offset;
            (void)viewer_timeline_seek(timeline, start);
            timeline->selected_track = index;
            return true;
        }
        return true;
    }
    return kind == VIEWER_TIMELINE_POINTER_MOVE || kind == VIEWER_TIMELINE_POINTER_UP;
}

bool viewer_timeline_process_event(viewer_timeline *timeline,
                                   const platform_event *event,
                                   viewer_editor *editor,
                                   uint32_t width, uint32_t height,
                                   double backing_scale)
{
    if (timeline == NULL || event == NULL || editor == NULL ||
        viewer_timeline_track_count(timeline) == 0U) return false;
    if (event->type == PLATFORM_EVENT_KEY_DOWN &&
        !viewer_editor_text_input_active(editor) &&
        (event->modifiers & (PLATFORM_MODIFIER_COMMAND | PLATFORM_MODIFIER_CONTROL |
                             PLATFORM_MODIFIER_OPTION)) == 0U) {
        return viewer_timeline_handle_key(timeline, event->key_code,
            event->text, event->text_len, timeline_monotonic_seconds());
    }
    viewer_timeline_layout layout;
    if (!viewer_timeline_layout_compute(timeline, width, height, backing_scale,
            &layout)) return false;
    double x = 0.0, y = 0.0;
    viewer_timeline_pointer_kind pointer_kind;
    switch (event->type) {
        case PLATFORM_EVENT_POINTER_DOWN:
            x = (double)event->logical_x * backing_scale;
            y = (double)event->logical_y * backing_scale;
            if (event->button != 0U) return rect_contains(layout.panel, x, y);
            pointer_kind = VIEWER_TIMELINE_POINTER_DOWN;
            break;
        case PLATFORM_EVENT_POINTER_MOVE:
            x = (double)event->logical_x * backing_scale;
            y = (double)event->logical_y * backing_scale;
            pointer_kind = VIEWER_TIMELINE_POINTER_MOVE;
            break;
        case PLATFORM_EVENT_POINTER_UP:
            x = (double)event->logical_x * backing_scale;
            y = (double)event->logical_y * backing_scale;
            pointer_kind = VIEWER_TIMELINE_POINTER_UP;
            break;
        case PLATFORM_EVENT_SCROLL:
            x = event->precise_x * backing_scale;
            y = event->precise_y * backing_scale;
            pointer_kind = VIEWER_TIMELINE_POINTER_SCROLL;
            break;
        default: return false;
    }
    if (!isfinite(x) || !isfinite(y)) return false;
    return viewer_timeline_pointer(timeline, &layout, pointer_kind, x, y,
        event->delta_y, timeline_monotonic_seconds());
}

const raster_tile *viewer_timeline_prepare_frame(viewer_timeline *timeline)
{
    if (timeline == NULL || timeline->document == NULL ||
        viewer_timeline_track_count(timeline) == 0U) return NULL;
    if (timeline->preview_valid && timeline->preview_time_seconds == timeline->time_seconds)
        return &timeline->preview;
    raster_tile_destroy(&timeline->preview);
    timeline->preview_valid = false;
    svg_shape_diagnostic diagnostic = {SVG_SHAPE_ERROR_NONE, 0U};
    if (!raster_shape_render_at(timeline->document, timeline->time_seconds,
            &timeline->preview, &diagnostic)) {
        timeline->preview_failed = true;
        return NULL;
    }
    timeline->preview_time_seconds = timeline->time_seconds;
    timeline->preview_valid = true;
    timeline->preview_failed = false;
    return &timeline->preview;
}
