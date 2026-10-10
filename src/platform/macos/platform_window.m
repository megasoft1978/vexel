#import <Cocoa/Cocoa.h>
#import <UniformTypeIdentifiers/UniformTypeIdentifiers.h>

#include "platform/platform_window.h"
#include "platform_window_internal.h"
#include "platform/macos/platform_metal_internal.h"
#include "test_driver/native_injection.h"

#include <limits.h>
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef struct platform_event_node {
    platform_event value;
    struct platform_event_node *next;
} platform_event_node;

@class VexelOpenPanelRequest;

struct platform_window {
    __strong NSWindow *native;
    __strong NSOpenPanel *open_panel;
    __strong VexelOpenPanelRequest *open_panel_request;
    platform_event_node *first;
    platform_event_node *last;
    uint32_t last_pixel_width;
    uint32_t last_pixel_height;
    double last_scale;
    size_t pending_count;
};

@interface VexelOpenPanelRequest : NSObject
@property(nonatomic, assign) platform_window *owner;
@property(nonatomic, strong) NSOpenPanel *panel;
@end

@implementation VexelOpenPanelRequest
@end

#define PLATFORM_EVENT_QUEUE_CAP 256U

static char platform_startup_path[PLATFORM_PATH_CAP];
static bool platform_startup_open_pending;

static bool platform_on_main_thread(void)
{
    return [NSThread isMainThread];
}

static uint32_t platform_modifiers(NSEventModifierFlags flags)
{
    uint32_t value = 0U;
    if ((flags & NSEventModifierFlagCommand) != 0U) value |= PLATFORM_MODIFIER_COMMAND;
    if ((flags & NSEventModifierFlagControl) != 0U) value |= PLATFORM_MODIFIER_CONTROL;
    if ((flags & NSEventModifierFlagShift) != 0U) value |= PLATFORM_MODIFIER_SHIFT;
    if ((flags & NSEventModifierFlagOption) != 0U) value |= PLATFORM_MODIFIER_OPTION;
    return value;
}

void *platform_window_native_view(platform_window *window)
{
    if (!platform_on_main_thread() || window == NULL) {
        return NULL;
    }
    return (__bridge void *)window->native.contentView;
}

static bool platform_dimensions_valid(int32_t width, int32_t height)
{
    return width > 0 && height > 0 && width <= 4096 && height <= 4096;
}

static bool platform_emit(platform_window *window, const platform_event *event)
{
    if (window == NULL || event == NULL) {
        return false;
    }
    if (event->type == PLATFORM_EVENT_POINTER_MOVE || event->type == PLATFORM_EVENT_RESIZE ||
        event->type == PLATFORM_EVENT_SCALE) {
        for (platform_event_node *cursor = window->first; cursor != NULL; cursor = cursor->next) {
            if (cursor->value.type == event->type) {
                cursor->value = *event;
                return true;
            }
        }
    }
    if (window->pending_count >= PLATFORM_EVENT_QUEUE_CAP) return false;
    platform_event_node *node = malloc(sizeof(*node));
    if (node == NULL) {
        return false;
    }
    node->value = *event;
    node->next = NULL;
    if (window->last == NULL) {
        window->first = node;
    } else {
        window->last->next = node;
    }
    window->last = node;
    window->pending_count++;
    return true;
}

static bool platform_path_supported(const char *path, size_t length)
{
    if (path == NULL || length == 0U || length >= PLATFORM_PATH_CAP ||
        memchr(path, '\0', length) != NULL) return false;
    NSString *value = [[NSString alloc] initWithBytes:path length:length
                                             encoding:NSUTF8StringEncoding];
    if (value == nil) return false;
    return [[value.pathExtension lowercaseString] isEqualToString:@"svg"];
}

bool platform_window_dispatch_file_open(platform_window *window, const char *path, size_t length)
{
    if (!platform_path_supported(path, length)) return false;
    if (window == NULL) {
        memcpy(platform_startup_path, path, length);
        platform_startup_path[length] = '\0';
        platform_startup_open_pending = true;
        return true;
    }
    platform_event value = {.type = PLATFORM_EVENT_FILE_OPEN};
    memcpy(value.path, path, length);
    value.path[length] = '\0';
    return platform_emit(window, &value);
}

