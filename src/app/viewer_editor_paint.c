#define _DARWIN_C_SOURCE 1
#define _USE_EXTENDED_LOCALES_ 1
#include "viewer_editor_internal.h"

#include <stdlib.h>
#include <string.h>

static int paint_hex_digit(uint8_t value)
{
    if (value >= '0' && value <= '9') return (int)(value - '0');
    if (value >= 'a' && value <= 'f') return (int)(value - 'a') + 10;
    if (value >= 'A' && value <= 'F') return (int)(value - 'A') + 10;
    return -1;
}

static bool paint_byte(uint8_t high, uint8_t low, double *out)
{
    int h = paint_hex_digit(high), l = paint_hex_digit(low);
    if (h < 0 || l < 0 || out == NULL) return false;
    *out = (double)(h * 16 + l) / 255.0;
    return true;
}

static bool parse_selected_paint(base_bytes paint, double rgba[4])
{
    if (paint.ptr == NULL || rgba == NULL) return false;
    if (paint.ptr[0] == '#' && (paint.len == 4U || paint.len == 5U ||
            paint.len == 7U || paint.len == 9U)) {
        if (paint.len == 4U || paint.len == 5U) {
            for (size_t i = 0U; i < 3U; ++i) {
                int digit = paint_hex_digit(paint.ptr[i + 1U]);
                if (digit < 0) return false;
                rgba[i] = (double)(digit * 17) / 255.0;
            }
            if (paint.len == 5U) {
                int digit = paint_hex_digit(paint.ptr[4U]);
                if (digit < 0) return false;
                rgba[3] = (double)(digit * 17) / 255.0;
            } else rgba[3] = 1.0;
            return true;
        }
        for (size_t i = 0U; i < 3U; ++i)
            if (!paint_byte(paint.ptr[1U + i * 2U],
                    paint.ptr[2U + i * 2U], &rgba[i])) return false;
        if (paint.len == 9U)
            return paint_byte(paint.ptr[7U], paint.ptr[8U], &rgba[3]);
        rgba[3] = 1.0;
        return true;
    }
    struct named_paint { const char *name; uint8_t r, g, b, a; };
    static const struct named_paint names[] = {
        {"black", 0, 0, 0, 255}, {"white", 255, 255, 255, 255},
        {"red", 255, 0, 0, 255}, {"green", 0, 128, 0, 255},
        {"blue", 0, 0, 255, 255}, {"transparent", 0, 0, 0, 0}
    };
    for (size_t i = 0U; i < sizeof(names) / sizeof(names[0]); ++i) {
        size_t length = strlen(names[i].name);
        if (paint.len != length || memcmp(paint.ptr, names[i].name, length) != 0)
            continue;
        rgba[0] = (double)names[i].r / 255.0;
        rgba[1] = (double)names[i].g / 255.0;
        rgba[2] = (double)names[i].b / 255.0;
        rgba[3] = (double)names[i].a / 255.0;
        return true;
    }
    return false;
}

bool viewer_editor_selected_paint_target(viewer_editor *editor,
    const char *property, uint8_t *id_buffer, size_t id_capacity,
    size_t *out_id_length, double out_rgba[4])
{
    if (out_id_length != NULL) *out_id_length = 0U;
    if (out_rgba != NULL) memset(out_rgba, 0, 4U * sizeof(double));
    if (editor == NULL || editor->edit == NULL || property == NULL ||
        id_buffer == NULL || id_capacity == 0U || out_id_length == NULL ||
        out_rgba == NULL || editor->canvas_selection_count > 1U) return false;
    size_t property_length = strlen(property);
    if (!((property_length == 4U && memcmp(property, "fill", 4U) == 0) ||
          (property_length == 6U && memcmp(property, "stroke", 6U) == 0)))
        return false;
    bool has_id = false, has_paint = false, style_paint = false;
    size_t id_length = 0U;
    uint8_t paint_storage[128U];
    size_t paint_length = 0U;
    size_t count = document_edit_attribute_count(editor->edit, editor->selected);
    for (size_t index = 0U; index < count; ++index) {
        document_edit_attribute attribute;
        base_bytes name = {0};
        if (!document_edit_attribute_at(editor->edit, editor->selected, index,
                &attribute) || !document_edit_span_bytes(editor->edit,
                attribute.name, &name)) return false;
        if (name.len == 2U && memcmp(name.ptr, "id", 2U) == 0) {
            if (has_id || !document_edit_decode_span(editor->edit, attribute.value,
                    id_buffer, id_capacity, &id_length) || id_length == 0U ||
                id_length > 256U) return false;
            has_id = true;
        } else if (name.len == property_length &&
                memcmp(name.ptr, property, property_length) == 0) {
            base_bytes paint = {0};
            if (has_paint || !document_edit_span_bytes(editor->edit,
                    attribute.value, &paint) || paint.len == 0U ||
                paint.len >= sizeof(paint_storage)) return false;
            memcpy(paint_storage, paint.ptr, paint.len);
            paint_length = paint.len;
            has_paint = true;
        } else if (name.len == 5U && memcmp(name.ptr, "style", 5U) == 0) {
            uint8_t style[4096U]; size_t style_length = 0U;
            if (!document_edit_decode_span(editor->edit, attribute.value,
                    style, sizeof(style), &style_length)) return false;
            for (size_t cursor = 0U; cursor + property_length <= style_length;
                    ++cursor)
                if ((cursor == 0U || style[cursor - 1U] == ';' ||
                        style[cursor - 1U] == ' ' || style[cursor - 1U] == '\t') &&
                    memcmp(style + cursor, property, property_length) == 0) {
                    size_t after = cursor + property_length;
                    while (after < style_length && (style[after] == ' ' ||
                            style[after] == '\t')) after++;
                    if (after < style_length && style[after] == ':')
                        style_paint = true;
                }
        }
    }
    if (!has_id || !has_paint || style_paint || id_length > id_capacity ||
        !parse_selected_paint((base_bytes){paint_storage, paint_length}, out_rgba))
        return false;
    const document_model *model = document_edit_model(editor->edit);
    if (model == NULL) return false;
    size_t matches = 0U;
    for (size_t slot = 0U; slot < document_model_node_count(model); ++slot) {
        document_handle node = {0};
        if (!document_model_node_at(model, slot, &node)) continue;
        for (size_t i = 0U; i < document_edit_attribute_count(editor->edit, node); ++i) {
            document_edit_attribute attribute; base_bytes name = {0};
            if (!document_edit_attribute_at(editor->edit, node, i, &attribute) ||
                !document_edit_span_bytes(editor->edit, attribute.name, &name))
                return false;
            if (name.len != 2U || memcmp(name.ptr, "id", 2U) != 0) continue;
            uint8_t decoded[257U]; size_t length = 0U;
            if (!document_edit_decode_span(editor->edit, attribute.value,
                    decoded, sizeof(decoded), &length)) return false;
            if (length == id_length && memcmp(decoded, id_buffer, length) == 0) {
                matches++;
                if (node.document_id != editor->selected.document_id ||
                    node.slot != editor->selected.slot ||
                    node.generation != editor->selected.generation) return false;
            }
        }
    }
    if (matches != 1U) return false;
    *out_id_length = id_length;
    return true;
}

