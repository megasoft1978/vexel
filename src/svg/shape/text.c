#include "shape_internal.h"

#include <math.h>
#include <stdlib.h>
#include <string.h>

typedef struct outline_context {
    const svg_shape_text_layout *layout;
    const double *x_values;
    size_t x_count;
    const double *y_values;
    size_t y_count;
    const double *dx_values;
    size_t dx_count;
    const double *dy_values;
    size_t dy_count;
    double start_x;
    double start_y;
    double anchor_offset;
    double unit_scale;
    double word_spacing;
    const uint8_t *text;
    size_t text_length;
} outline_context;

static bool text_span_equal(base_bytes source, svg_xml_span span,
                            const char *literal)
{
    size_t length = 0U;
    while (literal[length] != '\0') length++;
    return span.offset <= source.len && span.length == length &&
           length <= source.len - span.offset &&
           memcmp(source.ptr + span.offset, literal, length) == 0;
}

static bool text_attribute(base_bytes source, const svg_xml_token *tokens,
                           size_t count, size_t element, const char *local,
                           const char *prefix, svg_xml_span *out)
{
    for (size_t i = element + 1U;
         i < count && tokens[i].kind == SVG_XML_TOKEN_ATTRIBUTE; ++i) {
        if (!text_span_equal(source, tokens[i].local_name, local)) continue;
        if ((prefix == NULL && tokens[i].prefix.length == 0U) ||
            (prefix != NULL && text_span_equal(source, tokens[i].prefix,
                                                prefix))) {
            *out = tokens[i].value;
            return true;
        }
    }
    return false;
}

static bool normalize_text(base_bytes source, svg_xml_span raw,
                           bool preserve, svg_shape_text_state *state,
                           uint8_t **out_bytes, size_t *out_length,
                           size_t *error_offset)
{
    *out_bytes = NULL;
    *out_length = 0U;
    if (raw.offset > source.len || raw.length > source.len - raw.offset)
        return false;
    uint8_t *decoded = svg_shape_malloc(raw.length == 0U ? 1U : raw.length);
    if (decoded == NULL) return false;
    size_t decoded_length = 0U;
    size_t decode_error = 0U;
    if (!svg_xml_decode_attribute_value(source, raw, decoded, raw.length,
                                        &decoded_length, &decode_error)) {
        *error_offset = raw.offset + decode_error;
        svg_shape_free(decoded);
        return false;
    }
    if (preserve) {
        if (decoded_length != 0U) state->has_position = true;
        *out_bytes = decoded;
        *out_length = decoded_length;
        return true;
    }
    uint8_t *collapsed = svg_shape_malloc(decoded_length == 0U ? 1U :
                                           decoded_length);
    if (collapsed == NULL) {
        svg_shape_free(decoded);
        return false;
    }
    size_t output = 0U;
    size_t i = 0U;
    while (i < decoded_length) {
        uint8_t byte = decoded[i];
        bool whitespace = byte == ' ' || byte == '\t' || byte == '\r' ||
                          byte == '\n';
        if (whitespace) {
            if (state->has_position || output != 0U) state->pending_space = true;
            i++;
            continue;
        }
        if (state->pending_space && (state->has_position || output != 0U))
            collapsed[output++] = ' ';
        state->pending_space = false;
        collapsed[output++] = byte;
        state->has_position = true;
        i++;
    }
    svg_shape_free(decoded);
    *out_bytes = collapsed;
    *out_length = output;
    return true;
}

