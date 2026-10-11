#include "ui_widget_paint_internal.h"

#include "ui_icons.h"

#include <math.h>
#include <string.h>

#define UI_POINT_TEXT 13U

static size_t utf8_next(const char *text, size_t length, size_t offset)
{
    if (text == NULL || offset >= length) return length;
    uint8_t first = (uint8_t)text[offset];
    size_t step = first < 0x80U ? 1U : first < 0xe0U ? 2U :
        first < 0xf0U ? 3U : 4U;
    return step > length - offset ? length : offset + step;
}

static size_t utf8_count(const char *text, size_t length)
{
    size_t count = 0U;
    for (size_t offset = 0U; offset < length; offset = utf8_next(text, length, offset))
        ++count;
    return count;
}

static size_t utf8_offset_after(const char *text, size_t length, size_t count)
{
    size_t offset = 0U;
    while (offset < length && count > 0U) {
        offset = utf8_next(text, length, offset);
        --count;
    }
    return offset;
}

static size_t middle_ellipsis(const char *text, size_t length,
    size_t maximum_characters, char output[UI_TEXT_MAX_BYTES])
{
    static const char ellipsis[] = "\xe2\x80\xa6";
    size_t characters = utf8_count(text, length);
    if (characters <= maximum_characters && length < UI_TEXT_MAX_BYTES) {
        if (length > 0U) memcpy(output, text, length);
        output[length] = '\0';
        return length;
    }
    if (maximum_characters == 0U) {
        output[0] = '\0';
        return 0U;
    }
    size_t left_characters = maximum_characters / 2U;
    size_t right_characters = maximum_characters - left_characters - 1U;
    size_t left_bytes = utf8_offset_after(text, length, left_characters);
    size_t right_start = utf8_offset_after(text, length, characters - right_characters);
    size_t right_bytes = length - right_start;
    while (left_bytes + sizeof(ellipsis) - 1U >= UI_TEXT_MAX_BYTES &&
           left_characters > 0U) {
        --left_characters;
        left_bytes = utf8_offset_after(text, length, left_characters);
    }
    while (left_bytes + sizeof(ellipsis) - 1U + right_bytes >= UI_TEXT_MAX_BYTES &&
           right_characters > 0U) {
        --right_characters;
        right_start = utf8_offset_after(text, length, characters - right_characters);
        right_bytes = length - right_start;
    }
    if (left_bytes > 0U) memcpy(output, text, left_bytes);
    size_t used = left_bytes;
    memcpy(output + used, ellipsis, sizeof(ellipsis) - 1U);
    used += sizeof(ellipsis) - 1U;
    if (right_bytes > 0U) {
        memcpy(output + used, text + length - right_bytes, right_bytes);
        used += right_bytes;
    }
    output[used] = '\0';
    return used;
}

static size_t trailing_ellipsis(const char *text, size_t length,
    size_t maximum_characters, char output[UI_TEXT_MAX_BYTES])
{
    static const char ellipsis[] = "\xe2\x80\xa6";
    size_t characters = utf8_count(text, length);
    if (characters <= maximum_characters && length < UI_TEXT_MAX_BYTES) {
        if (length > 0U) memcpy(output, text, length);
        output[length] = '\0';
        return length;
    }
    if (maximum_characters == 0U) {
        output[0] = '\0';
        return 0U;
    }
    size_t prefix_characters = maximum_characters - 1U;
    size_t prefix_bytes = utf8_offset_after(text, length, prefix_characters);
    while (prefix_bytes + sizeof(ellipsis) - 1U >= UI_TEXT_MAX_BYTES &&
           prefix_characters > 0U) {
        --prefix_characters;
        prefix_bytes = utf8_offset_after(text, length, prefix_characters);
    }
    if (prefix_bytes > 0U) memcpy(output, text, prefix_bytes);
    memcpy(output + prefix_bytes, ellipsis, sizeof(ellipsis) - 1U);
    size_t used = prefix_bytes + sizeof(ellipsis) - 1U;
    output[used] = '\0';
    return used;
}

