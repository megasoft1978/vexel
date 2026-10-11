#include "viewer_editor_internal.h"

#include "platform/platform_text_ui.h"
#include "ui/ui_widget_paint_internal.h"
#include "viewer_chrome.h"

#include <math.h>
#include <string.h>

#define EDITOR_REVEAL_MAX_ID_BYTES (64U * 1024U)
#define EDITOR_REVEAL_MAX_WIDTH_POINTS 320U
#define EDITOR_REVEAL_HEIGHT_POINTS 204U
#define EDITOR_REVEAL_VISIBLE_LINES 6U

static bool is_id_name(base_bytes name)
{
    return name.len == 2U && memcmp(name.ptr, "id", 2U) == 0;
}

static bool is_long_tag(base_bytes tag)
{
    return tag.len > UI_TEXT_MAX_BYTES - 2U;
}

static bool is_long_id(document_edit *edit, document_source_span span)
{
    uint8_t preview[32];
    size_t length = 0U;
    return !document_edit_decode_span(edit, span, preview, sizeof(preview), &length) ||
        length > sizeof(preview);
}

bool editor_reveal_prepare(viewer_editor *editor, document_handle handle)
{
    if (editor == NULL || editor->edit == NULL) return false;
    editor->reveal_active = false;
    editor->reveal_exceeds_limit = false;
    editor->reveal_has_id = false;
    editor->reveal_id_length = 0U;
    editor->reveal_tag_length = 0U;
    editor->reveal_scroll_codepoints = 0U;
    base_bytes tag = {0};
    bool tag_ok = document_edit_tag_name(editor->edit, handle, &tag);
    if (tag_ok) {
        editor->reveal_tag_length = editor_copy_slice(editor->reveal_tag,
            sizeof(editor->reveal_tag), tag);
    }
    document_source_span id_span = {0};
    bool has_id = false;
    size_t count = document_edit_attribute_count(editor->edit, handle);
    for (size_t index = 0U; index < count; ++index) {
        document_edit_attribute attribute;
        base_bytes name = {0};
        if (!document_edit_attribute_at(editor->edit, handle, index, &attribute) ||
            !document_edit_span_bytes(editor->edit, attribute.name, &name)) return false;
        if (!is_id_name(name)) continue;
        id_span = attribute.value;
        has_id = true;
        break;
    }
    bool long_id = has_id && is_long_id(editor->edit, id_span);
    if (!long_id && !is_long_tag(tag)) return false;
    editor->reveal_handle = handle;
    if (has_id) {
        editor->reveal_has_id = true;
        if (document_edit_decode_span(editor->edit, id_span,
                (uint8_t *)editor->reveal_id, EDITOR_REVEAL_MAX_ID_BYTES,
                &editor->reveal_id_length)) {
            editor->reveal_id[editor->reveal_id_length] = '\0';
        } else {
            editor->reveal_exceeds_limit = true;
            editor->reveal_id[0] = '\0';
            editor->reveal_id_length = 0U;
        }
    }
    editor->reveal_active = true;
    editor->tree_ready = false;
    return true;
}

static ui_node *find_node(viewer_editor *editor, uint32_t id)
{
    for (size_t index = 0U; index < editor->tree.count; ++index)
        if (editor->tree.nodes[index].spec.id == id) return &editor->tree.nodes[index];
    return NULL;
}

static ui_node *reveal_row_for_handle(viewer_editor *editor, document_handle handle)
{
    for (size_t index = 0U; index < editor->tree.count; ++index) {
        const ui_node *node = &editor->tree.nodes[index];
        if (node->spec.id < EDITOR_OUTLINE_ROW_BASE ||
            node->spec.id >= EDITOR_OUTLINE_ROW_BASE + EDITOR_OUTLINE_VISIBLE) continue;
        size_t row = node->spec.id - EDITOR_OUTLINE_ROW_BASE;
        size_t model_index = 0U;
        if (editor_outline_visible_model_index(editor,
                editor->outline_visible_start + row, &model_index) &&
            editor_same_handle(editor->outline[model_index], handle))
            return (ui_node *)node;
    }
    return NULL;
}

