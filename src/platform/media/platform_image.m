#include "platform_image.h"

#include <CoreFoundation/CoreFoundation.h>
#include <CoreGraphics/CoreGraphics.h>
#include <ImageIO/ImageIO.h>

#include <math.h>
#include <limits.h>
#include <stdlib.h>
#include <string.h>

static bool size_mul(size_t left, size_t right, size_t *out)
{
    if (out == NULL || (right != 0U && left > SIZE_MAX / right)) {
        return false;
    }
    *out = left * right;
    return true;
}

static bool size_add(size_t left, size_t right, size_t *out)
{
    if (out == NULL || left > SIZE_MAX - right) {
        return false;
    }
    *out = left + right;
    return true;
}

static platform_media_status dimensions_status(uint32_t width, uint32_t height)
{
    if (width == 0U || height == 0U) {
        return PLATFORM_MEDIA_INVALID_ARGUMENT;
    }
    if (width > PLATFORM_IMAGE_MAX_DIMENSION || height > PLATFORM_IMAGE_MAX_DIMENSION ||
        (uint64_t)width * (uint64_t)height > PLATFORM_IMAGE_MAX_PIXELS) {
        return PLATFORM_MEDIA_LIMIT;
    }
    return PLATFORM_MEDIA_OK;
}

void platform_image_destroy(platform_image *image)
{
    if (image == NULL) {
        return;
    }
    free(image->rgba);
    *image = (platform_image)PLATFORM_IMAGE_INIT;
}

void platform_image_bytes_destroy(platform_image_bytes *bytes)
{
    if (bytes == NULL) {
        return;
    }
    free(bytes->bytes);
    *bytes = (platform_image_bytes)PLATFORM_IMAGE_BYTES_INIT;
}

static platform_media_status read_property_dimension(CFDictionaryRef properties,
                                                      CFStringRef key,
                                                      uint32_t *out_value)
{
    CFTypeRef value = CFDictionaryGetValue(properties, key);
    if (value == NULL || CFGetTypeID(value) != CFNumberGetTypeID()) {
        return PLATFORM_MEDIA_CORRUPT;
    }
    int64_t integer = 0;
    if (!CFNumberGetValue((CFNumberRef)value, kCFNumberSInt64Type, &integer) ||
        integer <= 0 || integer > UINT32_MAX) {
        return PLATFORM_MEDIA_CORRUPT;
    }
    *out_value = (uint32_t)integer;
    return PLATFORM_MEDIA_OK;
}

static bool image_type_allowed(CFStringRef type)
{
    return CFStringCompare(type, CFSTR("public.png"), 0) == kCFCompareEqualTo ||
           CFStringCompare(type, CFSTR("public.jpeg"), 0) == kCFCompareEqualTo ||
           CFStringCompare(type, CFSTR("public.gif"), 0) == kCFCompareEqualTo ||
           CFStringCompare(type, CFSTR("com.compuserve.gif"), 0) == kCFCompareEqualTo ||
           CFStringCompare(type, CFSTR("org.webmproject.webp"), 0) == kCFCompareEqualTo ||
           CFStringCompare(type, CFSTR("public.webp"), 0) == kCFCompareEqualTo;
}

