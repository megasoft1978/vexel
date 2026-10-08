#import <Cocoa/Cocoa.h>

#include "platform/platform_window.h"
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

struct platform_window {
    __strong NSWindow *native;
    platform_event_node *first;
    platform_event_node *last;
    uint32_t last_pixel_width;
    uint32_t last_pixel_height;
    double last_scale;
};

static bool platform_on_main_thread(void)
{
    return [NSThread isMainThread];
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
    return true;
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
- (BOOL)acceptsFirstResponder { return YES; }
- (void)keyDown:(NSEvent *)event
{
    platform_event value = {0};
    value.type = PLATFORM_EVENT_KEY_DOWN;
    value.key_code = event.keyCode;
    value.modifiers = (uint32_t)event.modifierFlags;
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
    value.modifiers = (uint32_t)event.modifierFlags;
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
- (void)emitButton:(NSEvent *)event type:(platform_event_type)type
{
    NSPoint point = [self convertPoint:event.locationInWindow fromView:nil];
    platform_event value = {0};
    value.type = type;
    value.logical_x = (int32_t)point.x;
    value.logical_y = (int32_t)(NSHeight(self.bounds) - point.y);
    value.button = (uint8_t)event.buttonNumber;
    value.modifiers = (uint32_t)event.modifierFlags;
    (void)platform_emit(self.owner, &value);
}
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
        window->native = native;
        native.owner = window;
        native.delegate = native;
        native.acceptsMouseMovedEvents = YES;
        native.contentMaxSize = NSMakeSize(4096.0, 4096.0);
        VexelView *view = [[VexelView alloc] initWithFrame:NSMakeRect(0.0, 0.0,
            config->width_points, config->height_points)];
        view.owner = window;
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
        *out_window = window;
        return true;
    }
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
        free(window);
    }
}

void platform_shutdown(void)
{
    if (platform_on_main_thread()) {
        [NSApp stop:nil];
    }
}
