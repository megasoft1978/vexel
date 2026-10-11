#import "platform_clipboard.h"

#import <AppKit/AppKit.h>

#include <string.h>

static NSString *const vexel_svg_type = @"public.svg-image";

bool platform_clipboard_write_svg(const uint8_t *bytes, size_t length)
{
    if ((length > 0U && bytes == NULL) || length == 0U ||
        length > PLATFORM_CLIPBOARD_SVG_LIMIT ||
        memchr(bytes, 0, length) != NULL) return false;
    @autoreleasepool {
        NSString *svg = [[NSString alloc] initWithBytes:bytes length:length
            encoding:NSUTF8StringEncoding];
        if (svg == nil) return false;
        NSPasteboard *pasteboard = [NSPasteboard generalPasteboard];
        [pasteboard clearContents];
        NSData *data = [NSData dataWithBytes:bytes length:length];
        return [pasteboard setString:svg forType:NSPasteboardTypeString] &&
            [pasteboard setData:data forType:vexel_svg_type];
    }
}

bool platform_clipboard_write_png(const uint8_t *bytes, size_t length)
{
    if (bytes == NULL || length == 0U || length > 64U * 1024U * 1024U ||
        length < 8U || memcmp(bytes, "\x89PNG\r\n\x1a\n", 8U) != 0) return false;
    @autoreleasepool {
        NSPasteboard *pasteboard = [NSPasteboard generalPasteboard];
        [pasteboard clearContents];
        NSData *data = [NSData dataWithBytes:bytes length:length];
        return [pasteboard setData:data forType:NSPasteboardTypePNG];
    }
}

bool platform_clipboard_can_paste_svg(void)
{
    @autoreleasepool {
        NSPasteboard *pasteboard = [NSPasteboard generalPasteboard];
        return [pasteboard availableTypeFromArray:@[vexel_svg_type,
            NSPasteboardTypeString]] != nil;
    }
}

platform_clipboard_result platform_clipboard_read_svg(uint8_t *buffer,
    size_t capacity, size_t *out_length)
{
    if (buffer == NULL || out_length == NULL) return PLATFORM_CLIPBOARD_FAILURE;
    *out_length = 0U;
    @autoreleasepool {
        NSPasteboard *pasteboard = [NSPasteboard generalPasteboard];
        NSData *data = [pasteboard dataForType:vexel_svg_type];
        if (data == nil) {
            NSString *string = [pasteboard stringForType:NSPasteboardTypeString];
            if (string == nil) return PLATFORM_CLIPBOARD_EMPTY;
            data = [string dataUsingEncoding:NSUTF8StringEncoding];
        }
        if (data == nil || data.length == 0U) return PLATFORM_CLIPBOARD_EMPTY;
        if (data.length > PLATFORM_CLIPBOARD_SVG_LIMIT || data.length > capacity)
            return PLATFORM_CLIPBOARD_LIMIT;
        if (memchr(data.bytes, 0, data.length) != NULL)
            return PLATFORM_CLIPBOARD_FAILURE;
        memcpy(buffer, data.bytes, data.length);
        *out_length = data.length;
        return PLATFORM_CLIPBOARD_OK;
    }
}
