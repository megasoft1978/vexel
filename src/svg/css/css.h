#ifndef VEXEL_SVG_CSS_H
#define VEXEL_SVG_CSS_H

#include "xml_tokenizer.h"

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

/* Bounded, source-preserving CSS paint subset. All spans address source bytes.
 */
#define SVG_CSS_PROPERTY_COUNT 38U
#define SVG_CSS_MAX_BYTES (1024U * 1024U)
#define SVG_CSS_MAX_RULES 4096U
#define SVG_CSS_MAX_DECLARATIONS 16384U
#define SVG_CSS_MAX_CLASSES 16U
#define SVG_CSS_MAX_SELECTOR_TERMS 4U
#define SVG_CSS_MAX_RULE_MATCHES 4194304U

typedef enum svg_css_property {
    SVG_CSS_FILL,
    SVG_CSS_FILL_RULE,
    SVG_CSS_STROKE,
    SVG_CSS_STROKE_WIDTH,
    SVG_CSS_STROKE_LINECAP,
    SVG_CSS_STROKE_LINEJOIN,
    SVG_CSS_STROKE_MITERLIMIT,
    SVG_CSS_STROKE_DASHARRAY,
    SVG_CSS_STROKE_DASHOFFSET,
    SVG_CSS_CLIP_PATH,
    SVG_CSS_STOP_COLOR,
    SVG_CSS_STOP_OPACITY,
    SVG_CSS_FILL_OPACITY,
    SVG_CSS_STROKE_OPACITY,
    SVG_CSS_OPACITY,
    SVG_CSS_VISIBILITY,
    SVG_CSS_DISPLAY,
    SVG_CSS_COLOR,
    SVG_CSS_FONT_FAMILY,
    SVG_CSS_FONT_SIZE,
    SVG_CSS_FONT_WEIGHT,
    SVG_CSS_FONT_STYLE,
    SVG_CSS_TEXT_ANCHOR,
    SVG_CSS_LETTER_SPACING,
    SVG_CSS_WORD_SPACING,
    SVG_CSS_FILTER,
    SVG_CSS_MASK,
    SVG_CSS_MIX_BLEND_MODE,
    SVG_CSS_ISOLATION,
    SVG_CSS_VECTOR_EFFECT,
    SVG_CSS_SHAPE_RENDERING,
    SVG_CSS_PAINT_ORDER,
    SVG_CSS_MARKER,
    SVG_CSS_MARKER_START,
    SVG_CSS_MARKER_MID,
    SVG_CSS_MARKER_END,
    SVG_CSS_MASK_TYPE,
    SVG_CSS_COLOR_INTERPOLATION_FILTERS
} svg_css_property;

typedef struct svg_css_value {
    svg_xml_span span;
    bool present;
} svg_css_value;
typedef struct svg_css_paint {
    svg_css_value values[SVG_CSS_PROPERTY_COUNT];
} svg_css_paint;
static inline bool svg_css_keyword_equals(base_bytes source, svg_xml_span span,
                                          const char *literal) {
    size_t n = 0U;
    while (literal[n] != '\0')
        n++;
    if (span.length != n || span.offset > source.len || n > source.len - span.offset)
        return false;
    for (size_t i = 0; i < n; i++) {
        uint8_t a = source.ptr[span.offset + i], b = (uint8_t)literal[i];
        if (a >= 'A' && a <= 'Z')
            a = (uint8_t)(a + ('a' - 'A'));
        if (b >= 'A' && b <= 'Z')
            b = (uint8_t)(b + ('a' - 'A'));
        if (a != b)
            return false;
    }
    return true;
}
static inline bool svg_css_paint_value(const svg_css_paint *paint, svg_css_property property,
                                       svg_xml_span *value) {
    if (paint == NULL || value == NULL || (unsigned)property >= SVG_CSS_PROPERTY_COUNT ||
        !paint->values[property].present)
        return false;
    *value = paint->values[property].span;
    return true;
}
typedef struct svg_css_sheet svg_css_sheet;
typedef enum svg_css_error {
    SVG_CSS_OK,
    SVG_CSS_LIMIT,
    SVG_CSS_OOM,
    SVG_CSS_UNSUPPORTED
} svg_css_error;

svg_css_error svg_css_sheet_create(base_bytes source, const svg_xml_token *tokens,
                                   size_t token_count, svg_css_sheet **out);
bool svg_css_is_style_element(base_bytes source, const svg_xml_token *token);
void svg_css_sheet_destroy(svg_css_sheet *sheet);
/* Parent paint is copied, then matched rules, presentation attributes and
 * inline style apply. */
svg_css_error svg_css_compute(svg_css_sheet *sheet, base_bytes source, const svg_xml_token *tokens,
                              size_t token_count, size_t element, const svg_css_paint *parent,
                              svg_css_paint *out);
svg_css_paint svg_css_initial_paint(void);

#endif
