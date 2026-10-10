#include "color.h"

#include <math.h>
#include <stddef.h>
#include <string.h>

typedef struct named_color { const char *name; uint32_t rgb; } named_color;

/* CSS Color 4 named colors, including the later rebeccapurple addition. */
static const named_color colors[] = {
    {"aliceblue",0xf0f8ff},{"antiquewhite",0xfaebd7},{"aqua",0x00ffff},
    {"aquamarine",0x7fffd4},{"azure",0xf0ffff},{"beige",0xf5f5dc},
    {"bisque",0xffe4c4},{"black",0x000000},{"blanchedalmond",0xffebcd},
    {"blue",0x0000ff},{"blueviolet",0x8a2be2},{"brown",0xa52a2a},
    {"burlywood",0xdeb887},{"cadetblue",0x5f9ea0},{"chartreuse",0x7fff00},
    {"chocolate",0xd2691e},{"coral",0xff7f50},{"cornflowerblue",0x6495ed},
    {"cornsilk",0xfff8dc},{"crimson",0xdc143c},{"cyan",0x00ffff},
    {"darkblue",0x00008b},{"darkcyan",0x008b8b},{"darkgoldenrod",0xb8860b},
    {"darkgray",0xa9a9a9},{"darkgreen",0x006400},{"darkgrey",0xa9a9a9},
    {"darkkhaki",0xbdb76b},{"darkmagenta",0x8b008b},{"darkolivegreen",0x556b2f},
    {"darkorange",0xff8c00},{"darkorchid",0x9932cc},{"darkred",0x8b0000},
    {"darksalmon",0xe9967a},{"darkseagreen",0x8fbc8f},{"darkslateblue",0x483d8b},
    {"darkslategray",0x2f4f4f},{"darkslategrey",0x2f4f4f},{"darkturquoise",0x00ced1},
    {"darkviolet",0x9400d3},{"deeppink",0xff1493},{"deepskyblue",0x00bfff},
    {"dimgray",0x696969},{"dimgrey",0x696969},{"dodgerblue",0x1e90ff},
    {"firebrick",0xb22222},{"floralwhite",0xfffaf0},{"forestgreen",0x228b22},
    {"fuchsia",0xff00ff},{"gainsboro",0xdcdcdc},{"ghostwhite",0xf8f8ff},
    {"gold",0xffd700},{"goldenrod",0xdaa520},{"gray",0x808080},
    {"green",0x008000},{"greenyellow",0xadff2f},{"grey",0x808080},
    {"honeydew",0xf0fff0},{"hotpink",0xff69b4},{"indianred",0xcd5c5c},
    {"indigo",0x4b0082},{"ivory",0xfffff0},{"khaki",0xf0e68c},
    {"lavender",0xe6e6fa},{"lavenderblush",0xfff0f5},{"lawngreen",0x7cfc00},
    {"lemonchiffon",0xfffacd},{"lightblue",0xadd8e6},{"lightcoral",0xf08080},
    {"lightcyan",0xe0ffff},{"lightgoldenrodyellow",0xfafad2},{"lightgray",0xd3d3d3},
    {"lightgreen",0x90ee90},{"lightgrey",0xd3d3d3},{"lightpink",0xffb6c1},
    {"lightsalmon",0xffa07a},{"lightseagreen",0x20b2aa},{"lightskyblue",0x87cefa},
    {"lightslategray",0x778899},{"lightslategrey",0x778899},{"lightsteelblue",0xb0c4de},
    {"lightyellow",0xffffe0},{"lime",0x00ff00},{"limegreen",0x32cd32},
    {"linen",0xfaf0e6},{"magenta",0xff00ff},{"maroon",0x800000},
    {"mediumaquamarine",0x66cdaa},{"mediumblue",0x0000cd},{"mediumorchid",0xba55d3},
    {"mediumpurple",0x9370db},{"mediumseagreen",0x3cb371},{"mediumslateblue",0x7b68ee},
    {"mediumspringgreen",0x00fa9a},{"mediumturquoise",0x48d1cc},{"mediumvioletred",0xc71585},
    {"midnightblue",0x191970},{"mintcream",0xf5fffa},{"mistyrose",0xffe4e1},
    {"moccasin",0xffe4b5},{"navajowhite",0xffdead},{"navy",0x000080},
    {"oldlace",0xfdf5e6},{"olive",0x808000},{"olivedrab",0x6b8e23},
    {"orange",0xffa500},{"orangered",0xff4500},{"orchid",0xda70d6},
    {"palegoldenrod",0xeee8aa},{"palegreen",0x98fb98},{"paleturquoise",0xafeeee},
    {"palevioletred",0xdb7093},{"papayawhip",0xffefd5},{"peachpuff",0xffdab9},
    {"peru",0xcd853f},{"pink",0xffc0cb},{"plum",0xdda0dd},
    {"powderblue",0xb0e0e6},{"purple",0x800080},{"rebeccapurple",0x663399},
    {"red",0xff0000},{"rosybrown",0xbc8f8f},{"royalblue",0x4169e1},
    {"saddlebrown",0x8b4513},{"salmon",0xfa8072},{"sandybrown",0xf4a460},
    {"seagreen",0x2e8b57},{"seashell",0xfff5ee},{"sienna",0xa0522d},
    {"silver",0xc0c0c0},{"skyblue",0x87ceeb},{"slateblue",0x6a5acd},
    {"slategray",0x708090},{"slategrey",0x708090},{"snow",0xfffafa},
    {"springgreen",0x00ff7f},{"steelblue",0x4682b4},{"tan",0xd2b48c},
    {"teal",0x008080},{"thistle",0xd8bfd8},{"tomato",0xff6347},
    {"turquoise",0x40e0d0},{"violet",0xee82ee},{"wheat",0xf5deb3},
    {"white",0xffffff},{"whitesmoke",0xf5f5f5},{"yellow",0xffff00},
    {"yellowgreen",0x9acd32}
};

