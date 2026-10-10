#include "svg/shape/media.h"

#include "platform/media/platform_image.h"
#include "platform/media/platform_text.h"

#include <stdlib.h>
#include <string.h>

svg_shape_media_providers svg_app_media_providers(void);

static svg_shape_media_status map_status(platform_media_status status)
{
    if (status == PLATFORM_MEDIA_OK) return SVG_SHAPE_MEDIA_OK;
    if (status == PLATFORM_MEDIA_LIMIT) return SVG_SHAPE_MEDIA_LIMIT;
    if (status == PLATFORM_MEDIA_OUT_OF_MEMORY) return SVG_SHAPE_MEDIA_OOM;
    if (status == PLATFORM_MEDIA_UNSUPPORTED ||
        status == PLATFORM_MEDIA_ANIMATED) return SVG_SHAPE_MEDIA_UNSUPPORTED;
    return SVG_SHAPE_MEDIA_INVALID;
}

static svg_shape_media_status app_text_layout(
    void *context, const svg_shape_text_request *request,
    svg_shape_text_layout *out)
{
    (void)context;
    if (request == NULL || out == NULL || request->text.ptr == NULL ||
        request->font_family_count > PLATFORM_TEXT_MAX_FAMILIES ||
        (request->font_family_count != 0U && request->font_families == NULL))
        return SVG_SHAPE_MEDIA_INVALID;
    platform_text_slice *families = request->font_family_count == 0U ? NULL :
        calloc(request->font_family_count, sizeof(*families));
    if (request->font_family_count != 0U && families == NULL)
        return SVG_SHAPE_MEDIA_OOM;
    for (size_t i = 0U; i < request->font_family_count; ++i) {
        families[i] = (platform_text_slice){
            request->font_families[i].bytes,
            request->font_families[i].length};
    }
    platform_text_request platform_request = {
        .text = {request->text.ptr, request->text.len},
        .font_families = families,
        .font_family_count = request->font_family_count,
        .size = request->size_points,
        .weight = request->weight,
        .style = request->style == SVG_SHAPE_TEXT_ITALIC ?
            PLATFORM_TEXT_STYLE_ITALIC :
            request->style == SVG_SHAPE_TEXT_OBLIQUE ?
                PLATFORM_TEXT_STYLE_OBLIQUE : PLATFORM_TEXT_STYLE_NORMAL,
        .letter_spacing = request->letter_spacing_points
    };
    platform_text_result platform_result = PLATFORM_TEXT_RESULT_INIT;
    platform_media_status status = platform_text_layout(
        &platform_request, &platform_result);
    free(families);
    if (status != PLATFORM_MEDIA_OK) {
        platform_text_result_destroy(&platform_result);
        return map_status(status);
    }
    svg_shape_text_glyph *glyphs = platform_result.glyph_count == 0U ? NULL :
        calloc(platform_result.glyph_count, sizeof(*glyphs));
    svg_shape_text_command *commands = platform_result.command_count == 0U ? NULL :
        calloc(platform_result.command_count, sizeof(*commands));
    if ((platform_result.glyph_count != 0U && glyphs == NULL) ||
        (platform_result.command_count != 0U && commands == NULL)) {
        free(glyphs);
        free(commands);
        platform_text_result_destroy(&platform_result);
        return SVG_SHAPE_MEDIA_OOM;
    }
    for (size_t i = 0U; i < platform_result.glyph_count; ++i) {
        const platform_text_glyph *source = &platform_result.glyphs[i];
        glyphs[i] = (svg_shape_text_glyph){
            source->x, source->y, source->advance,
            source->command_start, source->command_count};
    }
    for (size_t i = 0U; i < platform_result.command_count; ++i) {
        const platform_text_path_command *source = &platform_result.commands[i];
        commands[i].verb = (svg_shape_text_verb)source->verb;
        memcpy(commands[i].values, source->values, sizeof(commands[i].values));
    }
    for (size_t i = 0U; i < platform_result.glyph_count; ++i) {
        const platform_text_glyph *glyph = &platform_result.glyphs[i];
        if (glyph->command_start > platform_result.command_count ||
            glyph->command_count > platform_result.command_count -
                                   glyph->command_start) {
            free(glyphs);
            free(commands);
            platform_text_result_destroy(&platform_result);
            return SVG_SHAPE_MEDIA_INVALID;
        }
        size_t end = glyph->command_start + glyph->command_count;
        for (size_t command = glyph->command_start; command < end; ++command) {
            size_t coordinate_count = commands[command].verb == SVG_SHAPE_TEXT_CUBIC ?
                                      6U : commands[command].verb == SVG_SHAPE_TEXT_CLOSE ?
                                      0U : 2U;
            for (size_t coordinate = 0U; coordinate + 1U < coordinate_count;
                 coordinate += 2U) {
                commands[command].values[coordinate] -= glyph->x;
                commands[command].values[coordinate + 1U] -= glyph->y;
            }
        }
    }
    *out = (svg_shape_text_layout){
        glyphs, platform_result.glyph_count, commands,
        platform_result.command_count, platform_result.advance_width,
        platform_result.ascent, platform_result.descent,
        platform_result.leading};
    platform_text_result_destroy(&platform_result);
    return SVG_SHAPE_MEDIA_OK;
}

static void app_text_release(void *context, svg_shape_text_layout *layout)
{
    (void)context;
    if (layout == NULL) return;
    free(layout->glyphs);
    free(layout->commands);
    *layout = (svg_shape_text_layout){0};
}

static svg_shape_media_status app_image_decode(
    void *context, base_bytes encoded, svg_shape_decoded_image *out)
{
    (void)context;
    if (out == NULL || (encoded.ptr == NULL && encoded.len != 0U))
        return SVG_SHAPE_MEDIA_INVALID;
    platform_image *image = calloc(1U, sizeof(*image));
    if (image == NULL) return SVG_SHAPE_MEDIA_OOM;
    platform_media_status status = platform_image_decode(
        encoded.ptr, encoded.len, image);
    if (status != PLATFORM_MEDIA_OK) {
        platform_image_destroy(image);
        free(image);
        return map_status(status);
    }
    *out = (svg_shape_decoded_image){
        image->rgba, image->byte_count, image->row_stride,
        image->width, image->height, image};
    return SVG_SHAPE_MEDIA_OK;
}

static void app_image_release(void *context, svg_shape_decoded_image *image)
{
    (void)context;
    if (image == NULL) return;
    platform_image *owned = image->owner;
    if (owned != NULL) {
        platform_image_destroy(owned);
        free(owned);
    }
    *image = (svg_shape_decoded_image){0};
}

svg_shape_media_providers svg_app_media_providers(void)
{
    return (svg_shape_media_providers){
        .layout_text = app_text_layout,
        .release_text = app_text_release,
        .decode_image = app_image_decode,
        .release_image = app_image_release
    };
}
