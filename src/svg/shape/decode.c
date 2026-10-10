#include "traverse_internal.h"

#include "reference.h"
#include "../paint/paint.h"

#include <stdint.h>

svg_shape_error svg_shape_decode_source(base_bytes source,
                                        svg_shape_document *out,
                                        svg_shape_diagnostic *diagnostic)
{
    return svg_shape_decode_source_with_bounds(source, out, diagnostic, NULL);
}

svg_shape_error svg_shape_decode_source_with_media(
    base_bytes source, const svg_shape_media_providers *providers,
    svg_shape_document *out, svg_shape_diagnostic *diagnostic)
{
    return svg_shape_decode_source_with_media_and_bounds(
        source, providers, out, diagnostic, NULL);
}

svg_shape_error svg_shape_decode_source_with_bounds(
    base_bytes source, svg_shape_document *out,
    svg_shape_diagnostic *diagnostic,
    svg_shape_decode_memory_bounds *memory_bounds)
{
    return svg_shape_decode_source_with_media_and_bounds(
        source, NULL, out, diagnostic, memory_bounds);
}

static svg_shape_error decode_source_with_language_and_media_and_bounds(
    base_bytes source, const svg_shape_media_providers *providers,
    svg_shape_document *out, svg_shape_diagnostic *diagnostic,
    svg_shape_decode_memory_bounds *memory_bounds,
    base_bytes system_language)
{
    svg_shape_diagnostic local_diagnostic;
    if (diagnostic == NULL) diagnostic = &local_diagnostic;
    if (memory_bounds != NULL)
        *memory_bounds = (svg_shape_decode_memory_bounds){0};
    if (out == NULL || !base_bytes_valid(source) ||
        !base_bytes_valid(system_language) || system_language.len == 0U ||
        system_language.len > 64U) {
        svg_shape_traverse_fail(diagnostic, SVG_SHAPE_ERROR_INVALID_ARGUMENT, 0U);
        return SVG_SHAPE_ERROR_INVALID_ARGUMENT;
    }
    for (size_t i = 0U; i < system_language.len; ++i) {
        uint8_t byte = system_language.ptr[i];
        bool valid = (byte >= 'a' && byte <= 'z') ||
                     (byte >= 'A' && byte <= 'Z') ||
                     (byte >= '0' && byte <= '9') || byte == '-';
        if (!valid || (byte == '-' && (i == 0U ||
                                      i + 1U == system_language.len ||
                                      system_language.ptr[i - 1U] == '-'))) {
            svg_shape_traverse_fail(diagnostic,
                SVG_SHAPE_ERROR_INVALID_ARGUMENT, i);
            return SVG_SHAPE_ERROR_INVALID_ARGUMENT;
        }
    }
    *out = (svg_shape_document){0};
    *diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_NONE, 0U};
    if (source.len > SHAPE_MAX_BYTES) {
        svg_shape_traverse_fail(diagnostic, SVG_SHAPE_ERROR_LIMIT, SHAPE_MAX_BYTES);
        return SVG_SHAPE_ERROR_LIMIT;
    }
    size_t capacity = source.len / 2U + 2U;
    if (capacity > SIZE_MAX / sizeof(svg_xml_token)) {
        svg_shape_traverse_fail(diagnostic, SVG_SHAPE_ERROR_LIMIT, 0U);
        return SVG_SHAPE_ERROR_LIMIT;
    }
    svg_xml_token *tokens = svg_shape_calloc(capacity, sizeof(*tokens));
    if (tokens == NULL) {
        svg_shape_traverse_fail(diagnostic, SVG_SHAPE_ERROR_OOM, 0U);
        return SVG_SHAPE_ERROR_OOM;
    }
    (void)svg_shape_alloc_set_tag(tokens, SVG_SHAPE_ALLOC_TAG_TOKEN_ARRAY);
    svg_xml_result xml = svg_xml_tokenize(source, svg_xml_default_options(),
                                          tokens, capacity);
    if (xml.diagnostic.category != SVG_XML_ERROR_NONE) {
        svg_shape_free(tokens);
        svg_shape_traverse_fail(diagnostic, SVG_SHAPE_ERROR_XML, xml.diagnostic.offset);
        return SVG_SHAPE_ERROR_XML;
    }

    shape_traversal traversal = {
        .source = source,
        .system_language = system_language,
        .tokens = tokens,
        .token_count = xml.token_count,
        .diagnostic = diagnostic,
        .media = providers,
        .selection_element = SIZE_MAX
    };
    svg_shape_error error = svg_reference_index_create(
        source, tokens, xml.token_count, &traversal.references, diagnostic);
    if (error != SVG_SHAPE_ERROR_NONE) goto cleanup;
    if (memory_bounds != NULL)
        memory_bounds->reference_index_bytes =
            svg_reference_index_storage_bytes(&traversal.references);
    svg_css_error css_error = svg_css_sheet_create(
        source, tokens, xml.token_count, &traversal.css);
    if (css_error != SVG_CSS_OK) {
        error = css_error == SVG_CSS_LIMIT ? SVG_SHAPE_ERROR_LIMIT :
                                            SVG_SHAPE_ERROR_OOM;
        svg_shape_traverse_fail(diagnostic, error, 0U);
        goto cleanup;
    }
    if (providers != NULL && providers->layout_text != NULL) {
        traversal.text_cache = svg_shape_text_cache_create(providers);
        if (traversal.text_cache == NULL) {
            svg_shape_traverse_fail(diagnostic, SVG_SHAPE_ERROR_OOM, 0U);
            error = SVG_SHAPE_ERROR_OOM;
            goto cleanup;
        }
    }
    traversal.builder = (svg_shape_builder){
        .doc = out,
        .references = &traversal.references,
        .css = traversal.css,
        .paint_cache = &traversal.paint_cache,
        .paint_token_visits = &traversal.paint_token_visits,
        .diag = diagnostic,
        .transform = geom_affine_identity(),
        .media = providers,
        .text_cache = traversal.text_cache,
        .traversal = &traversal
    };
    out->source = source;
    if (providers != NULL) {
        out->media_context = providers->context;
        out->release_image = providers->release_image;
    }
    if (!svg_shape_traverse_walk_range(&traversal, 0U, xml.token_count,
                    geom_affine_identity(), NULL, 0U)) {
        error = diagnostic->category;
        goto cleanup;
    }
    if (memory_bounds != NULL)
        memory_bounds->reference_lookup_probes =
            svg_reference_index_lookup_probes(&traversal.references);
    if (memory_bounds != NULL)
        memory_bounds->paint_token_visits = traversal.paint_token_visits;
    if (!traversal.root_seen) {
        svg_shape_traverse_fail(diagnostic, SVG_SHAPE_ERROR_VIEWPORT, 0U);
        error = SVG_SHAPE_ERROR_VIEWPORT;
        goto cleanup;
    }
    size_t animation_error_offset = 0U;
    if (!svg_anim_parse(source, tokens, xml.token_count, &out->animation,
                        &animation_error_offset)) {
        error = animation_error_offset == 0U ? SVG_SHAPE_ERROR_OOM :
                                               SVG_SHAPE_ERROR_LIMIT;
        svg_shape_traverse_fail(diagnostic, error, animation_error_offset);
        goto cleanup;
    }
    error = SVG_SHAPE_ERROR_NONE;

cleanup:
    svg_shape_text_cache_destroy(traversal.text_cache);
    svg_paint_cache_destroy(traversal.paint_cache);
    svg_css_sheet_destroy(traversal.css);
    svg_reference_index_destroy(&traversal.references);
    svg_shape_free(tokens);
    if (error != SVG_SHAPE_ERROR_NONE) svg_shape_document_destroy(out);
    return error;
}

svg_shape_error svg_shape_decode_source_with_system_language(
    base_bytes source, base_bytes system_language, svg_shape_document *out,
    svg_shape_diagnostic *diagnostic)
{
    return decode_source_with_language_and_media_and_bounds(
        source, NULL, out, diagnostic, NULL, system_language);
}

svg_shape_error svg_shape_decode_source_with_media_and_bounds(
    base_bytes source, const svg_shape_media_providers *providers,
    svg_shape_document *out, svg_shape_diagnostic *diagnostic,
    svg_shape_decode_memory_bounds *memory_bounds)
{
    static const uint8_t default_language[] = {'e', 'n'};
    return decode_source_with_language_and_media_and_bounds(
        source, providers, out, diagnostic, memory_bounds,
        (base_bytes){default_language, sizeof(default_language)});
}