typedef struct reader { base_bytes source; size_t i, end; } reader;
typedef struct component { double value; bool percent; size_t suffix, suffix_length; } component;

static bool ws(uint8_t c)
{
    return c == ' ' || c == '\t' || c == '\r' || c == '\n' || c == '\f';
}

static uint8_t lower(uint8_t c)
{
    return c >= 'A' && c <= 'Z' ? (uint8_t)(c + ('a' - 'A')) : c;
}

static bool span_eq_ci(base_bytes source, svg_xml_span span, const char *text)
{
    size_t n = strlen(text);
    if (span.length != n || span.offset > source.len || n > source.len - span.offset)
        return false;
    for (size_t i = 0; i < n; ++i)
        if (lower(source.ptr[span.offset + i]) != (uint8_t)text[i]) return false;
    return true;
}

static void skip_ws(reader *r)
{
    while (r->i < r->end && ws(r->source.ptr[r->i])) r->i++;
}

static bool number(reader *r, component *out)
{
    skip_ws(r);
    size_t start = r->i;
    bool negative = false;
    if (r->i < r->end && (r->source.ptr[r->i] == '+' || r->source.ptr[r->i] == '-')) {
        negative = r->source.ptr[r->i] == '-';
        r->i++;
    }
    double value = 0.0;
    size_t digits = 0U;
    while (r->i < r->end && r->source.ptr[r->i] >= '0' && r->source.ptr[r->i] <= '9') {
        value = value * 10.0 + (double)(r->source.ptr[r->i++] - '0');
        if (!isfinite(value) || value > 1e100) return false;
        digits++;
    }
    if (r->i < r->end && r->source.ptr[r->i] == '.') {
        r->i++;
        double place = 0.1;
        while (r->i < r->end && r->source.ptr[r->i] >= '0' && r->source.ptr[r->i] <= '9') {
            value += (double)(r->source.ptr[r->i++] - '0') * place;
            place *= 0.1;
            digits++;
        }
    }
    if (digits == 0U) { r->i = start; return false; }
    int exponent = 0;
    bool exponent_negative = false;
    if (r->i < r->end && (r->source.ptr[r->i] == 'e' || r->source.ptr[r->i] == 'E')) {
        r->i++;
        if (r->i < r->end && (r->source.ptr[r->i] == '+' || r->source.ptr[r->i] == '-')) {
            exponent_negative = r->source.ptr[r->i] == '-';
            r->i++;
        }
        size_t exponent_digits = 0U;
        while (r->i < r->end && r->source.ptr[r->i] >= '0' && r->source.ptr[r->i] <= '9') {
            if (exponent < 1000) exponent = exponent * 10 + (int)(r->source.ptr[r->i] - '0');
            exponent_digits++;
            r->i++;
        }
        if (exponent_digits == 0U) return false;
    }
    if (exponent_negative) exponent = -exponent;
    if (exponent > 308 || exponent < -324) return false;
    value *= pow(10.0, (double)exponent);
    if (negative) value = -value;
    if (!isfinite(value)) return false;
    size_t suffix = r->i;
    while (r->i < r->end && ((r->source.ptr[r->i] >= 'a' && r->source.ptr[r->i] <= 'z') ||
                             (r->source.ptr[r->i] >= 'A' && r->source.ptr[r->i] <= 'Z') ||
                             r->source.ptr[r->i] == '%')) r->i++;
    *out = (component){value, false, suffix, r->i - suffix};
    if (out->suffix_length == 1U && r->source.ptr[suffix] == '%') out->percent = true;
    return true;
}