static bool parse_position_list(base_bytes source, svg_xml_span span,
                                double reference, double **out_values,
                                size_t *out_count, size_t *error_offset)
{
    *out_values = NULL;
    *out_count = 0U;
    if (span.length == 0U || span.offset > source.len ||
        span.length > source.len - span.offset) {
        *error_offset = span.offset;
        return false;
    }
    size_t capacity = span.length < SVG_SHAPE_MEDIA_MAX_GLYPHS ?
                      span.length : SVG_SHAPE_MEDIA_MAX_GLYPHS;
    if (capacity == 0U || capacity > SIZE_MAX / sizeof(double)) {
        *error_offset = span.offset;
        return false;
    }
    double *values = svg_shape_malloc(capacity * sizeof(*values));
    if (values == NULL) return false;
    size_t i = 0U, count = 0U;
    bool need_value = true;
    while (i < span.length) {
        while (i < span.length && (source.ptr[span.offset + i] == ' ' ||
               source.ptr[span.offset + i] == '\t' ||
               source.ptr[span.offset + i] == '\r' ||
               source.ptr[span.offset + i] == '\n')) i++;
        if (i == span.length) break;
        if (source.ptr[span.offset + i] == ',') {
            if (need_value) { *error_offset = span.offset + i; goto invalid; }
            need_value = true;
            i++;
            continue;
        }
        size_t start = i;
        while (i < span.length && source.ptr[span.offset + i] != ',' &&
               source.ptr[span.offset + i] != ' ' &&
               source.ptr[span.offset + i] != '\t' &&
               source.ptr[span.offset + i] != '\r' &&
               source.ptr[span.offset + i] != '\n') i++;
        if (count == capacity || start == i ||
            !svg_shape_read_length(source,
                (svg_xml_span){span.offset + start, i - start}, reference,
                &values[count])) {
            *error_offset = span.offset + start;
            goto invalid;
        }
        count++;
        need_value = false;
    }
    if (count == 0U || need_value) {
        *error_offset = span.offset + span.length;
        goto invalid;
    }
    *out_values = values;
    *out_count = count;
    return true;
invalid:
    svg_shape_free(values);
    return false;
}

static bool parse_families(base_bytes source, const svg_css_paint *style,
                           svg_shape_text_family families[
                               SVG_SHAPE_MEDIA_MAX_FAMILIES],
                           size_t *out_count, size_t *error_offset)
{
    *out_count = 0U;
    svg_xml_span span;
    if (!svg_css_paint_value(style, SVG_CSS_FONT_FAMILY, &span)) return true;
    if (span.offset > source.len || span.length > source.len - span.offset) {
        *error_offset = span.offset;
        return false;
    }
    size_t i = span.offset, end = span.offset + span.length;
    while (i < end) {
        while (i < end && (source.ptr[i] == ' ' || source.ptr[i] == '\t' ||
                           source.ptr[i] == '\r' || source.ptr[i] == '\n')) i++;
        if (i == end) break;
        if (*out_count >= SVG_SHAPE_MEDIA_MAX_FAMILIES) {
            *error_offset = i;
            return false;
        }
        size_t start = i, finish;
        uint8_t quote = source.ptr[i] == '\'' || source.ptr[i] == '"' ?
                        source.ptr[i++] : 0U;
        if (quote != 0U) {
            start = i;
            while (i < end && source.ptr[i] != quote) i++;
            if (i == end) { *error_offset = start - 1U; return false; }
            finish = i;
            i++;
        } else {
            while (i < end && source.ptr[i] != ',') i++;
            finish = i;
            while (finish > start &&
                   (source.ptr[finish - 1U] == ' ' ||
                    source.ptr[finish - 1U] == '\t' ||
                    source.ptr[finish - 1U] == '\r' ||
                    source.ptr[finish - 1U] == '\n')) finish--;
        }
        if (finish == start) { *error_offset = start; return false; }
        families[*out_count] = (svg_shape_text_family){
            source.ptr + start, finish - start};
        (*out_count)++;
        while (i < end && source.ptr[i] != ',') {
            if (source.ptr[i] != ' ' && source.ptr[i] != '\t' &&
                source.ptr[i] != '\r' && source.ptr[i] != '\n') {
                *error_offset = i;
                return false;
            }
            i++;
        }
        if (i < end && source.ptr[i] == ',') i++;
    }
    return *out_count != 0U;
}

