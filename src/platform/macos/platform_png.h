#ifndef VEXEL_PLATFORM_MACOS_PNG_H
#define VEXEL_PLATFORM_MACOS_PNG_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#define PLATFORM_PNG_MAX_BYTES (64U * 1024U * 1024U)
#define PLATFORM_PNG_MAX_PIXELS (16U * 1000U * 1000U)

typedef struct platform_png_blob {
    uint8_t *bytes;
    size_t length;
} platform_png_blob;

bool platform_png_encode_rgba8(const uint8_t *rgba, size_t byte_count,
    size_t row_stride, uint32_t width, uint32_t height,
    platform_png_blob *out_png);
void platform_png_blob_destroy(platform_png_blob *png);

#endif
