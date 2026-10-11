#import <Cocoa/Cocoa.h>

#include "platform/platform_window.h"

#include <math.h>

static NSBezierPath *cursor_arrow(double x, double y, double dx, double dy)
{
    NSBezierPath *path = [NSBezierPath bezierPath];
    [path moveToPoint:NSMakePoint(x, y)];
    [path lineToPoint:NSMakePoint(x + dx, y + dy)];
    double angle = atan2(dy, dx);
    double size = 4.0;
    [path moveToPoint:NSMakePoint(x, y)];
    [path lineToPoint:NSMakePoint(x + cos(angle + 0.65) * size,
        y + sin(angle + 0.65) * size)];
    [path moveToPoint:NSMakePoint(x, y)];
    [path lineToPoint:NSMakePoint(x + cos(angle - 0.65) * size,
        y + sin(angle - 0.65) * size)];
    return path;
}

static NSCursor *cursor_image(NSImage *image, NSPoint hot_spot)
{
    return [[NSCursor alloc] initWithImage:image hotSpot:hot_spot];
}

static NSCursor *diagonal_cursor(bool descending)
{
    static __strong NSCursor *northwest_southeast;
    static __strong NSCursor *northeast_southwest;
    NSCursor *cached = descending ? northwest_southeast : northeast_southwest;
    if (cached != nil) return cached;
    NSImage *image = [[NSImage alloc] initWithSize:NSMakeSize(24.0, 24.0)];
    [image lockFocus];
    [[NSColor whiteColor] setStroke];
    NSBezierPath *halo = [NSBezierPath bezierPath];
    halo.lineWidth = 4.0;
    [halo appendBezierPath:cursor_arrow(descending ? 4.0 : 4.0,
        descending ? 4.0 : 20.0,
        descending ? 16.0 : 16.0,
        descending ? 16.0 : -16.0)];
    [halo stroke];
    [[NSColor blackColor] setStroke];
    NSBezierPath *line = [NSBezierPath bezierPath];
    line.lineWidth = 1.8;
    [line appendBezierPath:cursor_arrow(4.0, descending ? 4.0 : 20.0,
        16.0, descending ? 16.0 : -16.0)];
    [line stroke];
    [image unlockFocus];
    cached = cursor_image(image, NSMakePoint(12.0, 12.0));
    if (descending) northwest_southeast = cached;
    else northeast_southwest = cached;
    return cached;
}

static NSCursor *rotate_cursor(void)
{
    static __strong NSCursor *cached;
    if (cached != nil) return cached;
    NSImage *image = [[NSImage alloc] initWithSize:NSMakeSize(24.0, 24.0)];
    [image lockFocus];
    [[NSColor whiteColor] setStroke];
    NSBezierPath *halo = [NSBezierPath bezierPath];
    halo.lineWidth = 4.0;
    [halo appendBezierPathWithArcWithCenter:NSMakePoint(12.0, 12.0)
        radius:7.0 startAngle:35.0 endAngle:305.0 clockwise:NO];
    [halo stroke];
    [[NSColor blackColor] setStroke];
    NSBezierPath *arc = [NSBezierPath bezierPath];
    arc.lineWidth = 1.8;
    [arc appendBezierPathWithArcWithCenter:NSMakePoint(12.0, 12.0)
        radius:7.0 startAngle:35.0 endAngle:305.0 clockwise:NO];
    [arc stroke];
    [[NSColor blackColor] setFill];
    NSBezierPath *head = [NSBezierPath bezierPath];
    [head moveToPoint:NSMakePoint(20.0, 18.0)];
    [head lineToPoint:NSMakePoint(15.0, 18.0)];
    [head lineToPoint:NSMakePoint(18.0, 14.0)];
    [head closePath];
    [head fill];
    [image unlockFocus];
    cached = cursor_image(image, NSMakePoint(12.0, 12.0));
    return cached;
}

bool platform_window_set_cursor(platform_window *window,
                                platform_cursor_kind cursor)
{
    if (window == NULL || ![NSThread isMainThread]) return false;
    NSCursor *value = [NSCursor arrowCursor];
    switch (cursor) {
        case PLATFORM_CURSOR_ARROW:
            break;
        case PLATFORM_CURSOR_OPEN_HAND:
            value = [NSCursor openHandCursor];
            break;
        case PLATFORM_CURSOR_CLOSED_HAND:
            value = [NSCursor closedHandCursor];
            break;
        case PLATFORM_CURSOR_RESIZE_NORTH_SOUTH:
        case PLATFORM_CURSOR_RESIZE_EAST_WEST:
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wdeprecated-declarations"
            value = cursor == PLATFORM_CURSOR_RESIZE_NORTH_SOUTH ?
                [NSCursor resizeUpDownCursor] : [NSCursor resizeLeftRightCursor];
#pragma clang diagnostic pop
            break;
        case PLATFORM_CURSOR_RESIZE_NORTHWEST_SOUTHEAST:
            value = diagonal_cursor(true);
            break;
        case PLATFORM_CURSOR_RESIZE_NORTHEAST_SOUTHWEST:
            value = diagonal_cursor(false);
            break;
        case PLATFORM_CURSOR_ROTATE:
            value = rotate_cursor();
            break;
        default:
            return false;
    }
    [value set];
    return true;
}
