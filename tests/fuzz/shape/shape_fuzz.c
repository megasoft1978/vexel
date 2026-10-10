#include "shape.h"
#include "shape_render.h"

#include <errno.h>
#include <time.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define INPUT_CAP 4096U
#define SHAPE_SOURCE_LIMIT (16U * 1024U * 1024U)
#define FEATURE_COUNT 11U

static const char *const feature_names[FEATURE_COUNT] = {
    "path_stroke", "rounded_rect", "colors", "transforms", "opacity",
    "nested_viewport", "gradients", "css_selectors", "length_units",
    "text", "image"
};

static void identify_features(const char *shape, bool features[FEATURE_COUNT])
{
    features[0] = strstr(shape, "<path") != NULL &&
                  strstr(shape, "stroke=") != NULL;
    features[1] = strstr(shape, "rx=") != NULL || strstr(shape, "ry=") != NULL;
    features[2] = strstr(shape, "rgb(") != NULL || strstr(shape, "hsl(") != NULL ||
                  strstr(shape, "currentColor") != NULL ||
                  strstr(shape, "rebeccapurple") != NULL;
    features[3] = strstr(shape, "skewX(") != NULL ||
                  strstr(shape, "skewY(") != NULL ||
                  strstr(shape, "rotate(") != NULL ||
                  strstr(shape, "matrix(") != NULL;
    features[4] = strstr(shape, "opacity=") != NULL ||
                  strstr(shape, "opacity:") != NULL;
    features[5] = strstr(shape, "<svg ") != NULL &&
                  strstr(shape, "viewBox=") != NULL;
    features[6] = strstr(shape, "Gradient") != NULL ||
                  strstr(shape, "gradient") != NULL ||
                  strstr(shape, "url(#") != NULL;
    features[7] = strstr(shape, "<style>") != NULL;
    features[8] = strstr(shape, "em\"") != NULL || strstr(shape, "mm") != NULL ||
                  strstr(shape, "cm\"") != NULL || strstr(shape, "pt\"") != NULL ||
                  strstr(shape, "in\"") != NULL;
    features[9] = strstr(shape, "<text") != NULL || strstr(shape, "<tspan") != NULL;
    features[10] = strstr(shape, "<image") != NULL ||
                   strstr(shape, "data:image/") != NULL;
}

static svg_shape_media_status fuzz_layout_text(
    void *context, const svg_shape_text_request *request,
    svg_shape_text_layout *out)
{
    (void)context;
    if (request == NULL || out == NULL || request->text.len >
        SVG_SHAPE_MEDIA_MAX_TEXT_BYTES) return SVG_SHAPE_MEDIA_LIMIT;
    size_t glyph_count = 0U;
    for (size_t i = 0U; i < request->text.len; ++i)
        if ((request->text.ptr[i] & 0xC0U) != 0x80U) glyph_count++;
    if (glyph_count > SVG_SHAPE_MEDIA_MAX_GLYPHS) return SVG_SHAPE_MEDIA_LIMIT;
    size_t command_count = glyph_count == 0U ? 0U : 5U;
    svg_shape_text_glyph *glyphs = glyph_count == 0U ? NULL :
        calloc(glyph_count, sizeof(*glyphs));
    svg_shape_text_command *commands = command_count == 0U ? NULL :
        calloc(command_count, sizeof(*commands));
    if ((glyph_count != 0U && glyphs == NULL) ||
        (command_count != 0U && commands == NULL)) {
        free(glyphs);
        free(commands);
        return SVG_SHAPE_MEDIA_OOM;
    }
    if (command_count != 0U) {
        commands[0] = (svg_shape_text_command){SVG_SHAPE_TEXT_MOVE, {0.0, 0.0}};
        commands[1] = (svg_shape_text_command){SVG_SHAPE_TEXT_LINE, {4.0, 0.0}};
        commands[2] = (svg_shape_text_command){SVG_SHAPE_TEXT_LINE, {4.0, 6.0}};
        commands[3] = (svg_shape_text_command){SVG_SHAPE_TEXT_LINE, {0.0, 6.0}};
        commands[4] = (svg_shape_text_command){SVG_SHAPE_TEXT_CLOSE, {0.0}};
    }
    for (size_t i = 0U; i < glyph_count; ++i) {
        glyphs[i] = (svg_shape_text_glyph){
            .x = (double)i * 5.0, .advance = 5.0,
            .command_start = i == 0U ? 0U : command_count,
            .command_count = i == 0U ? command_count : 0U};
    }
    *out = (svg_shape_text_layout){
        .glyphs = glyphs, .glyph_count = glyph_count,
        .commands = commands, .command_count = command_count,
        .advance_width = (double)glyph_count * 5.0,
        .ascent = 6.0, .descent = 1.0};
    return SVG_SHAPE_MEDIA_OK;
}