void platform_window_dispatch_action(platform_window *window, platform_viewer_action action)
{
    if (window != NULL) {
        platform_event event = {.type = PLATFORM_EVENT_ACTION, .action = action};
        (void)platform_emit(window, &event);
    }
}

static bool platform_size_event(platform_window *window, platform_event_type type)
{
    NSWindow *native = window->native;
    NSSize size = native.contentView.bounds.size;
    double scale = native.backingScaleFactor;
    double pixel_width = size.width * scale;
    double pixel_height = size.height * scale;
    if (!(scale > 0.0) || size.width < 1.0 || size.height < 1.0 ||
        size.width > 4096.0 || size.height > 4096.0 ||
        pixel_width < 1.0 || pixel_height < 1.0 ||
        pixel_width > (double)UINT32_MAX - 0.5 ||
        pixel_height > (double)UINT32_MAX - 0.5) {
        return false;
    }
    platform_event event = {0};
    event.type = type;
    event.pixel_width = (uint32_t)llround(pixel_width);
    event.pixel_height = (uint32_t)llround(pixel_height);
    event.backing_scale = scale;
    if (type == PLATFORM_EVENT_RESIZE) {
        if (window->last_pixel_width == event.pixel_width &&
            window->last_pixel_height == event.pixel_height) {
            return true;
        }
    } else if (type == PLATFORM_EVENT_SCALE) {
        if (window->last_scale == scale) {
            return true;
        }
    }
    if (!platform_emit(window, &event)) {
        return false;
    }
    if (type == PLATFORM_EVENT_RESIZE) {
        window->last_pixel_width = event.pixel_width;
        window->last_pixel_height = event.pixel_height;
    } else if (type == PLATFORM_EVENT_SCALE) {
        window->last_scale = scale;
    }
    return true;
}

@interface VexelView : NSView
@property(nonatomic, assign) platform_window *owner;
@end

