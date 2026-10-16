#include "viewer_timeline.h"

#include "platform/macos/platform_accessibility.h"
#include "viewer_editor.h"

#include <stdio.h>
#include <string.h>

#define VIEWER_TIMELINE_A11Y_BASE UINT32_C(0x71000000)
#define VIEWER_TIMELINE_A11Y_TEXT_LIMIT 512U

enum {
    TIMELINE_A11Y_PANEL = VIEWER_TIMELINE_A11Y_BASE + 1U,
    TIMELINE_A11Y_PLAY,
    TIMELINE_A11Y_SLIDER,
    TIMELINE_A11Y_LOOP,
    TIMELINE_A11Y_SPEED,
    TIMELINE_A11Y_FIT,
    TIMELINE_A11Y_COLLAPSE,
    TIMELINE_A11Y_TRACKS,
    TIMELINE_A11Y_ONION,
    TIMELINE_A11Y_KEY_OPACITY,
    TIMELINE_A11Y_KEY_TRANSFORM,
    TIMELINE_A11Y_KEY_FILL,
    TIMELINE_A11Y_KEY_STROKE,
    TIMELINE_A11Y_KEY_VISIBILITY,
    TIMELINE_A11Y_DRAWING_ASSET,
    TIMELINE_A11Y_ADD_CEL,
    TIMELINE_A11Y_EASING_MODE,
    TIMELINE_A11Y_SPLINE_X1,
    TIMELINE_A11Y_SPLINE_Y1,
    TIMELINE_A11Y_SPLINE_X2,
    TIMELINE_A11Y_SPLINE_Y2,
    TIMELINE_A11Y_ROW_BASE = VIEWER_TIMELINE_A11Y_BASE + 32U
};

static ui_tree timeline_accessibility_tree;
static ui_semantics_snapshot editor_semantics_scratch;
static ui_semantics_snapshot timeline_semantics_scratch;

static bool timeline_is_indefinite(const viewer_timeline *timeline)
{
    if (timeline == NULL || timeline->native_project || timeline->document == NULL) return false;
    size_t count = viewer_timeline_track_count(timeline);
    for (size_t index = 0U; index < count; ++index) {
        const svg_anim_track *track = &timeline->document->animation.tracks[index];
        if (track->duration_indefinite || track->repeat_indefinite ||
            track->repeat_duration_indefinite) return true;
    }
    return false;
}

static size_t utf8_prefix(const char *text, size_t length, size_t maximum)
{
    size_t amount = length < maximum ? length : maximum;
    while (amount > 0U && amount < length &&
        (((uint8_t)text[amount] & 0xc0U) == 0x80U)) amount--;
    return amount;
}

static bool snapshot_append(ui_semantics_snapshot *destination,
    const ui_semantics_snapshot *source, const ui_semantics_node *node,
    uint32_t id, uint32_t parent_id)
{
    if (destination == NULL || source == NULL || node == NULL ||
        destination->count >= UI_MAX_NODES) return false;
    const char *label = NULL, *value = NULL;
    size_t label_length = 0U, value_length = 0U;
    if (!ui_semantics_text(source, node, false, &label, &label_length) ||
        !ui_semantics_text(source, node, true, &value, &value_length)) return false;
    label_length = utf8_prefix(label, label_length, VIEWER_TIMELINE_A11Y_TEXT_LIMIT);
    value_length = utf8_prefix(value, value_length, VIEWER_TIMELINE_A11Y_TEXT_LIMIT);
    if (label_length > UI_TEXT_TOTAL_MAX_BYTES - destination->text_bytes_used ||
        value_length > UI_TEXT_TOTAL_MAX_BYTES - destination->text_bytes_used - label_length)
        return false;
    ui_semantics_node *copy = &destination->nodes[destination->count];
    *copy = *node;
    copy->id = id;
    copy->parent_id = parent_id;
    copy->label_full_len = label_length;
    copy->value_full_len = value_length;
    copy->label_len = utf8_prefix(node->label, node->label_len, label_length);
    copy->value_len = utf8_prefix(node->value, node->value_len, value_length);
    copy->label_offset = destination->text_bytes_used;
    if (label_length > 0U)
        memcpy(destination->text_storage + copy->label_offset, label, label_length);
    copy->value_offset = copy->label_offset + label_length;
    if (value_length > 0U)
        memcpy(destination->text_storage + copy->value_offset, value, value_length);
    destination->text_bytes_used += label_length + value_length;
    destination->count++;
    return true;
}

