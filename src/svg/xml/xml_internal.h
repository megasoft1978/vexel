#ifndef VEXEL_XML_INTERNAL_H
#define VEXEL_XML_INTERNAL_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

bool svg_xml_internal_char(uint32_t codepoint);
bool svg_xml_internal_utf8_next(const uint8_t *bytes, size_t length,
                                 size_t *position, uint32_t *codepoint);
bool svg_xml_internal_name_start(uint32_t codepoint);
bool svg_xml_internal_name_char(uint32_t codepoint);
bool svg_xml_internal_whitespace(uint8_t byte);

#endif