@implementation VexelView
- (NSDragOperation)draggingEntered:(id<NSDraggingInfo>)sender
{
    NSPasteboard *pasteboard = sender.draggingPasteboard;
    NSArray<NSURL *> *urls = [pasteboard readObjectsForClasses:@[[NSURL class]]
        options:@{NSPasteboardURLReadingFileURLsOnlyKey: @YES}];
    for (NSURL *url in urls) {
        const char *bytes = url.path.fileSystemRepresentation;
        size_t length = bytes == NULL ? 0U : strnlen(bytes, PLATFORM_PATH_CAP);
        if (platform_path_supported(bytes, length)) return NSDragOperationCopy;
    }
    return NSDragOperationNone;
}
- (BOOL)performDragOperation:(id<NSDraggingInfo>)sender
{
    NSPasteboard *pasteboard = sender.draggingPasteboard;
    NSArray<NSURL *> *urls = [pasteboard readObjectsForClasses:@[[NSURL class]]
        options:@{NSPasteboardURLReadingFileURLsOnlyKey: @YES}];
    for (NSURL *url in urls) {
        const char *bytes = url.path.fileSystemRepresentation;
        size_t length = bytes == NULL ? 0U : strnlen(bytes, PLATFORM_PATH_CAP);
        if (platform_window_dispatch_file_open(self.owner, bytes, length)) return YES;
    }
    return NO;
}
- (BOOL)acceptsFirstResponder { return YES; }
- (void)keyDown:(NSEvent *)event
{
    platform_event value = {0};
    value.type = PLATFORM_EVENT_KEY_DOWN;
    value.key_code = event.keyCode;
    value.modifiers = platform_modifiers(event.modifierFlags);
    NSData *data = [event.charactersIgnoringModifiers dataUsingEncoding:NSUTF8StringEncoding];
    size_t amount = data.length < sizeof(value.text) - 1 ? (size_t)data.length : sizeof(value.text) - 1;
    if (amount > 0) {
        memcpy(value.text, data.bytes, amount);
        while (amount > 0 && amount < data.length &&
               (((const uint8_t *)data.bytes)[amount] & 0xc0) == 0x80) {
            amount--;
        }
        value.text_len = amount;
    }
    (void)platform_emit(self.owner, &value);
}
- (void)keyUp:(NSEvent *)event
{
    platform_event value = {0};
    value.type = PLATFORM_EVENT_KEY_UP;
    value.key_code = event.keyCode;
    value.modifiers = platform_modifiers(event.modifierFlags);
    (void)platform_emit(self.owner, &value);
}
- (void)mouseMoved:(NSEvent *)event
{
    NSPoint point = [self convertPoint:event.locationInWindow fromView:nil];
    platform_event value = {0};
    value.type = PLATFORM_EVENT_POINTER_MOVE;
    value.logical_x = (int32_t)point.x;
    value.logical_y = (int32_t)(NSHeight(self.bounds) - point.y);
    value.modifiers = (uint32_t)event.modifierFlags;
    (void)platform_emit(self.owner, &value);
}
- (void)mouseDown:(NSEvent *)event { [self emitButton:event type:PLATFORM_EVENT_POINTER_DOWN]; }
- (void)mouseUp:(NSEvent *)event { [self emitButton:event type:PLATFORM_EVENT_POINTER_UP]; }
- (void)scrollWheel:(NSEvent *)event
{
    NSPoint point = [self convertPoint:event.locationInWindow fromView:nil];
    double x = point.x, y = NSHeight(self.bounds) - point.y;
    double dx = event.scrollingDeltaX, dy = event.scrollingDeltaY;
    if (!isfinite(x) || !isfinite(y) || !isfinite(dx) || !isfinite(dy) ||
        fabs(dx) > 10000.0 || fabs(dy) > 10000.0) return;
    platform_event value = {0};
    value.type = PLATFORM_EVENT_SCROLL;
    value.logical_x = (int32_t)llround(x); value.logical_y = (int32_t)llround(y);
    value.precise_x = x; value.precise_y = y; value.delta_x = dx; value.delta_y = dy;
    value.modifiers = platform_modifiers(event.modifierFlags);
    value.backing_scale = self.window.backingScaleFactor;
    if (!isfinite(value.backing_scale) || !(value.backing_scale > 0.0)) return;
    (void)platform_emit(self.owner, &value);
}
- (void)magnifyWithEvent:(NSEvent *)event
{
    NSPoint point = [self convertPoint:event.locationInWindow fromView:nil];
    double x = point.x, y = NSHeight(self.bounds) - point.y, amount = event.magnification;
    if (!isfinite(x) || !isfinite(y) || !isfinite(amount) || fabs(amount) > 100.0) return;
    platform_event value = {0};
    value.type = PLATFORM_EVENT_MAGNIFY;
    value.logical_x = (int32_t)llround(x); value.logical_y = (int32_t)llround(y);
    value.precise_x = x; value.precise_y = y; value.magnification = amount;
    value.backing_scale = self.window.backingScaleFactor;
    if (!isfinite(value.backing_scale) || !(value.backing_scale > 0.0)) return;
    (void)platform_emit(self.owner, &value);
}
- (void)emitButton:(NSEvent *)event type:(platform_event_type)type
{
    NSPoint point = [self convertPoint:event.locationInWindow fromView:nil];
    platform_event value = {0};
    value.type = type;
    value.logical_x = (int32_t)point.x;
    value.logical_y = (int32_t)(NSHeight(self.bounds) - point.y);
    value.button = (uint8_t)event.buttonNumber;
    value.modifiers = platform_modifiers(event.modifierFlags);
    (void)platform_emit(self.owner, &value);
}
@end

@interface VexelSyntheticMagnifyEvent : NSEvent
@property(nonatomic) NSPoint injectedLocation;
@property(nonatomic) CGFloat injectedMagnification;
@end

@interface VexelSyntheticScrollEvent : NSEvent
@property(nonatomic) NSPoint injectedLocation;
@property(nonatomic) CGFloat injectedDeltaX;
@property(nonatomic) CGFloat injectedDeltaY;
@end

