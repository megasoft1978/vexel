#ifndef VEXEL_PLATFORM_IMAGE_H
#define VEXEL_PLATFORM_IMAGE_H

#include "platform_media.h"

#include <stddef.h>
#include <stdint.h>

#define PLATFORM_IMAGE_MAX_INPUT_BYTES (32U * 1024U * 1024U)
#define PLATFORM_IMAGE_MAX_PIXELS (64U * 1024U * 1024U)
#define PLATFORM_IMAGE_MAX_DIMENSION 16384U
#define PLATFORM_IMAGE_DATA_URI_PREFIX "data:image/png;base64,"
#define PLATFORM_IMAGE_DATA_URI_PREFIX_LENGTH 22U
#define PLATFORM_IMAGE_MAX_DATA_URI_BYTES 44739266U

typedef struct platform_image {
    uint8_t *rgba; /* owned, premultiplied RGBA8, top row first */
    size_t byte_count;
    size_t row_stride;
    uint32_t width;
    uint32_t height;
} platform_image;

typedef struct platform_image_view {
    const uint8_t *rgba; /* borrowed premultiplied RGBA8 */
    size_t byte_count;
    size_t row_stride;
    uint32_t width;
    uint32_t height;
} platform_image_view;

typedef struct platform_image_bytes {
    uint8_t *bytes; /* owned decoded data-URI payload */
    size_t length;
} platform_image_bytes;

#define PLATFORM_IMAGE_INIT { NULL, 0U, 0U, 0U, 0U }
#define PLATFORM_IMAGE_BYTES_INIT { NULL, 0U }

/* Decodes one static PNG, JPEG, GIF, or ImageIO-supported WebP from borrowed
 * bytes. GIF's sole frame is decoded; multi-frame animation is rejected.
 * No filesystem or network access occurs. The output must be empty and remains
 * empty on failure. Input bytes stay caller-owned. */
platform_media_status platform_image_decode(const uint8_t *bytes, size_t length,
                                            platform_image *out_image);
void platform_image_destroy(platform_image *image);

/* Strict, whitespace-free decoder for the exact data:image/png;base64, form.
 * out_bytes must be empty; returned payload bytes are owned by it. */
platform_media_status platform_image_decode_data_uri(const uint8_t *uri,
                                                     size_t uri_length,
                                                     platform_image_bytes *out_bytes);
void platform_image_bytes_destroy(platform_image_bytes *bytes);

/* Bilinear scaling of a borrowed premultiplied RGBA8 image into an owned image.
 * The source stays caller-owned; out_image must be empty and stays empty on error. */
platform_media_status platform_image_scale_bilinear(platform_image_view source,
                                                    uint32_t width,
                                                    uint32_t height,
                                                    platform_image *out_image);

#endif
