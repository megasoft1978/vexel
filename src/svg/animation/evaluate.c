#include "animation.h"

#include <math.h>
#include <stdlib.h>
#include <string.h>

static double track_duration(const svg_anim_track *track)
{
    double duration = track->duration_indefinite ? INFINITY :
                      track->duration_seconds;
    if (track->repeat_indefinite && isfinite(duration)) duration = INFINITY;
    else if (isfinite(duration))
        duration *= track->repeat_count;
    if (!track->repeat_duration_indefinite &&
        track->repeat_duration < duration) duration = track->repeat_duration;
    if (!isfinite(duration) || duration > SVG_ANIM_MAX_TIMELINE_SECONDS)
        duration = SVG_ANIM_MAX_TIMELINE_SECONDS;
    return duration;
}

static bool resolve_start(const svg_anim_document *document, size_t index,
                          uint8_t state[SVG_ANIM_MAX_TRACKS],
                          double starts[SVG_ANIM_MAX_TRACKS])
{
    if (index >= document->track_count) return false;
    if (state[index] == 2U) return true;
    if (state[index] == 1U) {
        starts[index] = INFINITY;
        state[index] = 2U;
        return true;
    }
    state[index] = 1U;
    const svg_anim_track *track = &document->tracks[index];
    if (!track->supported || track->begin_indefinite) {
        starts[index] = INFINITY;
    } else if (!track->has_begin_sync) {
        starts[index] = track->begin_offset;
    } else {
        size_t reference = track->begin_sync_track;
        if (reference >= document->track_count ||
            !resolve_start(document, reference, state, starts)) return false;
        double reference_time = starts[reference];
        if (track->begin_sync_end) {
            if (!isfinite(reference_time)) {
                starts[index] = INFINITY;
                state[index] = 2U;
                return true;
            }
            reference_time += track_duration(&document->tracks[reference]);
        }
        starts[index] = reference_time + track->begin_offset;
    }
    if (isfinite(starts[index]) &&
        (starts[index] < 0.0 || starts[index] > SVG_ANIM_MAX_TIMELINE_SECONDS))
        starts[index] = INFINITY;
    state[index] = 2U;
    return true;
}

static double active_duration(const svg_anim_document *document, size_t index,
                              const uint8_t state[SVG_ANIM_MAX_TRACKS],
                              const double starts[SVG_ANIM_MAX_TRACKS])
{
    const svg_anim_track *track = &document->tracks[index];
    double duration = track_duration(track);
    if (track->has_end) {
        double end_time = track->end_seconds;
        if (track->end_is_sync) {
            size_t reference = track->end_sync_track;
            if (reference >= document->track_count || state[reference] != 2U ||
                !isfinite(starts[reference])) return 0.0;
            end_time += starts[reference] +
                        track_duration(&document->tracks[reference]);
        }
        double until_end = end_time - starts[index];
        if (until_end < duration) duration = until_end;
    }
    if (duration < 0.0) return 0.0;
    if (duration > SVG_ANIM_MAX_TIMELINE_SECONDS)
        return SVG_ANIM_MAX_TIMELINE_SECONDS;
    return duration;
}

static double value_distance(const svg_anim_value *left,
                             const svg_anim_value *right)
{
    if (left->kind != right->kind) return 0.0;
    if (left->kind == SVG_ANIM_VALUE_COLOR) {
        double sum = 0.0;
        for (size_t channel = 0U; channel < 4U; ++channel) {
            double delta = (double)right->rgba[channel] - left->rgba[channel];
            sum += delta * delta;
        }
        return sqrt(sum);
    }
    if (left->kind == SVG_ANIM_VALUE_NUMBER) return fabs(right->number[0] - left->number[0]);
    if (left->kind == SVG_ANIM_VALUE_TRANSFORM) {
        double sum = 0.0;
        size_t count = (size_t)left->number[3];
        if (count > 3U) count = 3U;
        for (size_t index = 0U; index < count; ++index) {
            double delta = right->number[index] - left->number[index];
            sum += delta * delta;
        }
        return sqrt(sum);
    }
    return 0.0;
}

static double bezier_coordinate(double t, double p1, double p2)
{
    double inverse = 1.0 - t;
    return 3.0 * inverse * inverse * t * p1 +
           3.0 * inverse * t * t * p2 + t * t * t;
}

static double spline_progress(double progress, const double curve[4])
{
    double low = 0.0, high = 1.0, parameter = progress;
    for (size_t iteration = 0U; iteration < 48U; ++iteration) {
        parameter = (low + high) * 0.5;
        double x = bezier_coordinate(parameter, curve[0], curve[2]);
        if (x < progress) low = parameter;
        else high = parameter;
    }
    return bezier_coordinate(parameter, curve[1], curve[3]);
}

