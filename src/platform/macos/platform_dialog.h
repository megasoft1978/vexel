#ifndef VEXEL_PLATFORM_MACOS_DIALOG_H
#define VEXEL_PLATFORM_MACOS_DIALOG_H

#include <stdbool.h>
#include <stddef.h>

#define PLATFORM_DIALOG_PATH_CAP 1024U
#define PLATFORM_DIALOG_TEXT_CAP 4096U

typedef enum platform_dialog_status {
    PLATFORM_DIALOG_ERROR = -1,
    PLATFORM_DIALOG_CANCEL = 0,
    PLATFORM_DIALOG_ACCEPT = 1
} platform_dialog_status;

typedef enum platform_unsaved_choice {
    PLATFORM_UNSAVED_ERROR = -1,
    PLATFORM_UNSAVED_SAVE = 0,
    PLATFORM_UNSAVED_DONT_SAVE = 1,
    PLATFORM_UNSAVED_CANCEL = 2
} platform_unsaved_choice;

/* Main-thread-only. On accept, out_path is a NUL-terminated UTF-8 path and
 * out_path_length excludes the terminator. The suggested filename is made to
 * end in .svg before the panel is shown. */
platform_dialog_status platform_dialog_save_svg(const char *suggested_name_utf8,
    size_t suggested_name_length, char *out_path, size_t out_path_capacity,
    size_t *out_path_length);
platform_dialog_status platform_dialog_save_png(const char *suggested_name_utf8,
    size_t suggested_name_length, uint8_t maximum_scale,
    char *out_path, size_t out_path_capacity, size_t *out_path_length,
    uint8_t *out_scale, bool *out_selection_only);

/* Main-thread-only. title and message are borrowed UTF-8 slices. */
platform_unsaved_choice platform_dialog_unsaved_changes(
    const char *title_utf8, size_t title_length,
    const char *message_utf8, size_t message_length);

/* Main-thread-only. Returns false for invalid input or a call from another
 * thread; true once the alert has been dismissed. */
bool platform_dialog_show_error(const char *title_utf8, size_t title_length,
    const char *message_utf8, size_t message_length);

#if defined(PLATFORM_DIALOG_TESTING)
typedef struct platform_dialog_test_hooks {
    void *context;
    platform_dialog_status (*save_svg)(void *context,
        const char *suggested_name_utf8, size_t suggested_name_length,
        char *out_path, size_t out_path_capacity, size_t *out_path_length);
    platform_dialog_status (*save_png)(void *context,
        const char *suggested_name_utf8, size_t suggested_name_length,
        uint8_t maximum_scale, char *out_path, size_t out_path_capacity,
        size_t *out_path_length, uint8_t *out_scale, bool *out_selection_only);
    platform_unsaved_choice (*unsaved_changes)(void *context,
        const char *title_utf8, size_t title_length,
        const char *message_utf8, size_t message_length);
    bool (*show_error)(void *context, const char *title_utf8,
        size_t title_length, const char *message_utf8, size_t message_length);
} platform_dialog_test_hooks;

bool platform_dialog_set_test_hooks(const platform_dialog_test_hooks *hooks);
#endif

#endif
