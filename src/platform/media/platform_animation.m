#include "platform_animation.h"

#include <CoreFoundation/CoreFoundation.h>
#include <CoreGraphics/CoreGraphics.h>
#include <ImageIO/ImageIO.h>

#include <stdlib.h>
#include <string.h>

static bool checked_image_bytes(uint32_t width, uint32_t height, size_t *out_bytes)
{
    if (out_bytes == NULL || width == 0U || height == 0U ||
        width > PLATFORM_ANIMATION_MAX_DIMENSION ||
        height > PLATFORM_ANIMATION_MAX_DIMENSION ||
        (uint64_t)width * height > PLATFORM_ANIMATION_MAX_TOTAL_PIXELS) return false;
    size_t pixels = (size_t)width * (size_t)height;
    if (pixels > SIZE_MAX / 4U) return false;
    *out_bytes = pixels * 4U;
    return true;
}

void platform_animation_bytes_destroy(platform_animation_bytes *bytes)
{
    if (bytes == NULL) return;
    free(bytes->bytes);
    *bytes = (platform_animation_bytes){0};
}

platform_media_status platform_animation_encode_gif(
    const platform_animation_frame *frames, size_t frame_count, uint32_t fps,
    platform_animation_bytes *out)
{
    if (out == NULL || frames == NULL || frame_count == 0U || fps == 0U || fps > 60U)
        return PLATFORM_MEDIA_INVALID_ARGUMENT;
    *out = (platform_animation_bytes){0};
    if (frame_count > PLATFORM_ANIMATION_MAX_FRAMES) return PLATFORM_MEDIA_LIMIT;
    size_t expected_bytes = 0U;
    if (!checked_image_bytes(frames[0].width, frames[0].height, &expected_bytes))
        return PLATFORM_MEDIA_LIMIT;
    uint64_t total_bytes = 0U;
    for (size_t index = 0U; index < frame_count; index++) {
        size_t frame_bytes = 0U;
        if (!checked_image_bytes(frames[index].width, frames[index].height, &frame_bytes) ||
            frames[index].width != frames[0].width ||
            frames[index].height != frames[0].height ||
            frame_bytes != expected_bytes || frames[index].byte_count < frame_bytes ||
            frames[index].rgba == NULL) return PLATFORM_MEDIA_INVALID_ARGUMENT;
        total_bytes += frame_bytes;
        if (total_bytes > PLATFORM_ANIMATION_MAX_ENCODED_BYTES) return PLATFORM_MEDIA_LIMIT;
    }

    CFMutableDataRef output = CFDataCreateMutable(kCFAllocatorDefault, 0);
    if (output == NULL) return PLATFORM_MEDIA_OUT_OF_MEMORY;
    CGImageDestinationRef destination = CGImageDestinationCreateWithData(
        output, CFSTR("com.compuserve.gif"), frame_count, NULL);
    if (destination == NULL) {
        CFRelease(output);
        return PLATFORM_MEDIA_UNSUPPORTED;
    }
    int loop_count = 0;
    CFNumberRef loop_value = CFNumberCreate(kCFAllocatorDefault,
        kCFNumberIntType, &loop_count);
    const void *gif_keys[] = {kCGImagePropertyGIFLoopCount};
    const void *gif_values[] = {loop_value};
    CFDictionaryRef gif_properties = loop_value == NULL ? NULL :
        CFDictionaryCreate(kCFAllocatorDefault, gif_keys, gif_values, 1,
            &kCFTypeDictionaryKeyCallBacks, &kCFTypeDictionaryValueCallBacks);
    const void *destination_keys[] = {kCGImagePropertyGIFDictionary};
    const void *destination_values[] = {gif_properties};
    CFDictionaryRef destination_properties = gif_properties == NULL ? NULL :
        CFDictionaryCreate(kCFAllocatorDefault, destination_keys, destination_values, 1,
            &kCFTypeDictionaryKeyCallBacks, &kCFTypeDictionaryValueCallBacks);
    if (destination_properties != NULL)
        CGImageDestinationSetProperties(destination, destination_properties);

    CGColorSpaceRef color_space = CGColorSpaceCreateDeviceRGB();
    bool okay = color_space != NULL && destination_properties != NULL;
    double delay = 1.0 / (double)fps;
    CFNumberRef delay_value = CFNumberCreate(kCFAllocatorDefault,
        kCFNumberDoubleType, &delay);
    const void *frame_gif_keys[] = {kCGImagePropertyGIFUnclampedDelayTime,
                                    kCGImagePropertyGIFDelayTime};
    const void *frame_gif_values[] = {delay_value, delay_value};
    CFDictionaryRef frame_gif = delay_value == NULL ? NULL :
        CFDictionaryCreate(kCFAllocatorDefault, frame_gif_keys, frame_gif_values, 2,
            &kCFTypeDictionaryKeyCallBacks, &kCFTypeDictionaryValueCallBacks);
    const void *frame_keys[] = {kCGImagePropertyGIFDictionary};
    const void *frame_values[] = {frame_gif};
    CFDictionaryRef frame_properties = frame_gif == NULL ? NULL :
        CFDictionaryCreate(kCFAllocatorDefault, frame_keys, frame_values, 1,
            &kCFTypeDictionaryKeyCallBacks, &kCFTypeDictionaryValueCallBacks);
    if (frame_properties == NULL) okay = false;

    for (size_t index = 0U; okay && index < frame_count; index++) {
        CGDataProviderRef provider = CGDataProviderCreateWithData(NULL,
            frames[index].rgba, expected_bytes, NULL);
        if (provider == NULL) {
            okay = false;
            break;
        }
        CGImageRef image = CGImageCreate(frames[index].width, frames[index].height,
            8U, 32U, (size_t)frames[index].width * 4U, color_space,
            (CGBitmapInfo)kCGBitmapByteOrder32Big |
                (CGBitmapInfo)kCGImageAlphaPremultipliedLast,
            provider, NULL, false, kCGRenderingIntentDefault);
        if (image == NULL) okay = false;
        else CGImageDestinationAddImage(destination, image, frame_properties);
        if (image != NULL) CGImageRelease(image);
        CGDataProviderRelease(provider);
    }
    if (okay) okay = CGImageDestinationFinalize(destination);
    if (frame_properties != NULL) CFRelease(frame_properties);
    if (frame_gif != NULL) CFRelease(frame_gif);
    if (delay_value != NULL) CFRelease(delay_value);
    if (color_space != NULL) CGColorSpaceRelease(color_space);
    if (destination_properties != NULL) CFRelease(destination_properties);
    if (gif_properties != NULL) CFRelease(gif_properties);
    if (loop_value != NULL) CFRelease(loop_value);
    CFRelease(destination);
    if (!okay) {
        CFRelease(output);
        return PLATFORM_MEDIA_CORRUPT;
    }
    CFIndex encoded_length = CFDataGetLength(output);
    if (encoded_length <= 0 || (uint64_t)encoded_length >
        PLATFORM_ANIMATION_MAX_ENCODED_BYTES) {
        CFRelease(output);
        return PLATFORM_MEDIA_LIMIT;
    }
    if (encoded_length >= 6) {
        UInt8 *encoded = CFDataGetMutableBytePtr(output);
        if (encoded != NULL && memcmp(encoded, "GIF87a", 6U) == 0)
            memcpy(encoded, "GIF89a", 6U);
    }
    uint8_t *copy = malloc((size_t)encoded_length);
    if (copy == NULL) {
        CFRelease(output);
        return PLATFORM_MEDIA_OUT_OF_MEMORY;
    }
    memcpy(copy, CFDataGetBytePtr(output), (size_t)encoded_length);
    CFRelease(output);
    out->bytes = copy;
    out->length = (size_t)encoded_length;
    return PLATFORM_MEDIA_OK;
}