static void fuzz_release_text(void *context, svg_shape_text_layout *layout)
{
    (void)context;
    if (layout == NULL) return;
    free(layout->glyphs);
    free(layout->commands);
    *layout = (svg_shape_text_layout){0};
}

static const uint8_t fuzz_image_pixels[4] = {40U, 120U, 200U, 255U};

static svg_shape_media_status fuzz_decode_image(
    void *context, base_bytes encoded, svg_shape_decoded_image *out)
{
    (void)context;
    if (out == NULL || encoded.ptr == NULL || encoded.len == 0U)
        return SVG_SHAPE_MEDIA_INVALID;
    *out = (svg_shape_decoded_image){
        fuzz_image_pixels, sizeof(fuzz_image_pixels), 4U, 1U, 1U, NULL};
    return SVG_SHAPE_MEDIA_OK;
}

static void fuzz_release_image(void *context, svg_shape_decoded_image *image)
{
    (void)context;
    if (image != NULL) *image = (svg_shape_decoded_image){0};
}

static uint32_t next_u32(uint32_t *state)
{
    uint32_t x = *state; x ^= x << 13U; x ^= x >> 17U; x ^= x << 5U; *state = x; return x;
}

static bool command_cap_probe(size_t *input_bytes)
{
    static const uint8_t prefix[] = "<svg width=\"16\" height=\"16\"><path d=\"M0 0";
    static const uint8_t segment[] = " L1 1";
    static const uint8_t suffix[] = "\"/></svg>";
    size_t groups = (size_t)SVG_SHAPE_MAX_COMMANDS;
    if (groups > (SIZE_MAX - sizeof(prefix) - sizeof(suffix)) / (sizeof(segment) - 1U)) return false;
    size_t total = (sizeof(prefix) - 1U) + groups * (sizeof(segment) - 1U) + (sizeof(suffix) - 1U);
    if (total > SHAPE_SOURCE_LIMIT) return false;
    uint8_t *input = malloc(total);
    if (input == NULL) return false;
    size_t offset = 0U;
    memcpy(input + offset, prefix, sizeof(prefix) - 1U); offset += sizeof(prefix) - 1U;
    for (size_t i = 0; i < groups; i++) {
        memcpy(input + offset, segment, sizeof(segment) - 1U);
        offset += sizeof(segment) - 1U;
    }
    memcpy(input + offset, suffix, sizeof(suffix) - 1U); offset += sizeof(suffix) - 1U;
    svg_shape_document doc;
    svg_shape_diagnostic diag;
    svg_shape_error error = svg_shape_decode((base_bytes){input, offset}, &doc, &diag);
    if (error == SVG_SHAPE_ERROR_NONE) svg_shape_document_destroy(&doc);
    free(input);
    *input_bytes = offset;
    return error == SVG_SHAPE_ERROR_LIMIT && diag.category == SVG_SHAPE_ERROR_LIMIT;
}