static bool parse_size(base_bytes source, const svg_css_paint *style,
                       double reference, double *out, size_t *error_offset)
{
    svg_xml_span span;
    *out = 16.0;
    if (!svg_css_paint_value(style, SVG_CSS_FONT_SIZE, &span)) return true;
    if (!svg_shape_read_length(source, span, reference, out) || *out <= 0.0) {
        *error_offset = span.offset;
        return false;
    }
    return true;
}

static bool parse_weight(base_bytes source, const svg_css_paint *style,
                         double *out, size_t *error_offset)
{
    svg_xml_span span;
    *out = 400.0;
    if (!svg_css_paint_value(style, SVG_CSS_FONT_WEIGHT, &span)) return true;
    if (svg_css_keyword_equals(source, span, "normal")) return true;
    if (svg_css_keyword_equals(source, span, "bold")) { *out = 700.0; return true; }
    if (!svg_shape_read_number(source, span, out) || *out < 100.0 ||
        *out > 900.0 || fmod(*out, 100.0) != 0.0) {
        *error_offset = span.offset;
        return false;
    }
    return true;
}

static bool parse_spacing(base_bytes source, const svg_css_paint *style,
                          svg_css_property property, double reference,
                          double *out, size_t *error_offset)
{
    svg_xml_span span;
    *out = 0.0;
    if (!svg_css_paint_value(style, property, &span) ||
        svg_css_keyword_equals(source, span, "normal")) return true;
    if (!svg_shape_read_length(source, span, reference, out)) {
        *error_offset = span.offset;
        return false;
    }
    return true;
}

static bool parse_text_style(base_bytes source, const svg_css_paint *style,
                             double viewport_width,
                             double viewport_height,
                             svg_shape_text_request *request,
                             svg_shape_text_family families[
                                 SVG_SHAPE_MEDIA_MAX_FAMILIES],
                             size_t *family_count, double *word_spacing,
                             bool *anchor_middle, bool *anchor_end,
                             size_t *error_offset)
{
    double font_size = 16.0;
    if (!parse_size(source, style, viewport_height, &font_size, error_offset) ||
        !parse_weight(source, style, &request->weight, error_offset) ||
        !parse_families(source, style, families, family_count, error_offset) ||
        !parse_spacing(source, style, SVG_CSS_LETTER_SPACING,
                       viewport_width, &request->letter_spacing_points,
                       error_offset) ||
        !parse_spacing(source, style, SVG_CSS_WORD_SPACING,
                       viewport_width, word_spacing, error_offset)) return false;
    request->font_families = families;
    request->font_family_count = *family_count;
    request->size_points = font_size * 0.75;
    request->letter_spacing_points *= 0.75;
    request->style = SVG_SHAPE_TEXT_NORMAL;
    svg_xml_span span;
    if (svg_css_paint_value(style, SVG_CSS_FONT_STYLE, &span)) {
        if (svg_css_keyword_equals(source, span, "italic"))
            request->style = SVG_SHAPE_TEXT_ITALIC;
        else if (svg_css_keyword_equals(source, span, "oblique"))
            request->style = SVG_SHAPE_TEXT_OBLIQUE;
        else if (!svg_css_keyword_equals(source, span, "normal")) {
            *error_offset = span.offset;
            return false;
        }
    }
    *anchor_middle = false;
    *anchor_end = false;
    if (svg_css_paint_value(style, SVG_CSS_TEXT_ANCHOR, &span)) {
        if (svg_css_keyword_equals(source, span, "middle"))
            *anchor_middle = true;
        else if (svg_css_keyword_equals(source, span, "end"))
            *anchor_end = true;
        else if (!svg_css_keyword_equals(source, span, "start")) {
            *error_offset = span.offset;
            return false;
        }
    }
    return true;
}

