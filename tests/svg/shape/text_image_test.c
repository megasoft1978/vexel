#include "shape.h"
#include "shape_render.h"

#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef struct fake_media_state {
    size_t layout_calls;
    size_t text_releases;
    size_t image_calls;
    size_t image_releases;
    char last_text[128];
    size_t last_text_length;
    char last_family[64];
    char first_family[64];
    double last_size;
    double first_size;
    double last_weight;
    double first_weight;
    svg_shape_text_style last_style;
    svg_shape_text_style first_style;
    double last_letter_spacing;
    double first_letter_spacing;
} fake_media_state;

static size_t count_codepoints(base_bytes text)
{
    size_t count = 0U;
    for (size_t i = 0U; i < text.len; ++i)
        if ((text.ptr[i] & 0xC0U) != 0x80U) count++;
    return count;
}

static svg_shape_media_status fake_layout_text(
    void *opaque, const svg_shape_text_request *request,
    svg_shape_text_layout *out)
{
    fake_media_state *state = opaque;
    if (request == NULL || out == NULL || request->text.len > 127U)
        return SVG_SHAPE_MEDIA_INVALID;
    state->layout_calls++;
    memcpy(state->last_text, request->text.ptr, request->text.len);
    state->last_text[request->text.len] = '\0';
    state->last_text_length = request->text.len;
    state->last_family[0] = '\0';
    if (request->font_family_count != 0U) {
        size_t length = request->font_families[0].length;
        if (length >= sizeof(state->last_family)) return SVG_SHAPE_MEDIA_LIMIT;
        memcpy(state->last_family, request->font_families[0].bytes, length);
        state->last_family[length] = '\0';
    }
    state->last_size = request->size_points;
    state->last_weight = request->weight;
    state->last_style = request->style;
    state->last_letter_spacing = request->letter_spacing_points;
    if (state->layout_calls == 1U) {
        memcpy(state->first_family, state->last_family,
               sizeof(state->first_family));
        state->first_size = state->last_size;
        state->first_weight = state->last_weight;
        state->first_style = state->last_style;
        state->first_letter_spacing = state->last_letter_spacing;
    }
    size_t glyph_count = count_codepoints(request->text);
    size_t drawable = 0U;
    for (size_t i = 0U; i < request->text.len; ++i)
        if (request->text.ptr[i] != ' ' &&
            (request->text.ptr[i] & 0xC0U) != 0x80U) drawable++;
    if (drawable > SIZE_MAX / 5U || glyph_count > SIZE_MAX / sizeof(svg_shape_text_glyph))
        return SVG_SHAPE_MEDIA_LIMIT;
    size_t command_count = drawable * 5U;
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
    size_t glyph = 0U, command = 0U;
    for (size_t i = 0U; i < request->text.len; ++i) {
        uint8_t byte = request->text.ptr[i];
        if ((byte & 0xC0U) == 0x80U) continue;
        size_t first = command;
        size_t count = 0U;
        if (byte != ' ') {
            commands[command++] = (svg_shape_text_command){
                SVG_SHAPE_TEXT_MOVE, {0.0, 0.0}};
            commands[command++] = (svg_shape_text_command){
                SVG_SHAPE_TEXT_LINE, {6.0, 0.0}};
            commands[command++] = (svg_shape_text_command){
                SVG_SHAPE_TEXT_LINE, {6.0, 8.0}};
            commands[command++] = (svg_shape_text_command){
                SVG_SHAPE_TEXT_LINE, {0.0, 8.0}};
            commands[command++] = (svg_shape_text_command){
                SVG_SHAPE_TEXT_CLOSE, {0.0}};
            count = 5U;
        }
        glyphs[glyph] = (svg_shape_text_glyph){
            (double)glyph * 8.0, 0.0, 8.0, first, count};
        glyph++;
    }
    *out = (svg_shape_text_layout){glyphs, glyph_count, commands,
        command_count, (double)glyph_count * 8.0, 9.0, 3.0, 0.0};
    return SVG_SHAPE_MEDIA_OK;
}

