#include "feedback_diagnostics.h"

#include <stdio.h>
#include <string.h>

#define FEEDBACK_VERSION_FIELD_CAP 128U

static bool feedback_field_valid(const char *text, size_t length)
{
    if (text == NULL || length == 0U || length >= FEEDBACK_VERSION_FIELD_CAP ||
        memchr(text, '\0', length) != NULL) return false;
    for (size_t index = 0U; index < length; ++index) {
        unsigned char value = (unsigned char)text[index];
        if (value < 0x20U || value == 0x7fU) return false;
    }
    return true;
}

bool feedback_diagnostics_build(const char *app_version, size_t app_version_length,
    const char *macos_version, size_t macos_version_length,
    bool include_optional_details, bool checkerboard_default,
    bool pixel_grid_default, char *out_text, size_t capacity,
    size_t *out_length)
{
    if (!feedback_field_valid(app_version, app_version_length) ||
        !feedback_field_valid(macos_version, macos_version_length) ||
        out_text == NULL || capacity == 0U || out_length == NULL) return false;
    int amount = snprintf(out_text, capacity,
        "Vexel version: %.*s\nmacOS version: %.*s\n\n"
        "Document names, paths, and SVG contents are excluded.",
        (int)app_version_length, app_version,
        (int)macos_version_length, macos_version);
    if (amount < 0 || (size_t)amount >= capacity) return false;
    size_t length = (size_t)amount;
    if (include_optional_details) {
        size_t remaining = capacity - length;
        amount = snprintf(out_text + length, remaining,
            "\nDiagnostics format: 1\nCanvas defaults: checkerboard=%s; pixel grid=%s.",
            checkerboard_default ? "on" : "off",
            pixel_grid_default ? "on" : "off");
        if (amount < 0 || (size_t)amount >= remaining) return false;
        length += (size_t)amount;
    }
    *out_length = length;
    return true;
}