bool editor_reveal_focus(viewer_editor *editor)
{
    if (editor == NULL || editor->focus_id < EDITOR_OUTLINE_ROW_BASE ||
        editor->focus_id >= EDITOR_OUTLINE_ROW_BASE + EDITOR_OUTLINE_VISIBLE)
        return false;
    size_t row = editor->focus_id - EDITOR_OUTLINE_ROW_BASE;
    size_t model_index = 0U;
    if (!editor_outline_visible_model_index(editor,
            editor->outline_visible_start + row, &model_index)) return false;
    bool ready = editor_reveal_prepare(editor, editor->outline[model_index]);
    if (ready) editor->hovered_id = editor->focus_id;
    return ready;
}

void editor_reveal_layout_nodes(viewer_editor *editor)
{
    if (editor == NULL || !editor->reveal_active) return;
    viewer_chrome_layout layout;
    ui_node *row = reveal_row_for_handle(editor, editor->reveal_handle);
    ui_node *body = find_node(editor, EDITOR_NODE_ID_REVEAL);
    if (row == NULL || body == NULL ||
        !viewer_editor_get_chrome_layout(editor, &layout) || !layout.structure_visible) {
        editor->reveal_active = false;
        return;
    }
    double scale = editor->backing_scale *
        ((double)viewer_editor_ui_scale_percent(editor) / 100.0);
    if (!isfinite(scale) || scale <= 0.0) { editor->reveal_active = false; return; }
    uint32_t right = layout.inspector_visible ? layout.inspector.x : editor->width;
    uint32_t available = right > layout.structure.x + layout.structure.width ?
        right - (layout.structure.x + layout.structure.width) : 0U;
    uint32_t width = (uint32_t)ceil((double)EDITOR_REVEAL_MAX_WIDTH_POINTS * scale);
    if (width > available) width = available;
    uint32_t minimum = (uint32_t)ceil(160.0 * scale);
    if (width < minimum) width = available < minimum ? available : minimum;
    uint32_t height = (uint32_t)ceil((double)EDITOR_REVEAL_HEIGHT_POINTS * scale);
    uint32_t x = layout.structure.x + layout.structure.width;
    uint32_t bottom = layout.workspace.y + layout.workspace.height;
    uint32_t y = row->bounds.y;
    if (y > bottom || height > bottom - y) {
        y = row->bounds.y > height ? row->bounds.y - height : layout.workspace.y;
    }
    if (y < layout.workspace.y) y = layout.workspace.y;
    body->bounds = (ui_rect){x, y, width, height};
    ui_node *tag = find_node(editor, EDITOR_NODE_REVEAL_TAG);
    if (tag != NULL) tag->bounds = (ui_rect){x, y, width,
        (uint32_t)ceil(44.0 * scale)};
    ui_node *copy = find_node(editor, EDITOR_BUTTON_COPY_FULL_ID);
    if (copy != NULL) {
        uint32_t button_width = (uint32_t)ceil(104.0 * scale);
        uint32_t button_height = (uint32_t)ceil(28.0 * scale);
        uint32_t padding = (uint32_t)ceil(12.0 * scale);
        copy->bounds = (ui_rect){x + (width > button_width + padding ?
            width - button_width - padding : 0U),
            y + (height > button_height + padding ? height - button_height - padding : 0U),
            button_width < width ? button_width : width, button_height};
    }
}

static size_t utf8_next(const char *text, size_t length, size_t offset)
{
    if (offset >= length) return length;
    ++offset;
    while (offset < length && (((uint8_t)text[offset] & 0xc0U) == 0x80U)) ++offset;
    return offset;
}

static size_t byte_offset_for_codepoints(const char *text, size_t length, size_t count)
{
    size_t offset = 0U;
    while (count > 0U && offset < length) {
        offset = utf8_next(text, length, offset);
        --count;
    }
    return offset;
}