static int hex(uint8_t c)
{
    if (c >= '0' && c <= '9') return (int)(c - '0');
    c = lower(c);
    return c >= 'a' && c <= 'f' ? (int)(c - 'a' + 10) : -1;
}

static bool parse_hex(base_bytes source, svg_xml_span span, uint8_t rgba[4])
{
    if (span.length != 4U && span.length != 5U && span.length != 7U && span.length != 9U)
        return false;
    if (source.ptr[span.offset] != '#') return false;
    bool short_form = span.length <= 5U;
    size_t channels = span.length == 5U || span.length == 9U ? 4U : 3U;
    for (size_t i = 0U; i < channels; ++i) {
        size_t at = span.offset + 1U + i * (short_form ? 1U : 2U);
        int hi = hex(source.ptr[at]);
        if (hi < 0) return false;
        if (short_form) rgba[i] = (uint8_t)(hi * 17);
        else {
            int lo = hex(source.ptr[at + 1U]);
            if (lo < 0) return false;
            rgba[i] = (uint8_t)(hi * 16 + lo);
        }
    }
    if (channels == 3U) rgba[3] = 255U;
    return true;
}

static bool parse_component_triplet(reader *r, bool hue, component out[4], size_t *count)
{
    bool comma_mode = false;
    for (size_t i = 0U; i < 3U; ++i) {
        if (!number(r, &out[i])) return false;
        if (i == 0U) {
            size_t before = r->i;
            skip_ws(r);
            bool spaced = r->i != before;
            if (r->i < r->end && r->source.ptr[r->i] == ',') {
                comma_mode = true;
                r->i++;
                skip_ws(r);
            } else if (!spaced) return false;
        } else if (i == 1U) {
            size_t before = r->i;
            skip_ws(r);
            bool spaced = r->i != before;
            if (comma_mode) {
                if (r->i >= r->end || r->source.ptr[r->i] != ',') return false;
                r->i++;
                skip_ws(r);
            } else if (!spaced) return false;
        }
    }
    *count = 3U;
    size_t before = r->i;
    skip_ws(r);
    bool spaced = r->i != before;
    bool alpha = false;
    if (comma_mode && r->i < r->end && r->source.ptr[r->i] == ',') {
        r->i++;
        skip_ws(r);
        alpha = true;
    } else if (!comma_mode && r->i < r->end && r->source.ptr[r->i] == '/') {
        r->i++;
        alpha = true;
    } else if (!comma_mode && spaced && r->i < r->end && r->source.ptr[r->i] != ')') {
        if (r->source.ptr[r->i] == '/') r->i++;
        alpha = true;
    }
    if (alpha) {
        if (!number(r, &out[3])) return false;
        *count = 4U;
    }
    if (hue) {
        component *h = &out[0];
        if (h->suffix_length != 0U) {
            svg_xml_span unit = {h->suffix, h->suffix_length};
            if (span_eq_ci(r->source, unit, "deg")) {
                /* already degrees */
            } else if (span_eq_ci(r->source, unit, "rad")) h->value *= 57.29577951308232;
            else if (span_eq_ci(r->source, unit, "grad")) h->value *= 0.9;
            else if (span_eq_ci(r->source, unit, "turn")) h->value *= 360.0;
            else return false;
        }
        if (!out[1].percent || !out[2].percent) return false;
    } else {
        for (size_t i = 0U; i < 3U; ++i)
            if (out[i].suffix_length != 0U && !out[i].percent) return false;
    }
    if (*count == 4U && out[3].suffix_length != 0U && !out[3].percent) return false;
    return true;
}

static double clamp(double value, double low, double high)
{
    return value < low ? low : value > high ? high : value;
}

static uint8_t byte(double value)
{
    return (uint8_t)floor(clamp(value, 0.0, 255.0) + 0.5);
}