static void fake_text_release(void *opaque, svg_shape_text_layout *layout)
{
    fake_media_state *state = opaque;
    if (state != NULL) state->text_releases++;
    free(layout->glyphs);
    free(layout->commands);
    *layout = (svg_shape_text_layout){0};
}

static const uint8_t fake_pixels[16] = {
    255U, 0U, 0U, 255U, 0U, 255U, 0U, 255U,
    0U, 0U, 255U, 255U, 255U, 255U, 255U, 255U
};

static svg_shape_media_status fake_decode_image(
    void *opaque, base_bytes encoded, svg_shape_decoded_image *out)
{
    fake_media_state *state = opaque;
    if (encoded.len != 3U || encoded.ptr[0] != 1U ||
        encoded.ptr[1] != 2U || encoded.ptr[2] != 3U)
        return SVG_SHAPE_MEDIA_INVALID;
    state->image_calls++;
    *out = (svg_shape_decoded_image){fake_pixels, sizeof(fake_pixels),
                                     8U, 2U, 2U, state};
    return SVG_SHAPE_MEDIA_OK;
}

static void fake_image_release(void *opaque, svg_shape_decoded_image *image)
{
    fake_media_state *state = opaque;
    if (state != NULL) state->image_releases++;
    *image = (svg_shape_decoded_image){0};
}

static svg_shape_media_providers fake_providers(fake_media_state *state)
{
    return (svg_shape_media_providers){
        .context = state,
        .layout_text = fake_layout_text,
        .release_text = fake_text_release,
        .decode_image = fake_decode_image,
        .release_image = fake_image_release
    };
}

static bool check(bool condition, const char *message)
{
    if (!condition) fprintf(stderr, "text/image check failed: %s\n", message);
    return condition;
}

static bool text_checks(void)
{
    static const char source[] =
        "<svg width=\"80\" height=\"40\"><style>"
        ".label{font-family:'Mock Sans',sans-serif;font-size:12px;"
        "font-weight:700;font-style:italic;text-anchor:middle;"
        "letter-spacing:2px;word-spacing:1px;fill:red}"
        "</style><g class=\"label\"><text x=\"30\" y=\"20\">A B"
        "<tspan dx=\"2\" fill=\"blue\">C</tspan></text></g>"
        "<text x=\"50\" y=\"20\">A</text>"
        "<text x=\"60\" y=\"20\">A</text></svg>";
    fake_media_state state = {0};
    svg_shape_media_providers providers = fake_providers(&state);
    svg_shape_document document = {0};
    svg_shape_diagnostic diagnostic = {0};
    base_bytes bytes = {(const uint8_t *)source, sizeof(source) - 1U};
    bool decoded = svg_shape_decode_with_media(bytes, &providers, &document,
                                                &diagnostic) == SVG_SHAPE_ERROR_NONE;
    if (!check(decoded, "CSS text and nested tspan decode")) return false;
    bool okay = check(document.shape_count == 4U,
                      "one outline shape per text run") &&
        check(state.layout_calls == 3U,
              "identical shaped run reuses the decode-local cache") &&
        check(strcmp(state.first_family, "Mock Sans") == 0,
              "font-family list strips CSS quotes") &&
        check(fabs(state.first_size - 9.0) < 1e-12 &&
              state.first_weight == 700.0 &&
              state.first_style == SVG_SHAPE_TEXT_ITALIC &&
              fabs(state.first_letter_spacing - 1.5) < 1e-12,
              "inherited size, weight, style and spacing reach provider") &&
        check(document.shape_sources[0].offset ==
              (size_t)(strstr(source, "<text") - source),
              "text shape has source element mapping") &&
        check(document.shape_count == 4U && document.edge_count != 0U,
              "CoreText outlines enter the path edge pipeline");
    svg_shape_document_destroy(&document);
    okay = okay && check(state.text_releases == 3U,
                         "cache releases each owned shaped run once");

    static const char whitespace[] =
        "<svg width=\"20\" height=\"10\"><text>A   B</text>"
        "<text xml:space=\"preserve\"> A  B </text></svg>";
    memset(&state, 0, sizeof(state));
    providers = fake_providers(&state);
    decoded = svg_shape_decode_with_media(
        (base_bytes){(const uint8_t *)whitespace, sizeof(whitespace) - 1U},
        &providers, &document, &diagnostic) == SVG_SHAPE_ERROR_NONE;
    okay = okay && check(decoded && state.layout_calls == 2U,
                         "xml:space modes produce two text runs");
    if (decoded) svg_shape_document_destroy(&document);
    okay = okay && check(strcmp(state.last_text, " A  B ") == 0,
                         "xml:space preserve keeps authored whitespace");

    static const char no_provider[] =
        "<svg width=\"8\" height=\"8\"><text>x</text></svg>";
    svg_shape_error no_media = svg_shape_decode(
        (base_bytes){(const uint8_t *)no_provider, sizeof(no_provider) - 1U},
        &document, &diagnostic);
    okay = okay && check(no_media == SVG_SHAPE_ERROR_UNSUPPORTED &&
                         diagnostic.category == SVG_SHAPE_ERROR_UNSUPPORTED,
                         "text without an injected provider fails closed");
    static const char text_path[] =
        "<svg width=\"8\" height=\"8\"><text><textPath href=\"#p\">x"
        "</textPath></text></svg>";
    providers = fake_providers(&state);
    no_media = svg_shape_decode_with_media(
        (base_bytes){(const uint8_t *)text_path, sizeof(text_path) - 1U},
        &providers, &document, &diagnostic);
    okay = okay && check(no_media == SVG_SHAPE_ERROR_UNSUPPORTED,
                         "textPath has a stable unsupported result");
    return okay;
}