bool ui_widget_paint_tree_row(const ui_widget_canvas *canvas, const ui_widget_draw_spec *spec)
{
    uint32_t bg = spec->state == UI_CONTROL_SELECTED ? canvas->tokens.selection_fill :
        spec->state == UI_CONTROL_HOVER || spec->state == UI_CONTROL_PRESSED ?
            canvas->tokens.panel_hover : 0U;
    if (spec->has_transition) {
        double amount = (double)spec->transition_progress_permille / 1000.0;
        bg = 0U;
        for (unsigned shift = 0U; shift < 32U; shift += 8U) {
            uint32_t left = (spec->transition_from_color >> shift) & 255U;
            uint32_t right = (spec->transition_to_color >> shift) & 255U;
            uint32_t channel = (uint32_t)llround((double)left * (1.0 - amount) +
                (double)right * amount);
            bg |= channel << shift;
        }
    }
    if (bg != 0U) ui_widget_paint_round(canvas, spec->bounds, 4U, bg, 0U, 0U);
    double selected_amount = spec->state == UI_CONTROL_SELECTED ? 1.0 : 0.0;
    if (spec->has_transition) {
        double progress = (double)spec->transition_progress_permille / 1000.0;
        selected_amount = spec->transition_from == UI_CONTROL_SELECTED ? 1.0 - progress :
            spec->transition_to == UI_CONTROL_SELECTED ? progress : 0.0;
    }
    if (selected_amount > 0.0)
        ui_widget_paint_round(canvas, (ui_rect){spec->bounds.x, spec->bounds.y + 4U, 2U,
            spec->bounds.height > 8U ? spec->bounds.height - 8U : 1U},
            1U, (canvas->tokens.accent & UINT32_C(0xffffff00)) |
                (uint32_t)llround(selected_amount * 255.0), 0U, 0U);
    uint32_t guide_x = spec->bounds.x + 12U;
    for (uint32_t depth = 0U; depth < spec->indent && depth < 6U; ++depth) {
        ui_rect guide = {guide_x + depth * UI_TREE_ROW_INDENT_POINTS,
            spec->bounds.y, 1U, spec->bounds.height};
        ui_widget_paint_rect(canvas, guide, canvas->tokens.separator);
    }
    uint32_t start = spec->bounds.x + 4U +
        (uint32_t)spec->indent * UI_TREE_ROW_INDENT_POINTS;
    if (spec->has_children) {
        ui_icon_kind disclosure = spec->expanded ? UI_ICON_CHEVRON_DOWN : UI_ICON_CHEVRON_RIGHT;
        bool drawn = spec->visible ? ui_widget_paint_icon(canvas, disclosure,
                (ui_rect){start, spec->bounds.y + 5U, 16U, 16U}, spec->state) :
            ui_widget_paint_icon_color(canvas, disclosure,
                (ui_rect){start, spec->bounds.y + 5U, 16U, 16U},
                canvas->tokens.text_secondary);
        if (!drawn) return false;
    }
    start += 18U;
    if (spec->icon != UI_ICON_NONE) {
        bool drawn = spec->visible ? ui_widget_paint_icon(canvas, spec->icon,
                (ui_rect){start, spec->bounds.y + 5U, 16U, 16U}, spec->state) :
            ui_widget_paint_icon_color(canvas, spec->icon,
                (ui_rect){start, spec->bounds.y + 5U, 16U, 16U},
                canvas->tokens.text_secondary);
        if (!drawn) return false;
    }
    start += 20U;
    uint32_t trailing_width = spec->show_row_actions ? 40U : 8U;
    uint32_t right = spec->bounds.x + spec->bounds.width;
    uint32_t text_right = right > trailing_width ? right - trailing_width : start;
    const char *primary = spec->label;
    size_t primary_length = spec->label_length;
    const char *identifier = NULL;
    size_t identifier_length = 0U;
    for (size_t index = 0U; index + 1U < spec->label_length; ++index) {
        if (spec->label[index] != ' ' || spec->label[index + 1U] != '#') continue;
        primary_length = index;
        identifier = spec->label + index + 1U;
        identifier_length = spec->label_length - index - 1U;
        break;
    }
    uint32_t available = text_right > start ? text_right - start : 0U;
    size_t primary_characters = utf8_count(primary, primary_length);
    uint64_t primary_natural_width = (uint64_t)primary_characters * 7U;
    uint32_t identifier_width = 0U;
    if (identifier != NULL && primary_natural_width + 32U < available) {
        uint32_t remaining = available - (uint32_t)primary_natural_width - 8U;
        identifier_width = remaining > 64U ? 64U : remaining;
        if (identifier_width < 21U) identifier_width = 0U;
    }
    uint32_t primary_width = identifier_width > 0U ?
        available - identifier_width - 4U : available;
    ui_rect primary_bounds = {start, spec->bounds.y, primary_width, spec->bounds.height};
    uint32_t primary_color = spec->visible ? ui_widget_paint_state_foreground(canvas, spec->state) :
        canvas->tokens.text_secondary;
    char primary_display[UI_TEXT_MAX_BYTES] = {0};
    size_t primary_display_length = middle_ellipsis(primary, primary_length,
        primary_width / 7U, primary_display);
    if (!ui_widget_paint_text(canvas, primary_display, primary_display_length, primary_bounds,
            primary_color, UI_FONT_SYSTEM, UI_FONT_REGULAR, UI_POINT_TEXT)) return false;
    if (identifier_width > 0U) {
        ui_rect id_bounds = {text_right - identifier_width, spec->bounds.y,
            identifier_width, spec->bounds.height};
        uint32_t id_color = spec->visible ? canvas->tokens.text_tertiary :
            canvas->tokens.text_secondary;
        char identifier_display[UI_TEXT_MAX_BYTES] = {0};
        size_t identifier_display_length = trailing_ellipsis(identifier, identifier_length,
            identifier_width / 7U, identifier_display);
        if (!ui_widget_paint_text(canvas, identifier_display, identifier_display_length, id_bounds,
                id_color, UI_FONT_SYSTEM, UI_FONT_REGULAR, 12U)) return false;
    }
    if (!spec->show_row_actions) return true;
    uint32_t trail_x = spec->bounds.x + (spec->bounds.width > 34U ? spec->bounds.width - 34U : 0U);
    ui_control_state action_state = spec->state == UI_CONTROL_SELECTED ?
        UI_CONTROL_SELECTED : UI_CONTROL_HOVER;
    if (!ui_widget_paint_icon(canvas, spec->visible ? UI_ICON_EYE : UI_ICON_EYE_OFF,
            (ui_rect){trail_x, spec->bounds.y + 4U, 16U, 16U}, action_state)) return false;
    return ui_widget_paint_icon(canvas, spec->locked ? UI_ICON_LOCK : UI_ICON_UNLOCK,
        (ui_rect){trail_x + 16U, spec->bounds.y + 4U, 16U, 16U}, action_state);
}

