#import "platform_png.h"

#import <CoreGraphics/CoreGraphics.h>
#import <ImageIO/ImageIO.h>

#include <stdlib.h>
#include <string.h>

bool platform_png_encode_rgba8(const uint8_t *rgba, size_t byte_count,
    size_t row_stride, uint32_t width, uint32_t height,
    platform_png_blob *out_png)
{
    if (out_png != NULL) *out_png = (platform_png_blob){0};
    if (out_png == NULL || rgba == NULL || width == 0U || height == 0U ||
        width > 4096U || height > 4096U || (size_t)width > SIZE_MAX / 4U ||
        row_stride < (size_t)width * 4U || (size_t)height > SIZE_MAX / row_stride ||
        byte_count < row_stride * (size_t)height ||
        (size_t)width * height > PLATFORM_PNG_MAX_PIXELS) return false;
    @autoreleasepool {
        CGColorSpaceRef color_space = CGColorSpaceCreateWithName(kCGColorSpaceSRGB);
        CGDataProviderRef provider = CGDataProviderCreateWithData(NULL, rgba,
            row_stride * (size_t)height, NULL);
        if (color_space == NULL || provider == NULL) {
            if (color_space != NULL) CGColorSpaceRelease(color_space);
            if (provider != NULL) CGDataProviderRelease(provider);
            return false;
        }
        CGBitmapInfo bitmap = (CGBitmapInfo)kCGBitmapByteOrder32Big |
            (CGBitmapInfo)kCGImageAlphaPremultipliedLast;
        CGImageRef image = CGImageCreate(width, height, 8U, 32U, row_stride,
            color_space, bitmap, provider, NULL, false,
            kCGRenderingIntentDefault);
        CGDataProviderRelease(provider);
        CGColorSpaceRelease(color_space);
        if (image == NULL) return false;
        CFMutableDataRef encoded = CFDataCreateMutable(kCFAllocatorDefault, 0);
        if (encoded == NULL) { CGImageRelease(image); return false; }
        CGImageDestinationRef destination = CGImageDestinationCreateWithData(
            encoded, CFSTR("public.png"), 1U, NULL);
        if (destination == NULL) {
            CFRelease(encoded); CGImageRelease(image); return false;
        }
        CGImageDestinationAddImage(destination, image, NULL);
        bool complete = CGImageDestinationFinalize(destination);
        CFRelease(destination);
        CGImageRelease(image);
        CFIndex encoded_length = CFDataGetLength(encoded);
        if (!complete || encoded_length <= 0 ||
            (uint64_t)encoded_length > PLATFORM_PNG_MAX_BYTES) {
            CFRelease(encoded);
            return false;
        }
        uint8_t *copy = malloc((size_t)encoded_length);
        if (copy == NULL) { CFRelease(encoded); return false; }
        CFDataGetBytes(encoded, CFRangeMake(0, encoded_length), copy);
        CFRelease(encoded);
        *out_png = (platform_png_blob){copy, (size_t)encoded_length};
        return true;
    }
}

void platform_png_blob_destroy(platform_png_blob *png)
{
    if (png == NULL) return;
    free(png->bytes);
    *png = (platform_png_blob){0};
}
