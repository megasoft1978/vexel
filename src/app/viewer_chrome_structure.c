#include "viewer_editor_internal.h"

#include "viewer_chrome.h"

#include <string.h>

static bool append_localized_row_state(char output[PLATFORM_LOCALIZATION_TEXT_CAP],
    size_t *used, const char *fallback)
{
    char localized[128] = {0};
    size_t length = 0U;
    if (!editor_localize_slice(fallback, strlen(fallback), localized,
            sizeof(localized), &length)) {
        length = strlen(fallback);
        if (length >= sizeof(localized)) return false;
        memcpy(localized, fallback, length + 1U);
    }
    size_t separator_length = *used == 0U ? 0U : sizeof(" · ") - 1U;
    size_t remaining = PLATFORM_LOCALIZATION_TEXT_CAP - 1U - *used;
    if (separator_length > remaining || length > remaining - separator_length) return false;
    if (separator_length > 0U) {
        memcpy(output + *used, " · ", separator_length);
        *used += separator_length;
    }
    memcpy(output + *used, localized, length);
    *used += length;
    output[*used] = '\0';
    return true;
}

static ui_icon_kind layer_icon(viewer_editor *editor, size_t index)
{
    base_bytes tag;
    if (!document_edit_tag_name(editor->edit, editor->outline[index], &tag))
        return UI_ICON_LAYERS;
    if (tag.len == 3U && memcmp(tag.ptr, "svg", 3U) == 0) return UI_ICON_DOCUMENT;
    if (tag.len == 1U && tag.ptr[0] == 'g') return UI_ICON_GROUP;
    if (tag.len == 4U && memcmp(tag.ptr, "defs", 4U) == 0) return UI_ICON_LAYERS;
    if (tag.len == 4U && memcmp(tag.ptr, "rect", 4U) == 0) return UI_ICON_RECTANGLE;
    if (tag.len == 6U && memcmp(tag.ptr, "circle", 6U) == 0) return UI_ICON_CIRCLE;
    if (tag.len == 6U && memcmp(tag.ptr, "ellipse", 6U) == 0) return UI_ICON_ELLIPSE;
    if (tag.len == 4U && memcmp(tag.ptr, "line", 4U) == 0) return UI_ICON_LINE;
    if (tag.len == 4U && memcmp(tag.ptr, "path", 4U) == 0) return UI_ICON_PATH;
    if (tag.len == 8U && memcmp(tag.ptr, "polyline", 8U) == 0) return UI_ICON_POLYLINE;
    if (tag.len == 7U && memcmp(tag.ptr, "polygon", 7U) == 0) return UI_ICON_PEN;
    if (tag.len == 4U && memcmp(tag.ptr, "text", 4U) == 0) return UI_ICON_TEXT;
    if (tag.len == 5U && memcmp(tag.ptr, "image", 5U) == 0) return UI_ICON_IMAGE;
    if (tag.len == 14U && memcmp(tag.ptr, "linearGradient", 14U) == 0)
        return UI_ICON_GRADIENT;
    if (tag.len == 14U && memcmp(tag.ptr, "radialGradient", 14U) == 0)
        return UI_ICON_GRADIENT;
    if (tag.len == 4U && memcmp(tag.ptr, "stop", 4U) == 0) return UI_ICON_STOP;
    return UI_ICON_LAYERS;
}

