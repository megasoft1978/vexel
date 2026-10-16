#include "viewer_timeline.h"

#include "viewer_chrome.h"

#include <math.h>
#include <stdio.h>
#include <string.h>

bool viewer_timeline_reset_native(viewer_timeline *timeline,
    const svg_shape_document *document, project_timeline_session *session,
    const project_bundle_view *bundle)
{
    if (timeline == NULL || session == NULL || bundle == NULL) return false;
    bool had_explicit_dock_state = timeline->dock_state_explicit;
    bool previous_dock_open = timeline->dock_open;
    viewer_timeline_reset(timeline, document);
    timeline->dock_state_explicit = had_explicit_dock_state;
    timeline->dock_open = previous_dock_open;
    timeline->native_project = true;
    timeline->native_context_valid = true;
    timeline->native_session = session;
    timeline->native_bundle = bundle;
    timeline->duration_seconds = 0.0;
    if (!viewer_timeline_refresh_native(timeline)) {
        timeline->native_project = false;
        timeline->native_session = NULL;
        timeline->native_bundle = NULL;
        timeline->native_view = (project_timeline_view){0};
        timeline->duration_seconds = 0.0;
        return false;
    }
    timeline->dock_open = had_explicit_dock_state ? previous_dock_open : true;
    return true;
}

bool viewer_timeline_reset_for_document(viewer_timeline *timeline,
    const svg_shape_document *document, bool native_project,
    project_timeline_session *session, const project_bundle_view *bundle)
{
    if (native_project && session != NULL && bundle != NULL)
        return viewer_timeline_reset_native(timeline, document, session, bundle);
    viewer_timeline_reset(timeline, document);
    return timeline != NULL;
}

bool viewer_timeline_native_preview_is_current(const viewer_timeline *timeline)
{
    uint32_t tick = 0U;
    return timeline != NULL && timeline->native_project &&
        timeline->native_context_valid && timeline->preview_valid &&
        timeline->preview.rgba != NULL &&
        timeline->preview_time_seconds == timeline->time_seconds &&
        timeline->native_onion_key_valid &&
        timeline->native_onion_before_key == timeline->native_view.onion_before &&
        timeline->native_onion_after_key == timeline->native_view.onion_after &&
        timeline->native_onion_opacity_key == (uint64_t)llround(
            timeline->native_view.onion_opacity * 1000000000.0) &&
        timeline->native_preview_playing_key == timeline->playing &&
        viewer_timeline_seconds_to_native_tick(timeline,
            timeline->time_seconds, &tick) && timeline->native_onion_tick == tick;
}

static uint64_t onion_context_key(const base_bytes source)
{
    uint64_t hash = UINT64_C(1469598103934665603);
    if (base_bytes_valid(source))
        for (size_t index = 0U; index < source.len; ++index) {
            hash ^= source.ptr[index];
            hash *= UINT64_C(1099511628211);
        }
    return hash;
}

static void onion_ghost_tint(raster_tile *ghost, bool before, double opacity)
{
    if (ghost == NULL || ghost->rgba == NULL) return;
    uint8_t tint_r = before ? 255U : 0U;
    uint8_t tint_g = before ? 32U : 224U;
    uint8_t tint_b = before ? 32U : 255U;
    for (uint32_t y = 0U; y < ghost->height; ++y) {
        uint8_t *row = ghost->rgba + (size_t)y * ghost->width * 4U;
        for (uint32_t x = 0U; x < ghost->width; ++x) {
            uint8_t *pixel = row + (size_t)x * 4U;
            double alpha = (double)pixel[3] * opacity;
            if (alpha > 255.0) alpha = 255.0;
            uint8_t a = (uint8_t)lround(alpha);
            pixel[0] = (uint8_t)(((uint32_t)tint_r * a + 127U) / 255U);
            pixel[1] = (uint8_t)(((uint32_t)tint_g * a + 127U) / 255U);
            pixel[2] = (uint8_t)(((uint32_t)tint_b * a + 127U) / 255U);
            pixel[3] = a;
        }
    }
}