@implementation VexelSyntheticScrollEvent
- (NSPoint)locationInWindow { return self.injectedLocation; }
- (CGFloat)scrollingDeltaX { return self.injectedDeltaX; }
- (CGFloat)scrollingDeltaY { return self.injectedDeltaY; }
@end

@implementation VexelSyntheticMagnifyEvent
- (NSPoint)locationInWindow { return self.injectedLocation; }
- (CGFloat)magnification { return self.injectedMagnification; }
@end

@interface VexelNativeWindow : NSWindow <NSWindowDelegate>
@property(nonatomic, assign) platform_window *owner;
@end

@implementation VexelNativeWindow
- (BOOL)windowShouldClose:(id)sender
{
    (void)sender;
    platform_event event = {0};
    event.type = PLATFORM_EVENT_CLOSE_REQUEST;
    (void)platform_emit(self.owner, &event);
    return NO; /* C owns the opaque handle; close is only a request. */
}
- (void)windowDidResize:(NSNotification *)notification
{
    (void)notification;
    (void)platform_size_event(self.owner, PLATFORM_EVENT_RESIZE);
}
- (void)windowDidChangeBackingProperties:(NSNotification *)notification
{
    (void)notification;
    (void)platform_size_event(self.owner, PLATFORM_EVENT_SCALE);
}
@end

bool platform_init(void)
{
    if (!platform_on_main_thread()) {
        return false;
    }
    (void)[NSApplication sharedApplication];
    [NSApp setActivationPolicy:NSApplicationActivationPolicyRegular];
    platform_macos_menus_init();
    [NSApp finishLaunching];
    return true;
}

bool platform_window_create(const platform_window_config *config, platform_window **out_window)
{
    if (!platform_on_main_thread() || config == NULL || out_window == NULL ||
        config->title_utf8 == NULL || config->title_len == 0 || config->title_len > 1024 ||
        !platform_dimensions_valid(config->width_points, config->height_points)) {
        (void)fprintf(stderr, "platform: invalid window configuration or wrong thread\n");
        return false;
    }
    *out_window = NULL;
    @autoreleasepool {
        for (size_t i = 0; i < config->title_len; i++) {
            if (config->title_utf8[i] == '\0') {
                (void)fputs("platform: title contains an embedded NUL byte\n", stderr);
                return false;
            }
        }
        NSString *title = [[NSString alloc] initWithBytes:config->title_utf8
            length:config->title_len encoding:NSUTF8StringEncoding];
        if (title == nil) {
            (void)fputs("platform: title is not valid UTF-8\n", stderr);
            return false;
        }
        platform_window *window = calloc(1, sizeof(*window));
        if (window == NULL) {
            (void)fputs("platform: window handle allocation failed\n", stderr);
            return false;
        }
        NSUInteger style = NSWindowStyleMaskTitled | NSWindowStyleMaskClosable | NSWindowStyleMaskMiniaturizable;
        if (config->resizable) {
            style |= NSWindowStyleMaskResizable;
        }
        NSRect frame = NSMakeRect(80.0, 80.0, config->width_points, config->height_points);
        VexelNativeWindow *native = [[VexelNativeWindow alloc] initWithContentRect:frame
            styleMask:style backing:NSBackingStoreBuffered defer:NO];
        if (native == nil) {
            (void)fputs("platform: AppKit returned no NSWindow\n", stderr);
            free(window);
            return false;
        }
        native.releasedWhenClosed = NO;
        window->native = native;
        platform_macos_menus_set_owner(window);
        native.owner = window;
        native.delegate = native;
        native.acceptsMouseMovedEvents = YES;
        native.contentMaxSize = NSMakeSize(4096.0, 4096.0);
        VexelView *view = [[VexelView alloc] initWithFrame:NSMakeRect(0.0, 0.0,
            config->width_points, config->height_points)];
        view.owner = window;
        [view registerForDraggedTypes:@[NSPasteboardTypeFileURL]];
        native.contentView = view;
        view.frame = NSMakeRect(0.0, 0.0, config->width_points, config->height_points);
        [native setContentSize:NSMakeSize(config->width_points, config->height_points)];
        native.title = title;
        [native makeKeyAndOrderFront:nil];
        (void)[native makeFirstResponder:view];
        [NSApp activateIgnoringOtherApps:YES];
        if (!platform_emit(window, &(platform_event){.type = PLATFORM_EVENT_OPEN})) {
            (void)fputs("platform: initial event allocation failed\n", stderr);
            platform_window_destroy(window);
            return false;
        }
        (void)platform_size_event(window, PLATFORM_EVENT_RESIZE);
        (void)platform_size_event(window, PLATFORM_EVENT_SCALE);
        if (platform_startup_open_pending) {
            (void)platform_window_dispatch_file_open(window, platform_startup_path,
                strnlen(platform_startup_path, PLATFORM_PATH_CAP));
            platform_startup_path[0] = '\0';
            platform_startup_open_pending = false;
        }
        *out_window = window;
        return true;
    }
}

