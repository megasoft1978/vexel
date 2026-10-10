#import <Cocoa/Cocoa.h>
#import <UniformTypeIdentifiers/UniformTypeIdentifiers.h>

#include "platform_dialog.h"

#include <string.h>

#if defined(PLATFORM_DIALOG_TESTING)
static platform_dialog_test_hooks dialog_test_hooks;
static bool dialog_test_hooks_installed;

bool platform_dialog_set_test_hooks(const platform_dialog_test_hooks *hooks)
{
    if (![NSThread isMainThread]) return false;
    if (hooks == NULL) {
        dialog_test_hooks = (platform_dialog_test_hooks){0};
        dialog_test_hooks_installed = false;
        return true;
    }
    dialog_test_hooks = *hooks;
    dialog_test_hooks_installed = true;
    return true;
}
#endif

static NSString *dialog_string(const char *bytes, size_t length, size_t limit,
                               bool allow_empty)
{
    if (length > limit || (length > 0U && bytes == NULL) ||
        (!allow_empty && length == 0U) ||
        (length > 0U && memchr(bytes, '\0', length) != NULL)) return nil;
    if (length == 0U) return @"";
    return [[NSString alloc] initWithBytes:bytes length:length
        encoding:NSUTF8StringEncoding];
}

static NSString *suggested_svg_name(const char *bytes, size_t length)
{
    NSString *provided = dialog_string(bytes, length, PLATFORM_DIALOG_TEXT_CAP, true);
    if (provided == nil) return nil;
    NSString *name = provided.lastPathComponent;
    if (name.length == 0U || [name isEqualToString:@"."] ||
        [name isEqualToString:@".."]) name = @"Untitled.svg";
    if (![[name.pathExtension lowercaseString] isEqualToString:@"svg"])
        name = [name stringByAppendingPathExtension:@"svg"];
    return name;
}

static NSString *suggested_png_name(const char *bytes, size_t length)
{
    NSString *provided = dialog_string(bytes, length, PLATFORM_DIALOG_TEXT_CAP, true);
    if (provided == nil) return nil;
    NSString *name = provided.lastPathComponent;
    if (name.length == 0U || [name isEqualToString:@"."] ||
        [name isEqualToString:@".."]) name = @"Untitled.png";
    if (![[name.pathExtension lowercaseString] isEqualToString:@"png"])
        name = [name.stringByDeletingPathExtension stringByAppendingPathExtension:@"png"];
    return name;
}

static bool copy_selected_path(NSString *path, char *out_path,
                               size_t out_capacity, size_t *out_length)
{
    if (path == nil || out_path == NULL || out_length == NULL ||
        ![[path.pathExtension lowercaseString] isEqualToString:@"svg"]) return false;
    NSData *encoded = [path dataUsingEncoding:NSUTF8StringEncoding];
    if (encoded == nil || encoded.length == 0U || encoded.length >= out_capacity ||
        encoded.length >= PLATFORM_DIALOG_PATH_CAP) return false;
    memcpy(out_path, encoded.bytes, encoded.length);
    out_path[encoded.length] = '\0';
    *out_length = encoded.length;
    return true;
}

static bool copy_selected_png_path(NSString *path, char *out_path,
    size_t out_capacity, size_t *out_length)
{
    if (path == nil || out_path == NULL || out_length == NULL ||
        ![[path.pathExtension lowercaseString] isEqualToString:@"png"]) return false;
    NSData *encoded = [path dataUsingEncoding:NSUTF8StringEncoding];
    if (encoded == nil || encoded.length == 0U || encoded.length >= out_capacity ||
        encoded.length >= PLATFORM_DIALOG_PATH_CAP) return false;
    memcpy(out_path, encoded.bytes, encoded.length);
    out_path[encoded.length] = '\0';
    *out_length = encoded.length;
    return true;
}

