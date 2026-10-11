#ifndef VEXEL_PLATFORM_ACCESSIBILITY_H
#define VEXEL_PLATFORM_ACCESSIBILITY_H

#include "ui/ui_core.h"

#include <stdbool.h>
#include <stdint.h>

typedef void (*platform_accessibility_view_callback)(bool structure_visible,
    bool inspector_visible, void *context);
typedef bool (*platform_accessibility_command_callback)(uint32_t command_id,
    void *context);

#define PLATFORM_ACCESSIBILITY_COMMAND_LAYER_VISIBILITY UINT32_C(0x40000000)
#define PLATFORM_ACCESSIBILITY_COMMAND_LAYER_LOCK UINT32_C(0x80000000)
#define PLATFORM_ACCESSIBILITY_COMMAND_MASK UINT32_C(0xc0000000)

bool platform_accessibility_update(const ui_semantics_snapshot *snapshot,
    uint32_t pixel_width, uint32_t pixel_height, double backing_scale);
bool platform_accessibility_update_tree(const ui_tree *tree,
    uint32_t pixel_width, uint32_t pixel_height, double backing_scale);
void platform_accessibility_set_view_callback(
    platform_accessibility_view_callback callback, void *context,
    bool structure_visible, bool inspector_visible);
void platform_accessibility_clear_view_callback(void *context);
void platform_accessibility_set_command_callback(
    platform_accessibility_command_callback callback, void *context);

#endif