static bool add_timeline_node(ui_tree *tree, uint32_t id, uint32_t parent,
    ui_role role, const char *label, const char *value, uint32_t state,
    ui_rect bounds)
{
    size_t label_length = label == NULL ? 0U : strlen(label);
    size_t value_length = value == NULL ? 0U : strlen(value);
    ui_node_spec spec = {
        .id = id,
        .parent_id = parent,
        .axis = UI_AXIS_VERTICAL,
        .role = role,
        .label = label,
        .label_len = label_length,
        .value = value,
        .value_len = value_length,
        .state = state
    };
    if (!ui_tree_add(tree, &spec)) return false;
    tree->nodes[tree->count - 1U].bounds = bounds;
    return true;
}

static const char *track_diagnostic(const viewer_timeline *timeline,
                                    const svg_anim_track *track)
{
    if (timeline == NULL || timeline->document == NULL || track == NULL) return NULL;
    const svg_anim_document *animation = &timeline->document->animation;
    for (size_t index = 0U; index < animation->diagnostic_count &&
            index < SVG_ANIM_MAX_DIAGNOSTICS; ++index) {
        const svg_anim_diagnostic *diagnostic = &animation->diagnostics[index];
        if (diagnostic->offset >= track->element_offset &&
            diagnostic->offset - track->element_offset <= track->element_length)
            return svg_anim_diagnostic_name(diagnostic->kind);
    }
    return NULL;
}

static const char *animation_mode_label(svg_anim_calc_mode mode)
{
    if (mode == SVG_ANIM_CALC_SPLINE) return "Spline";
    if (mode == SVG_ANIM_CALC_DISCRETE) return "Discrete";
    if (mode == SVG_ANIM_CALC_PACED) return "Paced";
    return "Linear";
}

static bool add_easing_semantics(ui_tree *tree, viewer_timeline *timeline,
    const viewer_timeline_layout *layout)
{
    if (timeline->native_project) return true;
    viewer_timeline_easing_layout easing;
    if (!viewer_timeline_easing_layout_compute(layout, &easing)) return true;
    if (!easing.visible) return true;
    if (timeline->selected_track >= viewer_timeline_track_count(timeline)) return false;
    viewer_timeline_easing_sync(timeline);
    svg_anim_calc_mode mode = timeline->easing_mode_override ?
        timeline->easing_mode :
        timeline->document->animation.tracks[timeline->selected_track].calc_mode;
    uint32_t disabled = viewer_timeline_easing_enabled(timeline) ?
        0U : UI_STATE_DISABLED;
    if (!add_timeline_node(tree, TIMELINE_A11Y_EASING_MODE,
            TIMELINE_A11Y_PANEL, UI_ROLE_BUTTON,
            "Interpolation mode", animation_mode_label(mode), disabled,
            easing.mode_buttons[2])) return false;
    static const uint32_t ids[4] = {
        TIMELINE_A11Y_SPLINE_X1, TIMELINE_A11Y_SPLINE_Y1,
        TIMELINE_A11Y_SPLINE_X2, TIMELINE_A11Y_SPLINE_Y2
    };
    static const char *labels[4] = {
        "Spline x1", "Spline y1", "Spline x2", "Spline y2"
    };
    char number[4][40];
    for (size_t index = 0U; index < 4U; ++index) {
        int amount = snprintf(number[index], sizeof(number[index]), "%.17g",
            timeline->easing_fields[index].value);
        if (amount <= 0 || (size_t)amount >= sizeof(number[index]) ||
            !add_timeline_node(tree, ids[index], TIMELINE_A11Y_PANEL,
                UI_ROLE_SLIDER, labels[index], number[index], disabled,
                easing.fields[index])) return false;
    }
    return true;
}