static bool copy_test_path(const char *path, size_t length, char *out_path,
                           size_t out_capacity, size_t *out_length)
{
    if (path == NULL || length == 0U || length >= out_capacity ||
        length >= PLATFORM_DIALOG_PATH_CAP || memchr(path, '\0', length) != NULL ||
        out_path == NULL || out_length == NULL) return false;
    NSString *value = [[NSString alloc] initWithBytes:path length:length
        encoding:NSUTF8StringEncoding];
    if (value == nil ||
        ![[value.pathExtension lowercaseString] isEqualToString:@"svg"]) return false;
    memcpy(out_path, path, length);
    out_path[length] = '\0';
    *out_length = length;
    return true;
}

static bool copy_test_png_path(const char *path, size_t length, char *out_path,
    size_t out_capacity, size_t *out_length)
{
    if (path == NULL || length == 0U || length >= out_capacity ||
        length >= PLATFORM_DIALOG_PATH_CAP || memchr(path, '\0', length) != NULL ||
        out_path == NULL || out_length == NULL) return false;
    NSString *value = [[NSString alloc] initWithBytes:path length:length
        encoding:NSUTF8StringEncoding];
    if (value == nil ||
        ![[value.pathExtension lowercaseString] isEqualToString:@"png"]) return false;
    memcpy(out_path, path, length);
    out_path[length] = '\0';
    *out_length = length;
    return true;
}

platform_dialog_status platform_dialog_save_svg(const char *suggested_name_utf8,
    size_t suggested_name_length, char *out_path, size_t out_path_capacity,
    size_t *out_path_length)
{
    if (out_path != NULL && out_path_capacity > 0U) out_path[0] = '\0';
    if (out_path_length != NULL) *out_path_length = 0U;
    if (![NSThread isMainThread] || out_path == NULL || out_path_capacity == 0U ||
        out_path_length == NULL) return PLATFORM_DIALOG_ERROR;
    @autoreleasepool {
        NSString *name = suggested_svg_name(suggested_name_utf8,
            suggested_name_length);
        if (name == nil) return PLATFORM_DIALOG_ERROR;
#if defined(PLATFORM_DIALOG_TESTING)
        if (dialog_test_hooks_installed) {
            if (dialog_test_hooks.save_svg == NULL) return PLATFORM_DIALOG_ERROR;
            const char *name_bytes = name.UTF8String;
            size_t name_length = name_bytes == NULL ? 0U : strlen(name_bytes);
            char selected[PLATFORM_DIALOG_PATH_CAP] = {0};
            size_t selected_length = 0U;
            platform_dialog_status status = dialog_test_hooks.save_svg(
                dialog_test_hooks.context, name_bytes, name_length, selected,
                sizeof(selected), &selected_length);
            if (status != PLATFORM_DIALOG_ACCEPT) return
                status == PLATFORM_DIALOG_CANCEL ? status : PLATFORM_DIALOG_ERROR;
            if (selected_length >= sizeof(selected) ||
                selected[selected_length] != '\0' ||
                !copy_test_path(selected, selected_length, out_path,
                    out_path_capacity, out_path_length)) return PLATFORM_DIALOG_ERROR;
            return PLATFORM_DIALOG_ACCEPT;
        }
#endif
        UTType *svg_type = [UTType typeWithFilenameExtension:@"svg"];
        if (svg_type == nil) return PLATFORM_DIALOG_ERROR;
        NSSavePanel *panel = [NSSavePanel savePanel];
        panel.allowedContentTypes = @[svg_type];
        panel.canCreateDirectories = YES;
        panel.nameFieldStringValue = name;
        if ([panel runModal] != NSModalResponseOK) return PLATFORM_DIALOG_CANCEL;
        if (panel.URL == nil || !panel.URL.isFileURL ||
            !copy_selected_path(panel.URL.path, out_path, out_path_capacity,
                out_path_length)) return PLATFORM_DIALOG_ERROR;
        return PLATFORM_DIALOG_ACCEPT;
    }
}