static size_t layer_type_label(base_bytes tag, char out[PLATFORM_LOCALIZATION_TEXT_CAP])
{
    static const struct { const char *tag; const char *title; } names[] = {
        {"svg", "Document"}, {"g", "Group"}, {"defs", "Definitions"},
        {"rect", "Rectangle"}, {"circle", "Circle"}, {"ellipse", "Ellipse"},
        {"line", "Line"}, {"path", "Path"}, {"text", "Text"},
        {"tspan", "Text Span"}, {"image", "Image"}, {"polygon", "Polygon"},
        {"polyline", "Polyline"}, {"stop", "Stop"},
        {"linearGradient", "Linear Gradient"},
        {"radialGradient", "Radial Gradient"}, {"clipPath", "Clip Path"},
        {"mask", "Mask"}, {"use", "Use"}, {"symbol", "Symbol"}
    };
    for (size_t index = 0U; index < sizeof(names) / sizeof(names[0]); ++index) {
        size_t source_length = strlen(names[index].tag);
        if (tag.len != source_length || memcmp(tag.ptr, names[index].tag, source_length) != 0)
            continue;
        size_t length = strlen(names[index].title);
        size_t localized_length = 0U;
        if (editor_localize_slice(names[index].title, length, out,
                PLATFORM_LOCALIZATION_TEXT_CAP, &localized_length))
            return localized_length;
        memcpy(out, names[index].title, length + 1U);
        return length;
    }
    static const char ellipsis[] = "\xe2\x80\xa6";
    size_t at = 0U;
    bool word_start = true;
    uint8_t prior = 0U;
    for (size_t index = 0U; index < tag.len &&
         at < PLATFORM_LOCALIZATION_TEXT_CAP - 1U; ++index) {
        uint8_t byte = tag.ptr[index];
        if (byte == '-' || byte == '_' || byte == ':') {
            if (at > 0U && out[at - 1U] != ' ' &&
                at < PLATFORM_LOCALIZATION_TEXT_CAP - 1U)
                out[at++] = ' ';
            word_start = true;
            prior = byte;
            continue;
        }
        if (byte >= 'A' && byte <= 'Z' && prior >= 'a' && prior <= 'z' &&
            at + 1U < PLATFORM_LOCALIZATION_TEXT_CAP - 1U) {
            out[at++] = ' ';
            word_start = true;
        }
        if (word_start && byte >= 'a' && byte <= 'z') byte -= 'a' - 'A';
        else if (!word_start && byte >= 'A' && byte <= 'Z') byte += 'a' - 'A';
        out[at++] = (char)byte;
        word_start = false;
        prior = tag.ptr[index];
    }
    if (at < tag.len && at + sizeof(ellipsis) <= PLATFORM_LOCALIZATION_TEXT_CAP) {
        memcpy(out + at, ellipsis, sizeof(ellipsis) - 1U);
        at += sizeof(ellipsis) - 1U;
    }
    out[at] = '\0';
    return at;
}

static size_t layer_label(viewer_editor *editor, size_t index,
                          char out[PLATFORM_LOCALIZATION_TEXT_CAP])
{
    static const char ellipsis[] = "\xe2\x80\xa6";
    base_bytes tag = {0};
    size_t used = 0U;
    if (document_edit_tag_name(editor->edit, editor->outline[index], &tag)) {
        used = layer_type_label(tag, out);
    }
    size_t attribute_count = document_edit_attribute_count(editor->edit,
        editor->outline[index]);
    for (size_t attribute_index = 0U; attribute_index < attribute_count; ++attribute_index) {
        document_edit_attribute attribute;
        base_bytes name;
        if (!document_edit_attribute_at(editor->edit, editor->outline[index],
                attribute_index, &attribute) ||
            !document_edit_span_bytes(editor->edit, attribute.name, &name) ||
            name.len != 2U || memcmp(name.ptr, "id", 2U) != 0) continue;
        uint8_t id[EDITOR_NAME_CAP];
        size_t id_length = 0U;
        if (document_edit_decode_span(editor->edit, attribute.value, id,
                sizeof(id), &id_length) && id_length > 0U &&
            used + 3U < PLATFORM_LOCALIZATION_TEXT_CAP) {
            out[used++] = ' ';
            out[used++] = '#';
            size_t remaining = PLATFORM_LOCALIZATION_TEXT_CAP - used - 1U;
            size_t take = editor_utf8_prefix(id, id_length, remaining);
            if (take > 0U) memcpy(out + used, id, take);
            used += take;
            if (take < id_length && used + sizeof(ellipsis) <=
                    PLATFORM_LOCALIZATION_TEXT_CAP) {
                memcpy(out + used, ellipsis, sizeof(ellipsis) - 1U);
                used += sizeof(ellipsis) - 1U;
            }
        }
        break;
    }
    out[used] = '\0';
    return used;
}

