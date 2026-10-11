#include "platform_text.h"

#include <CoreFoundation/CoreFoundation.h>
#include <CoreGraphics/CoreGraphics.h>
#include <CoreText/CoreText.h>

#include <math.h>
#include <stdlib.h>
#include <string.h>

typedef struct command_buffer {
    platform_text_path_command *items;
    size_t count;
    size_t capacity;
    platform_media_status status;
} command_buffer;

typedef struct path_context {
    command_buffer *buffer;
    double offset_x;
    double offset_y;
    double current_x;
    double current_y;
} path_context;

static bool finite_point(CGPoint point)
{
    return isfinite((double)point.x) && isfinite((double)point.y);
}

static bool buffer_append(command_buffer *buffer, platform_text_path_verb verb,
                          const double *values, size_t value_count)
{
    if (buffer->status != PLATFORM_MEDIA_OK) {
        return false;
    }
    if (buffer->count >= PLATFORM_TEXT_MAX_PATH_SEGMENTS) {
        buffer->status = PLATFORM_MEDIA_LIMIT;
        return false;
    }
    if (buffer->count == buffer->capacity) {
        size_t next = buffer->capacity == 0U ? 256U : buffer->capacity * 2U;
        if (next > PLATFORM_TEXT_MAX_PATH_SEGMENTS) {
            next = PLATFORM_TEXT_MAX_PATH_SEGMENTS;
        }
        if (next > SIZE_MAX / sizeof(*buffer->items)) {
            buffer->status = PLATFORM_MEDIA_LIMIT;
            return false;
        }
        platform_text_path_command *items = realloc(buffer->items,
                                                     next * sizeof(*items));
        if (items == NULL) {
            buffer->status = PLATFORM_MEDIA_OUT_OF_MEMORY;
            return false;
        }
        buffer->items = items;
        buffer->capacity = next;
    }
    platform_text_path_command command = {0};
    command.verb = verb;
    for (size_t i = 0U; i < value_count; ++i) {
        if (!isfinite(values[i])) {
            buffer->status = PLATFORM_MEDIA_UNSUPPORTED;
            return false;
        }
        command.values[i] = values[i];
    }
    buffer->items[buffer->count++] = command;
    return true;
}

static void path_apply(void *info, const CGPathElement *element)
{
    path_context *context = info;
    if (context->buffer->status != PLATFORM_MEDIA_OK) {
        return;
    }
    const CGPoint *points = element->points;
    switch (element->type) {
    case kCGPathElementMoveToPoint: {
        if (!finite_point(points[0])) {
            context->buffer->status = PLATFORM_MEDIA_UNSUPPORTED;
            return;
        }
        context->current_x = (double)points[0].x + context->offset_x;
        context->current_y = (double)points[0].y + context->offset_y;
        const double values[] = {context->current_x, context->current_y};
        (void)buffer_append(context->buffer, PLATFORM_TEXT_PATH_MOVE, values, 2U);
        break;
    }
    case kCGPathElementAddLineToPoint: {
        if (!finite_point(points[0])) {
            context->buffer->status = PLATFORM_MEDIA_UNSUPPORTED;
            return;
        }
        context->current_x = (double)points[0].x + context->offset_x;
        context->current_y = (double)points[0].y + context->offset_y;
        const double values[] = {context->current_x, context->current_y};
        (void)buffer_append(context->buffer, PLATFORM_TEXT_PATH_LINE, values, 2U);
        break;
    }
    case kCGPathElementAddQuadCurveToPoint: {
        if (!finite_point(points[0]) || !finite_point(points[1])) {
            context->buffer->status = PLATFORM_MEDIA_UNSUPPORTED;
            return;
        }
        double qx = (double)points[0].x + context->offset_x;
        double qy = (double)points[0].y + context->offset_y;
        double ex = (double)points[1].x + context->offset_x;
        double ey = (double)points[1].y + context->offset_y;
        const double values[] = {
            context->current_x + (2.0 / 3.0) * (qx - context->current_x),
            context->current_y + (2.0 / 3.0) * (qy - context->current_y),
            ex + (2.0 / 3.0) * (qx - ex),
            ey + (2.0 / 3.0) * (qy - ey),
            ex, ey
        };
        context->current_x = ex;
        context->current_y = ey;
        (void)buffer_append(context->buffer, PLATFORM_TEXT_PATH_CUBIC, values, 6U);
        break;
    }
    case kCGPathElementAddCurveToPoint: {
        if (!finite_point(points[0]) || !finite_point(points[1]) ||
            !finite_point(points[2])) {
            context->buffer->status = PLATFORM_MEDIA_UNSUPPORTED;
            return;
        }
        const double values[] = {
            (double)points[0].x + context->offset_x,
            (double)points[0].y + context->offset_y,
            (double)points[1].x + context->offset_x,
            (double)points[1].y + context->offset_y,
            (double)points[2].x + context->offset_x,
            (double)points[2].y + context->offset_y
        };
        context->current_x = values[4];
        context->current_y = values[5];
        (void)buffer_append(context->buffer, PLATFORM_TEXT_PATH_CUBIC, values, 6U);
        break;
    }
    case kCGPathElementCloseSubpath:
        (void)buffer_append(context->buffer, PLATFORM_TEXT_PATH_CLOSE, NULL, 0U);
        break;
    }
}

