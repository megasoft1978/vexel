#include "svg/shape/media.h"
#include "svg/shape/shape.h"

#include <stdio.h>
#include <string.h>

svg_shape_media_providers svg_app_media_providers(void);

int main(void)
{
    static const char source[] =
        "<svg xmlns='http://www.w3.org/2000/svg' width='16' height='16'>"
        "<text x='1' y='12' font-size='10'>A</text>"
        "<image x='8' y='8' width='4' height='4' href='data:image/png;base64,"
        "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJ"
        "AAAADUlEQVR4nGP4z8DwHwAFAAH/iZk9HQAAAABJRU5ErkJggg=='/>"
        "</svg>";
    svg_shape_media_providers providers = svg_app_media_providers();
    svg_shape_document document = {0};
    svg_shape_diagnostic diagnostic = {0};
    svg_shape_error error = svg_shape_decode_with_media(
        (base_bytes){(const uint8_t *)source, sizeof(source) - 1U},
        &providers, &document, &diagnostic);
    if (error != SVG_SHAPE_ERROR_NONE) {
        (void)fprintf(stderr, "media decode failed: %s at %zu\n",
            svg_shape_error_name(error), diagnostic.offset);
        return 1;
    }
    bool has_image = false;
    for (size_t index = 0U; index < document.shape_count; ++index)
        if (document.images != NULL && document.images[index].rgba != NULL &&
            document.images[index].pixel_width == 1U &&
            document.images[index].pixel_height == 1U)
            has_image = true;
    bool okay = document.shape_count >= 2U && document.edge_count > 0U &&
        has_image;
    svg_shape_document_destroy(&document);
    if (!okay) {
        (void)fputs("media provider did not produce text outlines and a decoded image\n",
            stderr);
        return 1;
    }
    (void)puts("media provider: text outline and embedded PNG decode passed");
    return 0;
}
