#ifndef VEXEL_PLATFORM_SETTINGS_H
#define VEXEL_PLATFORM_SETTINGS_H

#include "platform_window.h"

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

typedef struct platform_settings_values {
    bool show_welcome_on_launch;
    bool remember_workspace;
    bool structure_visible;
    bool inspector_visible;
    bool default_checkerboard;
    bool default_pixel_grid;
    bool modifier_scroll_zooms;
    /* Active task workspace name; empty when unset. Bounded short identifier. */
    char workspace[16];
    uint8_t density;
} platform_settings_values;

void platform_settings_get(platform_settings_values *out_values);
void platform_settings_show(platform_window *owner_window);
void platform_settings_store_sidebar_visibility(bool structure_visible,
                                               bool inspector_visible);
void platform_settings_store_workspace(const char *name);
void platform_settings_store_density(uint8_t density);
/* Test seam: redirect every settings read and write to a throwaway defaults
 * domain so tests never touch the user's real preferences. NULL or empty
 * restores the real store and deletes the throwaway domain. Also zeroes the
 * write counter. */
void platform_settings_use_test_domain(const char *domain);
/* Number of settings writes issued since the last platform_settings_use_test_domain. */
size_t platform_settings_write_count(void);

#endif
