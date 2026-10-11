#define _POSIX_C_SOURCE 200809L

#include "viewer_timeline.h"

#include "raster/shape_render.h"
#include "viewer_anim_edit.h"
#include "viewer_editor.h"
#include "viewer_chrome.h"

#include <math.h>
#include <stdio.h>
#include <string.h>
#include <time.h>

#define VIEWER_TIMELINE_FALLBACK_SECONDS 5.0
#define VIEWER_TIMELINE_FRAME_INTERVAL (1.0 / 30.0)
#define VIEWER_TIMELINE_SOURCE_SCAN_CAP 4096U
#define VIEWER_TIMELINE_TAG_CAP 32U
#define VIEWER_TIMELINE_ID_CAP 48U

static double timeline_monotonic_seconds(void)
{
    struct timespec now;
    if (clock_gettime(CLOCK_MONOTONIC, &now) != 0) return 0.0;
    return (double)now.tv_sec + (double)now.tv_nsec / 1000000000.0;
}

static bool timeline_rect_contains(ui_rect rect, double x, double y)
{
    return rect.width > 0U && rect.height > 0U && isfinite(x) && isfinite(y) &&
        x >= (double)rect.x && y >= (double)rect.y &&
        x - (double)rect.x < (double)rect.width &&
        y - (double)rect.y < (double)rect.height;
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
    bool indefinite = false;
    for (size_t index = 0U; index < count; ++index) {
        const svg_anim_track *track = &document->animation.tracks[index];
        if (track->duration_indefinite || track->repeat_indefinite ||
            track->repeat_duration_indefinite) indefinite = true;
        double end = track_display_end(&document->animation.tracks[index]);
        if (end > duration) duration = end;
    }
    return indefinite ? VIEWER_TIMELINE_FALLBACK_SECONDS : duration;
}

void viewer_timeline_init(viewer_timeline *timeline)
{
    if (timeline == NULL) return;
    *timeline = (viewer_timeline){0};
    timeline->dock_height_points = VIEWER_TIMELINE_DOCK_DEFAULT_POINTS;
    timeline->zoom_factor = 1.0;
    timeline->playback_speed = 1.0;
    timeline->mode = VIEWER_TIMELINE_MODE_ONE_SHOT;
    timeline->playback_direction = 1;
}

void viewer_timeline_reset(viewer_timeline *timeline,
                           const svg_shape_document *document)
{
    if (timeline == NULL) return;
    raster_tile_destroy(&timeline->preview);
    bool dock_state_explicit = timeline->dock_state_explicit;
    bool dock_open = timeline->dock_open;
    bool reduced_motion = timeline->reduced_motion;
    uint16_t dock_height_points = timeline->dock_height_points;
    viewer_timeline_mode mode = timeline->mode;
    double speed = timeline->playback_speed;
    *timeline = (viewer_timeline){0};
    timeline->document = document;
    timeline->duration_seconds = document_display_duration(document);
    timeline->dock_state_explicit = dock_state_explicit;
    timeline->dock_open = timeline_track_count_for_document(document) > 0U &&
        (dock_state_explicit ? dock_open : true);
    timeline->dock_height_points = dock_height_points >= VIEWER_TIMELINE_DOCK_MIN_POINTS &&
        dock_height_points <= VIEWER_TIMELINE_DOCK_MAX_POINTS ? dock_height_points :
            VIEWER_TIMELINE_DOCK_DEFAULT_POINTS;
    timeline->reduced_motion = reduced_motion;
    timeline->mode = mode >= VIEWER_TIMELINE_MODE_ONE_SHOT &&
        mode <= VIEWER_TIMELINE_MODE_PING_PONG ? mode :
        VIEWER_TIMELINE_MODE_ONE_SHOT;
    timeline->loop = timeline->mode == VIEWER_TIMELINE_MODE_LOOP;
    timeline->playback_speed = speed == 0.25 || speed == 0.5 || speed == 1.0 ||
        speed == 2.0 ? speed : 1.0;
    timeline->zoom_factor = 1.0;
    timeline->playback_direction = 1;
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
    size_t same_attribute_before = 0U;
    for (size_t index = 0U; index < track_index; ++index) {
        const svg_anim_track *prior = &timeline->document->animation.tracks[index];
        if (prior->target_offset == track->target_offset &&
            prior->target_length == track->target_length &&
            prior->attribute == track->attribute) same_attribute_before++;
    }
    if (same_attribute_before > 0U && amount > 0 &&
        (size_t)amount < capacity) {
        int suffix = snprintf(buffer + (size_t)amount, capacity - (size_t)amount,
            " · %zu", same_attribute_before + 1U);
        if (suffix < 0 || (size_t)suffix >= capacity - (size_t)amount)
            amount = (int)capacity;
        else amount += suffix;
    }
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
    return viewer_timeline_set_mode(timeline, loop ? VIEWER_TIMELINE_MODE_LOOP :
        VIEWER_TIMELINE_MODE_ONE_SHOT);
}