static bool ascii_family_equal(platform_text_slice family, const char *literal)
{
    size_t length = strlen(literal);
    if (family.length != length || family.bytes == NULL) {
        return false;
    }
    for (size_t i = 0U; i < length; ++i) {
        uint8_t value = family.bytes[i];
        if (value >= (uint8_t)'A' && value <= (uint8_t)'Z') {
            value = (uint8_t)(value + ((uint8_t)'a' - (uint8_t)'A'));
        }
        if (value != (uint8_t)literal[i]) {
            return false;
        }
    }
    return true;
}

static CFStringRef family_string(platform_text_slice family)
{
    if (ascii_family_equal(family, "serif")) {
        return CFStringCreateWithCString(kCFAllocatorDefault, "Times",
                                         kCFStringEncodingUTF8);
    }
    if (ascii_family_equal(family, "sans-serif")) {
        return CFStringCreateWithCString(kCFAllocatorDefault, "Helvetica Neue",
                                         kCFStringEncodingUTF8);
    }
    if (ascii_family_equal(family, "monospace")) {
        return CFStringCreateWithCString(kCFAllocatorDefault, "Menlo",
                                         kCFStringEncodingUTF8);
    }
    return CFStringCreateWithBytes(kCFAllocatorDefault, family.bytes,
                                   (CFIndex)family.length,
                                   kCFStringEncodingUTF8, false);
}

static CFNumberRef number_create(double value)
{
    return CFNumberCreate(kCFAllocatorDefault, kCFNumberDoubleType, &value);
}

static CTFontDescriptorRef descriptor_create(platform_text_slice family,
                                             double weight,
                                             platform_text_style style)
{
    CFStringRef name = family_string(family);
    if (name == NULL) {
        return NULL;
    }
    double core_weight = (weight - 400.0) / 500.0;
    if (core_weight < -0.8) core_weight = -0.8;
    if (core_weight > 0.8) core_weight = 0.8;
    CFNumberRef weight_number = number_create(core_weight);
    int32_t symbolic = style == PLATFORM_TEXT_STYLE_NORMAL ? 0 : kCTFontItalicTrait;
    CFNumberRef symbolic_number = CFNumberCreate(kCFAllocatorDefault,
                                                  kCFNumberSInt32Type,
                                                  &symbolic);
    CFMutableDictionaryRef traits = CFDictionaryCreateMutable(
        kCFAllocatorDefault, 0, &kCFTypeDictionaryKeyCallBacks,
        &kCFTypeDictionaryValueCallBacks);
    CFMutableDictionaryRef attributes = CFDictionaryCreateMutable(
        kCFAllocatorDefault, 0, &kCFTypeDictionaryKeyCallBacks,
        &kCFTypeDictionaryValueCallBacks);
    if (weight_number == NULL || symbolic_number == NULL || traits == NULL ||
        attributes == NULL) {
        if (weight_number != NULL) CFRelease(weight_number);
        if (symbolic_number != NULL) CFRelease(symbolic_number);
        if (traits != NULL) CFRelease(traits);
        if (attributes != NULL) CFRelease(attributes);
        CFRelease(name);
        return NULL;
    }
    CFDictionarySetValue(traits, kCTFontWeightTrait, weight_number);
    CFDictionarySetValue(traits, kCTFontSymbolicTrait, symbolic_number);
    CFDictionarySetValue(attributes, kCTFontFamilyNameAttribute, name);
    CFDictionarySetValue(attributes, kCTFontTraitsAttribute, traits);
    CTFontDescriptorRef descriptor = CTFontDescriptorCreateWithAttributes(attributes);
    CFRelease(attributes);
    CFRelease(traits);
    CFRelease(symbolic_number);
    CFRelease(weight_number);
    CFRelease(name);
    return descriptor;
}