bool platform_window_open_panel(platform_window *window)
{
    if (!platform_on_main_thread() || window == NULL) return false;
    if (window->open_panel_request != nil) return true;
    @autoreleasepool {
        NSOpenPanel *panel = [NSOpenPanel openPanel];
        UTType *svg_type = [UTType typeWithFilenameExtension:@"svg"];
        if (svg_type == nil) return false;
        panel.allowedContentTypes = @[svg_type];
        panel.canChooseFiles = YES;
        panel.canChooseDirectories = NO;
        panel.allowsMultipleSelection = NO;
        panel.resolvesAliases = YES;
        VexelOpenPanelRequest *request = [[VexelOpenPanelRequest alloc] init];
        request.owner = window;
        request.panel = panel;
        window->open_panel = panel;
        window->open_panel_request = request;
        [panel beginSheetModalForWindow:window->native
            completionHandler:^(NSModalResponse response) {
                platform_window *owner = request.owner;
                if (owner == NULL || owner->open_panel_request != request) return;
                owner->open_panel = nil;
                owner->open_panel_request = nil;
                request.owner = NULL;
                if (response != NSModalResponseOK || panel.URL == nil ||
                    !panel.URL.isFileURL) return;
                const char *path = panel.URL.path.fileSystemRepresentation;
                size_t length = path == NULL ? 0U : strnlen(path, PLATFORM_PATH_CAP);
                (void)platform_window_dispatch_file_open(owner, path, length);
            }];
        return true;
    }
}

bool platform_window_recent_add(platform_window *window, const char *path, size_t path_len)
{
    if (!platform_on_main_thread() || window == NULL) return false;
    return platform_macos_menus_add_recent(path, path_len);
}

bool platform_window_set_option_state(platform_window *window, platform_viewer_action action,
                                      bool enabled)
{
    if (!platform_on_main_thread() || window == NULL) return false;
    return platform_macos_menus_set_option_state(window, action, enabled);
}

bool platform_window_resize(platform_window *window, int32_t width_points, int32_t height_points)
{
    if (!platform_on_main_thread() || window == NULL ||
        !platform_dimensions_valid(width_points, height_points)) {
        return false;
    }
    [window->native setContentSize:NSMakeSize(width_points, height_points)];
    return true;
}

bool platform_window_set_title(platform_window *window, const char *title_utf8, size_t title_len)
{
    if (!platform_on_main_thread() || window == NULL || title_utf8 == NULL ||
        title_len == 0U || title_len > 1024U) {
        return false;
    }
    for (size_t index = 0U; index < title_len; index++) {
        if (title_utf8[index] == '\0') return false;
    }
    @autoreleasepool {
        NSString *title = [[NSString alloc] initWithBytes:title_utf8
            length:title_len encoding:NSUTF8StringEncoding];
        if (title == nil) return false;
        window->native.title = title;
        return true;
    }
}

