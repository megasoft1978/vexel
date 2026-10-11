#ifndef VEXEL_PLATFORM_LOCALIZATION_INTERNAL_H
#define VEXEL_PLATFORM_LOCALIZATION_INTERNAL_H

#include <stdbool.h>
#include <stddef.h>

typedef struct platform_localization_default {
    const char *key;
    const char *fallback;
    const char *comment;
} platform_localization_default;

bool platform_localization_default_find(const char *text, size_t length,
    platform_localization_default *out_entry);

#endif