bool viewer_timeline_set_mode(viewer_timeline *timeline, viewer_timeline_mode mode)
{
    if (timeline == NULL || viewer_timeline_track_count(timeline) == 0U ||
        mode < VIEWER_TIMELINE_MODE_ONE_SHOT ||
        mode > VIEWER_TIMELINE_MODE_PING_PONG || timeline->mode == mode) return false;
    timeline->mode = mode;
    timeline->loop = mode == VIEWER_TIMELINE_MODE_LOOP;
    timeline->playback_direction = 1;
    return true;
}

bool viewer_timeline_set_speed(viewer_timeline *timeline, double speed)
{
    if (timeline == NULL || viewer_timeline_track_count(timeline) == 0U ||
        (speed != 0.25 && speed != 0.5 && speed != 1.0 && speed != 2.0) ||
        timeline->playback_speed == speed) return false;
    timeline->playback_speed = speed;
    return true;
}

double viewer_timeline_view_duration(const viewer_timeline *timeline)
{
    if (timeline == NULL) return 0.0;
    double duration = viewer_timeline_duration(timeline);
    double zoom = timeline->zoom_factor;
    if (!(zoom >= 1.0) || !isfinite(zoom)) zoom = 1.0;
    double view_duration = duration / zoom;
    return isfinite(view_duration) && view_duration >= 0.0 ? view_duration : 0.0;
}

bool viewer_timeline_zoom_at(viewer_timeline *timeline, double anchor_seconds,
                             double factor)
{
    if (timeline == NULL || viewer_timeline_track_count(timeline) == 0U ||
        !isfinite(anchor_seconds) || !isfinite(factor) || factor <= 0.0 ||
        !(timeline->duration_seconds > 0.0)) return false;
    double old_zoom = timeline->zoom_factor;
    if (!isfinite(old_zoom) || old_zoom < 1.0) old_zoom = 1.0;
    double next_zoom = old_zoom * factor;
    if (!isfinite(next_zoom) || next_zoom < 1.0) next_zoom = 1.0;
    if (next_zoom > 64.0) next_zoom = 64.0;
    if (next_zoom == old_zoom) return false;
    double old_span = timeline->duration_seconds / old_zoom;
    double old_start = timeline->view_start_seconds;
    if (!isfinite(old_start) || old_start < 0.0) old_start = 0.0;
    double fraction = old_span > 0.0 ? (anchor_seconds - old_start) / old_span : 0.0;
    if (!isfinite(fraction)) fraction = 0.5;
    if (fraction < 0.0) fraction = 0.0;
    if (fraction > 1.0) fraction = 1.0;
    double next_span = timeline->duration_seconds / next_zoom;
    double next_start = anchor_seconds - fraction * next_span;
    double max_start = timeline->duration_seconds - next_span;
    if (!isfinite(next_start) || next_start < 0.0) next_start = 0.0;
    if (next_start > max_start) next_start = max_start;
    timeline->zoom_factor = next_zoom;
    timeline->view_start_seconds = next_start;
    return true;
}