static size_t codepoint_count(const char *text, size_t length)
{
    size_t count = 0U;
    for (size_t index = 0U; index < length; ++index)
        if (((uint8_t)text[index] & 0xc0U) != 0x80U) ++count;
    return count;
}

bool editor_reveal_scroll(viewer_editor *editor, uint32_t x, uint32_t y,
                          double delta_y)
{
    if (editor == NULL || !editor->reveal_active || delta_y == 0.0) return false;
    ui_node *body = find_node(editor, EDITOR_NODE_ID_REVEAL);
    if (body == NULL || x < body->bounds.x || y < body->bounds.y ||
        x - body->bounds.x >= body->bounds.width || y - body->bounds.y >= body->bounds.height)
        return false;
    size_t total = codepoint_count(editor->reveal_id, editor->reveal_id_length);
    size_t step = 36U;
    if (delta_y < 0.0) {
        editor->reveal_scroll_codepoints = editor->reveal_scroll_codepoints + step > total ?
            total : editor->reveal_scroll_codepoints + step;
    } else {
        editor->reveal_scroll_codepoints = editor->reveal_scroll_codepoints > step ?
            editor->reveal_scroll_codepoints - step : 0U;
    }
    editor->tree_ready = false;
    editor->redraw_requested = true;
    return true;
}

bool viewer_editor_copy_full_id(const viewer_editor *editor, uint8_t *buffer,
    size_t capacity, size_t *out_length)
{
    if (out_length == NULL) return false;
    *out_length = 0U;
    if (editor == NULL || buffer == NULL || !editor->reveal_active ||
        !editor->reveal_has_id || editor->reveal_exceeds_limit ||
        editor->reveal_id_length == 0U || editor->reveal_id_length > capacity)
        return false;
    memcpy(buffer, editor->reveal_id, editor->reveal_id_length);
    *out_length = editor->reveal_id_length;
    return true;
}

