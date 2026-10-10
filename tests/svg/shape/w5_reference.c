#include "test_reference.h"
#include "shape.h"
#include "shape_render.h"

#include <stdint.h>
#include <string.h>

bool svg_shape_w5_reference_tests(void)
{
    static const uint8_t source[] =
        "<svg width=\"12\" height=\"6\"><a href=\"https://invalid.example/\">"
        "<switch><rect systemLanguage=\"it\" width=\"12\" height=\"6\" fill=\"red\"/>"
        "<rect systemLanguage=\"en\" width=\"12\" height=\"6\" fill=\"blue\"/>"
        "<rect width=\"12\" height=\"6\" fill=\"green\"/>"
        "</switch></a></svg>";
    static const uint8_t english[] = "en";
    static const uint8_t italian[] = "it";
    static const uint8_t german[] = "de";
    const base_bytes languages[] = {
        {english, sizeof(english) - 1U},
        {italian, sizeof(italian) - 1U},
        {german, sizeof(german) - 1U}};
    const uint8_t expected[3][4] = {
        {0U, 0U, 255U, 255U},
        {255U, 0U, 0U, 255U},
        {0U, 128U, 0U, 255U}};
    for (size_t i = 0U; i < 3U; ++i) {
        svg_shape_document document = {0};
        svg_shape_diagnostic diagnostic = {0};
        raster_tile tile = {0};
        if (svg_shape_decode_with_system_language(
                (base_bytes){source, sizeof(source) - 1U}, languages[i],
                &document, &diagnostic) != SVG_SHAPE_ERROR_NONE ||
            !raster_shape_render(&document, &tile, &diagnostic)) {
            svg_shape_document_destroy(&document);
            if (tile.rgba != NULL) raster_tile_destroy(&tile);
            return false;
        }
        size_t offset = (2U * tile.width + 2U) * 4U;
        bool matches = tile.width == 12U && tile.height == 6U &&
            memcmp(tile.rgba + offset, expected[i], 4U) == 0;
        raster_tile_destroy(&tile);
        svg_shape_document_destroy(&document);
        if (!matches) return false;
    }
    return true;
}

