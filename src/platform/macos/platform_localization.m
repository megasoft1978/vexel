#import <Foundation/Foundation.h>

#include "platform/platform_localization.h"
#include "platform/platform_localization_internal.h"
#include "platform/platform_text_ui.h"

#include <math.h>
#include <string.h>

static BOOL pseudo_locale_enabled;

static NSString *platform_localization_pseudo(NSString *text)
{
    if (!pseudo_locale_enabled) return text;
    return [NSString stringWithFormat:@"\u202b⟦%@ · %@⟧\u202c", text, text];
}

static NSString *platform_localization_raw_string(NSString *key,
    NSString *fallback)
{
    if (key.length == 0U || key.length > PLATFORM_LOCALIZATION_KEY_CAP ||
        fallback == nil || fallback.length > PLATFORM_LOCALIZATION_TEXT_CAP) return @"";
    return [NSBundle.mainBundle localizedStringForKey:key value:fallback
        table:@"Localizable"];
}

NSString *platform_localization_string(NSString *key, NSString *fallback,
                                       NSString *comment)
{
    if (key.length == 0U || key.length > PLATFORM_LOCALIZATION_KEY_CAP ||
        fallback == nil || fallback.length > PLATFORM_LOCALIZATION_TEXT_CAP ||
        comment.length > PLATFORM_LOCALIZATION_TEXT_CAP) return @"";
    return platform_localization_pseudo(platform_localization_raw_string(key, fallback));
}

static bool platform_localization_copy_string(NSString *text, char *output,
    size_t capacity, size_t *out_length)
{
    if (text == nil || output == NULL || capacity == 0U || out_length == NULL)
        return false;
    NSData *bytes = [text dataUsingEncoding:NSUTF8StringEncoding];
    if (bytes == nil || bytes.length >= capacity) return false;
    if (bytes.length > 0U) memcpy(output, bytes.bytes, bytes.length);
    output[bytes.length] = '\0';
    *out_length = bytes.length;
    return true;
}

bool platform_localization_copy(const char *key_utf8, const char *fallback_utf8,
    const char *comment_utf8, char *output, size_t capacity, size_t *out_length)
{
    if (key_utf8 == NULL || fallback_utf8 == NULL || comment_utf8 == NULL ||
        output == NULL || capacity == 0U || out_length == NULL) return false;
    size_t key_length = strnlen(key_utf8, PLATFORM_LOCALIZATION_KEY_CAP + 1U);
    size_t fallback_length = strnlen(fallback_utf8,
        PLATFORM_LOCALIZATION_TEXT_CAP + 1U);
    size_t comment_length = strnlen(comment_utf8,
        PLATFORM_LOCALIZATION_TEXT_CAP + 1U);
    if (key_length == 0U || key_length > PLATFORM_LOCALIZATION_KEY_CAP ||
        fallback_length > PLATFORM_LOCALIZATION_TEXT_CAP ||
        comment_length > PLATFORM_LOCALIZATION_TEXT_CAP) return false;
    @autoreleasepool {
        NSString *key = [[NSString alloc] initWithBytes:key_utf8 length:key_length
            encoding:NSUTF8StringEncoding];
        NSString *fallback = [[NSString alloc] initWithBytes:fallback_utf8
            length:fallback_length encoding:NSUTF8StringEncoding];
        NSString *comment = [[NSString alloc] initWithBytes:comment_utf8
            length:comment_length encoding:NSUTF8StringEncoding];
        if (key == nil || fallback == nil || comment == nil) return false;
        NSString *localized = platform_localization_string(key, fallback, comment);
        return platform_localization_copy_string(localized, output, capacity,
            out_length);
    }
}

bool platform_localization_copy_default(const char *fallback_utf8,
    size_t fallback_length, char *output, size_t capacity, size_t *out_length)
{
    if (fallback_utf8 == NULL || fallback_length > PLATFORM_LOCALIZATION_TEXT_CAP ||
        output == NULL || capacity == 0U || out_length == NULL) return false;
    platform_localization_default entry;
    if (!platform_localization_default_find(fallback_utf8, fallback_length, &entry))
        return false;
    return platform_localization_copy(entry.key, entry.fallback, entry.comment,
        output, capacity, out_length);
}