platform_media_status platform_image_decode(const uint8_t *bytes, size_t length,
                                            platform_image *out_image)
{
    if (out_image == NULL || (length != 0U && bytes == NULL)) {
        return PLATFORM_MEDIA_INVALID_ARGUMENT;
    }
    *out_image = (platform_image)PLATFORM_IMAGE_INIT;
    if (length > PLATFORM_IMAGE_MAX_INPUT_BYTES) {
        return PLATFORM_MEDIA_LIMIT;
    }
    if (length == 0U) {
        return PLATFORM_MEDIA_CORRUPT;
    }
    if (length > (size_t)LONG_MAX) {
        return PLATFORM_MEDIA_LIMIT;
    }
    CFDataRef data = CFDataCreate(kCFAllocatorDefault, bytes, (CFIndex)length);
    if (data == NULL) {
        return PLATFORM_MEDIA_OUT_OF_MEMORY;
    }
    CGImageSourceRef source = CGImageSourceCreateWithData(data, NULL);
    CFRelease(data);
    if (source == NULL) {
        return PLATFORM_MEDIA_CORRUPT;
    }
    CFStringRef type = CGImageSourceGetType(source);
    if (type == NULL) {
        CFRelease(source);
        return PLATFORM_MEDIA_CORRUPT;
    }
    if (!image_type_allowed(type)) {
        CFRelease(source);
        return PLATFORM_MEDIA_UNSUPPORTED;
    }
    size_t frame_count = CGImageSourceGetCount(source);
    if (frame_count == 0U) {
        CFRelease(source);
        return PLATFORM_MEDIA_CORRUPT;
    }
    if (frame_count > 1U) {
        CFRelease(source);
        return PLATFORM_MEDIA_ANIMATED;
    }
    CFDictionaryRef properties = CGImageSourceCopyPropertiesAtIndex(source, 0U, NULL);
    if (properties == NULL) {
        CFRelease(source);
        return PLATFORM_MEDIA_CORRUPT;
    }
    uint32_t width = 0U;
    uint32_t height = 0U;
    platform_media_status status = read_property_dimension(
        properties, kCGImagePropertyPixelWidth, &width);
    if (status == PLATFORM_MEDIA_OK) {
        status = read_property_dimension(properties, kCGImagePropertyPixelHeight,
                                         &height);
    }
    CFRelease(properties);
    if (status != PLATFORM_MEDIA_OK) {
        CFRelease(source);
        return status;
    }
    status = dimensions_status(width, height);
    if (status == PLATFORM_MEDIA_LIMIT) {
        CFRelease(source);
        return PLATFORM_MEDIA_LIMIT;
    }
    if (status != PLATFORM_MEDIA_OK) {
        CFRelease(source);
        return PLATFORM_MEDIA_CORRUPT;
    }
    size_t row_stride = 0U;
    size_t byte_count = 0U;
    if (!size_mul((size_t)width, 4U, &row_stride) ||
        !size_mul(row_stride, (size_t)height, &byte_count)) {
        CFRelease(source);
        return PLATFORM_MEDIA_LIMIT;
    }
    uint8_t *rgba = calloc(byte_count, 1U);
    if (rgba == NULL) {
        CFRelease(source);
        return PLATFORM_MEDIA_OUT_OF_MEMORY;
    }
    CGImageRef image = CGImageSourceCreateImageAtIndex(source, 0U, NULL);
    CFRelease(source);
    if (image == NULL) {
        free(rgba);
        return PLATFORM_MEDIA_CORRUPT;
    }
    if (CGImageGetWidth(image) != width || CGImageGetHeight(image) != height) {
        CGImageRelease(image);
        free(rgba);
        return PLATFORM_MEDIA_CORRUPT;
    }
    CGColorSpaceRef color_space = CGColorSpaceCreateWithName(kCGColorSpaceSRGB);
    if (color_space == NULL) {
        color_space = CGColorSpaceCreateDeviceRGB();
    }
    if (color_space == NULL) {
        CGImageRelease(image);
        free(rgba);
        return PLATFORM_MEDIA_OUT_OF_MEMORY;
    }
    CGContextRef context = CGBitmapContextCreate(
        rgba, (size_t)width, (size_t)height, 8U, row_stride, color_space,
        (CGBitmapInfo)kCGImageAlphaPremultipliedLast |
        (CGBitmapInfo)kCGBitmapByteOrder32Big);
    CGColorSpaceRelease(color_space);
    if (context == NULL) {
        CGImageRelease(image);
        free(rgba);
        return PLATFORM_MEDIA_OUT_OF_MEMORY;
    }
    CGContextSetBlendMode(context, kCGBlendModeCopy);
    CGContextDrawImage(context, CGRectMake(0.0, 0.0, (CGFloat)width,
                                           (CGFloat)height), image);
    CGContextRelease(context);
    CGImageRelease(image);
    *out_image = (platform_image){rgba, byte_count, row_stride, width, height};
    return PLATFORM_MEDIA_OK;
}

static int base64_value(uint8_t byte)
{
    if (byte >= (uint8_t)'A' && byte <= (uint8_t)'Z') return (int)(byte - (uint8_t)'A');
    if (byte >= (uint8_t)'a' && byte <= (uint8_t)'z') return (int)(byte - (uint8_t)'a') + 26;
    if (byte >= (uint8_t)'0' && byte <= (uint8_t)'9') return (int)(byte - (uint8_t)'0') + 52;
    if (byte == (uint8_t)'+') return 62;
    if (byte == (uint8_t)'/') return 63;
    return -1;
}