platform_dialog_status platform_dialog_save_png(const char *suggested_name_utf8,
    size_t suggested_name_length, uint8_t maximum_scale,
    char *out_path, size_t out_path_capacity, size_t *out_path_length,
    uint8_t *out_scale, bool *out_selection_only)
{
    if (out_path != NULL && out_path_capacity > 0U) out_path[0] = '\0';
    if (out_path_length != NULL) *out_path_length = 0U;
    if (out_scale != NULL) *out_scale = 0U;
    if (out_selection_only != NULL) *out_selection_only = false;
    if (![NSThread isMainThread] || out_path == NULL || out_path_capacity == 0U ||
        out_path_length == NULL || out_scale == NULL || out_selection_only == NULL ||
        (maximum_scale != 1U && maximum_scale != 2U && maximum_scale != 4U))
        return PLATFORM_DIALOG_ERROR;
    @autoreleasepool {
        NSString *name = suggested_png_name(suggested_name_utf8,
            suggested_name_length);
        if (name == nil) return PLATFORM_DIALOG_ERROR;
#if defined(PLATFORM_DIALOG_TESTING)
        if (dialog_test_hooks_installed) {
            if (dialog_test_hooks.save_png == NULL) return PLATFORM_DIALOG_ERROR;
            const char *name_bytes = name.UTF8String;
            size_t name_length = name_bytes == NULL ? 0U : strlen(name_bytes);
            char selected[PLATFORM_DIALOG_PATH_CAP] = {0};
            size_t selected_length = 0U;
            uint8_t scale = 0U;
            bool selection_only = false;
            platform_dialog_status status = dialog_test_hooks.save_png(
                dialog_test_hooks.context, name_bytes, name_length, maximum_scale,
                selected, sizeof(selected), &selected_length, &scale, &selection_only);
            if (status != PLATFORM_DIALOG_ACCEPT)
                return status == PLATFORM_DIALOG_CANCEL ? status : PLATFORM_DIALOG_ERROR;
            if (selected_length >= sizeof(selected) || selected[selected_length] != '\0' ||
                (scale != 1U && scale != 2U && scale != 4U) || scale > maximum_scale ||
                !copy_test_png_path(selected, selected_length, out_path,
                    out_path_capacity, out_path_length)) return PLATFORM_DIALOG_ERROR;
            *out_scale = scale;
            *out_selection_only = selection_only;
            return PLATFORM_DIALOG_ACCEPT;
        }
#endif
        UTType *png_type = [UTType typeWithFilenameExtension:@"png"];
        if (png_type == nil) return PLATFORM_DIALOG_ERROR;
        NSSavePanel *panel = [NSSavePanel savePanel];
        panel.allowedContentTypes = @[png_type];
        panel.canCreateDirectories = YES;
        panel.nameFieldStringValue = name;
        NSView *accessory = [[NSView alloc] initWithFrame:NSMakeRect(0, 0, 280, 68)];
        NSTextField *scale_label = [NSTextField labelWithString:@"Scale"];
        scale_label.frame = NSMakeRect(0, 40, 50, 22);
        NSPopUpButton *scale_menu = [[NSPopUpButton alloc]
            initWithFrame:NSMakeRect(54, 36, 96, 28) pullsDown:NO];
        uint8_t scales[3] = {1U, 2U, 4U};
        for (size_t index = 0U; index < 3U; ++index) {
            if (scales[index] <= maximum_scale)
                [scale_menu addItemWithTitle:[NSString stringWithFormat:@"%ux", scales[index]]];
        }
        [scale_menu selectItemAtIndex:0];
        NSButton *selection = [NSButton checkboxWithTitle:@"Selection only"
            target:nil action:nil];
        selection.frame = NSMakeRect(0, 4, 180, 24);
        [accessory addSubview:scale_label];
        [accessory addSubview:scale_menu];
        [accessory addSubview:selection];
        panel.accessoryView = accessory;
        if ([panel runModal] != NSModalResponseOK) return PLATFORM_DIALOG_CANCEL;
        if (panel.URL == nil || !panel.URL.isFileURL ||
            !copy_selected_png_path(panel.URL.path, out_path, out_path_capacity,
                out_path_length)) return PLATFORM_DIALOG_ERROR;
        NSInteger selected_index = scale_menu.indexOfSelectedItem;
        if (selected_index < 0 || selected_index >= 3) return PLATFORM_DIALOG_ERROR;
        uint8_t selected_scale = scales[(size_t)selected_index];
        if (selected_scale > maximum_scale) return PLATFORM_DIALOG_ERROR;
        *out_scale = selected_scale;
        *out_selection_only = selection.state == NSControlStateValueOn;
        return PLATFORM_DIALOG_ACCEPT;
    }
}

