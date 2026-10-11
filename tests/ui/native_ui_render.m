#import <Cocoa/Cocoa.h>

#include "platform/macos/platform_window_internal.h"
#include "platform/platform_recents.h"

#include <stdio.h>
#include <string.h>

NSView *platform_welcome_golden_view(void);
NSView *platform_settings_golden_view(NSInteger pane);
NSView *platform_help_golden_view(void);
NSView *platform_feedback_golden_view(void);

bool platform_window_dispatch_file_open(platform_window *window,
    const char *path, size_t length)
{
    (void)window;
    (void)path;
    (void)length;
    return false;
}

void platform_window_dispatch_action(platform_window *window,
    platform_viewer_action action)
{
    (void)window;
    (void)action;
}

bool platform_window_open_panel(platform_window *window)
{
    (void)window;
    return false;
}

void platform_macos_menus_refresh_recent(void) {}

static NSView *native_view_named(const char *name, const char *resource_path)
{
    if (strcmp(name, "welcome-empty") == 0 || strcmp(name, "welcome-recent") == 0) {
        platform_recents_clear();
        if (strcmp(name, "welcome-recent") == 0 && resource_path != NULL) {
            size_t path_length = strlen(resource_path);
            if (!platform_recents_add(resource_path, path_length)) return nil;
        }
        return platform_welcome_golden_view();
    }
    if (strcmp(name, "settings-general") == 0)
        return platform_settings_golden_view(0);
    if (strcmp(name, "settings-canvas") == 0)
        return platform_settings_golden_view(1);
    if (strcmp(name, "help") == 0) return platform_help_golden_view();
    if (strcmp(name, "feedback") == 0) return platform_feedback_golden_view();
    if (strcmp(name, "app-icon") == 0 && resource_path != NULL) {
        NSString *path = [NSString stringWithUTF8String:resource_path];
        NSImage *image = [[NSImage alloc] initWithContentsOfFile:path];
        if (image == nil) return nil;
        NSView *view = [[NSView alloc] initWithFrame:NSMakeRect(0.0, 0.0, 256.0, 256.0)];
        NSImageView *icon = [[NSImageView alloc] initWithFrame:NSMakeRect(64.0, 64.0,
            128.0, 128.0)];
        icon.image = image;
        icon.imageScaling = NSImageScaleProportionallyUpOrDown;
        [view addSubview:icon];
        return view;
    }
    return nil;
}

int main(int argc, char **argv)
{
    if (argc < 3 || argc > 4) {
        fprintf(stderr, "usage: native_ui_render SCREEN light|dark [RESOURCE_PATH]\n");
        return 2;
    }
    @autoreleasepool {
        [NSApplication sharedApplication];
        bool dark = strcmp(argv[2], "dark") == 0;
        if (!dark && strcmp(argv[2], "light") != 0) {
            fprintf(stderr, "unknown appearance: %s\n", argv[2]);
            return 2;
        }
        NSAppearanceName appearance_name = dark ? NSAppearanceNameDarkAqua :
            NSAppearanceNameAqua;
        NSAppearance *appearance = [NSAppearance appearanceNamed:appearance_name];
        NSView *view = native_view_named(argv[1], argc == 4 ? argv[3] : NULL);
        if (view == nil || appearance == nil) {
            fprintf(stderr, "could not construct native view: %s\n", argv[1]);
            return 1;
        }
        view.appearance = appearance;
        [view layoutSubtreeIfNeeded];

        NSView *host = [[NSView alloc] initWithFrame:view.bounds];
        host.appearance = appearance;
        host.wantsLayer = YES;
        [appearance performAsCurrentDrawingAppearance:^{
            host.layer.backgroundColor = NSColor.windowBackgroundColor.CGColor;
        }];
        [view removeFromSuperview];
        view.frame = host.bounds;
        [host addSubview:view];
        [host layoutSubtreeIfNeeded];
        NSBitmapImageRep *bitmap = [host bitmapImageRepForCachingDisplayInRect:host.bounds];
        if (bitmap == nil) {
            fprintf(stderr, "could not allocate offscreen bitmap\n");
            return 1;
        }
        [host cacheDisplayInRect:host.bounds toBitmapImageRep:bitmap];
        NSData *png = [bitmap representationUsingType:NSBitmapImageFileTypePNG properties:@{}];
        if (png == nil || fwrite(png.bytes, 1U, png.length, stdout) != png.length) {
            fprintf(stderr, "could not write rendered PNG\n");
            return 1;
        }
        fflush(stdout);
    }
    return 0;
}
