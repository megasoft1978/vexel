#import <Cocoa/Cocoa.h>

#include "platform/platform_window.h"

static NSTimer *animation_timer;
static platform_window *animation_window;

void platform_window_request_animation_ticks(platform_window *window, bool enabled)
{
    if (![NSThread isMainThread] || window == NULL) return;
    if (!enabled) {
        if (animation_window != window) return;
        [animation_timer invalidate];
        animation_timer = nil;
        animation_window = NULL;
        return;
    }
    animation_window = window;
    if (animation_timer != nil) return;
    animation_timer = [NSTimer scheduledTimerWithTimeInterval:1.0 / 60.0 repeats:YES
        block:^(NSTimer *timer) {
            (void)timer;
            platform_window *target = animation_window;
            if (target != NULL) (void)platform_window_dispatch_animation_tick(target);
        }];
}