platform_unsaved_choice platform_dialog_unsaved_changes(
    const char *title_utf8, size_t title_length,
    const char *message_utf8, size_t message_length)
{
    if (![NSThread isMainThread]) return PLATFORM_UNSAVED_ERROR;
    @autoreleasepool {
        NSString *title = dialog_string(title_utf8, title_length,
            PLATFORM_DIALOG_TEXT_CAP, false);
        NSString *message = dialog_string(message_utf8, message_length,
            PLATFORM_DIALOG_TEXT_CAP, false);
        if (title == nil || message == nil) return PLATFORM_UNSAVED_ERROR;
#if defined(PLATFORM_DIALOG_TESTING)
        if (dialog_test_hooks_installed) {
            if (dialog_test_hooks.unsaved_changes == NULL)
                return PLATFORM_UNSAVED_ERROR;
            platform_unsaved_choice answer = dialog_test_hooks.unsaved_changes(
                dialog_test_hooks.context, title_utf8, title_length,
                message_utf8, message_length);
            return answer >= PLATFORM_UNSAVED_SAVE &&
                answer <= PLATFORM_UNSAVED_CANCEL ? answer : PLATFORM_UNSAVED_ERROR;
        }
#endif
        NSAlert *alert = [[NSAlert alloc] init];
        alert.alertStyle = NSAlertStyleWarning;
        alert.messageText = title;
        alert.informativeText = message;
        [alert addButtonWithTitle:@"Save"];
        [alert addButtonWithTitle:@"Don't Save"];
        [alert addButtonWithTitle:@"Cancel"];
        NSModalResponse response = [alert runModal];
        if (response == NSAlertFirstButtonReturn) return PLATFORM_UNSAVED_SAVE;
        if (response == NSAlertFirstButtonReturn + 1) return PLATFORM_UNSAVED_DONT_SAVE;
        if (response == NSAlertFirstButtonReturn + 2) return PLATFORM_UNSAVED_CANCEL;
        return PLATFORM_UNSAVED_ERROR;
    }
}

bool platform_dialog_show_error(const char *title_utf8, size_t title_length,
    const char *message_utf8, size_t message_length)
{
    if (![NSThread isMainThread]) return false;
    @autoreleasepool {
        NSString *title = dialog_string(title_utf8, title_length,
            PLATFORM_DIALOG_TEXT_CAP, false);
        NSString *message = dialog_string(message_utf8, message_length,
            PLATFORM_DIALOG_TEXT_CAP, false);
        if (title == nil || message == nil) return false;
#if defined(PLATFORM_DIALOG_TESTING)
        if (dialog_test_hooks_installed) {
            return dialog_test_hooks.show_error != NULL &&
                dialog_test_hooks.show_error(dialog_test_hooks.context,
                    title_utf8, title_length, message_utf8, message_length);
        }
#endif
        NSAlert *alert = [[NSAlert alloc] init];
        alert.alertStyle = NSAlertStyleCritical;
        alert.messageText = title;
        alert.informativeText = message;
        [alert addButtonWithTitle:@"OK"];
        (void)[alert runModal];
        return true;
    }
}
