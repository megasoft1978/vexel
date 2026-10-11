#include "platform/macos/platform_png.h"
#include "platform/media/platform_image.h"

#include <stdio.h>

int main(void)
{
    static const uint8_t pixels[] = {
        255U, 0U, 0U, 255U, 0U, 255U, 0U, 255U
    };
    platform_png_blob png = {0};
    if (!platform_png_encode_rgba8(pixels, sizeof(pixels), 8U, 2U, 1U, &png))
        return 1;
    platform_image decoded = PLATFORM_IMAGE_INIT;
    bool okay = platform_image_decode(png.bytes, png.length, &decoded) ==
        PLATFORM_MEDIA_OK && decoded.width == 2U && decoded.height == 1U &&
        decoded.row_stride >= 8U && decoded.rgba[0] == 255U &&
        decoded.rgba[1] == 0U && decoded.rgba[4] == 0U && decoded.rgba[5] == 255U;
    platform_image_destroy(&decoded);
    platform_png_blob_destroy(&png);
    if (!okay) return 1;
    platform_png_blob rejected = {0};
    if (platform_png_encode_rgba8(pixels, 67108864U, 16384U, 4096U, 4096U,
            &rejected) || rejected.bytes != NULL || rejected.length != 0U)
        return 1;
    if (platform_png_encode_rgba8(pixels, 16388U, 16388U, 4097U, 1U,
            &rejected) || rejected.bytes != NULL || rejected.length != 0U)
        return 1;
    (void)puts("platform PNG: ImageIO encode and decode round trip passed");
    return 0;
}
