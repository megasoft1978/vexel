#include "viewer_editor_internal.h"
#include "platform/platform_localization.h"

#include <math.h>
#include <stdlib.h>
#include <stdio.h>
#include <string.h>

static size_t editor_full_attribute_value(viewer_editor *editor,
                                          document_source_span span,
                                          const char **out_value)
{
    size_t length = 0U;
    static const char too_long[] = "Value exceeds 64 KiB";
    if (document_edit_decode_span(editor->edit, span,
            (uint8_t *)editor->display_text, sizeof(editor->display_text) - 1U,
            &length)) {
        editor->display_text[length] = '\0';
        *out_value = editor->display_text;
        return length;
    }
    *out_value = too_long;
    return sizeof(too_long) - 1U;
}

static size_t editor_attribute_label(base_bytes name, char out[UI_TEXT_MAX_BYTES])
{
    struct label_pair { const char *source; const char *label; };
    static const struct label_pair labels[] = {
        {"x", "X"}, {"y", "Y"}, {"cx", "Center X"}, {"cy", "Center Y"},
        {"width", "Width"}, {"height", "Height"}, {"r", "Radius"},
        {"rx", "Radius X"}, {"ry", "Radius Y"}, {"x1", "Start X"},
        {"y1", "Start Y"}, {"x2", "End X"}, {"y2", "End Y"},
        {"fill", "Fill"}, {"stroke", "Stroke"}, {"stroke-width", "Stroke width"},
        {"opacity", "Opacity"}, {"fill-opacity", "Fill opacity"},
        {"stroke-opacity", "Stroke opacity"}, {"font-size", "Font size"},
        {"font-family", "Font family"}, {"text-anchor", "Text anchor"}
    };
    for (size_t index = 0U; index < sizeof(labels) / sizeof(labels[0]); ++index) {
        size_t length = strlen(labels[index].source);
        if (length == name.len && memcmp(name.ptr, labels[index].source, length) == 0) {
            size_t label_length = strlen(labels[index].label);
            memcpy(out, labels[index].label, label_length);
            out[label_length] = '\0';
            return label_length;
        }
    }
    static const char ellipsis[] = "\xe2\x80\xa6";
    size_t take = editor_utf8_prefix(name.ptr, name.len,
        UI_TEXT_MAX_BYTES - sizeof(ellipsis));
    if (take > 0U) memcpy(out, name.ptr, take);
    if (take > 0U && out[0] >= 'a' && out[0] <= 'z') out[0] -= 'a' - 'A';
    if (take < name.len && take + sizeof(ellipsis) <= UI_TEXT_MAX_BYTES) {
        memcpy(out + take, ellipsis, sizeof(ellipsis) - 1U);
        take += sizeof(ellipsis) - 1U;
    }
    out[take] = '\0';
    return take;
}

static bool editor_is_identity_attribute(base_bytes name)
{
    return name.len == 2U && memcmp(name.ptr, "id", 2U) == 0;
}

static bool editor_is_measurement_attribute(base_bytes name)
{
    static const char *const names[] = {"x", "y", "width", "height", "cx", "cy",
        "r", "rx", "ry", "x1", "y1", "x2", "y2", "stroke-width", "font-size"};
    for (size_t index = 0U; index < sizeof(names) / sizeof(names[0]); ++index)
        if (strlen(names[index]) == name.len &&
            memcmp(name.ptr, names[index], name.len) == 0) return true;
    return false;
}

