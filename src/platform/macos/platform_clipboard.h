#ifndef VEXEL_PLATFORM_MACOS_CLIPBOARD_H
#define VEXEL_PLATFORM_MACOS_CLIPBOARD_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#define PLATFORM_CLIPBOARD_SVG_LIMIT (64U * 1024U)

typedef enum platform_clipboard_result {
    PLATFORM_CLIPBOARD_OK = 0,
    PLATFORM_CLIPBOARD_EMPTY,
    PLATFORM_CLIPBOARD_LIMIT,
    PLATFORM_CLIPBOARD_FAILURE
} platform_clipboard_result;

bool platform_clipboard_write_svg(const uint8_t *bytes, size_t length);
bool platform_clipboard_write_png(const uint8_t *bytes, size_t length);
platform_clipboard_result platform_clipboard_read_svg(uint8_t *buffer,
    size_t capacity, size_t *out_length);

#endif