bool platform_test_inject_key(platform_window *window, bool key_down)
{
    if (!platform_on_main_thread() || window == NULL) {
        return false;
    }
    @autoreleasepool {
        NSEventType type = key_down ? NSEventTypeKeyDown : NSEventTypeKeyUp;
        NSEvent *event = [NSEvent keyEventWithType:type location:NSZeroPoint
            modifierFlags:0 timestamp:0.0 windowNumber:window->native.windowNumber context:nil
            characters:@"a" charactersIgnoringModifiers:@"a" isARepeat:NO keyCode:0];
        [NSApp sendEvent:event];
        return true;
    }
}

bool platform_test_inject_key_text(platform_window *window, const char *text, size_t text_len,
                                   uint16_t key_code, uint32_t modifiers)
{
    if (!platform_on_main_thread() || window == NULL || text == NULL || text_len > 8U ||
        (modifiers & ~(uint32_t)(PLATFORM_MODIFIER_COMMAND | PLATFORM_MODIFIER_CONTROL |
                                PLATFORM_MODIFIER_SHIFT | PLATFORM_MODIFIER_OPTION)) != 0U) {
        return false;
    }
    @autoreleasepool {
        NSString *characters = [[NSString alloc] initWithBytes:text length:text_len
            encoding:NSUTF8StringEncoding];
        if (characters == nil) return false;
        NSEventModifierFlags flags = 0;
        if ((modifiers & PLATFORM_MODIFIER_COMMAND) != 0U) flags |= NSEventModifierFlagCommand;
        if ((modifiers & PLATFORM_MODIFIER_CONTROL) != 0U) flags |= NSEventModifierFlagControl;
        if ((modifiers & PLATFORM_MODIFIER_SHIFT) != 0U) flags |= NSEventModifierFlagShift;
        if ((modifiers & PLATFORM_MODIFIER_OPTION) != 0U) flags |= NSEventModifierFlagOption;
        NSEvent *event = [NSEvent keyEventWithType:NSEventTypeKeyDown location:NSZeroPoint
            modifierFlags:flags timestamp:0.0 windowNumber:window->native.windowNumber context:nil
            characters:characters charactersIgnoringModifiers:characters isARepeat:NO keyCode:key_code];
        [NSApp sendEvent:event];
        return true;
    }
}

bool platform_test_inject_pointer(platform_window *window, platform_event_type type,
                                  int32_t logical_x, int32_t logical_y)
{
    if (!platform_on_main_thread() || window == NULL || logical_x < 0 || logical_y < 0 ||
        (type != PLATFORM_EVENT_POINTER_MOVE && type != PLATFORM_EVENT_POINTER_DOWN &&
         type != PLATFORM_EVENT_POINTER_UP)) {
        return false;
    }
    @autoreleasepool {
        NSPoint point = NSMakePoint(logical_x,
            NSHeight(window->native.contentView.bounds) - logical_y);
        NSEventType event_type = NSEventTypeMouseMoved;
        if (type == PLATFORM_EVENT_POINTER_DOWN) {
            event_type = NSEventTypeLeftMouseDown;
        } else if (type == PLATFORM_EVENT_POINTER_UP) {
            event_type = NSEventTypeLeftMouseUp;
        }
        NSEvent *event = [NSEvent mouseEventWithType:event_type location:point
            modifierFlags:0 timestamp:0.0 windowNumber:window->native.windowNumber context:nil
            eventNumber:0 clickCount:(type == PLATFORM_EVENT_POINTER_MOVE ? 0 : 1)
            pressure:(type == PLATFORM_EVENT_POINTER_DOWN ? 1.0 : 0.0)];
        VexelView *view = (VexelView *)window->native.contentView;
        /* Test injection must not depend on LaunchServices activating the window. */
        if (type == PLATFORM_EVENT_POINTER_MOVE) {
            [view mouseMoved:event];
        } else if (type == PLATFORM_EVENT_POINTER_DOWN) {
            [view mouseDown:event];
        } else {
            [view mouseUp:event];
        }
        return true;
    }
}