bool ui_widget_paint_empty_state(const ui_widget_canvas *canvas,
    const ui_widget_draw_spec *spec)
{
    bool roomy = spec->bounds.width >= 240U && spec->bounds.height >= 112U;
    uint32_t icon_size = roomy ? 40U : 24U;
    uint32_t block_height = roomy ? 112U : 76U;
    uint32_t block_top = spec->bounds.height > block_height ?
        (spec->bounds.height - block_height) / 2U : 0U;
    uint32_t icon_top = block_top + (roomy ? 8U : 3U);
    uint32_t heading_top = block_top + (roomy ? 54U : 29U);
    uint32_t heading_size = roomy ? ui_theme_type_scale().empty_heading_pt : 16U;
    uint32_t message_top = block_top + (roomy ? 86U : 53U);
    ui_rect icon_rect = {spec->bounds.x + (spec->bounds.width - icon_size) / 2U,
        spec->bounds.y + icon_top, icon_size, icon_size};
    if (!ui_widget_paint_icon(canvas,
            spec->icon == UI_ICON_NONE ? UI_ICON_LAYERS : spec->icon,
            icon_rect, UI_CONTROL_REST)) return false;
    ui_rect heading = {spec->bounds.x + 6U, spec->bounds.y + heading_top,
        spec->bounds.width > 12U ? spec->bounds.width - 12U : 0U,
        heading_size + 6U};
    if (!ui_widget_paint_text(canvas, spec->label, spec->label_length, heading,
            canvas->tokens.text_primary, UI_FONT_SYSTEM, UI_FONT_SEMIBOLD,
            heading_size)) return false;
    ui_rect message = {spec->bounds.x + 8U, spec->bounds.y + message_top,
        spec->bounds.width > 16U ? spec->bounds.width - 16U : 0U,
        roomy ? 24U : 20U};
    return ui_widget_paint_text(canvas, spec->secondary, spec->secondary_length,
        message, canvas->tokens.text_secondary, UI_FONT_SYSTEM, UI_FONT_REGULAR,
        roomy ? 12U : 11U);
}