static bool is_emoji_code_point(uint32_t code_point)
{
    return (code_point >= 0x1f000U && code_point <= 0x1faffU) ||
           (code_point >= 0x2600U && code_point <= 0x27bfU);
}

static platform_media_status scan_utf8(const uint8_t *bytes, size_t length,
                                       bool *out_has_emoji)
{
    *out_has_emoji = false;
    size_t index = 0U;
    while (index < length) {
        uint8_t first = bytes[index];
        uint32_t code_point = 0U;
        size_t extra = 0U;
        if (first <= 0x7fU) {
            code_point = first;
        } else if (first >= 0xc2U && first <= 0xdfU) {
            code_point = (uint32_t)(first & 0x1fU);
            extra = 1U;
        } else if (first >= 0xe0U && first <= 0xefU) {
            code_point = (uint32_t)(first & 0x0fU);
            extra = 2U;
        } else if (first >= 0xf0U && first <= 0xf4U) {
            code_point = (uint32_t)(first & 0x07U);
            extra = 3U;
        } else {
            return PLATFORM_MEDIA_INVALID_UTF8;
        }
        if (extra > length - index - 1U) {
            return PLATFORM_MEDIA_INVALID_UTF8;
        }
        for (size_t i = 1U; i <= extra; ++i) {
            uint8_t continuation = bytes[index + i];
            if ((continuation & 0xc0U) != 0x80U) {
                return PLATFORM_MEDIA_INVALID_UTF8;
            }
            if (i == 1U && ((first == 0xe0U && continuation < 0xa0U) ||
                            (first == 0xedU && continuation >= 0xa0U) ||
                            (first == 0xf0U && continuation < 0x90U) ||
                            (first == 0xf4U && continuation >= 0x90U))) {
                return PLATFORM_MEDIA_INVALID_UTF8;
            }
            code_point = (code_point << 6U) | (uint32_t)(continuation & 0x3fU);
        }
        if (is_emoji_code_point(code_point)) {
            *out_has_emoji = true;
        }
        index += extra + 1U;
    }
    return PLATFORM_MEDIA_OK;
}

void platform_text_result_destroy(platform_text_result *result)
{
    if (result == NULL) {
        return;
    }
    free(result->glyphs);
    free(result->commands);
    *result = (platform_text_result)PLATFORM_TEXT_RESULT_INIT;
}