void editor_build_structure_panel(viewer_editor *editor, const ui_tokens *tokens)
{
    bool okay = editor_add_node(editor, EDITOR_NODE_OUTLINE, EDITOR_NODE_WORKSPACE,
            UI_AXIS_VERTICAL, EDITOR_LEFT_WIDTH, 0U, 8U, 4U, tokens->panel,
            UI_ROLE_LIST, "Layers", 6U, NULL, 0U, 0U, 0U) &&
        editor_add_node(editor, EDITOR_NODE_OUTLINE_HEADER, EDITOR_NODE_OUTLINE,
            UI_AXIS_HORIZONTAL, 0U, 28U, 0U, 4U, 0U, UI_ROLE_GROUP,
            NULL, 0U, NULL, 0U, 0U, 0U) &&
        editor_add_widget_node(editor, EDITOR_NODE_OUTLINE_TITLE,
            EDITOR_NODE_OUTLINE_HEADER, UI_AXIS_HORIZONTAL, 0U, 28U,
            4U, 0U, 0U, UI_ROLE_TEXT, UI_WIDGET_LABEL, UI_ICON_NONE,
            "Layers", 6U, NULL, 0U, 0U, 0U) &&
        editor_add_widget_node(editor, EDITOR_BUTTON_HIDE_STRUCTURE,
            EDITOR_NODE_OUTLINE_HEADER, UI_AXIS_HORIZONTAL, 28U, 28U,
            0U, 0U, 0U, UI_ROLE_BUTTON, UI_WIDGET_ICON_BUTTON, UI_ICON_SIDEBAR_LEFT,
            "Hide layers panel", 17U, NULL, 0U, 0U, EDITOR_BUTTON_HIDE_STRUCTURE) &&
        editor_add_widget_node(editor, EDITOR_OUTLINE_SEARCH_ID, EDITOR_NODE_OUTLINE,
            UI_AXIS_HORIZONTAL, 0U, 30U, 0U, 0U, tokens->panel_raised,
            UI_ROLE_TEXT_FIELD, UI_WIDGET_TEXT_FIELD, UI_ICON_SEARCH,
            "Search layers", 13U,
            editor->field_kind == EDITOR_FIELD_SEARCH ? editor->field :
                editor->outline_filter,
            editor->field_kind == EDITOR_FIELD_SEARCH ? editor->field_length :
                editor->outline_filter_length,
            editor->field_kind == EDITOR_FIELD_SEARCH ? UI_STATE_FOCUSED : 0U,
            EDITOR_OUTLINE_SEARCH_ID);
    if (!okay) return;
    size_t visible_count = 0U;
    for (size_t row = 0U; row < EDITOR_OUTLINE_VISIBLE; ++row) {
        size_t visible_index = editor->outline_visible_start + row;
        size_t item = 0U;
        if (!editor_outline_visible_model_index(editor, visible_index, &item)) break;
        char label[PLATFORM_LOCALIZATION_TEXT_CAP] = {0};
        size_t length = layer_label(editor, item, label);
        bool children = document_model_first_child(editor_model(editor),
            editor->outline[item], &(document_handle){0});
        bool hidden = !editor_outline_is_visible(editor, editor->outline[item]);
        bool locked = editor_outline_is_locked(editor, editor->outline[item]);
        document_source_span item_span = {0U, 0U};
        bool canvas_selected = editor->canvas_selection_valid &&
            document_edit_source_span(editor->edit, editor->outline[item], &item_span) &&
            editor_canvas_selection_contains(editor, item_span.offset);
        char row_state[PLATFORM_LOCALIZATION_TEXT_CAP] = {0};
        size_t row_state_length = 0U;
        if (canvas_selected)
            (void)append_localized_row_state(row_state, &row_state_length,
                "Selected on canvas");
        if (hidden)
            (void)append_localized_row_state(row_state, &row_state_length, "Hidden");
        if (locked)
            (void)append_localized_row_state(row_state, &row_state_length, "Locked");
        uint32_t state = editor->selection_active &&
            item == editor->selected_index ? UI_STATE_CHECKED : 0U;
        if (canvas_selected) state |= UI_STATE_SELECTED;
        if (children) state |= UI_STATE_HAS_CHILDREN;
        if (hidden) state |= UI_STATE_HIDDEN;
        if (locked) state |= UI_STATE_LOCKED;
        if (children && !editor_outline_is_collapsed(editor, item))
            state |= UI_STATE_EXPANDED;
        okay = editor_add_widget_node(editor, EDITOR_OUTLINE_ROW_BASE + (uint32_t)row,
            EDITOR_NODE_OUTLINE, UI_AXIS_HORIZONTAL, 0U, 28U, 4U, 0U,
            0U, UI_ROLE_ROW, UI_WIDGET_TREE_ROW, layer_icon(editor, item),
            label, length, row_state, row_state_length, state,
            EDITOR_OUTLINE_ROW_BASE + (uint32_t)row);
        if (!okay) return;
        visible_count++;
    }
    if (visible_count == 0U) {
        const char *heading = editor->outline_filter_length > 0U ?
            "No layers found" : "No layers";
        const char *detail = editor->outline_filter_length > 0U ?
            "Try a different search." : "Open or create an SVG document.";
        (void)editor_add_widget_node(editor, EDITOR_OUTLINE_ROW_BASE,
            EDITOR_NODE_OUTLINE, UI_AXIS_VERTICAL, 0U, 88U, 4U, 0U,
            0U, UI_ROLE_GROUP, UI_WIDGET_EMPTY_STATE, UI_ICON_LAYERS,
            heading, strlen(heading), detail, strlen(detail), 0U,
            EDITOR_OUTLINE_ROW_BASE);
    }
}