static bool paced_interval(const svg_anim_track *track, double progress,
                           size_t *left, size_t *right,
                           double *local_progress)
{
    double total = 0.0;
    for (size_t index = 0U; index + 1U < track->key_count; ++index)
        total += value_distance(&track->keys[index].value,
                                &track->keys[index + 1U].value);
    if (!(total > 0.0)) {
        *left = 0U;
        *right = track->key_count > 1U ? 1U : 0U;
        *local_progress = progress;
        return true;
    }
    double wanted = progress * total, traversed = 0.0;
    for (size_t index = 0U; index + 1U < track->key_count; ++index) {
        double length = value_distance(&track->keys[index].value,
                                       &track->keys[index + 1U].value);
        if (wanted <= traversed + length || index + 2U == track->key_count) {
            *left = index;
            *right = index + 1U;
            *local_progress = length > 0.0 ?
                (wanted - traversed) / length : 0.0;
            return true;
        }
        traversed += length;
    }
    return false;
}

static bool interpolate(const svg_anim_track *track, double progress,
                        svg_anim_value *out)
{
    if (track->key_count == 0U) return false;
    if (track->key_count == 1U) {
        *out = track->keys[0].value;
        return true;
    }
    if (progress <= 0.0) {
        *out = track->keys[0].value;
        return true;
    }
    if (progress >= 1.0) {
        *out = track->keys[track->key_count - 1U].value;
        return true;
    }
    size_t left = 0U, right = 1U;
    double local = 0.0;
    if (track->calc_mode == SVG_ANIM_CALC_PACED) {
        if (!paced_interval(track, progress, &left, &right, &local)) return false;
    } else {
        while (right + 1U < track->key_count &&
               progress >= track->keys[right].time) {
            left++;
            right++;
        }
        double interval = track->keys[right].time - track->keys[left].time;
        local = interval > 0.0 ?
            (progress - track->keys[left].time) / interval : 0.0;
    }
    if (track->calc_mode == SVG_ANIM_CALC_DISCRETE ||
        track->keys[left].value.kind == SVG_ANIM_VALUE_PATH ||
        track->keys[left].value.kind == SVG_ANIM_VALUE_MOTION) {
        *out = local < 1.0 ? track->keys[left].value : track->keys[right].value;
        return true;
    }
    if (track->calc_mode == SVG_ANIM_CALC_SPLINE && left < track->spline_count)
        local = spline_progress(local, track->splines[left]);
    const svg_anim_value *start = &track->keys[left].value;
    const svg_anim_value *end = &track->keys[right].value;
    if (start->kind != end->kind) return false;
    *out = *start;
    if (start->kind == SVG_ANIM_VALUE_COLOR) {
        for (size_t channel = 0U; channel < 4U; ++channel) {
            double value = (double)start->rgba[channel] +
                ((double)end->rgba[channel] - start->rgba[channel]) * local;
            if (value < 0.0) value = 0.0;
            if (value > 255.0) value = 255.0;
            out->rgba[channel] = (uint8_t)floor(value + 0.5);
        }
        out->raw_offset = 0U;
        out->raw_length = 0U;
        return true;
    }
    if (start->kind == SVG_ANIM_VALUE_NUMBER) {
        out->number[0] = start->number[0] +
            (end->number[0] - start->number[0]) * local;
        return isfinite(out->number[0]);
    }
    if (start->kind == SVG_ANIM_VALUE_TRANSFORM) {
        size_t count = (size_t)start->number[3];
        if (count > 3U || count != (size_t)end->number[3]) return false;
        for (size_t index = 0U; index < count; ++index)
            out->number[index] = start->number[index] +
                (end->number[index] - start->number[index]) * local;
        return true;
    }
    *out = *start;
    return true;
}

static void add_accumulation(const svg_anim_track *track, size_t iteration,
                             svg_anim_value *value)
{
    if (!track->accumulate || iteration == 0U || track->key_count < 2U) return;
    double repeat = (double)iteration;
    const svg_anim_value *first = &track->keys[0].value;
    const svg_anim_value *last = &track->keys[track->key_count - 1U].value;
    if (value->kind == SVG_ANIM_VALUE_NUMBER && first->kind == value->kind &&
        last->kind == value->kind) {
        value->number[0] += repeat * (last->number[0] - first->number[0]);
    } else if (value->kind == SVG_ANIM_VALUE_COLOR && first->kind == value->kind &&
               last->kind == value->kind) {
        for (size_t channel = 0U; channel < 4U; ++channel) {
            double result = value->rgba[channel] + repeat *
                ((double)last->rgba[channel] - first->rgba[channel]);
            if (result < 0.0) result = 0.0;
            if (result > 255.0) result = 255.0;
            value->rgba[channel] = (uint8_t)floor(result + 0.5);
        }
    }
}