platform_media_status platform_image_decode_data_uri(const uint8_t *uri,
                                                     size_t uri_length,
                                                     platform_image_bytes *out_bytes)
{
    if (out_bytes == NULL || (uri_length != 0U && uri == NULL)) {
        return PLATFORM_MEDIA_INVALID_ARGUMENT;
    }
    *out_bytes = (platform_image_bytes)PLATFORM_IMAGE_BYTES_INIT;
    if (uri_length > PLATFORM_IMAGE_MAX_DATA_URI_BYTES) {
        return PLATFORM_MEDIA_LIMIT;
    }
    if (uri_length <= PLATFORM_IMAGE_DATA_URI_PREFIX_LENGTH ||
        memcmp(uri, PLATFORM_IMAGE_DATA_URI_PREFIX,
               PLATFORM_IMAGE_DATA_URI_PREFIX_LENGTH) != 0) {
        return PLATFORM_MEDIA_INVALID_ARGUMENT;
    }
    size_t encoded_length = uri_length - PLATFORM_IMAGE_DATA_URI_PREFIX_LENGTH;
    const uint8_t *encoded = uri + PLATFORM_IMAGE_DATA_URI_PREFIX_LENGTH;
    if (encoded_length == 0U || encoded_length % 4U != 0U) {
        return PLATFORM_MEDIA_INVALID_ARGUMENT;
    }
    size_t padding = 0U;
    if (encoded[encoded_length - 1U] == (uint8_t)'=') padding++;
    if (encoded_length >= 2U && encoded[encoded_length - 2U] == (uint8_t)'=') padding++;
    size_t decoded_length = 0U;
    if (!size_mul(encoded_length / 4U, 3U, &decoded_length) ||
        decoded_length < padding) {
        return PLATFORM_MEDIA_LIMIT;
    }
    decoded_length -= padding;
    if (decoded_length == 0U || decoded_length > PLATFORM_IMAGE_MAX_INPUT_BYTES) {
        return decoded_length > PLATFORM_IMAGE_MAX_INPUT_BYTES
                   ? PLATFORM_MEDIA_LIMIT : PLATFORM_MEDIA_INVALID_ARGUMENT;
    }
    uint8_t *decoded = malloc(decoded_length);
    if (decoded == NULL) {
        return PLATFORM_MEDIA_OUT_OF_MEMORY;
    }
    size_t output_index = 0U;
    size_t quartet_count = encoded_length / 4U;
    for (size_t group = 0U; group < quartet_count; ++group) {
        size_t offset = group * 4U;
        bool last = group + 1U == quartet_count;
        uint8_t c0 = encoded[offset];
        uint8_t c1 = encoded[offset + 1U];
        uint8_t c2 = encoded[offset + 2U];
        uint8_t c3 = encoded[offset + 3U];
        int v0 = base64_value(c0);
        int v1 = base64_value(c1);
        int v2 = c2 == (uint8_t)'=' ? -1 : base64_value(c2);
        int v3 = c3 == (uint8_t)'=' ? -1 : base64_value(c3);
        if (v0 < 0 || v1 < 0 || (!last && (v2 < 0 || v3 < 0)) ||
            (v2 < 0 && c2 != (uint8_t)'=') ||
            (v3 < 0 && c3 != (uint8_t)'=') ||
            (c2 == (uint8_t)'=' && c3 != (uint8_t)'=') ||
            (!last && (c2 == (uint8_t)'=' || c3 == (uint8_t)'=')) ||
            (c2 == (uint8_t)'=' && ((v1 & 0x0f) != 0)) ||
            (c3 == (uint8_t)'=' && c2 != (uint8_t)'=' && ((v2 & 0x03) != 0))) {
            free(decoded);
            return PLATFORM_MEDIA_INVALID_ARGUMENT;
        }
        uint32_t packed = ((uint32_t)v0 << 18U) | ((uint32_t)v1 << 12U) |
                          ((uint32_t)(v2 < 0 ? 0 : v2) << 6U) |
                          (uint32_t)(v3 < 0 ? 0 : v3);
        if (output_index < decoded_length) decoded[output_index++] = (uint8_t)(packed >> 16U);
        if (output_index < decoded_length) decoded[output_index++] = (uint8_t)(packed >> 8U);
        if (output_index < decoded_length) decoded[output_index++] = (uint8_t)packed;
    }
    if (output_index != decoded_length) {
        free(decoded);
        return PLATFORM_MEDIA_INVALID_ARGUMENT;
    }
    *out_bytes = (platform_image_bytes){decoded, decoded_length};
    return PLATFORM_MEDIA_OK;
}