static bool text_use_symbol_checks(void)
{
    static const char source[] =
        "<svg width=\"32\" height=\"16\"><defs>"
        "<text id=\"plain\" x=\"2\" y=\"8\">U</text>"
        "<symbol id=\"mark\" viewBox=\"0 0 8 8\"><text x=\"1\" y=\"6\">S"
        "</text></symbol></defs><use href=\"#plain\" x=\"3\"/>"
        "<use href=\"#mark\" x=\"14\" y=\"2\" width=\"8\" height=\"8\"/>"
        "</svg>";
    fake_media_state state = {0};
    svg_shape_media_providers providers = fake_providers(&state);
    svg_shape_document document = {0};
    svg_shape_diagnostic diagnostic = {0};
    bool decoded = svg_shape_decode_with_media(
        (base_bytes){(const uint8_t *)source, sizeof(source) - 1U},
        &providers, &document, &diagnostic) == SVG_SHAPE_ERROR_NONE;
    if (!check(decoded, "text under use and symbol decodes")) return false;
    const char *first_use = strstr(source, "<use");
    const char *second_use = first_use == NULL ? NULL : strstr(first_use + 4, "<use");
    bool okay = check(document.shape_count == 2U && state.layout_calls == 2U,
                      "referenced text is shaped once per source run") &&
        check(first_use != NULL && second_use != NULL &&
              document.shape_sources[0].offset == (size_t)(first_use - source) &&
              document.shape_sources[1].offset == (size_t)(second_use - source),
              "referenced text selection maps to each host use element") &&
        check(document.edges[0].a.x >= 5.0 && document.edges[0].a.x < 9.0,
              "text use-site translation reaches glyph outline geometry");
    svg_shape_document_destroy(&document);
    okay = okay && check(state.text_releases == 2U,
                         "referenced text layouts release after decode");
    return okay;
}

