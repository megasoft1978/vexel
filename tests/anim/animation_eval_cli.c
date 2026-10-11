#include "base.h"
#include "svg/animation/animation.h"
#include "xml_tokenizer.h"

#include <errno.h>
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define CLI_MAX_SOURCE (16U * 1024U * 1024U)

static bool read_source(const char *path, uint8_t **out, size_t *length)
{
    FILE *file = fopen(path, "rb");
    if (file == NULL) return false;
    bool okay = fseek(file, 0L, SEEK_END) == 0;
    long file_length = okay ? ftell(file) : -1L;
    if (file_length <= 0L || (unsigned long)file_length > CLI_MAX_SOURCE ||
        fseek(file, 0L, SEEK_SET) != 0) okay = false;
    uint8_t *bytes = okay ? malloc((size_t)file_length) : NULL;
    if (bytes == NULL) okay = false;
    if (okay) okay = fread(bytes, 1U, (size_t)file_length, file) == (size_t)file_length;
    if (fclose(file) != 0) okay = false;
    if (!okay) {
        free(bytes);
        return false;
    }
    *out = bytes;
    *length = (size_t)file_length;
    return true;
}

static bool parse_time(const char *text, double *out)
{
    if (text == NULL || text[0] == '\0') return false;
    errno = 0;
    char *end = NULL;
    double value = strtod(text, &end);
    if (errno != 0 || end == text || *end != '\0' || !isfinite(value) ||
        value < 0.0 || value > SVG_ANIM_MAX_TIMELINE_SECONDS) return false;
    *out = value;
    return true;
}

static bool print_value(const svg_anim_value *value)
{
    if (value->kind == SVG_ANIM_VALUE_NUMBER)
        return printf("{\"kind\":\"number\",\"value\":%.17g}",
                      value->number[0]) >= 0;
    if (value->kind == SVG_ANIM_VALUE_COLOR)
        return printf("{\"kind\":\"color\",\"value\":[%u,%u,%u,%u]}",
            value->rgba[0], value->rgba[1], value->rgba[2], value->rgba[3]) >= 0;
    if (value->kind == SVG_ANIM_VALUE_TRANSFORM)
        return printf("{\"kind\":\"transform\",\"value\":[%.17g,%.17g,%.17g,%.17g]}",
            value->number[0], value->number[1], value->number[2], value->number[3]) >= 0;
    if (value->kind == SVG_ANIM_VALUE_MOTION)
        return printf("{\"kind\":\"motion\",\"value\":[%.17g,%.17g,%.17g]}",
            value->number[0], value->number[1], value->number[2]) >= 0;
    return fputs("{\"kind\":\"path\"}", stdout) >= 0;
}

int main(int argc, char **argv)
{
    if (argc != 3) return 2;
    double seconds = 0.0;
    uint8_t *source_memory = NULL;
    size_t source_length = 0U;
    if (!parse_time(argv[2], &seconds) ||
        !read_source(argv[1], &source_memory, &source_length)) return 2;
    base_bytes source = {source_memory, source_length};
    size_t token_capacity = source_length / 2U + 2U;
    if (token_capacity > SIZE_MAX / sizeof(svg_xml_token)) {
        free(source_memory);
        return 2;
    }
    svg_xml_token *tokens = calloc(token_capacity, sizeof(*tokens));
    if (tokens == NULL) {
        free(source_memory);
        return 2;
    }
    svg_xml_result xml = svg_xml_tokenize(source, svg_xml_default_options(),
                                          tokens, token_capacity);
    svg_anim_document document = {0};
    svg_anim_overlay overlay = {0};
    size_t error_offset = 0U;
    bool okay = xml.diagnostic.category == SVG_XML_ERROR_NONE &&
        svg_anim_parse(source, tokens, xml.token_count, &document, &error_offset) &&
        svg_anim_eval(&document, seconds, &overlay);
    if (okay) {
        okay = printf("{\"time\":%.17g,\"tracks\":[", seconds) >= 0;
        for (size_t index = 0U; okay && index < overlay.count; ++index) {
            const svg_anim_track *track = &document.tracks[index];
            const svg_anim_overlay_item *item = &overlay.items[index];
            if (index > 0U) okay = fputc(',', stdout) >= 0;
            if (okay) okay = printf("{\"attribute\":\"%s\",\"active\":%s,\"iteration\":%zu,\"sample\":",
                svg_anim_attribute_name(track->attribute), item->active ? "true" : "false",
                item->iteration) >= 0;
            if (okay && item->active) okay = print_value(&item->value);
            else if (okay) okay = fputs("null", stdout) >= 0;
            if (okay) okay = fputc('}', stdout) >= 0;
        }
        if (okay) okay = printf("],\"diagnostic_count\":%zu}\n",
                                  document.diagnostic_count) >= 0;
    }
    svg_anim_overlay_destroy(&overlay);
    svg_anim_document_destroy(&document);
    free(tokens);
    free(source_memory);
    return okay ? 0 : 1;
}