static void onion_composite_behind(raster_tile *current,
    const raster_tile *ghost)
{
    if (current == NULL || ghost == NULL || current->rgba == NULL ||
        ghost->rgba == NULL || current->width != ghost->width ||
        current->height != ghost->height) return;
    for (uint32_t y = 0U; y < current->height; ++y) {
        uint8_t *dst = current->rgba + (size_t)y * current->width * 4U;
        const uint8_t *src = ghost->rgba + (size_t)y * ghost->width * 4U;
        for (uint32_t x = 0U; x < current->width; ++x) {
            uint8_t *d = dst + (size_t)x * 4U;
            const uint8_t *s = src + (size_t)x * 4U;
            uint32_t inverse = 255U - d[3];
            for (size_t channel = 0U; channel < 3U; ++channel)
                d[channel] = (uint8_t)(d[channel] +
                    ((uint32_t)s[channel] * inverse + 127U) / 255U);
            d[3] = (uint8_t)(d[3] + ((uint32_t)s[3] * inverse + 127U) / 255U);
        }
    }
}

const raster_tile *viewer_timeline_prepare_native_frame(viewer_timeline *timeline,
    base_bytes current_scene_svg)
{
    if (timeline == NULL || !timeline->native_project ||
        timeline->native_session == NULL || timeline->native_bundle == NULL ||
        !timeline->native_context_valid ||
        !base_bytes_valid(current_scene_svg) || current_scene_svg.len == 0U ||
        viewer_timeline_track_count(timeline) == 0U) {
        if (timeline != NULL && timeline->native_project)
            timeline->preview_failed = true;
        return NULL;
    }
    uint32_t tick = 0U;
    if (!viewer_timeline_seconds_to_native_tick(timeline,
            timeline->time_seconds, &tick)) return NULL;
    uint64_t context_key = onion_context_key(current_scene_svg);
    uint64_t opacity_key = (uint64_t)llround(timeline->native_view.onion_opacity *
        1000000000.0);
    if (timeline->preview_valid && timeline->native_onion_key_valid &&
        timeline->preview_time_seconds == timeline->time_seconds &&
        timeline->native_onion_tick == tick &&
        timeline->native_onion_before_key == timeline->native_view.onion_before &&
        timeline->native_onion_after_key == timeline->native_view.onion_after &&
        timeline->native_onion_opacity_key == (uint64_t)llround(
            timeline->native_view.onion_opacity * 1000000000.0) &&
        timeline->native_onion_opacity_key == opacity_key &&
        timeline->native_onion_context_key == context_key &&
        timeline->native_preview_playing_key == timeline->playing)
        return &timeline->preview;
    raster_tile_destroy(&timeline->preview);
    timeline->preview_valid = false;
    svg_shape_document scene = {0};
    svg_shape_diagnostic diagnostic = {SVG_SHAPE_ERROR_NONE, 0U};
    if (svg_shape_decode(current_scene_svg, &scene, &diagnostic) !=
            SVG_SHAPE_ERROR_NONE) {
        timeline->preview_failed = true;
        return NULL;
    }
    project_animation_render_result rendered = project_animation_render_frame(
        &scene, timeline->native_bundle, &timeline->native_view, tick,
        &timeline->preview);
    if (rendered.error != PROJECT_ANIMATION_RENDER_OK) {
        svg_shape_document_destroy(&scene);
        raster_tile_destroy(&timeline->preview);
        timeline->preview_failed = true;
        return NULL;
    }
    timeline->native_onion_limited = false;
    timeline->native_onion_pixels = 0U;
    if (!timeline->playing && (timeline->native_view.onion_before > 0U ||
            timeline->native_view.onion_after > 0U)) {
        uint64_t frame_pixels = (uint64_t)timeline->preview.width *
            timeline->preview.height;
        uint64_t frame_ticks_numerator =
            (uint64_t)timeline->native_ticks_per_second *
            timeline->native_view.fps_denominator;
        uint64_t frame_ticks_wide = timeline->native_view.fps_numerator == 0U ? 0U :
            frame_ticks_numerator / timeline->native_view.fps_numerator;
        uint32_t frame_ticks = frame_ticks_wide <= UINT32_MAX ?
            (uint32_t)frame_ticks_wide : 0U;
        bool integral = frame_ticks > 0U &&
            frame_ticks_numerator % timeline->native_view.fps_numerator == 0U;
        raster_tile ghost = {0};
        uint32_t sample_count = timeline->native_view.onion_before >
            timeline->native_view.onion_after ? timeline->native_view.onion_before :
                timeline->native_view.onion_after;
        for (uint32_t distance = 1U; integral && distance <= sample_count; ++distance) {
            for (unsigned side = 0U; side < 2U; ++side) {
                bool before = side == 0U;
                uint32_t count = before ? timeline->native_view.onion_before :
                    timeline->native_view.onion_after;
                if (distance > count) continue;
                uint64_t delta = (uint64_t)distance * frame_ticks;
                if ((before && delta > tick) || (!before &&
                        ((uint64_t)tick + delta >= timeline->native_duration_ticks)))
                    continue;
                if (frame_pixels > VIEWER_TIMELINE_ONION_PIXEL_BUDGET -
                        timeline->native_onion_pixels) {
                    timeline->native_onion_limited = true;
                    continue;
                }
                timeline->native_onion_pixels += frame_pixels;
                uint32_t ghost_tick = before ? tick - (uint32_t)delta :
                    tick + (uint32_t)delta;
                project_animation_render_result ghost_result =
                    project_animation_render_frame(&scene, timeline->native_bundle,
                        &timeline->native_view, ghost_tick, &ghost);
                if (ghost_result.error != PROJECT_ANIMATION_RENDER_OK) {
                    timeline->native_onion_limited = true;
                    raster_tile_destroy(&ghost);
                    continue;
                }
                onion_ghost_tint(&ghost, before, timeline->native_view.onion_opacity);
                onion_composite_behind(&timeline->preview, &ghost);
                raster_tile_destroy(&ghost);
            }
        }
        if (!integral) timeline->native_onion_limited = true;
    }
    svg_shape_document_destroy(&scene);
    timeline->preview_time_seconds = timeline->time_seconds;
    timeline->preview_valid = true;
    timeline->preview_failed = false;
    timeline->native_onion_tick = tick;
    timeline->native_onion_before_key = timeline->native_view.onion_before;
    timeline->native_onion_after_key = timeline->native_view.onion_after;
    timeline->native_onion_opacity_key = opacity_key;
    timeline->native_onion_context_key = context_key;
    timeline->native_preview_playing_key = timeline->playing;
    timeline->native_onion_key_valid = true;
    return &timeline->preview;
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

static bool rect_contains(ui_rect rect, double x, double y)
{
    return rect.width > 0U && rect.height > 0U && isfinite(x) && isfinite(y) &&
        x >= (double)rect.x && y >= (double)rect.y &&
        x - (double)rect.x < (double)rect.width &&
        y - (double)rect.y < (double)rect.height;
}


bool viewer_timeline_layout_compute(viewer_timeline *timeline,
                                    uint32_t width, uint32_t height,
                                    double backing_scale,
                                    viewer_timeline_layout *out_layout)
{
    if (timeline == NULL || out_layout == NULL || width == 0U || height == 0U ||
        width > 8192U || height > 8192U || !isfinite(backing_scale) ||
        backing_scale < 0.5 || backing_scale > 4.0 ||
        (viewer_timeline_track_count(timeline) == 0U &&
            (!timeline->native_project || timeline->native_session == NULL)))
        return false;
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
    out_layout->first_track = out_layout->track_count == 0U ? 0U :
        timeline->first_track;
    if (out_layout->track_count > 0U &&
        out_layout->first_track >= out_layout->track_count)
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
    uint32_t onion_width = 0U, key_opacity_width = 0U, key_transform_width = 0U;
    uint32_t key_fill_width = 0U, key_stroke_width = 0U;
    uint32_t key_visibility_width = 0U;
    uint32_t drawing_asset_width = 0U, add_cel_width = 0U;
    if (!scaled_points(6U, backing_scale, &gap) ||
        !scaled_points(10U, backing_scale, &button_x) ||
        !scaled_points(52U, backing_scale, &play_width) ||
        !scaled_points(102U, backing_scale, &time_width) ||
        !scaled_points(96U, backing_scale, &mode_width) ||
        !scaled_points(58U, backing_scale, &speed_width) ||
        !scaled_points(44U, backing_scale, &fit_width) ||
        !scaled_points(62U, backing_scale, &onion_width) ||
        !scaled_points(82U, backing_scale, &key_opacity_width) ||
        !scaled_points(82U, backing_scale, &key_transform_width) ||
        !scaled_points(72U, backing_scale, &key_fill_width) ||
        !scaled_points(78U, backing_scale, &key_stroke_width) ||
        !scaled_points(92U, backing_scale, &key_visibility_width) ||
        !scaled_points(104U, backing_scale, &drawing_asset_width) ||
        !scaled_points(66U, backing_scale, &add_cel_width)) return false;
    uint32_t controls_y = out_layout->panel.y +
        (uint32_t)ceil(26.0 * backing_scale);
    if ((uint64_t)controls_y + controls_height > UINT32_MAX) return false;
    uint32_t available = out_layout->panel.width > button_x * 2U ?
        out_layout->panel.width - button_x * 2U : 0U;
    bool compact = (uint64_t)play_width + time_width + mode_width + speed_width +
        fit_width + onion_width + key_opacity_width + key_transform_width +
        key_fill_width + key_stroke_width + drawing_asset_width + add_cel_width +
        key_visibility_width + gap * 12U > available;
    if (compact) {
        if (!scaled_points(78U, backing_scale, &time_width) ||
            !scaled_points(46U, backing_scale, &mode_width) ||
            !scaled_points(34U, backing_scale, &speed_width) ||
            !scaled_points(32U, backing_scale, &fit_width) ||
            !scaled_points(42U, backing_scale, &play_width) ||
            !scaled_points(48U, backing_scale, &onion_width) ||
            !scaled_points(66U, backing_scale, &key_opacity_width) ||
            !scaled_points(66U, backing_scale, &key_transform_width) ||
            !scaled_points(56U, backing_scale, &key_fill_width) ||
            !scaled_points(62U, backing_scale, &key_stroke_width) ||
            !scaled_points(72U, backing_scale, &key_visibility_width) ||
            !scaled_points(82U, backing_scale, &drawing_asset_width) ||
            !scaled_points(58U, backing_scale, &add_cel_width)) return false;
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
    cursor += fit_width + gap;
    uint32_t control_right = out_layout->panel.width > button_x ?
        out_layout->panel.width - button_x : 0U;
    if (timeline->native_project && cursor <= control_right &&
        onion_width <= control_right - cursor)
        out_layout->onion_button = rect_at(cursor, controls_y, onion_width, controls_height);
    cursor += onion_width + gap;
    if (timeline->native_project && cursor <= control_right &&
        key_opacity_width <= control_right - cursor)
        out_layout->key_opacity_button = rect_at(cursor, controls_y,
            key_opacity_width, controls_height);
    cursor += key_opacity_width + gap;
    if (timeline->native_project && cursor <= control_right &&
        key_transform_width <= control_right - cursor)
        out_layout->key_transform_button = rect_at(cursor, controls_y,
            key_transform_width, controls_height);
    cursor += key_transform_width + gap;
    if (timeline->native_project && cursor <= control_right &&
        key_fill_width <= control_right - cursor)
        out_layout->key_fill_button = rect_at(cursor, controls_y,
            key_fill_width, controls_height);
    cursor += key_fill_width + gap;
    if (timeline->native_project && cursor <= control_right &&
        key_stroke_width <= control_right - cursor)
        out_layout->key_stroke_button = rect_at(cursor, controls_y,
            key_stroke_width, controls_height);
    cursor += key_stroke_width + gap;
    if (timeline->native_project && cursor <= control_right &&
        key_visibility_width <= control_right - cursor)
        out_layout->key_visibility_button = rect_at(cursor, controls_y,
            key_visibility_width, controls_height);
    cursor += key_visibility_width + gap;
    if (timeline->native_project &&
        viewer_timeline_native_svg_asset_count(timeline) > 0U &&
        cursor <= control_right && drawing_asset_width <= control_right - cursor)
        out_layout->drawing_asset_button = rect_at(cursor, controls_y,
            drawing_asset_width, controls_height);
    cursor += drawing_asset_width + gap;
    if (timeline->native_project && cursor <= control_right &&
        add_cel_width <= control_right - cursor)
        out_layout->add_cel_button = rect_at(cursor, controls_y,
            add_cel_width, controls_height);

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

static bool key_near_pointer(const viewer_timeline *timeline,
    const viewer_timeline_layout *layout, size_t track_index, size_t row,
    double x, double y, double *out_time, size_t *out_key)
{
    if (timeline != NULL && timeline->native_project) return false;
    if (timeline == NULL || layout == NULL || out_time == NULL || out_key == NULL ||
        track_index >= viewer_timeline_track_count(timeline)) return false;
    const svg_anim_track *track = &timeline->document->animation.tracks[track_index];
    if (!track->supported || track->keys == NULL || track->key_count == 0U) return false;
    double duration = track->duration_seconds;
    if (!isfinite(duration) || duration <= 0.0) return false;
    double start = viewer_timeline_track_start_time(track);
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

static bool native_item_near_pointer(const viewer_timeline *timeline,
    const viewer_timeline_layout *layout, size_t track_index, size_t row,
    double x, double y, uint32_t *out_tick)
{
    if (timeline == NULL || layout == NULL || out_tick == NULL ||
        !timeline->native_project || timeline->native_ticks_per_second == 0U)
        return false;
    uint32_t radius = viewer_timeline_key_hit_radius(layout->backing_scale);
    double center_y = (double)layout->track_rows[row].y +
        (double)layout->track_rows[row].height / 2.0;
    if (fabs(y - center_y) > (double)radius) return false;
    project_timeline_track_view track;
    if (!viewer_timeline_native_track_at(timeline, track_index, &track)) return false;
    size_t count = track.item_count < 512U ? track.item_count : 512U;
    project_timeline_iterator iterator = {0};
    project_timeline_item_view item;
    double nearest = (double)radius + 1.0;
    bool found = false;
    for (size_t index = 0U; index < count; ++index) {
        if (!project_timeline_item_next(&timeline->native_view, track_index,
                &iterator, &item)) break;
        double seconds = (double)item.tick /
            (double)timeline->native_ticks_per_second;
        if (seconds < timeline->view_start_seconds ||
            seconds > timeline->view_start_seconds +
                viewer_timeline_view_duration(timeline)) continue;
        double span = viewer_timeline_view_duration(timeline);
        if (!(span > 0.0)) continue;
        double marker_x = (double)layout->ruler.x +
            ((seconds - timeline->view_start_seconds) / span) * layout->ruler.width;
        double distance = fabs(x - marker_x);
        if (isfinite(distance) && distance <= (double)radius && distance < nearest) {
            nearest = distance;
            *out_tick = item.tick;
            found = true;
        }
    }
    return found;
}

static void sync_editor_dock(viewer_timeline *timeline, viewer_editor *editor)
{
    if (timeline == NULL || editor == NULL) return;
    (void)viewer_editor_set_timeline_state(editor, true, timeline->dock_open,
        timeline->dock_height_points);
}

bool viewer_timeline_pointer_ex(viewer_timeline *timeline,
    const viewer_timeline_layout *layout, viewer_timeline_pointer_kind kind,
    double x, double y, double delta_x, double delta_y, uint32_t modifiers,
    double now_seconds, viewer_editor *editor)
{
    if (timeline == NULL || layout == NULL ||
        (viewer_timeline_track_count(timeline) == 0U &&
            (!timeline->native_project || timeline->native_session == NULL)) ||
        !isfinite(x) ||
        !isfinite(y) || !isfinite(delta_x) || !isfinite(delta_y)) return false;
    if (kind == VIEWER_TIMELINE_POINTER_UP &&
        (timeline->dragging_scrubber || timeline->dragging_resize)) {
        timeline->dragging_scrubber = false;
        timeline->dragging_resize = false;
        return true;
    }
    if (kind == VIEWER_TIMELINE_POINTER_MOVE && timeline->dragging_scrubber) {
        (void)viewer_timeline_seek(timeline, viewer_timeline_time_from_x(timeline, layout, x));
        timeline->dragging_scrubber = true;
        return true;
    }
    if (kind == VIEWER_TIMELINE_POINTER_MOVE && timeline->native_dragging_item) {
        double seconds = viewer_timeline_time_from_x(timeline, layout, x);
        uint32_t tick = 0U;
        if (viewer_timeline_seconds_to_native_tick(timeline, seconds, &tick)) {
            timeline->native_drag_to_tick = tick;
            timeline->time_seconds = (double)tick /
                (double)timeline->native_ticks_per_second;
            timeline->playing = false;
            timeline->has_tick_time = true;
            viewer_timeline_invalidate_preview(timeline);
        }
        return true;
    }
    if (kind == VIEWER_TIMELINE_POINTER_MOVE && timeline->dragging_key) {
        double next = viewer_timeline_time_from_x(timeline, layout, x);
        if (timeline->dragging_key_track < viewer_timeline_track_count(timeline)) {
            const svg_anim_track *track = &timeline->document->animation.tracks[
                timeline->dragging_key_track];
            size_t key = timeline->dragging_key_index;
            if (track->keys != NULL && key > 0U && key + 1U < track->key_count) {
                double start = viewer_timeline_track_start_time(track);
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
                double anchor = viewer_timeline_time_from_x(timeline, layout, x);
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
        if (timeline->native_project &&
            rect_contains(layout->onion_button, x, y)) {
            uint32_t next = timeline->native_onion_before == 0U ? 1U :
                timeline->native_onion_before == 1U ? 2U :
                timeline->native_onion_before == 2U ? 4U :
                timeline->native_onion_before == 4U ? 8U : 0U;
            project_timeline_edit_result changed =
                project_timeline_session_set_onion(timeline->native_session,
                    next, next, timeline->native_view.onion_opacity);
            if (changed.error == PROJECT_TIMELINE_OK &&
                viewer_timeline_refresh_native(timeline)) {
                viewer_timeline_invalidate_preview(timeline);
                timeline->native_mutation_pending = true;
            }
            return true;
        }
        if (timeline->native_project &&
            rect_contains(layout->key_opacity_button, x, y)) {
            (void)viewer_timeline_key_selected_opacity(timeline, editor);
            return true;
        }
        if (timeline->native_project &&
            rect_contains(layout->key_transform_button, x, y)) {
            (void)viewer_timeline_key_selected_transform(timeline, editor);
            return true;
        }
        if (timeline->native_project &&
            rect_contains(layout->key_fill_button, x, y)) {
            (void)viewer_timeline_key_selected_paint(timeline, editor, "fill");
            return true;
        }
        if (timeline->native_project &&
            rect_contains(layout->key_stroke_button, x, y)) {
            (void)viewer_timeline_key_selected_paint(timeline, editor, "stroke");
            return true;
        }
        if (timeline->native_project &&
            rect_contains(layout->key_visibility_button, x, y)) {
            (void)viewer_timeline_key_selected_visibility(timeline, editor);
            return true;
        }
        if (timeline->native_project &&
            rect_contains(layout->drawing_asset_button, x, y)) {
            (void)viewer_timeline_cycle_native_asset(timeline, editor);
            return true;
        }
        if (timeline->native_project &&
            rect_contains(layout->add_cel_button, x, y)) {
            (void)viewer_timeline_add_native_cel(timeline, editor);
            return true;
        }
        if (rect_contains(layout->ruler, x, y)) {
            timeline->dragging_scrubber = true;
            (void)viewer_timeline_seek(timeline, viewer_timeline_time_from_x(timeline, layout, x));
            timeline->dragging_scrubber = true;
            return true;
        }
        for (size_t row = 0U; row < layout->visible_track_count; ++row) {
            if (!rect_contains(layout->track_rows[row], x, y)) continue;
            size_t index = layout->first_track + row;
            if (timeline->native_project) {
                (void)viewer_timeline_select_track(timeline, editor, index);
                uint32_t native_tick = 0U;
                if (rect_contains(layout->time_grid, x, y) &&
                    native_item_near_pointer(timeline, layout, index, row,
                        x, y, &native_tick)) {
                    timeline->native_dragging_item = true;
                    timeline->native_drag_track = index;
                    timeline->native_drag_from_tick = native_tick;
                    timeline->native_drag_to_tick = native_tick;
                    timeline->playing = false;
                    timeline->time_seconds = (double)native_tick /
                        (double)timeline->native_ticks_per_second;
                    timeline->has_tick_time = true;
                    viewer_timeline_invalidate_preview(timeline);
                }
                return true;
            }
            double key_time = 0.0;
            size_t key_index = 0U;
            bool on_key = rect_contains(layout->time_grid, x, y) &&
                key_near_pointer(timeline, layout, index, row, x, y,
                    &key_time, &key_index);
            if (!viewer_timeline_select_track(timeline, editor, index)) return false;
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