static bool editor_format_measurement(base_bytes name, const char *raw,
    size_t raw_length, char output[128])
{
    if (!editor_is_measurement_attribute(name) || raw == NULL || raw_length == 0U ||
        raw_length >= 128U) return false;
    char input[128];
    memcpy(input, raw, raw_length);
    input[raw_length] = '\0';
    char *suffix = NULL;
    double value = strtod(input, &suffix);
    if (suffix == input || !isfinite(value)) return false;
    size_t suffix_length = strlen(suffix);
    if (suffix_length > 8U) return false;
    static const char *const units[] = {"", "%", "px", "em", "ex", "ch", "rem",
        "in", "cm", "mm", "pt", "pc"};
    bool valid_unit = false;
    for (size_t index = 0U; index < sizeof(units) / sizeof(units[0]); ++index)
        if (strcmp(units[index], suffix) == 0) { valid_unit = true; break; }
    if (!valid_unit) return false;
    char localized[96] = {0};
    size_t localized_length = 0U;
    if (!platform_localization_format_number(value, 6U, localized,
            sizeof(localized), &localized_length) ||
        localized_length + suffix_length >= 128U) return false;
    memcpy(output, localized, localized_length);
    if (suffix_length > 0U)
        memcpy(output + localized_length, suffix, suffix_length);
    output[localized_length + suffix_length] = '\0';
    return true;
}

static uint8_t editor_attribute_section(base_bytes name)
{
    static const char *const geometry[] = {"x", "y", "width", "height", "cx", "cy",
        "r", "rx", "ry", "x1", "y1", "x2", "y2", "d", "points", "transform",
        "viewBox"};
    static const char *const appearance[] = {"fill", "stroke", "stroke-width", "opacity",
        "fill-opacity", "stroke-opacity", "stroke-linecap", "stroke-linejoin",
        "font-size", "font-family", "text-anchor"};
    for (size_t index = 0U; index < sizeof(geometry) / sizeof(geometry[0]); ++index)
        if (strlen(geometry[index]) == name.len &&
            memcmp(name.ptr, geometry[index], name.len) == 0) return 0U;
    for (size_t index = 0U; index < sizeof(appearance) / sizeof(appearance[0]); ++index)
        if (strlen(appearance[index]) == name.len &&
            memcmp(name.ptr, appearance[index], name.len) == 0) return 1U;
    return 2U;
}

/* Object, Fill, Stroke and Document, as in Blender's Properties editor. */
#define EDITOR_INSPECTOR_TAB_OBJECT 0U
#define EDITOR_INSPECTOR_TAB_FILL 1U
#define EDITOR_INSPECTOR_TAB_STROKE 2U
#define EDITOR_INSPECTOR_TAB_DOCUMENT 3U
#define EDITOR_INSPECTOR_TAB_COUNT 4U

static const char *inspector_tab_title(uint8_t tab)
{
    static const char *const titles[EDITOR_INSPECTOR_TAB_COUNT] = {
        "Object", "Fill", "Stroke", "Document"};
    return tab < EDITOR_INSPECTOR_TAB_COUNT ? titles[tab] : titles[0];
}

/* The node id and the command id differ: the node id names the widget, the
 * command id is what the dispatcher acts on. */
static uint32_t inspector_tab_node(uint8_t tab)
{
    return EDITOR_BUTTON_INSPECTOR_TAB_BASE + (uint32_t)tab;
}

static uint32_t inspector_tab_action(uint8_t tab)
{
    return EDITOR_BUTTON_INSPECTOR_TAB_OBJECT + (uint32_t)tab;
}

/* A tab accepts an attribute when it owns that attribute's group. Object keeps
 * geometry and the catch-all attributes so nothing becomes unreachable. */
static bool inspector_tab_accepts(uint8_t tab, base_bytes name, uint8_t section)
{
    if (tab == EDITOR_INSPECTOR_TAB_OBJECT) return section != 1U;
    if (tab == EDITOR_INSPECTOR_TAB_DOCUMENT) return false;
    bool wants_fill = tab == EDITOR_INSPECTOR_TAB_FILL;
    size_t prefix = wants_fill ? 4U : 6U;
    const char *wanted = wants_fill ? "fill" : "stroke";
    if (section != 1U || name.len < prefix) return false;
    return memcmp(name.ptr, wanted, prefix) == 0;
}

static const char *section_title(uint8_t section)
{
    if (section == 0U) return "Geometry";
    if (section == 1U) return "Appearance";
    return "Additional attributes";
}