static bool text_geometry(svg_shape_builder *builder, void *opaque)
{
    outline_context *context = opaque;
    const svg_shape_text_layout *layout = context->layout;
    size_t codepoint = 0U;
    double extra_x = 0.0, extra_y = 0.0, word_offset = 0.0;
    for (size_t glyph_index = 0U; glyph_index < layout->glyph_count;
         ++glyph_index) {
        const svg_shape_text_glyph *glyph = &layout->glyphs[glyph_index];
        bool x_override = glyph_index < context->x_count;
        bool y_override = glyph_index < context->y_count;
        if (glyph_index < context->dx_count) extra_x += context->dx_values[glyph_index];
        if (glyph_index < context->dy_count) extra_y += context->dy_values[glyph_index];
        if (x_override) extra_x = 0.0;
        if (y_override) extra_y = 0.0;
        double gx = (x_override ? context->x_values[glyph_index] :
                     context->start_x + glyph->x * context->unit_scale) + extra_x +
                    context->anchor_offset + word_offset;
        double gy = (y_override ? context->y_values[glyph_index] :
                     context->start_y - glyph->y * context->unit_scale) + extra_y;
        while (codepoint < context->text_length &&
               (context->text[codepoint] & 0xC0U) == 0x80U) codepoint++;
        if (codepoint < context->text_length && context->text[codepoint] == ' ')
            word_offset += context->word_spacing;
        while (codepoint < context->text_length &&
               (context->text[codepoint] & 0xC0U) != 0x80U) codepoint++;
        geom_point current = {0.0, 0.0}, first = {0.0, 0.0};
        bool open = false;
        size_t end = glyph->command_start + glyph->command_count;
        for (size_t index = glyph->command_start; index < end; ++index) {
            const svg_shape_text_command *command = &layout->commands[index];
            if (command->verb == SVG_SHAPE_TEXT_MOVE) {
                if (open && !svg_shape_builder_add_edge(builder, current, first))
                    return false;
                current = (geom_point){gx + command->values[0] * context->unit_scale,
                                       gy - command->values[1] * context->unit_scale};
                first = current;
                open = true;
            } else if (command->verb == SVG_SHAPE_TEXT_LINE) {
                geom_point next = {gx + command->values[0] * context->unit_scale,
                                   gy - command->values[1] * context->unit_scale};
                if (!open) { first = current; open = true; }
                if (!svg_shape_builder_add_edge(builder, current, next))
                    return false;
                current = next;
            } else if (command->verb == SVG_SHAPE_TEXT_CUBIC) {
                geom_point c1 = {gx + command->values[0] * context->unit_scale,
                                 gy - command->values[1] * context->unit_scale};
                geom_point c2 = {gx + command->values[2] * context->unit_scale,
                                 gy - command->values[3] * context->unit_scale};
                geom_point next = {gx + command->values[4] * context->unit_scale,
                                   gy - command->values[5] * context->unit_scale};
                if (!open) { first = current; open = true; }
                if (!svg_shape_builder_add_curve(builder, current, c1, c2,
                                                 next)) return false;
                current = next;
            } else {
                if (open && !svg_shape_builder_add_edge(builder, current, first))
                    return false;
                current = first;
                open = false;
            }
        }
        if (open && !svg_shape_builder_add_edge(builder, current, first))
            return false;
    }
    return true;
}

static bool parse_xml_space(base_bytes source, const svg_xml_token *tokens,
                            size_t count, size_t element,
                            bool parent_preserve, bool *out_preserve,
                            size_t *error_offset)
{
    svg_xml_span span;
    *out_preserve = parent_preserve;
    if (!text_attribute(source, tokens, count, element, "space", "xml",
                        &span)) return true;
    if (svg_css_keyword_equals(source, span, "preserve")) {
        *out_preserve = true;
        return true;
    }
    if (svg_css_keyword_equals(source, span, "default")) {
        *out_preserve = false;
        return true;
    }
    *error_offset = span.offset;
    return false;
}