static bool view_valid(platform_image_view source, size_t *out_row_bytes)
{
    if (source.rgba == NULL || source.width == 0U || source.height == 0U) {
        return false;
    }
    size_t row_bytes = 0U;
    if (!size_mul((size_t)source.width, 4U, &row_bytes) ||
        source.row_stride < row_bytes) {
        return false;
    }
    size_t prefix_bytes = 0U;
    size_t required = 0U;
    if (!size_mul((size_t)(source.height - 1U), source.row_stride,
                  &prefix_bytes) || !size_add(prefix_bytes, row_bytes, &required) ||
        required > source.byte_count) {
        return false;
    }
    *out_row_bytes = row_bytes;
    return true;
}

platform_media_status platform_image_scale_bilinear(platform_image_view source,
                                                    uint32_t width,
                                                    uint32_t height,
                                                    platform_image *out_image)
{
    if (out_image == NULL) {
        return PLATFORM_MEDIA_INVALID_ARGUMENT;
    }
    *out_image = (platform_image)PLATFORM_IMAGE_INIT;
    if (source.rgba == NULL || source.width == 0U || source.height == 0U ||
        width == 0U || height == 0U) {
        return PLATFORM_MEDIA_INVALID_ARGUMENT;
    }
    if (dimensions_status(source.width, source.height) == PLATFORM_MEDIA_LIMIT ||
        dimensions_status(width, height) == PLATFORM_MEDIA_LIMIT) {
        return PLATFORM_MEDIA_LIMIT;
    }
    size_t source_row_bytes = 0U;
    if (!view_valid(source, &source_row_bytes) || width == 0U || height == 0U) {
        return PLATFORM_MEDIA_INVALID_ARGUMENT;
    }
    size_t row_stride = 0U;
    size_t byte_count = 0U;
    if (!size_mul((size_t)width, 4U, &row_stride) ||
        !size_mul(row_stride, (size_t)height, &byte_count)) {
        return PLATFORM_MEDIA_LIMIT;
    }
    uint8_t *pixels = malloc(byte_count);
    if (pixels == NULL) {
        return PLATFORM_MEDIA_OUT_OF_MEMORY;
    }
    (void)source_row_bytes;
    for (uint32_t y = 0U; y < height; ++y) {
        double source_y = (((double)y + 0.5) * (double)source.height /
                           (double)height) - 0.5;
        int64_t y0_signed = (int64_t)floor(source_y);
        double fy = source_y - (double)y0_signed;
        if (y0_signed < 0) { y0_signed = 0; fy = 0.0; }
        if ((uint64_t)y0_signed >= (uint64_t)source.height - 1U) {
            y0_signed = (int64_t)source.height - 1;
            fy = 0.0;
        }
        uint32_t y0 = (uint32_t)y0_signed;
        uint32_t y1 = y0 + (y0 + 1U < source.height ? 1U : 0U);
        const uint8_t *row0 = source.rgba + (size_t)y0 * source.row_stride;
        const uint8_t *row1 = source.rgba + (size_t)y1 * source.row_stride;
        for (uint32_t x = 0U; x < width; ++x) {
            double source_x = (((double)x + 0.5) * (double)source.width /
                               (double)width) - 0.5;
            int64_t x0_signed = (int64_t)floor(source_x);
            double fx = source_x - (double)x0_signed;
            if (x0_signed < 0) { x0_signed = 0; fx = 0.0; }
            if ((uint64_t)x0_signed >= (uint64_t)source.width - 1U) {
                x0_signed = (int64_t)source.width - 1;
                fx = 0.0;
            }
            uint32_t x0 = (uint32_t)x0_signed;
            uint32_t x1 = x0 + (x0 + 1U < source.width ? 1U : 0U);
            const uint8_t *p00 = row0 + (size_t)x0 * 4U;
            const uint8_t *p10 = row0 + (size_t)x1 * 4U;
            const uint8_t *p01 = row1 + (size_t)x0 * 4U;
            const uint8_t *p11 = row1 + (size_t)x1 * 4U;
            uint8_t *output = pixels + (size_t)y * row_stride + (size_t)x * 4U;
            for (size_t channel = 0U; channel < 4U; ++channel) {
                double top = (double)p00[channel] * (1.0 - fx) +
                             (double)p10[channel] * fx;
                double bottom = (double)p01[channel] * (1.0 - fx) +
                                (double)p11[channel] * fx;
                double value = top * (1.0 - fy) + bottom * fy;
                if (value < 0.0) value = 0.0;
                if (value > 255.0) value = 255.0;
                output[channel] = (uint8_t)(value + 0.5);
            }
        }
    }
    *out_image = (platform_image){pixels, byte_count, row_stride, width, height};
    return PLATFORM_MEDIA_OK;
}
