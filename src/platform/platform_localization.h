#ifndef VEXEL_PLATFORM_LOCALIZATION_H
#define VEXEL_PLATFORM_LOCALIZATION_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#define PLATFORM_LOCALIZATION_KEY_CAP 128U
#define PLATFORM_LOCALIZATION_TEXT_CAP 4096U

/*
 * Localizes an app-owned string. Key, fallback, and comment are bounded UTF-8
 * NUL-terminated strings; output is caller-owned and NUL-terminated. A false
 * result means the request was invalid or the output capacity was too small.
 */
bool platform_localization_copy(const char *key_utf8, const char *fallback_utf8,
    const char *comment_utf8, char *output, size_t capacity, size_t *out_length);

/* Translates an extracted editor label/status by its registered English default. */
bool platform_localization_copy_default(const char *fallback_utf8,
    size_t fallback_length, char *output, size_t capacity, size_t *out_length);

/* Formats app-owned UI measurements/counts with the current macOS locale. */
bool platform_localization_format_number(double value, uint8_t max_fraction_digits,
    char *output, size_t capacity, size_t *out_length);
bool platform_localization_format_count(size_t value, char *output,
    size_t capacity, size_t *out_length);
bool platform_localization_format_count_message(size_t value,
    const char *one_key, const char *one_fallback,
    const char *many_key, const char *many_fallback, const char *comment,
    char *output, size_t capacity, size_t *out_length);
bool platform_localization_format_one(const char *key_utf8,
    const char *fallback_utf8, const char *comment_utf8,
    const char *argument_utf8, char *output, size_t capacity,
    size_t *out_length);

/* Test hook used by the deterministic pseudo-locale extraction check. */
void platform_localization_test_set_pseudo_locale(bool enabled);

#ifdef __OBJC__
#import <Foundation/Foundation.h>
NSString *platform_localization_string(NSString *key, NSString *fallback,
    NSString *comment);
#endif

#endif