bool platform_test_inject_scroll(platform_window *window, double delta_x, double delta_y,
                                 double logical_x, double logical_y)
{
    if (!platform_on_main_thread() || window == NULL || !isfinite(delta_x) ||
        !isfinite(delta_y) || !isfinite(logical_x) || !isfinite(logical_y) ||
        fabs(delta_x) > 10000.0 || fabs(delta_y) > 10000.0 || logical_x < 0.0 ||
        logical_y < 0.0 || logical_x >= 4096.0 || logical_y >= 4096.0) return false;
    @autoreleasepool {
        VexelView *view = (VexelView *)window->native.contentView;
        NSPoint point = NSMakePoint(logical_x, NSHeight(view.bounds) - logical_y);
        VexelSyntheticScrollEvent *event = [[VexelSyntheticScrollEvent alloc] init];
        event.injectedLocation = point;
        event.injectedDeltaX = (CGFloat)delta_x;
        event.injectedDeltaY = (CGFloat)delta_y;
        [view scrollWheel:event];
        return true;
    }
}

bool platform_test_inject_magnify(platform_window *window, double magnification,
                                  double logical_x, double logical_y)
{
    if (!platform_on_main_thread() || window == NULL || !isfinite(magnification) ||
        fabs(magnification) > 100.0 || !isfinite(logical_x) || !isfinite(logical_y) ||
        logical_x < 0.0 || logical_y < 0.0 || logical_x >= 4096.0 || logical_y >= 4096.0) return false;
    @autoreleasepool {
        VexelView *view = (VexelView *)window->native.contentView;
        NSPoint point = NSMakePoint(logical_x, NSHeight(view.bounds) - logical_y);
        VexelSyntheticMagnifyEvent *event = [[VexelSyntheticMagnifyEvent alloc] init];
        event.injectedLocation = point;
        event.injectedMagnification = (CGFloat)magnification;
        [view magnifyWithEvent:event];
        return true;
    }
}

bool platform_test_request_close(platform_window *window)
{
    if (!platform_on_main_thread() || window == NULL) {
        return false;
    }
    @autoreleasepool {
        [window->native performClose:nil];
    }
    return true;
}

bool platform_poll_event(platform_window *window, platform_event *out_event)
{
    if (!platform_on_main_thread() || window == NULL || out_event == NULL) {
        return false;
    }
    @autoreleasepool {
        for (;;) {
            NSEvent *event = [NSApp nextEventMatchingMask:NSEventMaskAny untilDate:[NSDate distantPast]
                inMode:NSDefaultRunLoopMode dequeue:YES];
            if (event == nil) {
                break;
            }
            [NSApp sendEvent:event];
        }
        /* AppKit may settle the initial content view after its first event turn. */
        (void)platform_size_event(window, PLATFORM_EVENT_RESIZE);
        (void)platform_size_event(window, PLATFORM_EVENT_SCALE);
        platform_event_node *node = window->first;
        if (node == NULL) {
            return false;
        }
        window->first = node->next;
        if (window->first == NULL) {
            window->last = NULL;
        }
        *out_event = node->value;
        if (window->pending_count > 0U) window->pending_count--;
        free(node);
        return true;
    }
}

void platform_window_destroy(platform_window *window)
{
    if (!platform_on_main_thread() || window == NULL) {
        return;
    }
    @autoreleasepool {
        VexelOpenPanelRequest *request = window->open_panel_request;
        if (request != nil) {
            request.owner = NULL;
            if (window->open_panel != nil) {
                if (window->native.attachedSheet == window->open_panel)
                    [NSApp endSheet:window->open_panel
                        returnCode:NSModalResponseCancel];
                [window->open_panel orderOut:nil];
            }
            window->open_panel = nil;
            window->open_panel_request = nil;
        }
        [window->native orderOut:nil];
        window->native.delegate = nil;
        [(VexelView *)window->native.contentView setOwner:NULL];
        [(VexelNativeWindow *)window->native setOwner:NULL];
        [window->native close];
        window->native = nil;
        platform_event_node *node = window->first;
        while (node != NULL) {
            platform_event_node *next = node->next;
            free(node);
            node = next;
        }
        platform_macos_menus_clear_owner(window);
        window->pending_count = 0U;
        free(window);
    }
}

void platform_shutdown(void)
{
    if (platform_on_main_thread()) {
        [NSApp stop:nil];
    }
}