platform_media_status platform_text_layout(const platform_text_request *request,
                                           platform_text_result *out_result)
{
    if (request == NULL || out_result == NULL) {
        return PLATFORM_MEDIA_INVALID_ARGUMENT;
    }
    *out_result = (platform_text_result)PLATFORM_TEXT_RESULT_INIT;
    if ((request->text.length != 0U && request->text.bytes == NULL) ||
        request->text.length > PLATFORM_TEXT_MAX_BYTES ||
        (request->font_family_count != 0U && request->font_families == NULL) ||
        request->font_family_count > PLATFORM_TEXT_MAX_FAMILIES ||
        !isfinite(request->size) || request->size <= 0.0 || request->size > 4096.0 ||
        !isfinite(request->weight) || request->weight < 100.0 || request->weight > 900.0 ||
        !isfinite(request->letter_spacing) || fabs(request->letter_spacing) > 4096.0 ||
        request->style < PLATFORM_TEXT_STYLE_NORMAL ||
        request->style > PLATFORM_TEXT_STYLE_OBLIQUE) {
        return request->text.length > PLATFORM_TEXT_MAX_BYTES ||
               request->font_family_count > PLATFORM_TEXT_MAX_FAMILIES
                   ? PLATFORM_MEDIA_LIMIT : PLATFORM_MEDIA_INVALID_ARGUMENT;
    }
    for (size_t i = 0U; i < request->font_family_count; ++i) {
        if (request->font_families[i].bytes == NULL ||
            request->font_families[i].length == 0U ||
            request->font_families[i].length > PLATFORM_TEXT_MAX_FAMILY_BYTES) {
            return request->font_families[i].length > PLATFORM_TEXT_MAX_FAMILY_BYTES
                       ? PLATFORM_MEDIA_LIMIT : PLATFORM_MEDIA_INVALID_ARGUMENT;
        }
        if (memchr(request->font_families[i].bytes, 0,
                   request->font_families[i].length) != NULL) {
            return PLATFORM_MEDIA_INVALID_ARGUMENT;
        }
        CFStringRef validated_family = CFStringCreateWithBytes(
            kCFAllocatorDefault, request->font_families[i].bytes,
            (CFIndex)request->font_families[i].length,
            kCFStringEncodingUTF8, false);
        if (validated_family == NULL) {
            return PLATFORM_MEDIA_INVALID_UTF8;
        }
        CFRelease(validated_family);
    }
    bool has_emoji = false;
    platform_media_status status = scan_utf8(request->text.bytes,
                                             request->text.length, &has_emoji);
    if (status != PLATFORM_MEDIA_OK) {
        return status;
    }
    if (has_emoji) {
        return PLATFORM_MEDIA_UNSUPPORTED;
    }

    static const uint8_t default_family_bytes[] = "sans-serif";
    platform_text_slice default_family = {default_family_bytes,
                                          sizeof(default_family_bytes) - 1U};
    size_t family_count = request->font_family_count == 0U ? 1U :
                          request->font_family_count;
    CFMutableArrayRef cascade = CFArrayCreateMutable(
        kCFAllocatorDefault, 0, &kCFTypeArrayCallBacks);
    if (cascade == NULL) {
        return PLATFORM_MEDIA_OUT_OF_MEMORY;
    }
    CTFontDescriptorRef primary_descriptor = descriptor_create(
        request->font_family_count == 0U ? default_family : request->font_families[0],
        request->weight, request->style);
    if (primary_descriptor == NULL) {
        CFRelease(cascade);
        return PLATFORM_MEDIA_OUT_OF_MEMORY;
    }
    for (size_t i = 1U; i < family_count; ++i) {
        CTFontDescriptorRef descriptor = descriptor_create(
            request->font_families[i], request->weight, request->style);
        if (descriptor == NULL) {
            CFRelease(primary_descriptor);
            CFRelease(cascade);
            return PLATFORM_MEDIA_OUT_OF_MEMORY;
        }
        CFArrayAppendValue(cascade, descriptor);
        CFRelease(descriptor);
    }
    CFMutableDictionaryRef font_attributes = CFDictionaryCreateMutable(
        kCFAllocatorDefault, 0, &kCFTypeDictionaryKeyCallBacks,
        &kCFTypeDictionaryValueCallBacks);
    CFNumberRef font_size = number_create(request->size);
    if (font_attributes == NULL || font_size == NULL) {
        if (font_attributes != NULL) CFRelease(font_attributes);
        if (font_size != NULL) CFRelease(font_size);
        CFRelease(primary_descriptor);
        CFRelease(cascade);
        return PLATFORM_MEDIA_OUT_OF_MEMORY;
    }
    CFDictionarySetValue(font_attributes, kCTFontSizeAttribute, font_size);
    if (CFArrayGetCount(cascade) != 0) {
        CFDictionarySetValue(font_attributes, kCTFontCascadeListAttribute, cascade);
    }
    CTFontDescriptorRef descriptor = CTFontDescriptorCreateCopyWithAttributes(
        primary_descriptor, font_attributes);
    CFRelease(font_attributes);
    CFRelease(font_size);
    CFRelease(primary_descriptor);
    CFRelease(cascade);
    if (descriptor == NULL) {
        return PLATFORM_MEDIA_OUT_OF_MEMORY;
    }
    CTFontRef font = CTFontCreateWithFontDescriptor(descriptor, request->size, NULL);
    CFRelease(descriptor);
    if (font == NULL) {
        return PLATFORM_MEDIA_UNSUPPORTED;
    }

    static const uint8_t empty_text[] = "";
    const uint8_t *text_bytes = request->text.length == 0U ? empty_text :
                                request->text.bytes;
    CFStringRef text = CFStringCreateWithBytes(kCFAllocatorDefault, text_bytes,
        (CFIndex)request->text.length, kCFStringEncodingUTF8, false);
    if (text == NULL) {
        CFRelease(font);
        return PLATFORM_MEDIA_INVALID_UTF8;
    }
    CFMutableAttributedStringRef attributed = CFAttributedStringCreateMutable(
        kCFAllocatorDefault, 0);
    double kern_value = request->letter_spacing;
    CFNumberRef kern = number_create(kern_value);
    int32_t ligature_value = 1;
    CFNumberRef ligature = CFNumberCreate(kCFAllocatorDefault,
                                          kCFNumberSInt32Type,
                                          &ligature_value);
    CFMutableDictionaryRef attributes = CFDictionaryCreateMutable(
        kCFAllocatorDefault, 0, &kCFTypeDictionaryKeyCallBacks,
        &kCFTypeDictionaryValueCallBacks);
    if (attributed == NULL || kern == NULL || ligature == NULL || attributes == NULL) {
        if (attributed != NULL) CFRelease(attributed);
        if (kern != NULL) CFRelease(kern);
        if (ligature != NULL) CFRelease(ligature);
        if (attributes != NULL) CFRelease(attributes);
        CFRelease(text);
        CFRelease(font);
        return PLATFORM_MEDIA_OUT_OF_MEMORY;
    }
    CFDictionarySetValue(attributes, kCTFontAttributeName, font);
    CFDictionarySetValue(attributes, kCTKernAttributeName, kern);
    CFDictionarySetValue(attributes, kCTLigatureAttributeName, ligature);
    CFAttributedStringReplaceString(attributed, CFRangeMake(0, 0), text);
    CFIndex text_length = CFStringGetLength(text);
    if (text_length > 0) {
        CFAttributedStringSetAttributes(attributed, CFRangeMake(0, text_length),
                                        attributes, true);
    }
    CTLineRef line = CTLineCreateWithAttributedString(attributed);
    CFRelease(attributes);
    CFRelease(ligature);
    CFRelease(kern);
    CFRelease(attributed);
    CFRelease(text);
    if (line == NULL) {
        CFRelease(font);
        return PLATFORM_MEDIA_OUT_OF_MEMORY;
    }

    double ascent = 0.0;
    double descent = 0.0;
    double leading = 0.0;
    double width = CTLineGetTypographicBounds(line, &ascent, &descent, &leading);
    if (!isfinite(width) || !isfinite(ascent) || !isfinite(descent) ||
        !isfinite(leading) || width < 0.0) {
        CFRelease(line);
        CFRelease(font);
        return PLATFORM_MEDIA_UNSUPPORTED;
    }
    CFArrayRef runs = CTLineGetGlyphRuns(line);
    CFIndex run_count = CFArrayGetCount(runs);
    size_t glyph_count = 0U;
    for (CFIndex run_index = 0; run_index < run_count; ++run_index) {
        CTRunRef run = (CTRunRef)CFArrayGetValueAtIndex(runs, run_index);
        CFIndex run_glyph_count = CTRunGetGlyphCount(run);
        if (run_glyph_count < 0 || (size_t)run_glyph_count >
            PLATFORM_TEXT_MAX_GLYPHS - glyph_count) {
            CFRelease(line);
            CFRelease(font);
            return PLATFORM_MEDIA_LIMIT;
        }
        glyph_count += (size_t)run_glyph_count;
    }
    if (glyph_count == 0U) {
        ascent = (double)CTFontGetAscent(font);
        descent = (double)CTFontGetDescent(font);
        leading = (double)CTFontGetLeading(font);
    }
    if (glyph_count > SIZE_MAX / sizeof(platform_text_glyph) ||
        glyph_count > SIZE_MAX / sizeof(CGGlyph) ||
        glyph_count > SIZE_MAX / sizeof(CGPoint) ||
        glyph_count > SIZE_MAX / sizeof(CGSize)) {
        CFRelease(line);
        CFRelease(font);
        return PLATFORM_MEDIA_LIMIT;
    }
    platform_text_result result = PLATFORM_TEXT_RESULT_INIT;
    if (glyph_count != 0U) {
        result.glyphs = calloc(glyph_count, sizeof(*result.glyphs));
    }
    CGGlyph *glyph_buffer = glyph_count == 0U ? NULL :
        malloc(glyph_count * sizeof(*glyph_buffer));
    CGPoint *position_buffer = glyph_count == 0U ? NULL :
        malloc(glyph_count * sizeof(*position_buffer));
    CGSize *advance_buffer = glyph_count == 0U ? NULL :
        malloc(glyph_count * sizeof(*advance_buffer));
    if ((glyph_count != 0U && result.glyphs == NULL) ||
        (glyph_count != 0U && (glyph_buffer == NULL || position_buffer == NULL ||
                               advance_buffer == NULL))) {
        free(glyph_buffer);
        free(position_buffer);
        free(advance_buffer);
        platform_text_result_destroy(&result);
        CFRelease(line);
        CFRelease(font);
        return PLATFORM_MEDIA_OUT_OF_MEMORY;
    }
    command_buffer commands = {NULL, 0U, 0U, PLATFORM_MEDIA_OK};
    size_t glyph_output_index = 0U;
    for (CFIndex run_index = 0; run_index < run_count; ++run_index) {
        CTRunRef run = (CTRunRef)CFArrayGetValueAtIndex(runs, run_index);
        CFIndex run_glyph_count_cf = CTRunGetGlyphCount(run);
        size_t run_glyph_count = (size_t)run_glyph_count_cf;
        if (run_glyph_count == 0U) {
            continue;
        }
        CGGlyph *run_glyphs = glyph_buffer + glyph_output_index;
        CGPoint *run_positions = position_buffer + glyph_output_index;
        CGSize *run_advances = advance_buffer + glyph_output_index;
        CFRange range = CFRangeMake(0, run_glyph_count_cf);
        CTRunGetGlyphs(run, range, run_glyphs);
        CTRunGetPositions(run, range, run_positions);
        CTRunGetAdvances(run, range, run_advances);
        CFDictionaryRef run_attributes = CTRunGetAttributes(run);
        CTFontRef run_font = (CTFontRef)CFDictionaryGetValue(
            run_attributes, kCTFontAttributeName);
        if (run_font == NULL) {
            run_font = font;
        }
        for (size_t i = 0U; i < run_glyph_count; ++i) {
            size_t result_index = glyph_output_index + i;
            CGPoint position = run_positions[i];
            CGSize advance = run_advances[i];
            if (!finite_point(position) || !isfinite((double)advance.width) ||
                !isfinite((double)advance.height)) {
                commands.status = PLATFORM_MEDIA_UNSUPPORTED;
                break;
            }
            platform_text_glyph *glyph = &result.glyphs[result_index];
            glyph->glyph_id = (uint16_t)run_glyphs[i];
            glyph->x = (double)position.x;
            glyph->y = (double)position.y;
            glyph->advance = (double)advance.width;
            glyph->command_start = commands.count;
            CGPathRef path = CTFontCreatePathForGlyph(run_font, run_glyphs[i], NULL);
            if (path != NULL) {
                path_context context = {&commands, glyph->x, glyph->y,
                                        glyph->x, glyph->y};
                CGPathApply(path, &context, path_apply);
                CGPathRelease(path);
            }
            glyph->command_count = commands.count - glyph->command_start;
            if (commands.status != PLATFORM_MEDIA_OK) {
                break;
            }
        }
        glyph_output_index += run_glyph_count;
        if (commands.status != PLATFORM_MEDIA_OK) {
            break;
        }
    }
    free(glyph_buffer);
    free(position_buffer);
    free(advance_buffer);
    CFRelease(line);
    CFRelease(font);
    if (commands.status != PLATFORM_MEDIA_OK) {
        free(commands.items);
        platform_text_result_destroy(&result);
        return commands.status;
    }
    result.glyph_count = glyph_count;
    result.commands = commands.items;
    result.command_count = commands.count;
    result.advance_width = width;
    result.ascent = ascent;
    result.descent = descent;
    result.leading = leading;
    *out_result = result;
    return PLATFORM_MEDIA_OK;
}

platform_media_status platform_text_anchor_offset(double advance_width,
                                                  platform_text_anchor anchor,
                                                  double *out_offset)
{
    if (out_offset == NULL || !isfinite(advance_width) || advance_width < 0.0 ||
        anchor < PLATFORM_TEXT_ANCHOR_START || anchor > PLATFORM_TEXT_ANCHOR_END) {
        return PLATFORM_MEDIA_INVALID_ARGUMENT;
    }
    switch (anchor) {
    case PLATFORM_TEXT_ANCHOR_START:
        *out_offset = 0.0;
        return PLATFORM_MEDIA_OK;
    case PLATFORM_TEXT_ANCHOR_MIDDLE:
        *out_offset = -advance_width * 0.5;
        return PLATFORM_MEDIA_OK;
    case PLATFORM_TEXT_ANCHOR_END:
        *out_offset = -advance_width;
        return PLATFORM_MEDIA_OK;
    }
    return PLATFORM_MEDIA_INVALID_ARGUMENT;
}