bool svg_shape_text_render_element(
    svg_shape_builder *builder, base_bytes source, svg_xml_token *tokens,
    size_t token_count, size_t element, size_t end,
    const svg_css_paint *style, geom_affine transform,
    size_t source_offset, size_t source_length,
    svg_shape_text_state *state, bool parent_preserve_space,
    bool *out_preserve_space)
{
    if (builder == NULL || style == NULL || state == NULL ||
        out_preserve_space == NULL || element >= token_count || end >= token_count) {
        svg_shape_fail(builder == NULL ? NULL : builder->diag,
                       SVG_SHAPE_ERROR_INVALID_ARGUMENT, 0U);
        return false;
    }
    size_t error_offset = tokens[element].name.offset;
    bool preserve = parent_preserve_space;
    if (!parse_xml_space(source, tokens, token_count, element,
                         parent_preserve_space, &preserve, &error_offset)) {
        svg_shape_fail(builder->diag, SVG_SHAPE_ERROR_UNSUPPORTED,
                       error_offset);
        return false;
    }
    svg_shape_text_family families[SVG_SHAPE_MEDIA_MAX_FAMILIES] = {{0}};
    size_t family_count = 0U;
    svg_shape_text_request request = {0};
    double word_spacing = 0.0;
    bool anchor_middle = false, anchor_end = false;
    if (!parse_text_style(source, style, builder->viewport_width,
                          builder->viewport_height, &request, families,
                          &family_count, &word_spacing, &anchor_middle,
                          &anchor_end, &error_offset)) {
        svg_shape_fail(builder->diag, SVG_SHAPE_ERROR_UNSUPPORTED,
                       error_offset);
        return false;
    }
    for (size_t i = element + 1U; i < end; ++i) {
        const svg_xml_token *token = &tokens[i];
        if ((token->kind != SVG_XML_TOKEN_TEXT &&
             token->kind != SVG_XML_TOKEN_CDATA) ||
            token->depth != tokens[element].depth + 1U) continue;
        if (token->value.length > SVG_SHAPE_MEDIA_MAX_TEXT_BYTES ||
            token->value.length > SVG_SHAPE_MEDIA_MAX_DOCUMENT_TEXT_BYTES -
                                  builder->text_bytes) {
            svg_shape_fail(builder->diag, SVG_SHAPE_ERROR_LIMIT,
                           token->value.offset);
            return false;
        }
        builder->text_bytes += token->value.length;
        uint8_t *normalized = NULL;
        size_t normalized_length = 0U;
        size_t entity_error = token->value.offset;
        if (!normalize_text(source, token->value, preserve, state,
                            &normalized, &normalized_length, &entity_error)) {
            svg_shape_fail(builder->diag, entity_error == token->value.offset ?
                SVG_SHAPE_ERROR_OOM : SVG_SHAPE_ERROR_UNSUPPORTED,
                entity_error);
            return false;
        }
        if (normalized_length == 0U) {
            svg_shape_free(normalized);
            continue;
        }
        request.text = (base_bytes){normalized, normalized_length};
        svg_shape_text_layout layout = {0};
        bool cached = false;
        if (!svg_shape_text_layout_run(builder->text_cache, &request,
                                       &layout, &cached,
                         builder->diag, token->value.offset)) {
            svg_shape_free(normalized);
            return false;
        }
        if (layout.glyph_count > SVG_SHAPE_MEDIA_MAX_DOCUMENT_GLYPHS -
                                 builder->text_glyphs ||
            layout.command_count > SVG_SHAPE_MEDIA_MAX_DOCUMENT_PATH_COMMANDS -
                                   builder->text_commands) {
            if (!cached) svg_shape_text_layout_release(builder->media, &layout);
            svg_shape_free(normalized);
            svg_shape_fail(builder->diag, SVG_SHAPE_ERROR_LIMIT,
                           token->value.offset);
            return false;
        }
        builder->text_glyphs += layout.glyph_count;
        builder->text_commands += layout.command_count;
        size_t outline_commands = 0U;
        for (size_t g = 0U; g < layout.glyph_count; ++g)
            outline_commands += layout.glyphs[g].command_count;
        double x_reference = builder->viewport_width;
        double y_reference = builder->viewport_height;
        double *x_values = NULL, *y_values = NULL;
        double *dx_values = NULL, *dy_values = NULL;
        size_t x_count = 0U, y_count = 0U, dx_count = 0U, dy_count = 0U;
        const char *names[] = {"x", "y", "dx", "dy"};
        double **arrays[] = {&x_values, &y_values, &dx_values, &dy_values};
        size_t *counts[] = {&x_count, &y_count, &dx_count, &dy_count};
        double references[] = {x_reference, y_reference,
                               x_reference, y_reference};
        bool lists_ok = true;
        for (size_t list = 0U; list < 4U; ++list) {
            svg_xml_span position;
            if (text_attribute(source, tokens, token_count, element,
                               names[list], NULL, &position) &&
                !parse_position_list(source, position, references[list],
                    arrays[list], counts[list], &error_offset)) {
                lists_ok = false;
                break;
            }
        }
        if (!lists_ok) {
            svg_shape_free(x_values); svg_shape_free(y_values);
            svg_shape_free(dx_values); svg_shape_free(dy_values);
            if (!cached) svg_shape_text_layout_release(builder->media, &layout);
            svg_shape_free(normalized);
            svg_shape_fail(builder->diag, SVG_SHAPE_ERROR_NUMBER,
                           error_offset);
            return false;
        }
        double start_x = state->cursor_x;
        double start_y = state->cursor_y;
        if (x_count != 0U) start_x = x_values[0];
        if (y_count != 0U) start_y = y_values[0];
        if (dx_count != 0U) start_x += dx_values[0];
        if (dy_count != 0U) start_y += dy_values[0];
        const double unit_scale = 96.0 / 72.0;
        double advance_units = layout.advance_width * unit_scale;
        double anchor = anchor_middle ? -advance_units * 0.5 :
                        anchor_end ? -advance_units : 0.0;
        size_t codepoints = 0U, spaces = 0U;
        for (size_t p = 0U; p < normalized_length; ++p) {
            if ((normalized[p] & 0xC0U) != 0x80U) {
                codepoints++;
                if (normalized[p] == ' ') spaces++;
            }
        }
        if (word_spacing != 0.0 && codepoints != layout.glyph_count) {
            svg_shape_free(x_values); svg_shape_free(y_values);
            svg_shape_free(dx_values); svg_shape_free(dy_values);
            if (!cached) svg_shape_text_layout_release(builder->media, &layout);
            svg_shape_free(normalized);
            svg_shape_fail(builder->diag, SVG_SHAPE_ERROR_UNSUPPORTED,
                           token->value.offset);
            return false;
        }
        if (outline_commands != 0U) {
            outline_context context = {
                .layout = &layout,
                .x_values = x_values, .x_count = x_count,
                .y_values = y_values, .y_count = y_count,
                .dx_values = dx_values, .dx_count = dx_count,
                .dy_values = dy_values, .dy_count = dy_count,
                .start_x = start_x, .start_y = start_y,
                .anchor_offset = anchor, .unit_scale = unit_scale,
                .word_spacing = word_spacing,
                .text = normalized, .text_length = normalized_length
            };
            size_t first_shape = builder->doc->shape_count;
            if (!svg_shape_builder_add_custom_element(
                    builder, source, tokens, token_count, element, style,
                    transform, text_geometry, &context) ||
                !svg_shape_source_assign(builder->doc, first_shape,
                    builder->doc->shape_count, source_offset, source_length)) {
                svg_shape_free(x_values); svg_shape_free(y_values);
                svg_shape_free(dx_values); svg_shape_free(dy_values);
                if (!cached) svg_shape_text_layout_release(builder->media, &layout);
                svg_shape_free(normalized);
                return false;
            }
        }
        state->cursor_x = start_x + advance_units +
                          (double)spaces * word_spacing;
        state->cursor_y = start_y;
        state->has_position = true;
        svg_shape_free(x_values); svg_shape_free(y_values);
        svg_shape_free(dx_values); svg_shape_free(dy_values);
        if (!cached) svg_shape_text_layout_release(builder->media, &layout);
        svg_shape_free(normalized);
    }
    *out_preserve_space = preserve;
    return true;
}