static void hsl_to_rgb(double hue, double saturation, double lightness, uint8_t rgba[4])
{
    hue = fmod(hue, 360.0);
    if (hue < 0.0) hue += 360.0;
    saturation = clamp(saturation, 0.0, 1.0);
    lightness = clamp(lightness, 0.0, 1.0);
    double chroma = (1.0 - fabs(2.0 * lightness - 1.0)) * saturation;
    double h = hue / 60.0;
    double x = chroma * (1.0 - fabs(fmod(h, 2.0) - 1.0));
    double r = 0.0, g = 0.0, b = 0.0;
    if (h < 1.0) { r = chroma; g = x; }
    else if (h < 2.0) { r = x; g = chroma; }
    else if (h < 3.0) { g = chroma; b = x; }
    else if (h < 4.0) { g = x; b = chroma; }
    else if (h < 5.0) { r = x; b = chroma; }
    else { r = chroma; b = x; }
    double m = lightness - chroma * 0.5;
    rgba[0] = byte((r + m) * 255.0);
    rgba[1] = byte((g + m) * 255.0);
    rgba[2] = byte((b + m) * 255.0);
}

static bool parse_function(base_bytes source, svg_xml_span span, uint8_t rgba[4])
{
    size_t open = span.offset;
    while (open < span.offset + span.length && source.ptr[open] != '(') open++;
    if (open == span.offset || open >= span.offset + span.length ||
        source.ptr[span.offset + span.length - 1U] != ')') return false;
    size_t name_len = open - span.offset;
    bool hsl = false;
    if ((name_len == 3U && lower(source.ptr[span.offset]) == 'r' &&
         lower(source.ptr[span.offset + 1U]) == 'g' && lower(source.ptr[span.offset + 2U]) == 'b') ||
        (name_len == 4U && lower(source.ptr[span.offset]) == 'r' &&
         lower(source.ptr[span.offset + 1U]) == 'g' && lower(source.ptr[span.offset + 2U]) == 'b' &&
         lower(source.ptr[span.offset + 3U]) == 'a')) {
        hsl = false;
    } else if ((name_len == 3U && lower(source.ptr[span.offset]) == 'h' &&
                lower(source.ptr[span.offset + 1U]) == 's' && lower(source.ptr[span.offset + 2U]) == 'l') ||
               (name_len == 4U && lower(source.ptr[span.offset]) == 'h' &&
                lower(source.ptr[span.offset + 1U]) == 's' && lower(source.ptr[span.offset + 2U]) == 'l' &&
                lower(source.ptr[span.offset + 3U]) == 'a')) {
        hsl = true;
    } else return false;
    reader r = {source, open + 1U, span.offset + span.length - 1U};
    component values[4] = {{0}};
    size_t count = 0U;
    if (!parse_component_triplet(&r, hsl, values, &count)) return false;
    skip_ws(&r);
    if (r.i != r.end) return false;
    double alpha = count == 4U ? (values[3].percent ? values[3].value / 100.0 : values[3].value) : 1.0;
    if (hsl) {
        hsl_to_rgb(values[0].value, values[1].value / 100.0,
                   values[2].value / 100.0, rgba);
    } else {
        for (size_t i = 0U; i < 3U; ++i) {
            double component_value = values[i].percent ? values[i].value * 2.55 : values[i].value;
            rgba[i] = byte(component_value);
        }
    }
    rgba[3] = byte(clamp(alpha, 0.0, 1.0) * 255.0);
    return true;
}

bool svg_css_parse_color(base_bytes source, svg_xml_span value, uint8_t rgba[4])
{
    if (!base_bytes_valid(source) || rgba == NULL || value.offset > source.len ||
        value.length == 0U || value.length > source.len - value.offset || value.length > 256U)
        return false;
    size_t first = value.offset, end = value.offset + value.length;
    while (first < end && ws(source.ptr[first])) first++;
    while (end > first && ws(source.ptr[end - 1U])) end--;
    if (first == end) return false;
    svg_xml_span trimmed = {first, end - first};
    if (span_eq_ci(source, trimmed, "transparent")) {
        rgba[0] = rgba[1] = rgba[2] = rgba[3] = 0U;
        return true;
    }
    if (source.ptr[first] == '#') return parse_hex(source, trimmed, rgba);
    if (memchr(source.ptr + first, '(', trimmed.length) != NULL)
        return parse_function(source, trimmed, rgba);
    for (size_t i = 0U; i < sizeof(colors) / sizeof(colors[0]); ++i) {
        size_t n = strlen(colors[i].name);
        if (n != trimmed.length) continue;
        bool match = true;
        for (size_t c = 0U; c < n; ++c)
            match &= lower(source.ptr[first + c]) == (uint8_t)colors[i].name[c];
        if (match) {
            rgba[0] = (uint8_t)(colors[i].rgb >> 16U);
            rgba[1] = (uint8_t)(colors[i].rgb >> 8U);
            rgba[2] = (uint8_t)colors[i].rgb;
            rgba[3] = 255U;
            return true;
        }
    }
    return false;
}
