#ifndef VEXEL_PLATFORM_METAL_H
#define VEXEL_PLATFORM_METAL_H

#include "platform/platform_window.h"

/* Synchronously upload, draw, wait, and verify exact texture bytes. Caller
 * owns rgba until return; diagnostic is caller-owned and NUL terminated. */
bool platform_metal_present_and_verify(platform_window *window, const uint8_t *rgba,
                                       size_t byte_count, uint32_t width,
                                       uint32_t height, char *diagnostic,
                                       size_t diagnostic_capacity);
/* Headless deterministic replay: Metal blits the tile into a shared texture,
 * waits at most five seconds, then verifies and hashes the GPU readback. */
bool platform_metal_copy_and_hash(const uint8_t *rgba, size_t byte_count,
                                  uint32_t width, uint32_t height,
                                  uint8_t out_sha256[32], char *diagnostic,
                                  size_t diagnostic_capacity);
void platform_metal_reset(platform_window *window);

#endif
