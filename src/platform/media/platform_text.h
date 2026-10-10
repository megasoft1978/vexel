#ifndef VEXEL_PLATFORM_TEXT_H
#define VEXEL_PLATFORM_TEXT_H

#include "platform_media.h"

#include <stddef.h>
#include <stdint.h>

#define PLATFORM_TEXT_MAX_BYTES (64U * 1024U)
#define PLATFORM_TEXT_MAX_GLYPHS 16384U
#define PLATFORM_TEXT_MAX_PATH_SEGMENTS 262144U
#define PLATFORM_TEXT_MAX_FAMILIES 32U
#define PLATFORM_TEXT_MAX_FAMILY_BYTES 256U

typedef struct platform_text_slice {
    const uint8_t *bytes; /* borrowed UTF-8 */
    size_t length;
} platform_text_slice;

typedef enum platform_text_style {
    PLATFORM_TEXT_STYLE_NORMAL = 0,
    PLATFORM_TEXT_STYLE_ITALIC,
    PLATFORM_TEXT_STYLE_OBLIQUE
} platform_text_style;

typedef enum platform_text_anchor {
    PLATFORM_TEXT_ANCHOR_START = 0,
    PLATFORM_TEXT_ANCHOR_MIDDLE,
    PLATFORM_TEXT_ANCHOR_END
} platform_text_anchor;

typedef struct platform_text_request {
    platform_text_slice text;
    const platform_text_slice *font_families; /* borrowed, in fallback order */
    size_t font_family_count;
    double size; /* points; outline coordinates use the same units */
    double weight; /* CSS numeric weight, 100 through 900 */
    platform_text_style style;
    double letter_spacing; /* points between shaped glyphs */
} platform_text_request;

typedef enum platform_text_path_verb {
    PLATFORM_TEXT_PATH_MOVE = 0,
    PLATFORM_TEXT_PATH_LINE,
    PLATFORM_TEXT_PATH_CUBIC,
    PLATFORM_TEXT_PATH_CLOSE
} platform_text_path_verb;

typedef struct platform_text_path_command {
    platform_text_path_verb verb;
    /* MOVE/LINE: x,y. CUBIC: c1x,c1y,c2x,c2y,x,y. CLOSE: unused. */
    double values[6];
} platform_text_path_command;

typedef struct platform_text_glyph {
    uint16_t glyph_id;
    double x;
    double y;
    double advance;
    size_t command_start;
    size_t command_count;
} platform_text_glyph;

typedef struct platform_text_result {
    platform_text_glyph *glyphs; /* owned; visual run order, baseline-relative */
    size_t glyph_count;
    platform_text_path_command *commands; /* owned; y-up font coordinates */
    size_t command_count;
    double advance_width;
    double ascent;
    double descent;
    double leading;
} platform_text_result;

#define PLATFORM_TEXT_RESULT_INIT { NULL, 0U, NULL, 0U, 0.0, 0.0, 0.0, 0.0 }

/* Shapes borrowed UTF-8 using CoreText. The output must be empty or initialized
 * with PLATFORM_TEXT_RESULT_INIT; on failure it remains empty. Text, families,
 * and request storage remain caller-owned. Kerning and ligatures are enabled.
 * Emoji/color-only glyphs return UNSUPPORTED. Empty text succeeds with metrics. */
platform_media_status platform_text_layout(const platform_text_request *request,
                                           platform_text_result *out_result);
void platform_text_result_destroy(platform_text_result *result);

/* Returns the SVG text-anchor displacement for a finite measured width. */
platform_media_status platform_text_anchor_offset(double advance_width,
                                                  platform_text_anchor anchor,
                                                  double *out_offset);

#endif