static bool build_timeline_tree(viewer_timeline *timeline,
    const viewer_timeline_layout *layout, ui_theme theme,
    bool increased_contrast, size_t row_limit)
{
    if (!ui_tree_init_accessible(&timeline_accessibility_tree, theme,
            increased_contrast)) return false;
    if (!add_timeline_node(&timeline_accessibility_tree, TIMELINE_A11Y_PANEL,
            UI_ROOT_ID, UI_ROLE_GROUP, "Animation timeline", NULL, 0U,
            layout->panel) ||
        !add_timeline_node(&timeline_accessibility_tree, TIMELINE_A11Y_COLLAPSE,
            TIMELINE_A11Y_PANEL, UI_ROLE_BUTTON,
            layout->collapsed ? (timeline->dock_open ? "Close timeline" :
                "Expand timeline") : "Collapse timeline", NULL,
            0U, layout->collapse_toggle)) return false;
    if (!layout->collapsed) {
        char time_value[64];
        double duration = viewer_timeline_duration(timeline);
        bool indefinite = timeline_is_indefinite(timeline);
        uint32_t current_tick = 0U;
        if (timeline->native_project)
            (void)viewer_timeline_seconds_to_native_tick(timeline,
                timeline->time_seconds, &current_tick);
        int time_amount = indefinite ? snprintf(time_value, sizeof(time_value),
            "%.2fs; indefinite source; preview 0.00 to %.2fs",
            timeline->time_seconds, duration) :
            timeline->native_project ? snprintf(time_value, sizeof(time_value),
                "%.2f / %.2fs · tick %u/%u at %u Hz",
                timeline->time_seconds, duration,
                current_tick,
                timeline->native_duration_ticks, timeline->native_ticks_per_second) :
                snprintf(time_value, sizeof(time_value), "%.2f / %.2fs",
                    timeline->time_seconds, duration);
        if (time_amount < 0 || (size_t)time_amount >= sizeof(time_value)) return false;
        const char *mode = timeline->mode == VIEWER_TIMELINE_MODE_LOOP ? "Loop" :
            timeline->mode == VIEWER_TIMELINE_MODE_PING_PONG ? "Ping-Pong" : "One Shot";
        char speed[16];
        int speed_amount = snprintf(speed, sizeof(speed), "%.2g×", timeline->playback_speed);
        if (speed_amount < 0 || (size_t)speed_amount >= sizeof(speed)) return false;
        char onion[64];
        char drawing_asset_value[PROJECT_BUNDLE_MAX_ASSET_NAME_BYTES + 1U] = {0};
        project_bundle_asset drawing_asset = {0};
        if (layout->drawing_asset_button.width > 0U &&
            viewer_timeline_native_selected_asset(timeline, &drawing_asset)) {
            memcpy(drawing_asset_value, drawing_asset.name.ptr,
                drawing_asset.name.len);
        }
        int onion_amount = timeline->native_project ? snprintf(onion, sizeof(onion),
            "%u before, %u after%s", timeline->native_onion_before,
            timeline->native_onion_after,
            timeline->native_onion_limited ? "; onion preview limited" : "") : 0;
        if (timeline->native_project &&
            (onion_amount < 0 || (size_t)onion_amount >= sizeof(onion))) return false;
        if (!add_timeline_node(&timeline_accessibility_tree, TIMELINE_A11Y_PLAY,
                TIMELINE_A11Y_PANEL, UI_ROLE_BUTTON,
                timeline->playing ? "Pause animation" : "Play animation", NULL, 0U,
                layout->play_button) ||
            !add_timeline_node(&timeline_accessibility_tree, TIMELINE_A11Y_SLIDER,
                TIMELINE_A11Y_PANEL, UI_ROLE_SLIDER, "Timeline position",
                time_value, 0U, layout->scrubber) ||
            !add_timeline_node(&timeline_accessibility_tree, TIMELINE_A11Y_LOOP,
                TIMELINE_A11Y_PANEL, UI_ROLE_BUTTON, "Preview mode", mode,
                0U, layout->mode_button) ||
            !add_timeline_node(&timeline_accessibility_tree, TIMELINE_A11Y_SPEED,
                TIMELINE_A11Y_PANEL, UI_ROLE_BUTTON, "Preview speed", speed,
                0U, layout->speed_button) ||
            !add_timeline_node(&timeline_accessibility_tree, TIMELINE_A11Y_FIT,
                TIMELINE_A11Y_PANEL, UI_ROLE_BUTTON, "Fit timeline to work area",
                NULL, 0U, layout->fit_button) ||
            (timeline->native_project && layout->onion_button.width > 0U &&
                !add_timeline_node(&timeline_accessibility_tree, TIMELINE_A11Y_ONION,
                    TIMELINE_A11Y_PANEL, UI_ROLE_BUTTON, "Onion skin preview",
                    onion, 0U, layout->onion_button)) ||
            (timeline->native_project && layout->key_opacity_button.width > 0U &&
                !add_timeline_node(&timeline_accessibility_tree,
                    TIMELINE_A11Y_KEY_OPACITY, TIMELINE_A11Y_PANEL,
                    UI_ROLE_BUTTON, "Key selected object opacity at playhead",
                    NULL, 0U, layout->key_opacity_button)) ||
            (timeline->native_project && layout->key_transform_button.width > 0U &&
                !add_timeline_node(&timeline_accessibility_tree,
                    TIMELINE_A11Y_KEY_TRANSFORM, TIMELINE_A11Y_PANEL,
                    UI_ROLE_BUTTON, "Key selected object transform at playhead",
                    NULL, 0U, layout->key_transform_button)) ||
            (timeline->native_project && layout->key_fill_button.width > 0U &&
                !add_timeline_node(&timeline_accessibility_tree,
                    TIMELINE_A11Y_KEY_FILL, TIMELINE_A11Y_PANEL,
                    UI_ROLE_BUTTON, "Key selected object fill color at playhead",
                    NULL, 0U, layout->key_fill_button)) ||
            (timeline->native_project && layout->key_stroke_button.width > 0U &&
                !add_timeline_node(&timeline_accessibility_tree,
                    TIMELINE_A11Y_KEY_STROKE, TIMELINE_A11Y_PANEL,
                    UI_ROLE_BUTTON, "Key selected object stroke color at playhead",
                    NULL, 0U, layout->key_stroke_button)) ||
            (timeline->native_project && layout->key_visibility_button.width > 0U &&
                !add_timeline_node(&timeline_accessibility_tree,
                    TIMELINE_A11Y_KEY_VISIBILITY, TIMELINE_A11Y_PANEL,
                    UI_ROLE_BUTTON, "Key selected object visibility at playhead",
                    NULL, 0U, layout->key_visibility_button)) ||
            (timeline->native_project && layout->drawing_asset_button.width > 0U &&
                !add_timeline_node(&timeline_accessibility_tree,
                    TIMELINE_A11Y_DRAWING_ASSET, TIMELINE_A11Y_PANEL,
                    UI_ROLE_BUTTON, "Choose drawing asset",
                    drawing_asset_value, 0U, layout->drawing_asset_button)) ||
            (timeline->native_project && layout->add_cel_button.width > 0U &&
                !add_timeline_node(&timeline_accessibility_tree,
                    TIMELINE_A11Y_ADD_CEL, TIMELINE_A11Y_PANEL,
                    UI_ROLE_BUTTON,
                    viewer_timeline_selected_drawing_track(timeline) ?
                        "Add drawing cel at playhead" :
                        "Create drawing track for selected SVG object",
                    NULL, 0U, layout->add_cel_button)) ||
            !add_timeline_node(&timeline_accessibility_tree, TIMELINE_A11Y_TRACKS,
                TIMELINE_A11Y_PANEL, UI_ROLE_LIST, "Animation tracks", NULL, 0U,
                layout->panel)) return false;
        if (!add_easing_semantics(&timeline_accessibility_tree, timeline, layout))
            return false;
    }
    if (row_limit > layout->visible_track_count) row_limit = layout->visible_track_count;
    for (size_t row = 0U; row < row_limit; ++row) {
        if (layout->collapsed) break;
        size_t track_index = layout->first_track + row;
        char label[128] = {0};
        size_t label_length = 0U;
        if (!viewer_timeline_track_label(timeline, track_index, label,
                sizeof(label), &label_length)) return false;
        uint32_t state = track_index == timeline->selected_track ? UI_STATE_CHECKED : 0U;
        char description[192] = {0};
        const char *value = NULL;
        if (timeline->native_project) {
            project_timeline_track_view track;
            if (!viewer_timeline_native_track_at(timeline, track_index, &track))
                return false;
            int amount = snprintf(description, sizeof(description),
                "%s track · %zu cels · start ticks:",
                track.visible ? "Visible" : "Hidden", track.item_count);
            if (amount < 0 || (size_t)amount >= sizeof(description)) return false;
            size_t used = (size_t)amount;
            size_t marker_count = track.item_count < 8U ? track.item_count : 8U;
            for (size_t marker = 0U; marker < marker_count; ++marker) {
                project_timeline_item_view item;
                if (!viewer_timeline_native_item_at(timeline, track_index,
                        marker, &item)) return false;
                amount = snprintf(description + used, sizeof(description) - used,
                    "%s%u", marker == 0U ? " " : ", ", item.tick);
                if (amount < 0 || (size_t)amount >= sizeof(description) - used)
                    return false;
                used += (size_t)amount;
            }
            if (track.item_count > marker_count) {
                amount = snprintf(description + used, sizeof(description) - used,
                    " · %zu additional", track.item_count - marker_count);
                if (amount < 0 || (size_t)amount >= sizeof(description) - used)
                    return false;
            }
            value = description;
        } else {
            const svg_anim_track *track = &timeline->document->animation.tracks[track_index];
            if (!track->supported) {
                state |= UI_STATE_DISABLED;
                const char *diagnostic = track_diagnostic(timeline, track);
                int amount = snprintf(description, sizeof(description),
                    "Unsupported animation track%s%s · preview only",
                    diagnostic == NULL ? "" : ": ",
                    diagnostic == NULL ? "" : diagnostic);
                if (amount < 0 || (size_t)amount >= sizeof(description)) return false;
                value = description;
            }
        }
        if (!add_timeline_node(&timeline_accessibility_tree,
                TIMELINE_A11Y_ROW_BASE + (uint32_t)row, TIMELINE_A11Y_TRACKS,
                UI_ROLE_ROW, label, value, state, layout->track_rows[row])) return false;
    }
    return ui_semantics_snapshot_build(&timeline_accessibility_tree,
        &timeline_semantics_scratch);
}

