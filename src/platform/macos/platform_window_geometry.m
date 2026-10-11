#import <Cocoa/Cocoa.h>

#include "platform/platform_window.h"
#include "platform_metal_internal.h"

#include <math.h>

bool platform_window_content_screen_origin(platform_window *window,
                                           double *out_x, double *out_y)
{
    if (![NSThread isMainThread] || window == NULL || out_x == NULL || out_y == NULL)
        return false;
    @autoreleasepool {
        NSView *content = (__bridge NSView *)platform_window_native_view(window);
        NSWindow *native = content.window;
        NSScreen *screen = native.screen;
        if (content == nil || native == nil || screen == nil) return false;
        NSNumber *screen_number = screen.deviceDescription[@"NSScreenNumber"];
        if (screen_number == nil) return false;
        CGDirectDisplayID display = (CGDirectDisplayID)screen_number.unsignedIntValue;
        CGRect quartz_bounds = CGDisplayBounds(display);
        NSRect cocoa_bounds = screen.frame;
        NSRect content_in_window = [content convertRect:content.bounds toView:nil];
        NSRect content_on_screen = [native convertRectToScreen:content_in_window];
        double x = CGRectGetMinX(quartz_bounds) + NSMinX(content_on_screen) -
            NSMinX(cocoa_bounds);
        double y = CGRectGetMinY(quartz_bounds) + NSMaxY(cocoa_bounds) -
            NSMaxY(content_on_screen);
        if (!isfinite(x) || !isfinite(y)) return false;
        *out_x = x;
        *out_y = y;
        return true;
    }
}
