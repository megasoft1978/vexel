#include "xml_internal.h"

bool svg_xml_internal_char(uint32_t cp)
{
    return cp == 0x9U || cp == 0xaU || cp == 0xdU ||
           (cp >= 0x20U && cp <= 0xd7ffU) || (cp >= 0xe000U && cp <= 0xfffdU) ||
           (cp >= 0x10000U && cp <= 0x10ffffU);
}

bool svg_xml_internal_utf8_next(const uint8_t *bytes, size_t length, size_t *position, uint32_t *cp)
{
    if (position == NULL || cp == NULL || *position >= length) return false;
    size_t p = *position;
    uint8_t a = bytes[p];
    size_t count = 0;
    uint32_t value = 0;
    if (a < 0x80U) { count = 1; value = a; }
    else if (a >= 0xc2U && a <= 0xdfU) { count = 2; value = (uint32_t)(a & 0x1fU); }
    else if (a >= 0xe0U && a <= 0xefU) { count = 3; value = (uint32_t)(a & 0x0fU); }
    else if (a >= 0xf0U && a <= 0xf4U) { count = 4; value = (uint32_t)(a & 0x07U); }
    else return false;
    if (count > length - p) return false;
    for (size_t i = 1; i < count; ++i) {
        uint8_t b = bytes[p + i];
        if ((b & 0xc0U) != 0x80U) return false;
        value = (value << 6U) | (uint32_t)(b & 0x3fU);
    }
    if ((count == 3 && ((a == 0xe0U && bytes[p + 1] < 0xa0U) ||
                        (a == 0xedU && bytes[p + 1] >= 0xa0U))) ||
        (count == 4 && ((a == 0xf0U && bytes[p + 1] < 0x90U) ||
                        (a == 0xf4U && bytes[p + 1] >= 0x90U))) ||
        value > 0x10ffffU || (value >= 0xd800U && value <= 0xdfffU)) return false;
    *position = p + count;
    *cp = value;
    return true;
}

bool svg_xml_internal_name_start(uint32_t c)
{
    return c == ':' || c == '_' || (c >= 'A' && c <= 'Z') || (c >= 'a' && c <= 'z') ||
           (c >= 0xc0U && c <= 0xd6U) || (c >= 0xd8U && c <= 0xf6U) ||
           (c >= 0xf8U && c <= 0x2ffU) || (c >= 0x370U && c <= 0x37dU) ||
           (c >= 0x37fU && c <= 0x1fffU) || (c >= 0x200cU && c <= 0x200dU) ||
           (c >= 0x2070U && c <= 0x218fU) || (c >= 0x2c00U && c <= 0x2fefU) ||
           (c >= 0x3001U && c <= 0xd7ffU) || (c >= 0xf900U && c <= 0xfdcfU) ||
           (c >= 0xfdf0U && c <= 0xfffdU) || (c >= 0x10000U && c <= 0xeffffU);
}

bool svg_xml_internal_name_char(uint32_t c)
{
    return svg_xml_internal_name_start(c) || c == '-' || c == '.' || (c >= '0' && c <= '9') ||
           c == 0xb7U || (c >= 0x300U && c <= 0x36fU) || (c >= 0x203fU && c <= 0x2040U);
}

bool svg_xml_internal_whitespace(uint8_t c)
{
    return c == 0x20U || c == 0x9U || c == 0xaU || c == 0xdU;
}