static bool platform_localization_format_number_object(NSNumber *number,
    uint8_t max_fraction_digits, char *output, size_t capacity, size_t *out_length)
{
    if (number == nil || max_fraction_digits > 6U || output == NULL ||
        capacity == 0U || out_length == NULL) return false;
    @autoreleasepool {
        NSNumberFormatter *formatter = [[NSNumberFormatter alloc] init];
        formatter.numberStyle = NSNumberFormatterDecimalStyle;
        formatter.locale = platform_text_ui_app_is_active() ? NSLocale.currentLocale :
            [NSLocale localeWithLocaleIdentifier:@"en_US_POSIX"];
        formatter.maximumFractionDigits = max_fraction_digits;
        formatter.minimumFractionDigits = 0U;
        formatter.usesGroupingSeparator = platform_text_ui_app_is_active();
        NSString *formatted = [formatter stringFromNumber:number];
        if (formatted == nil) return false;
        return platform_localization_copy_string(formatted, output, capacity,
            out_length);
    }
}

bool platform_localization_format_number(double value, uint8_t max_fraction_digits,
    char *output, size_t capacity, size_t *out_length)
{
    if (!isfinite(value) || max_fraction_digits > 6U) return false;
    return platform_localization_format_number_object(@(value), max_fraction_digits,
        output, capacity, out_length);
}

bool platform_localization_format_count(size_t value, char *output,
    size_t capacity, size_t *out_length)
{
    return platform_localization_format_number_object(@((unsigned long long)value),
        0U, output, capacity, out_length);
}

bool platform_localization_format_count_message(size_t value,
    const char *one_key_utf8, const char *one_fallback_utf8,
    const char *many_key_utf8, const char *many_fallback_utf8,
    const char *comment_utf8, char *output, size_t capacity, size_t *out_length)
{
    if (one_key_utf8 == NULL || one_fallback_utf8 == NULL || many_key_utf8 == NULL ||
        many_fallback_utf8 == NULL || comment_utf8 == NULL || output == NULL ||
        capacity == 0U || out_length == NULL) return false;
    size_t one_key_length = strnlen(one_key_utf8, PLATFORM_LOCALIZATION_KEY_CAP + 1U);
    size_t many_key_length = strnlen(many_key_utf8, PLATFORM_LOCALIZATION_KEY_CAP + 1U);
    size_t one_fallback_length = strnlen(one_fallback_utf8,
        PLATFORM_LOCALIZATION_TEXT_CAP + 1U);
    size_t many_fallback_length = strnlen(many_fallback_utf8,
        PLATFORM_LOCALIZATION_TEXT_CAP + 1U);
    size_t comment_length = strnlen(comment_utf8, PLATFORM_LOCALIZATION_TEXT_CAP + 1U);
    if (one_key_length == 0U || one_key_length > PLATFORM_LOCALIZATION_KEY_CAP ||
        many_key_length == 0U || many_key_length > PLATFORM_LOCALIZATION_KEY_CAP ||
        one_fallback_length > PLATFORM_LOCALIZATION_TEXT_CAP ||
        many_fallback_length > PLATFORM_LOCALIZATION_TEXT_CAP ||
        comment_length > PLATFORM_LOCALIZATION_TEXT_CAP) return false;
    @autoreleasepool {
        NSString *one_key = [[NSString alloc] initWithBytes:one_key_utf8
            length:one_key_length encoding:NSUTF8StringEncoding];
        NSString *many_key = [[NSString alloc] initWithBytes:many_key_utf8
            length:many_key_length encoding:NSUTF8StringEncoding];
        NSString *comment = [[NSString alloc] initWithBytes:comment_utf8
            length:comment_length encoding:NSUTF8StringEncoding];
        NSString *one_fallback = [[NSString alloc] initWithBytes:one_fallback_utf8
            length:one_fallback_length encoding:NSUTF8StringEncoding];
        NSString *many_fallback = [[NSString alloc] initWithBytes:many_fallback_utf8
            length:many_fallback_length encoding:NSUTF8StringEncoding];
        if (one_key == nil || many_key == nil || comment == nil ||
            one_fallback == nil || many_fallback == nil) return false;
        if (value == 1U) {
            return platform_localization_copy_string(
                platform_localization_string(one_key, one_fallback, comment),
                output, capacity, out_length);
        }
        NSNumberFormatter *formatter = [[NSNumberFormatter alloc] init];
        formatter.numberStyle = NSNumberFormatterDecimalStyle;
        formatter.locale = platform_text_ui_app_is_active() ? NSLocale.currentLocale :
            [NSLocale localeWithLocaleIdentifier:@"en_US_POSIX"];
        formatter.maximumFractionDigits = 0U;
        formatter.usesGroupingSeparator = platform_text_ui_app_is_active();
        NSString *number = [formatter stringFromNumber:@((unsigned long long)value)];
        NSString *pattern = platform_localization_raw_string(many_key, many_fallback);
        if (number == nil || pattern.length == 0U) return false;
        NSString *message = [NSString stringWithFormat:pattern, number];
        return platform_localization_copy_string(platform_localization_pseudo(message),
            output, capacity, out_length);
    }
}

