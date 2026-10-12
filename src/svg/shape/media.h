#ifndef VEXEL_SVG_SHAPE_MEDIA_H
#define VEXEL_SVG_SHAPE_MEDIA_H

#include "base.h"

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

/* Per text token budget. */
#define SVG_SHAPE_MEDIA_MAX_TEXT_BYTES (64U * 1024U)
/* Document text budget. The accumulated budget used to be the per-token 64 KiB,
 * which refused any drawing with more than about 64 KiB of labels. */
#define SVG_SHAPE_MEDIA_MAX_DOCUMENT_TEXT_BYTES (64U * 1024U * 1024U)
/* Per text-run budgets: a single span or layout run stays bounded. */
#define SVG_SHAPE_MEDIA_MAX_GLYPHS 65536U
#define SVG_SHAPE_MEDIA_MAX_PATH_COMMANDS 262144U
/* Document budgets. Real drawings carry thousands of labels, so a document-wide
 * budget of 16384 glyphs refused documents a viewer must render; these stay
 * finite so a pathological input is still bounded. */
#define SVG_SHAPE_MEDIA_MAX_DOCUMENT_GLYPHS (1024U * 1024U)
#define SVG_SHAPE_MEDIA_MAX_DOCUMENT_PATH_COMMANDS (4U * 1024U * 1024U)
#define SVG_SHAPE_MEDIA_MAX_FAMILIES 32U
#define SVG_SHAPE_MEDIA_MAX_IMAGE_PIXELS (64U * 1024U * 1024U)

typedef struct svg_shape_text_family {
    const uint8_t *bytes; /* borrowed UTF-8 family name */
    size_t length;
} svg_shape_text_family;

typedef enum svg_shape_text_style {
    SVG_SHAPE_TEXT_NORMAL = 0,
    SVG_SHAPE_TEXT_ITALIC,
    SVG_SHAPE_TEXT_OBLIQUE
} svg_shape_text_style;

typedef struct svg_shape_text_request {
    base_bytes text; /* borrowed UTF-8 */
    const svg_shape_text_family *font_families; /* borrowed */
    size_t font_family_count;
    double size_points;
    double weight;
    svg_shape_text_style style;
    double letter_spacing_points;
} svg_shape_text_request;

typedef enum svg_shape_text_verb {
    SVG_SHAPE_TEXT_MOVE = 0,
    SVG_SHAPE_TEXT_LINE,
    SVG_SHAPE_TEXT_CUBIC,
    SVG_SHAPE_TEXT_CLOSE
} svg_shape_text_verb;

typedef struct svg_shape_text_command {
    svg_shape_text_verb verb;
    double values[6]; /* outline coordinates local to the corresponding glyph */
} svg_shape_text_command;

typedef struct svg_shape_text_glyph {
    double x;
    double y;
    double advance;
    size_t command_start;
    size_t command_count;
} svg_shape_text_glyph;

typedef struct svg_shape_text_layout {
    svg_shape_text_glyph *glyphs; /* owned by the provider */
    size_t glyph_count;
    svg_shape_text_command *commands; /* owned by the provider */
    size_t command_count;
    double advance_width;
    double ascent;
    double descent;
    double leading;
} svg_shape_text_layout;

typedef struct svg_shape_decoded_image {
    const uint8_t *rgba; /* owned by provider; premultiplied RGBA8, top row first */
    size_t byte_count;
    size_t row_stride;
    uint32_t width;
    uint32_t height;
    void *owner; /* provider-private release token */
} svg_shape_decoded_image;

typedef enum svg_shape_media_status {
    SVG_SHAPE_MEDIA_OK = 0,
    SVG_SHAPE_MEDIA_UNSUPPORTED,
    SVG_SHAPE_MEDIA_INVALID,
    SVG_SHAPE_MEDIA_LIMIT,
    SVG_SHAPE_MEDIA_OOM
} svg_shape_media_status;

typedef svg_shape_media_status (*svg_shape_text_layout_fn)(
    void *context, const svg_shape_text_request *request,
    svg_shape_text_layout *out_layout);
typedef void (*svg_shape_text_release_fn)(
    void *context, svg_shape_text_layout *layout);
typedef svg_shape_media_status (*svg_shape_image_decode_fn)(
    void *context, base_bytes encoded, svg_shape_decoded_image *out_image);
typedef void (*svg_shape_image_release_fn)(
    void *context, svg_shape_decoded_image *image);

/* The SVG decoder has no platform dependency. Applications inject these
 * providers; tests may supply deterministic fakes. Provider outputs remain
 * owned until their matching release callback. */
typedef struct svg_shape_media_providers {
    void *context;
    svg_shape_text_layout_fn layout_text;
    svg_shape_text_release_fn release_text;
    svg_shape_image_decode_fn decode_image;
    svg_shape_image_release_fn release_image;
} svg_shape_media_providers;

#endif