static bool editor_add_section(viewer_editor *editor, uint32_t parent,
                               uint32_t id, const char *title)
{
    return editor_add_widget_node(editor, id, parent, UI_AXIS_HORIZONTAL,
        0U, editor_section_height(editor), 0U, 0U, 0U, UI_ROLE_TEXT, UI_WIDGET_LABEL,
        UI_ICON_NONE, title, strlen(title), NULL, 0U,
        0U, 0U);
}

static bool editor_add_collapsible_section(viewer_editor *editor, uint32_t id,
    const char *title, bool expanded, uint32_t action)
{
    return editor_add_widget_node(editor, id, EDITOR_NODE_INSPECTOR,
        UI_AXIS_HORIZONTAL, 0U, 24U, 0U, 0U, 0U, UI_ROLE_BUTTON,
        UI_WIDGET_SECTION_HEADER, UI_ICON_NONE, title, strlen(title), NULL, 0U,
        expanded ? UI_STATE_EXPANDED : 0U,
        action);
}

static bool editor_add_attribute_rows(viewer_editor *editor, const ui_tokens *tokens)
{
    size_t count = document_edit_attribute_count(editor->edit, editor->selected);
    size_t start = editor->attribute_start;
    if (start > count) start = count;
    size_t visible = 0U;
    uint32_t section_sequence = 0U;
    if (!editor->selection_active) {
        bool added_header = false;
        for (size_t attribute_index = 0U;
             attribute_index < count && visible < EDITOR_ATTRIBUTE_VISIBLE;
             ++attribute_index) {
            document_edit_attribute attribute;
            base_bytes name;
            if (!document_edit_attribute_at(editor->edit, editor->selected,
                    attribute_index, &attribute) ||
                !document_edit_span_bytes(editor->edit, attribute.name, &name)) return false;
            bool wanted = (name.len == 5U && memcmp(name.ptr, "width", 5U) == 0) ||
                (name.len == 6U && memcmp(name.ptr, "height", 6U) == 0) ||
                (name.len == 7U && memcmp(name.ptr, "viewBox", 7U) == 0);
            if (!wanted) continue;
            if (!added_header && !editor_add_section(editor, EDITOR_NODE_INSPECTOR,
                    EDITOR_INSPECTOR_SECTION_BASE, "Geometry")) return false;
            added_header = true;
            char label[UI_TEXT_MAX_BYTES] = {0};
            size_t label_length = editor_attribute_label(name, label);
            const char *value = NULL;
            size_t value_length = editor_full_attribute_value(editor, attribute.value, &value);
            if (!editor_add_node(editor,
                    EDITOR_ATTRIBUTE_ROW_BASE + (uint32_t)attribute_index,
                    EDITOR_NODE_INSPECTOR, UI_AXIS_HORIZONTAL, 0U, editor_field_height(editor), 4U, 0U,
                    tokens->panel, UI_ROLE_TEXT_FIELD, label, label_length, value,
                    value_length, 0U,
                    EDITOR_ATTRIBUTE_ROW_BASE + (uint32_t)attribute_index)) return false;
            ++visible;
        }
        return true;
    }
    for (uint8_t section = 0U; section < 3U; ++section) {
        bool has_section = false;
        for (size_t index = 0U; index < count; ++index) {
            document_edit_attribute attribute;
            base_bytes name;
            if (!document_edit_attribute_at(editor->edit, editor->selected,
                    index, &attribute) ||
                !document_edit_span_bytes(editor->edit, attribute.name, &name)) return false;
            uint8_t name_section = editor_attribute_section(name);
            if (!editor_is_identity_attribute(name) && name_section == section &&
                inspector_tab_accepts(editor->inspector_tab, name, name_section)) {
                has_section = true;
                break;
            }
        }
        if (!has_section) continue;
        /* A filtered tab is itself the group selector, so its group is always
         * open; only the Object tab honours the collapsible section state. */
        bool expanded = editor->inspector_tab != EDITOR_INSPECTOR_TAB_OBJECT ? true :
            (section == 0U ? editor->inspector_geometry_expanded :
                section == 1U ? editor->inspector_appearance_expanded :
                    editor->inspector_other_attributes_expanded);
        uint32_t action = section == 0U ? EDITOR_BUTTON_INSPECTOR_GEOMETRY :
            section == 1U ? EDITOR_BUTTON_INSPECTOR_APPEARANCE :
                EDITOR_BUTTON_INSPECTOR_OTHER_ATTRIBUTES;
        if (!editor_add_collapsible_section(editor,
                EDITOR_INSPECTOR_SECTION_BASE + section_sequence++,
                section_title(section), expanded, action)) return false;
        if (!expanded) continue;
        if (visible >= EDITOR_ATTRIBUTE_VISIBLE) continue;

        for (size_t attribute_index = start;
             attribute_index < count && visible < EDITOR_ATTRIBUTE_VISIBLE;
             ++attribute_index) {
            document_edit_attribute attribute;
            base_bytes name;
            char label[UI_TEXT_MAX_BYTES] = {0};
            char localized_measurement[128] = {0};
            const char *value = NULL;
            size_t value_length = 0U;
            if (!document_edit_attribute_at(editor->edit, editor->selected,
                    attribute_index, &attribute) ||
                !document_edit_span_bytes(editor->edit, attribute.name, &name)) return false;
            uint8_t name_section = editor_attribute_section(name);
            if (editor_is_identity_attribute(name) || name_section != section ||
                !inspector_tab_accepts(editor->inspector_tab, name, name_section)) continue;
            size_t label_length = editor_attribute_label(name, label);
            if (editor->field_kind == EDITOR_FIELD_ATTRIBUTE &&
                editor->field_attribute == attribute_index) {
                value = editor->field;
                value_length = editor->field_length;
            } else {
                value_length = editor_full_attribute_value(editor, attribute.value, &value);
                if (editor_format_measurement(name, value, value_length,
                        localized_measurement)) {
                    value = localized_measurement;
                    value_length = strlen(localized_measurement);
                }
            }
            uint32_t state = attribute_index == editor->selected_attribute ?
                UI_STATE_CHECKED : 0U;
            if (editor->field_kind == EDITOR_FIELD_ATTRIBUTE &&
                editor->field_attribute == attribute_index) state |= UI_STATE_FOCUSED;
            uint32_t color = attribute_index == editor->selected_attribute ?
                tokens->selection_fill : tokens->panel;
            if (!editor_add_node(editor,
                    EDITOR_ATTRIBUTE_ROW_BASE + (uint32_t)attribute_index,
                    EDITOR_NODE_INSPECTOR, UI_AXIS_HORIZONTAL, 0U, editor_field_height(editor), 4U, 0U,
                    color, UI_ROLE_TEXT_FIELD, label, label_length, value,
                    value_length, state,
                    EDITOR_ATTRIBUTE_ROW_BASE + (uint32_t)attribute_index)) return false;
            ++visible;
        }
    }
    return true;
}

