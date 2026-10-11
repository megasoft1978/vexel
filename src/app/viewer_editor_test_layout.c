#include "viewer_editor_test_layout.h"

#include "viewer_editor_internal.h"

#include <math.h>
#include <string.h>

static size_t bounded_length(const char *text, size_t capacity)
{
    size_t length = 0U;
    if (text == NULL) return 0U;
    while (length < capacity && text[length] != '\0') ++length;
    return length;
}

static bool write_json_string(FILE *stream, const char *text, size_t length)
{
    if (fputc('"', stream) == EOF) return false;
    for (size_t index = 0U; index < length; ++index) {
        unsigned char byte = (unsigned char)text[index];
        if (byte == '"' || byte == '\\') {
            if (fputc('\\', stream) == EOF || fputc((int)byte, stream) == EOF)
                return false;
        } else if (byte < 0x20U) {
            if (fprintf(stream, "\\u%04x", (unsigned)byte) < 0) return false;
        } else if (fputc((int)byte, stream) == EOF) {
            return false;
        }
    }
    return fputc('"', stream) != EOF;
}

static bool write_rect(FILE *stream, ui_rect rect)
{
    return fprintf(stream,
        "{\"x\":%u,\"y\":%u,\"width\":%u,\"height\":%u}",
        rect.x, rect.y, rect.width, rect.height) >= 0;
}

static bool write_node(FILE *stream, const ui_node *node,
                       const viewer_editor *editor)
{
    size_t label_length = bounded_length(node->label, sizeof(node->label));
    size_t value_length = bounded_length(node->value, sizeof(node->value));
    if (fprintf(stream, "{\"id\":%u,\"state\":%u,\"bounds\":",
            node->spec.id, node->spec.state) < 0 ||
        !write_rect(stream, node->bounds) || fputs(",\"label\":", stream) == EOF ||
        !write_json_string(stream, node->label, label_length) ||
        fputs(",\"value\":", stream) == EOF ||
        !write_json_string(stream, node->value, value_length)) return false;

    if (node->spec.id < EDITOR_OUTLINE_ROW_BASE ||
        node->spec.id >= EDITOR_OUTLINE_ROW_BASE + EDITOR_OUTLINE_VISIBLE)
        return fputc('}', stream) != EOF;

    double scaled = ceil(editor_presentation_scale(editor));
    if (!isfinite(scaled) || scaled < 1.0 || scaled > 8.0) return false;
    uint32_t scale = (uint32_t)scaled;
    uint32_t action_span = EDITOR_OUTLINE_ACTION_WIDTH_POINTS * scale;
    uint32_t visibility_span = EDITOR_OUTLINE_VISIBILITY_WIDTH_POINTS * scale;
    uint32_t action_width = node->bounds.width < action_span ?
        node->bounds.width : action_span;
    uint32_t action_x = node->bounds.x + node->bounds.width - action_width;
    uint32_t visibility_width = node->bounds.x + node->bounds.width - action_x <
        visibility_span ? node->bounds.x + node->bounds.width - action_x :
        visibility_span;
    ui_rect select = {node->bounds.x, node->bounds.y,
        action_x - node->bounds.x, node->bounds.height};
    ui_rect visibility = {action_x, node->bounds.y, visibility_width,
        node->bounds.height};
    ui_rect lock = {action_x + visibility_width, node->bounds.y,
        node->bounds.x + node->bounds.width - action_x - visibility_width,
        node->bounds.height};
    return fputs(",\"targets\":{\"select\":", stream) != EOF &&
        write_rect(stream, select) && fputs(",\"visibility\":", stream) != EOF &&
        write_rect(stream, visibility) && fputs(",\"lock\":", stream) != EOF &&
        write_rect(stream, lock) && fputs("}}", stream) != EOF;
}

bool viewer_editor_test_layout_write_nodes(viewer_editor *editor, FILE *stream)
{
    if (editor == NULL || stream == NULL) return false;
    if (!editor->tree_ready) editor_build_tree(editor);
    if (!editor->tree_ready || fputc('[', stream) == EOF) return false;
    bool first = true;
    for (size_t index = 0U; index < editor->tree.count; ++index) {
        const ui_node *node = &editor->tree.nodes[index];
        if (!node->occupied) continue;
        if ((!first && fputc(',', stream) == EOF) ||
            !write_node(stream, node, editor)) return false;
        first = false;
    }
    return fputc(']', stream) != EOF;
}