void editor_reveal_paint(viewer_editor *editor, uint8_t *rgba,
                         size_t capacity, size_t stride)
{
    if (editor == NULL || rgba == NULL || !editor->reveal_active || !editor->tree_ready)
        return;
    ui_node *body = find_node(editor, EDITOR_NODE_ID_REVEAL);
    if (body == NULL || body->bounds.width == 0U || body->bounds.height == 0U) return;
    uint32_t backing = (uint32_t)llround(editor->backing_scale);
    if (backing == 0U) backing = 1U;
    if (backing > 3U) backing = 3U;
    double ui_scale = (double)viewer_editor_ui_scale_percent(editor) / 100.0;
    ui_widget_canvas canvas;
    if (!ui_widget_canvas_init_scaled(&canvas, rgba, capacity, editor->width,
            editor->height, stride, backing, ui_scale, editor->theme,
            editor->increased_contrast, platform_text_ui_draw_widget, NULL)) return;
    canvas.tokens = editor->tree.tokens;
    double scale = editor->backing_scale * ui_scale;
    ui_rect frame = {(uint32_t)llround((double)body->bounds.x / scale),
        (uint32_t)llround((double)body->bounds.y / scale),
        (uint32_t)llround((double)body->bounds.width / scale),
        (uint32_t)llround((double)body->bounds.height / scale)};
    ui_widget_paint_round(&canvas, frame, 8U, canvas.tokens.panel_raised,
        canvas.tokens.separator, 1U);
    ui_rect title = {frame.x + 12U, frame.y + 8U,
        frame.width > 24U ? frame.width - 24U : 0U, 18U};
    (void)ui_widget_paint_text(&canvas, "Full SVG ID", 11U, title,
        canvas.tokens.text_primary, UI_FONT_SYSTEM, UI_FONT_SEMIBOLD, 12U);
    ui_rect tag = {title.x, title.y + 20U, title.width, 18U};
    static const char tag_prefix[] = "Tag: ";
    if (editor->reveal_tag_length > 0U) {
        char tag_text[EDITOR_NAME_CAP + sizeof(tag_prefix)] = {0};
        memcpy(tag_text, tag_prefix, sizeof(tag_prefix) - 1U);
        size_t amount = editor->reveal_tag_length;
        if (amount > sizeof(tag_text) - sizeof(tag_prefix))
            amount = sizeof(tag_text) - sizeof(tag_prefix);
        memcpy(tag_text + sizeof(tag_prefix) - 1U, editor->reveal_tag, amount);
        (void)ui_widget_paint_text(&canvas, tag_text,
            sizeof(tag_prefix) - 1U + amount, tag, canvas.tokens.text_secondary,
            UI_FONT_MONOSPACED, UI_FONT_REGULAR, 11U);
    }
    uint32_t line_height = 16U;
    uint32_t button_height = 28U;
    uint32_t content_y = tag.y + 20U;
    uint32_t available_height = frame.height > content_y + button_height + 12U ?
        frame.height - content_y - button_height - 12U : 0U;
    size_t visible_lines = available_height / line_height;
    if (visible_lines > EDITOR_REVEAL_VISIBLE_LINES) visible_lines = EDITOR_REVEAL_VISIBLE_LINES;
    if (editor->reveal_exceeds_limit) {
        ui_rect line = {title.x, content_y, title.width, line_height};
        static const char over_limit[] = "ID exceeds 64 KiB reveal limit";
        (void)ui_widget_paint_text(&canvas, over_limit, sizeof(over_limit) - 1U,
            line, canvas.tokens.warning, UI_FONT_SYSTEM, UI_FONT_REGULAR, 11U);
    } else {
        size_t start = byte_offset_for_codepoints(editor->reveal_id,
            editor->reveal_id_length, editor->reveal_scroll_codepoints);
        size_t max_codepoints = title.width > 32U ?
            (size_t)(title.width - 24U) / 7U : 8U;
        if (max_codepoints < 8U) max_codepoints = 8U;
        for (size_t line_index = 0U; line_index < visible_lines && start < editor->reveal_id_length;
             ++line_index) {
            size_t end = start;
            size_t points = 0U;
            while (end < editor->reveal_id_length && points < max_codepoints) {
                end = utf8_next(editor->reveal_id, editor->reveal_id_length, end);
                ++points;
            }
            ui_rect bounds = {title.x, content_y + (uint32_t)line_index * line_height,
                title.width, line_height};
            (void)ui_widget_paint_text(&canvas, editor->reveal_id + start,
                end - start, bounds, canvas.tokens.text_primary,
                UI_FONT_MONOSPACED, UI_FONT_REGULAR, 11U);
            start = end;
        }
    }
    ui_node *copy = find_node(editor, EDITOR_BUTTON_COPY_FULL_ID);
    if (copy != NULL && editor->reveal_has_id && !editor->reveal_exceeds_limit) {
        ui_rect bounds = {(uint32_t)llround((double)copy->bounds.x / scale),
            (uint32_t)llround((double)copy->bounds.y / scale),
            (uint32_t)llround((double)copy->bounds.width / scale),
            (uint32_t)llround((double)copy->bounds.height / scale)};
        ui_control_state state = copy->spec.id == editor->hovered_id ?
            UI_CONTROL_HOVER : UI_CONTROL_REST;
        if (copy->spec.id == editor->pressed_id) state = UI_CONTROL_PRESSED;
        ui_widget_draw_spec button = {.kind = UI_WIDGET_BUTTON_SECONDARY,
            .state = state, .bounds = bounds, .label = "Copy Full ID",
            .label_length = 12U, .font_size_pt = 11U,
            .font_family = UI_FONT_SYSTEM, .font_weight = UI_FONT_SEMIBOLD,
            .visible = true};
        (void)ui_widget_paint(&canvas, &button);
    }
}
