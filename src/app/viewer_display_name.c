#include "viewer_internal.h"

#include <string.h>

size_t viewer_bounded_length(const char *text, size_t limit)
{
    size_t length = 0U;
    if (text == NULL) return 0U;
    while (length <= limit && text[length] != '\0') length++;
    return length;
}

void viewer_copy_display_name(const char *path, char out[256])
{
    if (path == NULL || out == NULL) return;
    const char *name = path;
    for (const char *cursor = path; *cursor != '\0'; cursor++)
        if (*cursor == '/') name = cursor + 1;
    if (*name == '\0') name = "Open SVG";
    size_t length = viewer_bounded_length(name, 240U);
    if (length > 240U) length = 240U;
    for (size_t index = 0U; index < length; index++) {
        unsigned char value = (unsigned char)name[index];
        out[index] = value >= 32U && value < 127U ? (char)value : '?';
    }
    out[length] = '\0';
}