int main(int argc, char **argv)
{
    if (argc != 4) return 2;
    char *end = NULL; errno = 0; unsigned long seed_long = strtoul(argv[1], &end, 10);
    if (errno != 0 || end == argv[1] || *end != '\0' || seed_long > UINT32_MAX) return 2;
    uint32_t state = (uint32_t)seed_long;
    errno = 0; unsigned long count = strtoul(argv[2], &end, 10);
    if (errno != 0 || end == argv[2] || *end != '\0' || count > 100000000UL) return 2;
    errno = 0; unsigned long duration = strtoul(argv[3], &end, 10);
    if (errno != 0 || end == argv[3] || *end != '\0' || duration > 600UL) return 2;
    size_t cap_case_bytes = 0U;
    if (!command_cap_probe(&cap_case_bytes)) return 5;
    clock_t started = clock();
    static const char *parts[] = {"M", "m", "L", "H", "V", "C", "c", "S", "s", "Q", "q", "T", "t", "A", "Z", " ", ",", "1e999", "nan", "-1000001", "0", "1", "2", "3", "4", "9"};
    static const char prefix[] = "<svg width=\"16\" height=\"16\"><path d=\"";
    static const char suffix[] = "\"/></svg>";
    static const char *shape_cases[] = {
        "<circle cx=\"8\" cy=\"8\" r=\"4\"/>",
        "<ellipse cx=\"8\" cy=\"8\" rx=\"4\" ry=\"2\"/>",
        "<polygon points=\"1,1 8,1 8,8 1,8\"/>",
        "<polyline points=\"1 1 8 1 8 8 1 8\"/>",
        "<line x1=\"1\" y1=\"1\" x2=\"8\" y2=\"8\"/>",
        "<g transform=\"translate(2 3) rotate(15)\"><rect width=\"4\" height=\"4\"/></g>",
        "<g transform=\"skewX(10)\"><circle r=\"3\"/></g>",
        "<polygon points=\"1 2 3\"/>",
        "<line x1=\"4\" y1=\"4\" x2=\"4\" y2=\"4\" stroke=\"red\" stroke-width=\"2\" stroke-linecap=\"round\"/>",
        "<rect width=\"0\" height=\"4\" stroke=\"blue\" stroke-dasharray=\"1 2 3\"/>",
        "<polyline points=\"2 2 8 2 8 2 8 8\" fill=\"none\" stroke=\"green\" stroke-width=\"1.5\" stroke-linejoin=\"bevel\"/>",
        "<polygon points=\"2 2 8 2 8 8 2 8\" fill=\"none\" stroke=\"#369\" stroke-width=\"2\" stroke-linejoin=\"miter\" stroke-miterlimit=\"2\"/>",
        "<line x1=\"1\" y1=\"8\" x2=\"15\" y2=\"8\" stroke=\"black\" stroke-dasharray=\"3 2 1\" stroke-dashoffset=\"-2\"/>"
        ,"<path fill=\"none\" stroke=\"blue\" stroke-width=\"1.5\" stroke-linecap=\"round\" stroke-dasharray=\"3mm 2mm\" d=\"M1 2 C3 0 5 4 7 2 A3 2 10 0 1 13 4\"/>"
        ,"<rect x=\"5%\" y=\"1em\" width=\"50%\" height=\"4cm\" rx=\"20%\" ry=\"2pt\" fill=\"rgba(0 255 0 / 50%)\"/>"
        ,"<g transform=\"matrix(1 0.1 0.2 1 1 1) skewY(5) rotate(10 4 4)\"><polygon points=\"1 1 8 2 6 9\" fill-rule=\"evenodd\" fill=\"hsl(240 100% 50%)\"/></g>"
        ,"<g opacity=\"0.5\"><rect x=\"1\" y=\"1\" width=\"6\" height=\"6\" fill=\"red\"/><circle cx=\"5\" cy=\"5\" r=\"4\" fill=\"blue\"/></g>"
        ,"<svg x=\"1\" y=\"1\" width=\"8\" height=\"8\" viewBox=\"0 0 4 4\" preserveAspectRatio=\"none\"><rect width=\"100%\" height=\"100%\" fill=\"rebeccapurple\"/></svg>"
        ,"<defs><linearGradient id=\"base\" gradientUnits=\"userSpaceOnUse\" x2=\"8\" spreadMethod=\"reflect\"><stop stop-color=\"red\"/><stop offset=\"1\" stop-color=\"blue\"/></linearGradient><linearGradient id=\"g\" href=\"#base\" gradientTransform=\"translate(2 0)\"/><radialGradient id=\"r\" cx=\"0.4\" cy=\"0.5\" r=\"0.5\" fx=\"0.2\"><stop stop-color=\"white\"/><stop offset=\"1\" stop-color=\"black\"/></radialGradient></defs><rect width=\"8\" height=\"8\" fill=\"url(#g)\"/><circle cx=\"8\" cy=\"8\" r=\"5\" fill=\"url(#r)\"/>"
        ,"<style>svg g.box rect.item{fill:currentColor;opacity:.75}</style><g class=\"box\" color=\"orange\"><rect class=\"item\" x=\"1\" y=\"1\" width=\"6\" height=\"6\"/></g>"
        ,"<rect x=\"2mm\" y=\"3pt\" width=\"1in\" height=\"2em\" fill=\"teal\"/>"
        ,"<text x=\"1 6\" y=\"8\" font-family=\"sans-serif\" font-size=\"6\" text-anchor=\"middle\">A<tspan dx=\"1\" fill=\"blue\">B</tspan></text>"
        ,"<text xml:space=\"preserve\" x=\"1\" y=\"8\"> A  B </text>"
        ,"<text x=\"1\" y=\"8\" font-size=\"bad\">x</text>"
        ,"<image x=\"1\" y=\"1\" width=\"8\" height=\"8\" href=\"data:image/png;base64,AQ==\"/>"
        ,"<image x=\"1\" y=\"1\" width=\"8\" height=\"8\" href=\"data:image/jpeg;base64,AA==\"/>"
        ,"<image x=\"1\" y=\"1\" width=\"8\" height=\"8\" href=\"data:image/png;base64,A===\"/>"
        ,"<image x=\"1\" y=\"1\" width=\"8\" height=\"8\" href=\"https://example.invalid/image.png\"/>"
    };
    static const char *coverage_cases[] = {
        "<rect x=\"1.125\" y=\"2.375\" width=\"8.5\" height=\"6.25\"/>",
        "<path fill-rule=\"evenodd\" d=\"M0.25 0.5 H14.75 V14.5 H0.25 Z M4.5 4.25 H11.5 V11.75 H4.5 Z\"/>",
        "<path d=\"M0.25 0.5 H14.75 V14.5 H0.25 Z M4.5 4.25 V11.75 H11.5 V4.25 Z\"/>"
    };
    static const char shape_prefix[] = "<svg width=\"16\" height=\"16\">";
    static const char shape_suffix[] = "</svg>";
    const svg_shape_media_providers providers = {
        .layout_text = fuzz_layout_text,
        .release_text = fuzz_release_text,
        .decode_image = fuzz_decode_image,
        .release_image = fuzz_release_image};
    unsigned long completed = 0;
    unsigned long fractional_cases = 0, evenodd_cases = 0, nonzero_cases = 0;
    unsigned long fractional_renders = 0, evenodd_renders = 0, nonzero_renders = 0;
    unsigned long stroke_cases = 0U, stroke_renders = 0U;
    unsigned long feature_cases[FEATURE_COUNT] = {0U};
    unsigned long feature_renders[FEATURE_COUNT] = {0U};
    for (unsigned long c = 0; c < count; c++) {
        uint8_t input[INPUT_CAP]; size_t n = 0;
        unsigned mode = (unsigned)(c % 4UL);
        bool features[FEATURE_COUNT] = {false};
        bool case_has_stroke = false;
        if (mode < 3U) {
            memcpy(input, shape_prefix, sizeof(shape_prefix)-1U); n=sizeof(shape_prefix)-1U;
            const char *shape=coverage_cases[mode];
            size_t length=strlen(shape);
            memcpy(input+n,shape,length);n+=length;
            memcpy(input+n,shape_suffix,sizeof(shape_suffix)-1U);n+=sizeof(shape_suffix)-1U;
            fractional_cases++;
            if (mode == 1U) evenodd_cases++;
            if (mode == 2U) nonzero_cases++;
        } else if ((next_u32(&state) & 1U) != 0U) {
            memcpy(input, shape_prefix, sizeof(shape_prefix)-1U); n=sizeof(shape_prefix)-1U;
            const char *shape=shape_cases[next_u32(&state)%(sizeof(shape_cases)/sizeof(shape_cases[0]))];
            case_has_stroke = strstr(shape, "stroke=") != NULL;
            if (case_has_stroke) stroke_cases++;
            identify_features(shape, features);
            for (size_t feature = 0U; feature < FEATURE_COUNT; ++feature)
                if (features[feature]) feature_cases[feature]++;
            size_t length=strlen(shape);
            memcpy(input+n,shape,length);n+=length;
            memcpy(input+n,shape_suffix,sizeof(shape_suffix)-1U);n+=sizeof(shape_suffix)-1U;
        } else {
            memcpy(input, prefix, sizeof(prefix)-1U); n = sizeof(prefix)-1U;
            size_t tokens = (size_t)(next_u32(&state) % 64U) + 1U;
            for (size_t i = 0; i < tokens; i++) {
                const char *part = parts[next_u32(&state) % (sizeof(parts)/sizeof(parts[0]))]; size_t len = strlen(part);
                if (len > INPUT_CAP - n - (sizeof(suffix)-1U)) break;
                memcpy(input+n, part, len); n += len;
            }
            memcpy(input+n, suffix, sizeof(suffix)-1U); n += sizeof(suffix)-1U;
        }
        svg_shape_document doc; svg_shape_diagnostic diag;
        svg_shape_error error = svg_shape_decode_with_media(
            (base_bytes){input,n}, &providers, &doc, &diag);
        if (error == SVG_SHAPE_ERROR_NONE) {
            if (doc.width == 0U || doc.width > 4096U || doc.height == 0U || doc.height > 4096U || doc.edge_count > SVG_SHAPE_MAX_EDGES) return 3;
            bool render_case = (mode < 3U && c % 128UL == mode) ||
                               (mode == 3U && c % 128UL == 3UL);
            if (render_case) {
                raster_tile tile;
                if (!raster_shape_render(&doc, &tile, &diag)) {
                    svg_shape_document_destroy(&doc);
                    return 6;
                }
                raster_tile_destroy(&tile);
                if (mode == 3U && case_has_stroke) stroke_renders++;
                if (mode == 3U)
                    for (size_t feature = 0U; feature < FEATURE_COUNT; ++feature)
                        if (features[feature]) feature_renders[feature]++;
                if (mode == 0U) fractional_renders++;
                else if (mode == 1U) evenodd_renders++;
                else if (mode == 2U) nonzero_renders++;
            }
            svg_shape_document_destroy(&doc);
        } else if (diag.category == SVG_SHAPE_ERROR_NONE || diag.offset > n) return 4;
        completed++;
        if ((completed & 1023UL) == 0UL && duration != 0UL &&
            (double)(clock() - started) / (double)CLOCKS_PER_SEC >= (double)duration) break;
    }
    double seconds = (double)(clock() - started) / (double)CLOCKS_PER_SEC;
    printf("seed=%lu cases=%lu duration=%.3f cap_exhaustion=1 cap_case_bytes=%zu fractional_cases=%lu evenodd_cases=%lu nonzero_cases=%lu fractional_renders=%lu evenodd_renders=%lu nonzero_renders=%lu stroke_cases=%lu stroke_renders=%lu",
           seed_long, completed, seconds, cap_case_bytes, fractional_cases, evenodd_cases, nonzero_cases,
           fractional_renders, evenodd_renders, nonzero_renders, stroke_cases, stroke_renders);
    for (size_t feature = 0U; feature < FEATURE_COUNT; ++feature)
        printf(" feature_%s_cases=%lu feature_%s_renders=%lu", feature_names[feature],
               feature_cases[feature], feature_names[feature],
               feature_renders[feature]);
    putchar('\n');
    return 0;
}
