#include "ui_widget_paint_internal.h"

#include "ui_icons.h"

#define UI_POINT_TEXT 13U

bool ui_widget_paint_tree_row(const ui_widget_canvas *canvas, const ui_widget_draw_spec *spec)
{
    uint32_t bg = spec->state == UI_CONTROL_SELECTED ? canvas->tokens.selection_fill :
        spec->state == UI_CONTROL_HOVER || spec->state == UI_CONTROL_PRESSED ?
            canvas->tokens.panel_hover : 0U;
    if (bg != 0U) ui_widget_paint_round(canvas, spec->bounds, 4U, bg, 0U, 0U);
    if (spec->state == UI_CONTROL_SELECTED)
        ui_widget_paint_round(canvas, (ui_rect){spec->bounds.x, spec->bounds.y + 4U, 2U,
            spec->bounds.height > 8U ? spec->bounds.height - 8U : 1U},
            1U, canvas->tokens.accent, 0U, 0U);
    uint32_t guide_x = spec->bounds.x + 12U;
    for (uint32_t depth = 0U; depth < spec->indent && depth < 6U; ++depth) {
        ui_rect guide = {guide_x + depth * 16U, spec->bounds.y, 1U, spec->bounds.height};
        ui_widget_paint_rect(canvas, guide, canvas->tokens.separator);
    }
    uint32_t start = spec->bounds.x + 4U + (uint32_t)spec->indent * 16U;
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
    uint32_t identifier_width = 0U;
    if (identifier != NULL && available > 48U) {
        uint64_t estimated = (uint64_t)identifier_length * 7U + 4U;
        identifier_width = estimated > 64U ? 64U : (uint32_t)estimated;
        if (identifier_width > available / 2U) identifier_width = available / 2U;
    }
    uint32_t primary_width = available > identifier_width + 4U ?
        available - identifier_width - (identifier_width > 0U ? 4U : 0U) : available;
    ui_rect primary_bounds = {start, spec->bounds.y, primary_width, spec->bounds.height};
    uint32_t primary_color = spec->visible ? ui_widget_paint_state_foreground(canvas, spec->state) :
        canvas->tokens.text_secondary;
    if (!ui_widget_paint_text(canvas, primary, primary_length, primary_bounds,
            primary_color, UI_FONT_SYSTEM, UI_FONT_REGULAR, UI_POINT_TEXT)) return false;
    if (identifier_width > 0U) {
        ui_rect id_bounds = {text_right - identifier_width, spec->bounds.y,
            identifier_width, spec->bounds.height};
        uint32_t id_color = spec->visible ? canvas->tokens.text_tertiary :
            canvas->tokens.text_secondary;
        if (!ui_widget_paint_text(canvas, identifier, identifier_length, id_bounds,
                id_color, UI_FONT_SYSTEM, UI_FONT_REGULAR, 12U)) return false;
    }
    if (!spec->show_row_actions) return true;
    uint32_t trail_x = spec->bounds.x + (spec->bounds.width > 34U ? spec->bounds.width - 34U : 0U);
    ui_control_state action_state = spec->state == UI_CONTROL_SELECTED ?
        UI_CONTROL_SELECTED : UI_CONTROL_HOVER;
    if (!ui_widget_paint_icon(canvas, spec->visible ? UI_ICON_EYE : UI_ICON_EYE_OFF,
            (ui_rect){trail_x, spec->bounds.y + 5U, 14U, 14U}, action_state)) return false;
    return ui_widget_paint_icon(canvas, spec->locked ? UI_ICON_LOCK : UI_ICON_UNLOCK,
        (ui_rect){trail_x + 16U, spec->bounds.y + 5U, 14U, 14U}, action_state);
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