bool platform_localization_format_one(const char *key_utf8,
    const char *fallback_utf8, const char *comment_utf8,
    const char *argument_utf8, char *output, size_t capacity, size_t *out_length)
{
    if (key_utf8 == NULL || fallback_utf8 == NULL || comment_utf8 == NULL ||
        argument_utf8 == NULL || output == NULL || capacity == 0U ||
        out_length == NULL) return false;
    size_t key_length = strnlen(key_utf8, PLATFORM_LOCALIZATION_KEY_CAP + 1U);
    size_t fallback_length = strnlen(fallback_utf8,
        PLATFORM_LOCALIZATION_TEXT_CAP + 1U);
    size_t comment_length = strnlen(comment_utf8,
        PLATFORM_LOCALIZATION_TEXT_CAP + 1U);
    size_t argument_length = strnlen(argument_utf8,
        PLATFORM_LOCALIZATION_TEXT_CAP + 1U);
    if (key_length == 0U || key_length > PLATFORM_LOCALIZATION_KEY_CAP ||
        fallback_length > PLATFORM_LOCALIZATION_TEXT_CAP ||
        comment_length > PLATFORM_LOCALIZATION_TEXT_CAP ||
        argument_length > PLATFORM_LOCALIZATION_TEXT_CAP) return false;
    @autoreleasepool {
        NSString *key = [[NSString alloc] initWithBytes:key_utf8 length:key_length
            encoding:NSUTF8StringEncoding];
        NSString *fallback = [[NSString alloc] initWithBytes:fallback_utf8
            length:fallback_length encoding:NSUTF8StringEncoding];
        NSString *comment = [[NSString alloc] initWithBytes:comment_utf8
            length:comment_length encoding:NSUTF8StringEncoding];
        NSString *argument = [[NSString alloc] initWithBytes:argument_utf8
            length:argument_length encoding:NSUTF8StringEncoding];
        if (key == nil || fallback == nil || comment == nil || argument == nil)
            return false;
        NSString *pattern = platform_localization_raw_string(key, fallback);
        if (pattern.length == 0U) return false;
        NSString *message = [NSString stringWithFormat:pattern, argument];
        return platform_localization_copy_string(platform_localization_pseudo(message),
            output, capacity, out_length);
    }
}

void platform_localization_test_set_pseudo_locale(bool enabled)
{
    if ([NSThread isMainThread]) pseudo_locale_enabled = enabled ? YES : NO;
}