static bool motion_at(const svg_anim_track *track, double progress,
                      svg_anim_value *out)
{
    if (track->motion_points == NULL || track->motion_point_count < 2U ||
        !(track->motion_length > 0.0)) return false;
    double wanted = progress * track->motion_length;
    size_t right = 1U;
    while (right + 1U < track->motion_point_count &&
           wanted > track->motion_points[right].distance) right++;
    const svg_anim_motion_point *left_point = &track->motion_points[right - 1U];
    const svg_anim_motion_point *right_point = &track->motion_points[right];
    double segment = right_point->distance - left_point->distance;
    double local = segment > 0.0 ?
        (wanted - left_point->distance) / segment : 0.0;
    if (local < 0.0) local = 0.0;
    if (local > 1.0) local = 1.0;
    *out = (svg_anim_value){.kind = SVG_ANIM_VALUE_MOTION};
    out->number[0] = left_point->x + (right_point->x - left_point->x) * local;
    out->number[1] = left_point->y + (right_point->y - left_point->y) * local;
    if (track->motion_rotate_auto) {
        double angle = atan2(right_point->y - left_point->y,
                             right_point->x - left_point->x) *
                       57.2957795130823208768;
        if (track->motion_rotate_reverse) angle += 180.0;
        out->number[2] = angle;
    } else out->number[2] = track->motion_fixed_rotation;
    out->number[3] = 1.0;
    return isfinite(out->number[0]) && isfinite(out->number[1]) &&
           isfinite(out->number[2]);
}

static bool evaluate_one(const svg_anim_document *document, size_t index,
                         const uint8_t state[SVG_ANIM_MAX_TRACKS],
                         const double starts[SVG_ANIM_MAX_TRACKS], double time,
                         svg_anim_overlay_item *out)
{
    const svg_anim_track *track = &document->tracks[index];
    *out = (svg_anim_overlay_item){.track_index = index};
    if (!track->supported || !isfinite(starts[index]) || time < starts[index]) return true;
    double duration = active_duration(document, index, state, starts);
    double elapsed = time - starts[index];
    bool finished = elapsed >= duration;
    if (finished && track->fill_mode != SVG_ANIM_FILL_FREEZE) return true;
    if (finished) elapsed = duration;
    double progress = 0.0;
    size_t iteration = 0U;
    if (track->kind == SVG_ANIM_KIND_SET || track->duration_indefinite ||
        !(track->duration_seconds > 0.0)) {
        progress = 1.0;
    } else if (finished) {
        double total_repeats = duration / track->duration_seconds;
        double fractional = total_repeats - floor(total_repeats);
        if (fractional > 0.0 && track->fill_mode == SVG_ANIM_FILL_FREEZE) {
            iteration = (size_t)floor(total_repeats);
            progress = fractional;
        } else {
            iteration = total_repeats > 0.0 ?
                        (size_t)floor(total_repeats - 1.0e-12) : 0U;
            progress = 1.0;
        }
    } else {
        iteration = (size_t)floor(elapsed / track->duration_seconds);
        progress = fmod(elapsed, track->duration_seconds) /
                   track->duration_seconds;
    }
    if (track->kind == SVG_ANIM_KIND_MOTION) {
        if (!motion_at(track, progress, &out->value)) return false;
    } else {
        if (!interpolate(track, progress, &out->value)) return false;
        add_accumulation(track, iteration, &out->value);
    }
    if (track->additive_sum && out->value.kind == SVG_ANIM_VALUE_NUMBER &&
        track->base_value.kind == SVG_ANIM_VALUE_NUMBER)
        out->value.number[0] += track->base_value.number[0];
    out->iteration = iteration;
    out->active = true;
    return true;
}

bool svg_anim_eval(const svg_anim_document *document, double time_seconds,
                   svg_anim_overlay *out)
{
    if (out == NULL) return false;
    *out = (svg_anim_overlay){0};
    if (document == NULL || !isfinite(time_seconds) || time_seconds < 0.0 ||
        time_seconds > SVG_ANIM_MAX_TIMELINE_SECONDS ||
        document->track_count > SVG_ANIM_MAX_TRACKS) return false;
    out->time_seconds = time_seconds;
    if (document->track_count == 0U) return true;
    if (document->track_count > SIZE_MAX / sizeof(*out->items)) return false;
    out->items = calloc(document->track_count, sizeof(*out->items));
    if (out->items == NULL) return false;
    out->count = document->track_count;
    uint8_t state[SVG_ANIM_MAX_TRACKS] = {0};
    double starts[SVG_ANIM_MAX_TRACKS] = {0.0};
    for (size_t index = 0U; index < document->track_count; ++index) {
        if (!resolve_start(document, index, state, starts)) {
            svg_anim_overlay_destroy(out);
            return false;
        }
    }
    for (size_t index = 0U; index < document->track_count; ++index) {
        if (!evaluate_one(document, index, state, starts, time_seconds,
                          &out->items[index])) {
            svg_anim_overlay_destroy(out);
            return false;
        }
    }
    return true;
}

void svg_anim_overlay_destroy(svg_anim_overlay *overlay)
{
    if (overlay == NULL) return;
    free(overlay->items);
    *overlay = (svg_anim_overlay){0};
}
