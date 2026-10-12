#ifndef VEXEL_SVG_ANIMATION_AUTHORING_INTERNAL_H
#define VEXEL_SVG_ANIMATION_AUTHORING_INTERNAL_H

#include "animation_internal.h"

#define AUTHORING_VALUE_LIMIT (64U * 1024U)
#define AUTHORING_PATCH_LIMIT (1024U * 1024U)

/* Growable owned byte buffer; the caller frees bytes. Bounded by the patch limit. */
typedef struct authoring_builder {
    uint8_t *bytes;
    size_t length;
    size_t capacity;
} authoring_builder;

bool authoring_builder_append(authoring_builder *builder, const uint8_t *bytes,
                              size_t length);
bool authoring_builder_byte(authoring_builder *builder, uint8_t byte);
bool authoring_builder_cstr(authoring_builder *builder, const char *text);
/* Appends value escaped for an XML attribute delimited by quote. */
bool authoring_builder_attribute_value(authoring_builder *builder, base_bytes value,
                                       uint8_t quote);
bool authoring_append_number(authoring_builder *builder, double value);

#endif