bool viewer_timeline_accessibility_snapshot(viewer_timeline *timeline,
    const viewer_timeline_layout *layout, ui_theme theme,
    bool increased_contrast, ui_semantics_snapshot *out_snapshot)
{
    if (timeline == NULL || layout == NULL || out_snapshot == NULL ||
        (viewer_timeline_track_count(timeline) == 0U &&
            (!timeline->native_project || timeline->native_session == NULL)) ||
        (theme != UI_THEME_DARK && theme != UI_THEME_LIGHT)) return false;
    if (!build_timeline_tree(timeline, layout, theme, increased_contrast,
            layout->visible_track_count)) return false;
    *out_snapshot = timeline_semantics_scratch;
    return true;
}

bool viewer_timeline_publish_accessibility(viewer_timeline *timeline,
    viewer_editor *editor, const viewer_timeline_layout *layout,
    uint32_t width, uint32_t height, double backing_scale, ui_theme theme,
    bool increased_contrast)
{
    if (!viewer_editor_semantics(editor, &editor_semantics_scratch)) return false;
    size_t editor_count = editor_semantics_scratch.count;
    if (editor_count == 0U || editor_count > UI_MAX_NODES) return false;
    size_t extras = UI_MAX_NODES - editor_count;
    size_t fixed_nodes = layout->collapsed ? 2U : 8U;
    if (!layout->collapsed && timeline->native_project &&
        layout->onion_button.width > 0U) fixed_nodes++;
    if (!layout->collapsed && timeline->native_project &&
        layout->key_opacity_button.width > 0U) fixed_nodes++;
    if (!layout->collapsed && timeline->native_project &&
        layout->key_transform_button.width > 0U) fixed_nodes++;
    if (!layout->collapsed && timeline->native_project &&
        layout->key_fill_button.width > 0U) fixed_nodes++;
    if (!layout->collapsed && timeline->native_project &&
        layout->key_stroke_button.width > 0U) fixed_nodes++;
    if (!layout->collapsed && timeline->native_project &&
        layout->key_visibility_button.width > 0U) fixed_nodes++;
    if (!layout->collapsed && timeline->native_project &&
        layout->drawing_asset_button.width > 0U) fixed_nodes++;
    if (!layout->collapsed && timeline->native_project &&
        layout->add_cel_button.width > 0U) fixed_nodes++;
    if (!layout->collapsed) {
        viewer_timeline_easing_layout easing;
        if (viewer_timeline_easing_layout_compute(layout, &easing) && easing.visible)
            fixed_nodes += 5U;
    }
    bool include_editor = extras >= fixed_nodes;
    size_t row_limit = 0U;
    if (include_editor && extras > fixed_nodes) {
        row_limit = extras - fixed_nodes;
        if (row_limit > layout->visible_track_count)
            row_limit = layout->visible_track_count;
    }
    if (!build_timeline_tree(timeline, layout, theme, increased_contrast,
            row_limit)) return false;
    uint32_t root_id = UI_ROOT_ID;
    if (include_editor) {
        root_id = editor_semantics_scratch.nodes[0].id;
    } else editor_semantics_scratch = (ui_semantics_snapshot){0};
    for (size_t index = 1U; index < timeline_semantics_scratch.count; ++index) {
        const ui_semantics_node *node = &timeline_semantics_scratch.nodes[index];
        if (node->id < UI_ROOT_ID || node->parent_id < UI_ROOT_ID) return false;
        uint32_t id = VIEWER_TIMELINE_A11Y_BASE + (node->id - UI_ROOT_ID);
        uint32_t parent = node->parent_id == UI_ROOT_ID ? root_id :
            VIEWER_TIMELINE_A11Y_BASE + (node->parent_id - UI_ROOT_ID);
        if (!snapshot_append(&editor_semantics_scratch,
                &timeline_semantics_scratch, node, id, parent)) return false;
    }
    return platform_accessibility_update(&editor_semantics_scratch, width, height,
        backing_scale);
}
