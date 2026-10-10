#ifndef VEXEL_SVG_CSS_COLOR_H
#define VEXEL_SVG_CSS_COLOR_H

#include "base.h"
#include "xml_tokenizer.h"

#include <stdbool.h>
#include <stdint.h>

/* Parses CSS/SVG colors, excluding the context-dependent currentColor keyword. */
bool svg_css_parse_color(base_bytes source, svg_xml_span value,
                         uint8_t rgba[4]);

#endif