static size_t selected_type_label(viewer_editor *editor, char out[UI_TEXT_MAX_BYTES])
{
    base_bytes tag;
    if (!document_edit_tag_name(editor->edit, editor->selected, &tag)) {
        memcpy(out, "Element", 7U);
        out[7] = '\0';
        return 7U;
    }
    if (tag.len == 3U && memcmp(tag.ptr, "svg", 3U) == 0) {
        memcpy(out, "Document", 8U); out[8] = '\0'; return 8U;
    }
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
        size_t tag_length = strlen(names[index].tag);
        if (tag.len != tag_length || memcmp(tag.ptr, names[index].tag, tag_length) != 0)
            continue;
        size_t length = strlen(names[index].title);
        memcpy(out, names[index].title, length + 1U);
        return length;
    }
    size_t length = editor_utf8_prefix(tag.ptr, tag.len, UI_TEXT_MAX_BYTES - 1U);
    if (length > 0U) memcpy(out, tag.ptr, length);
    if (length > 0U && out[0] >= 'a' && out[0] <= 'z') out[0] -= 'a' - 'A';
    out[length] = '\0';
    return length;
}

bool editor_build_inspector_panel(viewer_editor *editor, const ui_tokens *tokens)
{
    bool okay = editor_add_node(editor, EDITOR_NODE_INSPECTOR, EDITOR_NODE_WORKSPACE,
            UI_AXIS_VERTICAL, editor->layout_inspector_width_points, 0U, 8U, 3U,
            tokens->panel, UI_ROLE_GROUP, "Inspector", 9U, NULL, 0U, 0U, 0U) &&
        editor_add_node(editor, EDITOR_NODE_INSPECTOR_HEADER, EDITOR_NODE_INSPECTOR,
            UI_AXIS_HORIZONTAL, 0U, 28U, 0U, 4U, 0U, UI_ROLE_GROUP,
            NULL, 0U, NULL, 0U, 0U, 0U) &&
        editor_add_widget_node(editor, EDITOR_NODE_INSPECTOR_TITLE,
            EDITOR_NODE_INSPECTOR_HEADER, UI_AXIS_HORIZONTAL, 0U, 28U,
            4U, 0U, 0U, UI_ROLE_TEXT, UI_WIDGET_LABEL, UI_ICON_NONE,
            "Inspector", 9U, NULL, 0U, 0U, 0U) &&
        editor_add_widget_node(editor, EDITOR_BUTTON_HIDE_INSPECTOR,
            EDITOR_NODE_INSPECTOR_HEADER, UI_AXIS_HORIZONTAL, 28U, 28U,
            0U, 0U, 0U, UI_ROLE_BUTTON, UI_WIDGET_ICON_BUTTON, UI_ICON_SIDEBAR_RIGHT,
            "Hide inspector panel", 20U, NULL, 0U, 0U,
            EDITOR_BUTTON_HIDE_INSPECTOR);
    if (!okay) return false;
    /* Vertical icon tabs, as in Blender's Properties editor. */
    if (!editor_add_node(editor, EDITOR_NODE_INSPECTOR_TABS, EDITOR_NODE_INSPECTOR,
            UI_AXIS_HORIZONTAL, 0U, 26U, 4U, 0U, 0U, UI_ROLE_GROUP,
            "Properties", 10U, NULL, 0U, 0U, 0U)) return false;
    for (uint8_t tab = 0U; tab < EDITOR_INSPECTOR_TAB_COUNT; ++tab) {
        const char *title = inspector_tab_title(tab);
        uint32_t tab_width = (uint32_t)(strlen(title) * 8U + 20U);
        if (!editor_add_widget_node(editor, inspector_tab_node(tab),
                EDITOR_NODE_INSPECTOR_TABS, UI_AXIS_HORIZONTAL, tab_width, 24U, 4U, 0U,
                tab == editor->inspector_tab ? tokens->panel_raised : 0U,
                UI_ROLE_BUTTON, UI_WIDGET_BUTTON_SECONDARY, UI_ICON_NONE,
                title, strlen(title), NULL, 0U,
                tab == editor->inspector_tab ? UI_STATE_CHECKED : 0U,
                inspector_tab_action(tab))) return false;
    }
    /* The Document tab always describes the document, selection or not. */
    if (editor->inspector_tab == EDITOR_INSPECTOR_TAB_DOCUMENT) {
        char document_size[64] = {0};
        int amount = snprintf(document_size, sizeof(document_size), "%u × %u",
            editor->canvas_document_width, editor->canvas_document_height);
        if (amount < 0 || (size_t)amount >= sizeof(document_size))
            document_size[0] = '\0';
        return editor_add_widget_node(editor, EDITOR_NODE_SELECTED_TAG,
                EDITOR_NODE_INSPECTOR, UI_AXIS_HORIZONTAL, 0U, 26U, 4U, 0U,
                0U, UI_ROLE_TEXT, UI_WIDGET_LABEL, UI_ICON_NONE, "Document",
                8U, NULL, 0U, 0U, 0U) &&
            editor_add_widget_node(editor, EDITOR_NODE_INSPECTOR_DOCUMENT_SIZE,
                EDITOR_NODE_INSPECTOR, UI_AXIS_HORIZONTAL, 0U, editor_field_height(editor), 4U, 0U,
                tokens->panel, UI_ROLE_TEXT, UI_WIDGET_LABEL, UI_ICON_NONE,
                "Size", 4U, document_size, strlen(document_size), 0U, 0U) &&
            /* The document's own attributes stay editable here. */
            editor_add_attribute_rows(editor, tokens) &&
            editor_add_node(editor, EDITOR_BUTTON_FIT_CANVAS,
                EDITOR_NODE_INSPECTOR, UI_AXIS_HORIZONTAL, 0U, editor_field_height(editor), 4U, 0U,
                tokens->panel_raised, UI_ROLE_BUTTON, "Fit Canvas", 10U,
                NULL, 0U, 0U, EDITOR_BUTTON_FIT_CANVAS) &&
            editor_add_node(editor, EDITOR_BUTTON_ACTUAL_SIZE,
                EDITOR_NODE_INSPECTOR, UI_AXIS_HORIZONTAL, 0U, editor_field_height(editor), 4U, 0U,
                tokens->panel_raised, UI_ROLE_BUTTON, "Actual Size", 11U,
                NULL, 0U, 0U, EDITOR_BUTTON_ACTUAL_SIZE);
    }
    if (!editor->selection_active) {
        /* Document fields live on the Document tab, so the Object tab is honest
         * about having nothing to describe. */
        return editor_add_widget_node(editor, EDITOR_NODE_SELECTED_TAG,
                EDITOR_NODE_INSPECTOR, UI_AXIS_HORIZONTAL, 0U, 26U, 4U, 0U,
                0U, UI_ROLE_TEXT, UI_WIDGET_LABEL, UI_ICON_NONE,
                "No selection", 12U, NULL, 0U, 0U, 0U) &&
            editor_add_widget_node(editor, EDITOR_NODE_INSPECTOR_DOCUMENT_SIZE,
                EDITOR_NODE_INSPECTOR, UI_AXIS_HORIZONTAL, 0U, 26U, 4U, 0U,
                0U, UI_ROLE_TEXT, UI_WIDGET_LABEL, UI_ICON_NONE,
                "Select an element to edit it.", 29U, NULL, 0U, 0U, 0U);
    }
    char type[UI_TEXT_MAX_BYTES] = {0};
    size_t type_length = selected_type_label(editor, type);
    if (!editor_add_widget_node(editor, EDITOR_NODE_SELECTED_TAG,
            EDITOR_NODE_INSPECTOR, UI_AXIS_HORIZONTAL, 0U, 26U, 4U, 0U,
            0U, UI_ROLE_TEXT, UI_WIDGET_LABEL, UI_ICON_NONE, type,
            type_length, NULL, 0U, 0U, 0U)) return false;
    /* The name (id), text content and add/remove fields belong to Object, so
     * the id attribute is reachable on exactly one tab. */
    if (editor->inspector_tab != EDITOR_INSPECTOR_TAB_OBJECT)
        return editor_add_attribute_rows(editor, tokens);
    const char *name_value = editor->field_kind == EDITOR_FIELD_RENAME ?
        editor->field : (editor->selected_has_id ? editor->selected_id : "");
    size_t name_length = editor->field_kind == EDITOR_FIELD_RENAME ?
        editor->field_length : (editor->selected_has_id ? strlen(editor->selected_id) : 0U);
    uint32_t name_state = editor->field_kind == EDITOR_FIELD_RENAME ? UI_STATE_FOCUSED : 0U;
    if (!editor_add_node(editor, EDITOR_RENAME_ID, EDITOR_NODE_INSPECTOR,
            UI_AXIS_HORIZONTAL, 0U, 30U, 4U, 0U, tokens->panel_raised,
            UI_ROLE_TEXT_FIELD, "Name", 4U, name_value, name_length,
            name_state, EDITOR_RENAME_ID) || !editor_add_attribute_rows(editor, tokens))
        return false;
    base_bytes tag = {0};
    bool is_text_element = document_edit_tag_name(editor->edit, editor->selected, &tag) &&
        ((tag.len == 4U && memcmp(tag.ptr, "text", 4U) == 0) ||
         (tag.len == 5U && memcmp(tag.ptr, "tspan", 5U) == 0));
    uint8_t text_value[EDITOR_FIELD_CAP];
    size_t text_value_length = 0U;
    if (is_text_element && document_edit_text_content(editor->edit, editor->selected, text_value,
            sizeof(text_value), &text_value_length)) {
        const char *value = editor->field_kind == EDITOR_FIELD_TEXT ?
            editor->field : (const char *)text_value;
        size_t value_length = editor->field_kind == EDITOR_FIELD_TEXT ?
            editor->field_length : text_value_length;
        uint32_t state = editor->field_kind == EDITOR_FIELD_TEXT ? UI_STATE_FOCUSED : 0U;
        if (!editor_add_section(editor, EDITOR_NODE_INSPECTOR,
                EDITOR_INSPECTOR_SECTION_BASE + 16U, "Text") ||
            !editor_add_node(editor, EDITOR_TEXT_FIELD_ID, EDITOR_NODE_INSPECTOR,
                UI_AXIS_HORIZONTAL, 0U, 32U, 4U, 0U, tokens->panel_raised,
                UI_ROLE_TEXT_FIELD, "Content", 7U, value, value_length, state,
                EDITOR_TEXT_FIELD_ID)) return false;
    }
    if (!editor->inspector_other_attributes_expanded) return true;
    const char *new_name = editor->field_kind == EDITOR_FIELD_NEW_NAME ?
        editor->field : editor->new_name;
    size_t new_name_length = editor->field_kind == EDITOR_FIELD_NEW_NAME ?
        editor->field_length : editor->new_name_length;
    const char *new_value = editor->field_kind == EDITOR_FIELD_NEW_VALUE ?
        editor->field : editor->new_value;
    size_t new_value_length = editor->field_kind == EDITOR_FIELD_NEW_VALUE ?
        editor->field_length : editor->new_value_length;
    return editor_add_node(editor, EDITOR_NEW_NAME_ID, EDITOR_NODE_INSPECTOR,
            UI_AXIS_HORIZONTAL, 0U, 30U, 4U, 0U, tokens->panel_raised,
            UI_ROLE_TEXT_FIELD, "Attribute", 9U, new_name, new_name_length,
            editor->field_kind == EDITOR_FIELD_NEW_NAME ? UI_STATE_FOCUSED : 0U,
            EDITOR_NEW_NAME_ID) &&
        editor_add_node(editor, EDITOR_NEW_VALUE_ID, EDITOR_NODE_INSPECTOR,
            UI_AXIS_HORIZONTAL, 0U, 30U, 4U, 0U, tokens->panel_raised,
            UI_ROLE_TEXT_FIELD, "Value", 5U, new_value, new_value_length,
            editor->field_kind == EDITOR_FIELD_NEW_VALUE ? UI_STATE_FOCUSED : 0U,
            EDITOR_NEW_VALUE_ID) &&
        editor_add_node(editor, EDITOR_BUTTON_ADD_ATTRIBUTE, EDITOR_NODE_INSPECTOR,
            UI_AXIS_HORIZONTAL, 0U, 28U, 4U, 0U, tokens->accent,
            UI_ROLE_BUTTON, "Add attribute", 13U, NULL, 0U, 0U,
            EDITOR_BUTTON_ADD_ATTRIBUTE) &&
        editor_add_node(editor, EDITOR_BUTTON_REMOVE_ATTRIBUTE, EDITOR_NODE_INSPECTOR,
            UI_AXIS_HORIZONTAL, 0U, 28U, 4U, 0U, tokens->panel_raised,
            UI_ROLE_BUTTON, "Remove attribute", 16U, NULL, 0U, 0U,
            EDITOR_BUTTON_REMOVE_ATTRIBUTE);
}
