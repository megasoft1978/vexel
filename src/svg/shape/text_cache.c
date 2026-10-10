#include "shape_internal.h"

#include <math.h>
#include <stdlib.h>
#include <string.h>

#define TEXT_CACHE_ENTRIES 32U
#define TEXT_CACHE_BYTES (256U * 1024U)
#define TEXT_CACHE_COMMANDS SVG_SHAPE_MEDIA_MAX_PATH_COMMANDS

typedef struct text_cache_entry {
    uint8_t *text;
    size_t text_length;
    svg_shape_text_family *families;
    size_t family_count;
    double size_points;
    double weight;
    double letter_spacing_points;
    svg_shape_text_style style;
    svg_shape_text_layout layout;
} text_cache_entry;

struct svg_shape_text_cache {
    const svg_shape_media_providers *providers;
    text_cache_entry entries[TEXT_CACHE_ENTRIES];
    size_t count;
    size_t bytes;
    size_t commands;
};

void svg_shape_text_layout_release(const svg_shape_media_providers *providers,
                                    svg_shape_text_layout *layout)
{
    if (providers->release_text != NULL)
        providers->release_text(providers->context, layout);
    else {
        free(layout->glyphs);
        free(layout->commands);
        *layout = (svg_shape_text_layout){0};
    }
}

svg_shape_text_cache *svg_shape_text_cache_create(
    const svg_shape_media_providers *providers)
{
    if (providers == NULL || providers->layout_text == NULL) return NULL;
    svg_shape_text_cache *cache = svg_shape_calloc(1U, sizeof(*cache));
    if (cache != NULL) cache->providers = providers;
    return cache;
}

void svg_shape_text_cache_destroy(svg_shape_text_cache *cache)
{
    if (cache == NULL) return;
    for (size_t i = 0U; i < cache->count; ++i) {
        svg_shape_text_layout_release(cache->providers, &cache->entries[i].layout);
        svg_shape_free(cache->entries[i].text);
        svg_shape_free(cache->entries[i].families);
    }
    svg_shape_free(cache);
}

static bool request_matches(const text_cache_entry *entry,
                            const svg_shape_text_request *request)
{
    if (entry->text_length != request->text.len ||
        entry->family_count != request->font_family_count ||
        entry->size_points != request->size_points ||
        entry->weight != request->weight ||
        entry->style != request->style ||
        entry->letter_spacing_points != request->letter_spacing_points ||
        memcmp(entry->text, request->text.ptr, request->text.len) != 0)
        return false;
    for (size_t i = 0U; i < entry->family_count; ++i) {
        const svg_shape_text_family *a = &entry->families[i];
        const svg_shape_text_family *b = &request->font_families[i];
        if (a->length != b->length || memcmp(a->bytes, b->bytes, a->length) != 0)
            return false;
    }
    return true;
}

bool svg_shape_text_layout_run(svg_shape_text_cache *cache,
                        const svg_shape_text_request *request,
                        svg_shape_text_layout *out, bool *cached,
                        svg_shape_diagnostic *diagnostic,
                        size_t error_offset)
{
    *out = (svg_shape_text_layout){0};
    *cached = false;
    if (cache == NULL || cache->providers->layout_text == NULL ||
        cache->providers->release_text == NULL) {
        svg_shape_fail(diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED, error_offset);
        return false;
    }
    for (size_t i = 0U; i < cache->count; ++i) {
        if (request_matches(&cache->entries[i], request)) {
            *out = cache->entries[i].layout;
            *cached = true;
            return true;
        }
    }
    svg_shape_media_status status = cache->providers->layout_text(
        cache->providers->context, request, out);
    if (status != SVG_SHAPE_MEDIA_OK) {
        svg_shape_error error = status == SVG_SHAPE_MEDIA_LIMIT ?
            SVG_SHAPE_ERROR_LIMIT : status == SVG_SHAPE_MEDIA_OOM ?
            SVG_SHAPE_ERROR_OOM : SVG_SHAPE_ERROR_UNSUPPORTED;
        svg_shape_fail(diagnostic, error, error_offset);
        return false;
    }
    if (out->glyph_count > SVG_SHAPE_MEDIA_MAX_GLYPHS ||
        out->command_count > SVG_SHAPE_MEDIA_MAX_PATH_COMMANDS ||
        (out->glyph_count != 0U && out->glyphs == NULL) ||
        (out->command_count != 0U && out->commands == NULL) ||
        !isfinite(out->advance_width)) {
        svg_shape_text_layout_release(cache->providers, out);
        svg_shape_fail(diagnostic, SVG_SHAPE_ERROR_LIMIT, error_offset);
        return false;
    }
    for (size_t i = 0U; i < out->glyph_count; ++i) {
        const svg_shape_text_glyph *glyph = &out->glyphs[i];
        if (!isfinite(glyph->x) || !isfinite(glyph->y) ||
            !isfinite(glyph->advance) || glyph->command_start > out->command_count ||
            glyph->command_count > out->command_count - glyph->command_start) {
            svg_shape_text_layout_release(cache->providers, out);
            svg_shape_fail(diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED,
                           error_offset);
            return false;
        }
    }
    for (size_t i = 0U; i < out->command_count; ++i) {
        const svg_shape_text_command *command = &out->commands[i];
        if ((unsigned)command->verb > (unsigned)SVG_SHAPE_TEXT_CLOSE) {
            svg_shape_text_layout_release(cache->providers, out);
            svg_shape_fail(diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED,
                           error_offset);
            return false;
        }
        for (size_t value = 0U; value < 6U; ++value) {
            if (!isfinite(command->values[value])) {
                svg_shape_text_layout_release(cache->providers, out);
                svg_shape_fail(diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED,
                               error_offset);
                return false;
            }
        }
    }
    if (cache->count < TEXT_CACHE_ENTRIES &&
        request->text.len <= TEXT_CACHE_BYTES - cache->bytes &&
        out->command_count <= TEXT_CACHE_COMMANDS - cache->commands &&
        request->font_family_count <= SVG_SHAPE_MEDIA_MAX_FAMILIES) {
        uint8_t *key_text = svg_shape_malloc(request->text.len == 0U ? 1U :
                                              request->text.len);
        svg_shape_text_family *families = request->font_family_count == 0U ?
            NULL : svg_shape_malloc(request->font_family_count * sizeof(*families));
        if (key_text != NULL &&
            (request->font_family_count == 0U || families != NULL)) {
            if (request->text.len != 0U)
                memcpy(key_text, request->text.ptr, request->text.len);
            if (families != NULL)
                memcpy(families, request->font_families,
                       request->font_family_count * sizeof(*families));
            cache->entries[cache->count] = (text_cache_entry){
                .text = key_text,
                .text_length = request->text.len,
                .families = families,
                .family_count = request->font_family_count,
                .size_points = request->size_points,
                .weight = request->weight,
                .letter_spacing_points = request->letter_spacing_points,
                .style = request->style,
                .layout = *out
            };
            cache->count++;
            cache->bytes += request->text.len;
            cache->commands += out->command_count;
            *cached = true;
        } else {
            svg_shape_free(key_text);
            svg_shape_free(families);
        }
    }
    return true;
}

