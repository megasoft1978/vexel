#ifndef VEXEL_PLATFORM_ANIMATION_H
#define VEXEL_PLATFORM_ANIMATION_H

#include "platform_media.h"

#include <stddef.h>
#include <stdint.h>

#define PLATFORM_ANIMATION_MAX_FRAMES 120U
#define PLATFORM_ANIMATION_MAX_DIMENSION 4096U
#define PLATFORM_ANIMATION_MAX_TOTAL_PIXELS 32000000U
#define PLATFORM_ANIMATION_MAX_ENCODED_BYTES (128U * 1024U * 1024U)

typedef struct platform_animation_frame {
    const uint8_t *rgba; /* borrowed, top-row-first, premultiplied RGBA8 */
    size_t byte_count;
    uint32_t width;
    uint32_t height;
} platform_animation_frame;

typedef struct platform_animation_bytes {
    uint8_t *bytes;
    size_t length;
} platform_animation_bytes;

/* Encodes a looping GIF. Frames are borrowed until this call returns. */
platform_media_status platform_animation_encode_gif(
    const platform_animation_frame *frames, size_t frame_count, uint32_t fps,
    platform_animation_bytes *out);
void platform_animation_bytes_destroy(platform_animation_bytes *bytes);

#endif