bool viewer_timeline_pan(viewer_timeline *timeline, double delta_seconds)
{
    if (timeline == NULL || viewer_timeline_track_count(timeline) == 0U ||
        !isfinite(delta_seconds)) return false;
    double span = viewer_timeline_view_duration(timeline);
    double maximum = viewer_timeline_duration(timeline) - span;
    double next = timeline->view_start_seconds + delta_seconds;
    if (!isfinite(next)) return false;
    if (next < 0.0) next = 0.0;
    if (next > maximum) next = maximum;
    if (next == timeline->view_start_seconds) return false;
    timeline->view_start_seconds = next;
    return true;
}

bool viewer_timeline_fit(viewer_timeline *timeline)
{
    if (timeline == NULL || viewer_timeline_track_count(timeline) == 0U) return false;
    bool changed = timeline->zoom_factor != 1.0 || timeline->view_start_seconds != 0.0;
    timeline->zoom_factor = 1.0;
    timeline->view_start_seconds = 0.0;
    return changed;
}

bool viewer_timeline_dock_set_open(viewer_timeline *timeline, bool open)
{
    if (timeline == NULL || viewer_timeline_track_count(timeline) == 0U ||
        timeline->dock_open == open) return false;
    timeline->dock_open = open;
    timeline->dock_state_explicit = true;
    timeline->dragging_resize = false;
    return true;
}

bool viewer_timeline_dock_resize(viewer_timeline *timeline, int32_t delta_points)
{
    if (timeline == NULL || viewer_timeline_track_count(timeline) == 0U) return false;
    int64_t current = timeline->dock_height_points;
    int64_t next = current + (int64_t)delta_points;
    if (next < (int64_t)VIEWER_TIMELINE_DOCK_MIN_POINTS)
        next = (int64_t)VIEWER_TIMELINE_DOCK_MIN_POINTS;
    if (next > (int64_t)VIEWER_TIMELINE_DOCK_MAX_POINTS)
        next = (int64_t)VIEWER_TIMELINE_DOCK_MAX_POINTS;
    if (next == current) return false;
    timeline->dock_height_points = (uint16_t)next;
    timeline->dock_open = true;
    timeline->dock_state_explicit = true;
    return true;
}

void viewer_timeline_set_reduced_motion(viewer_timeline *timeline, bool reduced_motion)
{
    if (timeline != NULL) timeline->reduced_motion = reduced_motion;
}

void viewer_timeline_sync_selection(viewer_timeline *timeline, viewer_editor *editor)
{
    if (timeline == NULL || editor == NULL || viewer_timeline_track_count(timeline) == 0U)
        return;
    document_source_span selection = {0};
    if (!viewer_editor_selection_span(editor, &selection)) return;
    for (size_t index = 0U; index < viewer_timeline_track_count(timeline); ++index) {
        const svg_anim_track *track = &timeline->document->animation.tracks[index];
        if (track->target_offset >= selection.offset &&
            track->target_offset - selection.offset < selection.length) {
            timeline->selected_track = index;
            if (index < timeline->first_track ||
                index - timeline->first_track >= VIEWER_TIMELINE_VISIBLE_TRACK_CAP)
                timeline->first_track = index;
            return;
        }
    }
}

uint32_t viewer_timeline_key_hit_radius(double backing_scale)
{
    if (!isfinite(backing_scale) || backing_scale < 0.5 || backing_scale > 4.0)
        return 0U;
    double pixels = ceil(6.0 * backing_scale);
    if (pixels > (double)UINT32_MAX) return 0U;
    return (uint32_t)pixels;
}