static bool image_checks(void)
{
    static const char source[] =
        "<svg width=\"4\" height=\"2\"><image x=\"0\" y=\"0\""
        " width=\"4\" height=\"2\" preserveAspectRatio=\"xMidYMid meet\""
        " href=\"data:image/png;base64,AQID\"/></svg>";
    fake_media_state state = {0};
    svg_shape_media_providers providers = fake_providers(&state);
    svg_shape_document document = {0};
    svg_shape_diagnostic diagnostic = {0};
    bool decoded = svg_shape_decode_with_media(
        (base_bytes){(const uint8_t *)source, sizeof(source) - 1U},
        &providers, &document, &diagnostic) == SVG_SHAPE_ERROR_NONE;
    if (!check(decoded, "embedded PNG data URI is decoded")) return false;
    bool okay = check(state.image_calls == 1U && document.shape_count == 1U,
                      "image data is handed to injected provider") &&
        check(fabs(document.images[0].draw_x - 1.0) < 1e-12 &&
              fabs(document.images[0].draw_width - 2.0) < 1e-12,
              "meet preserves aspect ratio with centered letterbox") &&
        check(document.shape_sources[0].length != 0U,
              "image shape has source mapping");
    raster_tile tile = {0};
    bool rendered = raster_shape_render(&document, &tile, &diagnostic);
    okay = okay && check(rendered && tile.width == 4U && tile.height == 2U,
                         "embedded image reaches scalar rasterizer");
    if (rendered) {
        size_t first_alpha = tile.rgba[3U];
        size_t second_alpha = tile.rgba[(size_t)1U * 4U + 3U];
        size_t third_alpha = tile.rgba[(size_t)2U * 4U + 3U];
        okay = okay && check(first_alpha == 0U && second_alpha != 0U &&
                             third_alpha != 0U,
                             "meet letterbox and bilinear image coverage");
        raster_tile_destroy(&tile);
    }
    svg_shape_document_destroy(&document);
    okay = okay && check(state.image_releases == 1U,
                         "decoded image ownership releases with document");

    static const char scaled[] =
        "<svg width=\"4\" height=\"4\"><image width=\"4\" height=\"4\""
        " preserveAspectRatio=\"none\""
        " href=\"data:image/png;base64,AQID\"/></svg>";
    memset(&state, 0, sizeof(state));
    providers = fake_providers(&state);
    decoded = svg_shape_decode_with_media(
        (base_bytes){(const uint8_t *)scaled, sizeof(scaled) - 1U},
        &providers, &document, &diagnostic) == SVG_SHAPE_ERROR_NONE;
    raster_tile scaled_tile = {0};
    rendered = decoded && raster_shape_render(&document, &scaled_tile,
                                               &diagnostic);
    const uint8_t *interpolated = rendered ?
        scaled_tile.rgba + (size_t)(1U * scaled_tile.width + 1U) * 4U : NULL;
    okay = okay && check(rendered && interpolated != NULL &&
                         interpolated[0] == 159U && interpolated[1] == 64U &&
                         interpolated[2] == 64U && interpolated[3] == 255U,
                         "scaled 2x2 image uses premultiplied bilinear sampling");
    if (rendered) raster_tile_destroy(&scaled_tile);
    if (decoded) svg_shape_document_destroy(&document);

    static const char transformed[] =
        "<svg width=\"4\" height=\"2\"><image transform=\"translate(1 0)\""
        " width=\"1\" height=\"1\" preserveAspectRatio=\"none\""
        " href=\"data:image/jpeg;base64,AQID\"/></svg>";
    memset(&state, 0, sizeof(state));
    providers = fake_providers(&state);
    decoded = svg_shape_decode_with_media(
        (base_bytes){(const uint8_t *)transformed, sizeof(transformed) - 1U},
        &providers, &document, &diagnostic) == SVG_SHAPE_ERROR_NONE;
    okay = okay && check(decoded && state.image_calls == 1U &&
                         fabs(document.shape_sources[0].transform.e - 1.0) < 1e-12,
                         "JPEG data URI and element transform are retained");
    if (decoded) svg_shape_document_destroy(&document);

    static const char external[] =
        "<svg width=\"8\" height=\"8\"><image width=\"2\" height=\"2\""
        " href=\"https://example.invalid/p.png\"/></svg>";
    svg_shape_error error = svg_shape_decode_with_media(
        (base_bytes){(const uint8_t *)external, sizeof(external) - 1U},
        &providers, &document, &diagnostic);
    okay = okay && check(error == SVG_SHAPE_ERROR_UNSUPPORTED &&
                         state.image_calls == 1U,
                         "external image references are rejected without fetch");
    static const char malformed[] =
        "<svg width=\"8\" height=\"8\"><image width=\"2\" height=\"2\""
        " href=\"data:image/png;base64,AQ!D\"/></svg>";
    error = svg_shape_decode_with_media(
        (base_bytes){(const uint8_t *)malformed, sizeof(malformed) - 1U},
        &providers, &document, &diagnostic);
    okay = okay && check(error == SVG_SHAPE_ERROR_UNSUPPORTED,
                         "malformed base64 has a stable diagnostic");
    return okay;
}

