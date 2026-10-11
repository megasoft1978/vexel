#ifndef VEXEL_APP_FEEDBACK_DIAGNOSTICS_H
#define VEXEL_APP_FEEDBACK_DIAGNOSTICS_H

#include <stdbool.h>
#include <stddef.h>

bool feedback_diagnostics_build(const char *app_version, size_t app_version_length,
    const char *macos_version, size_t macos_version_length,
    bool include_optional_details, bool checkerboard_default,
    bool pixel_grid_default, char *out_text, size_t capacity,
    size_t *out_length);

#endif