bool viewer_timeline_toggle_play(viewer_timeline *timeline, double now_seconds)
{
    if (timeline == NULL || viewer_timeline_track_count(timeline) == 0U ||
        !(viewer_timeline_duration(timeline) > 0.0) || !isfinite(now_seconds)) return false;
    if (!timeline->playing && timeline->mode == VIEWER_TIMELINE_MODE_ONE_SHOT &&
        timeline->time_seconds >= timeline->duration_seconds)
        timeline->time_seconds = 0.0;
    if (!timeline->playing && timeline->time_seconds <= 0.0)
        timeline->playback_direction = 1;
    else if (!timeline->playing && timeline->time_seconds >= timeline->duration_seconds)
        timeline->playback_direction = -1;
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
    double scaled_delta = delta * timeline->playback_speed;
    if (!isfinite(scaled_delta)) scaled_delta = SVG_ANIM_MAX_TIMELINE_SECONDS;
    double next = timeline->time_seconds + scaled_delta;
    if (!isfinite(next)) next = duration;
    bool reached_end = next >= duration;
    if (timeline->mode == VIEWER_TIMELINE_MODE_PING_PONG && duration > 0.0) {
        double period = duration * 2.0;
        double phase = timeline->playback_direction >= 0 ?
            timeline->time_seconds : period - timeline->time_seconds;
        phase = fmod(phase + scaled_delta, period);
        if (!isfinite(phase) || phase < 0.0) phase = 0.0;
        if (phase <= duration) {
            next = phase;
            timeline->playback_direction = phase >= duration ? -1 : 1;
        } else {
            next = period - phase;
            timeline->playback_direction = -1;
        }
    } else if (reached_end && timeline->mode == VIEWER_TIMELINE_MODE_LOOP && duration > 0.0) {
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

bool viewer_timeline_process_event(viewer_timeline *timeline,
                                   const platform_event *event,
                                   viewer_editor *editor,
                                   uint32_t width, uint32_t height,
                                   double backing_scale)
{
    if (timeline == NULL || event == NULL || editor == NULL ||
        viewer_timeline_track_count(timeline) == 0U) return false;
    viewer_timeline_layout layout;
    if (!viewer_timeline_layout_compute(timeline, width, height, backing_scale,
            &layout)) return false;
    if (event->type == PLATFORM_EVENT_POINTER_UP && timeline->dragging_key) {
        size_t track_index = timeline->dragging_key_track;
        size_t key_index = timeline->dragging_key_index;
        timeline->dragging_key = false;
        uint32_t result = viewer_editor_animation_move_key(editor,
            &timeline->document->animation, track_index, key_index,
            timeline->time_seconds);
        if ((result & VIEWER_EDITOR_MUTATED) != 0U)
            timeline->edit_mutation_pending = true;
        return true;
    }
    if (viewer_timeline_easing_process_event(timeline, editor, event, &layout))
        return true;
    if (event->type == PLATFORM_EVENT_KEY_DOWN &&
        !viewer_editor_text_input_active(editor) &&
        (event->modifiers & (PLATFORM_MODIFIER_COMMAND | PLATFORM_MODIFIER_CONTROL |
                             PLATFORM_MODIFIER_OPTION)) == 0U) {
        return viewer_timeline_handle_key(timeline, event->key_code,
            event->text, event->text_len, timeline_monotonic_seconds());
    }
    double x = 0.0, y = 0.0;
    double delta_x = 0.0, delta_y = 0.0;
    viewer_timeline_pointer_kind pointer_kind;
    switch (event->type) {
        case PLATFORM_EVENT_POINTER_DOWN:
            x = (double)event->logical_x * backing_scale;
            y = (double)event->logical_y * backing_scale;
            if (event->button != 0U) return timeline_rect_contains(layout.panel, x, y);
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
            delta_x = event->delta_x * backing_scale;
            delta_y = event->delta_y;
            pointer_kind = VIEWER_TIMELINE_POINTER_SCROLL;
            break;
        default: return false;
    }
    if (!isfinite(x) || !isfinite(y)) return false;
    return viewer_timeline_pointer_ex(timeline, &layout, pointer_kind, x, y,
        delta_x, delta_y, event->modifiers, timeline_monotonic_seconds(), editor);
}

bool viewer_timeline_take_editor_mutation(viewer_timeline *timeline)
{
    if (timeline == NULL) return false;
    bool changed = timeline->edit_mutation_pending;
    timeline->edit_mutation_pending = false;
    return changed;
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