static bool selection_and_path_checks(void)
{
    static const char source[] =
        "<svg width=\"16\" height=\"16\"><defs><rect id=\"r\" width=\"4\""
        " height=\"4\"/></defs><use href=\"#r\" x=\"2\" y=\"3\"/>"
        "<path d=\"M1 1 L7 1 L7 7 Z\" fill=\"blue\"/></svg>";
    svg_shape_document document = {0};
    svg_shape_diagnostic diagnostic = {0};
    if (svg_shape_decode((base_bytes){(const uint8_t *)source,
                                      sizeof(source) - 1U},
                         &document, &diagnostic) != SVG_SHAPE_ERROR_NONE)
        return check(false, "source map fixture decodes");
    const char *use = strstr(source, "<use");
    const char *path = strstr(source, "<path");
    bool okay = check(document.shape_count == 2U && use != NULL && path != NULL,
                      "use and authored path create shapes") &&
        check(document.shape_sources[0].offset == (size_t)(use - source),
              "use instance maps to host use element") &&
        check(document.path_segment_count == 4U &&
              document.path_number_count == 6U,
              "path command topology retains every numeric source span") &&
        check(document.path_segments[0].command == 'M' &&
              document.path_segments[0].command_offset ==
                  (size_t)(strstr(source, "M1 1") - source),
              "path command span points into original d source") &&
        check(document.path_numbers[0].offset ==
                  (size_t)(strstr(source, "M1 1") - source) + 1U &&
              document.path_numbers[0].length == 1U &&
              document.path_numbers[0].value == 1.0 &&
              document.path_numbers[5].value == 7.0,
              "each path number retains its literal source span and value");
    svg_shape_selection selection = {0};
    okay = okay && check(svg_shape_hit_test(&document, 3.0, 3.0, 0.0,
                                             &selection) &&
                         selection.source_offset == (size_t)(path - source),
                         "topmost hit test returns source-backed shape") &&
        check(svg_shape_bounds_query(&document, 0U, &selection) &&
              fabs(selection.user_bounds.x - 2.0) < 1e-12 &&
              fabs(selection.user_bounds.y - 3.0) < 1e-12 &&
              fabs(selection.user_bounds.width - 4.0) < 1e-12,
              "bounds query returns user and device bounds") &&
        check(svg_shape_element_at_offset(&document,
                  (size_t)(strstr(source, "href=\"#r\"") - source),
                  &selection) && selection.source_offset == (size_t)(use - source),
              "editor byte offset resolves use host");
    svg_shape_document_destroy(&document);

    static const char stroke_source[] =
        "<svg width=\"16\" height=\"16\"><path fill=\"none\""
        " stroke=\"red\" stroke-width=\"2\" d=\"M1 5 L12 5\"/></svg>";
    if (svg_shape_decode((base_bytes){(const uint8_t *)stroke_source,
                                      sizeof(stroke_source) - 1U},
                         &document, &diagnostic) != SVG_SHAPE_ERROR_NONE)
        return false;
    okay = okay && check(svg_shape_hit_test(&document, 6.0, 5.5, 0.0,
                                             &selection),
                         "stroke-aware hit test accepts painted stroke");
    svg_shape_document_destroy(&document);
    return okay;
}

int main(void)
{
    if (!text_checks() || !text_use_symbol_checks() || !image_checks() ||
        !selection_and_path_checks())
        return 1;
    puts("text, image, selection and path-source checks passed");
    return 0;
}
