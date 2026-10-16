#include "css.h"
#include "color.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static bool color_is(const char *value, const uint8_t expected[4]) {
    base_bytes source = {(const uint8_t *)value, strlen(value)};
    uint8_t actual[4] = {0U};
    return svg_css_parse_color(source, (svg_xml_span){0U, source.len}, actual) &&
           memcmp(actual, expected, sizeof(actual)) == 0;
}

static bool color_parser_checks(void) {
    static const char names[] =
        "aliceblue antiquewhite aqua aquamarine azure beige bisque black "
        "blanchedalmond blue blueviolet brown burlywood cadetblue chartreuse "
        "chocolate coral cornflowerblue cornsilk crimson cyan darkblue darkcyan "
        "darkgoldenrod darkgray darkgreen darkgrey darkkhaki darkmagenta "
        "darkolivegreen darkorange darkorchid darkred darksalmon darkseagreen "
        "darkslateblue darkslategray darkslategrey darkturquoise darkviolet "
        "deeppink deepskyblue dimgray dimgrey dodgerblue firebrick floralwhite "
        "forestgreen fuchsia gainsboro ghostwhite gold goldenrod gray green "
        "greenyellow grey honeydew hotpink indianred indigo ivory khaki lavender "
        "lavenderblush lawngreen lemonchiffon lightblue lightcoral lightcyan "
        "lightgoldenrodyellow lightgray lightgreen lightgrey lightpink lightsalmon "
        "lightseagreen lightskyblue lightslategray lightslategrey lightsteelblue "
        "lightyellow lime limegreen linen magenta maroon mediumaquamarine "
        "mediumblue mediumorchid mediumpurple mediumseagreen mediumslateblue "
        "mediumspringgreen mediumturquoise mediumvioletred midnightblue mintcream "
        "mistyrose moccasin navajowhite navy oldlace olive olivedrab orange "
        "orangered orchid palegoldenrod palegreen paleturquoise palevioletred "
        "papayawhip peachpuff peru pink plum powderblue purple rebeccapurple red "
        "rosybrown royalblue saddlebrown salmon sandybrown seagreen seashell sienna "
        "silver skyblue slateblue slategray slategrey snow springgreen steelblue "
        "tan teal thistle tomato turquoise violet wheat white whitesmoke yellow "
        "yellowgreen";
    base_bytes source = {(const uint8_t *)names, sizeof(names) - 1U};
    size_t count = 0U, offset = 0U;
    while (offset < source.len) {
        while (offset < source.len && source.ptr[offset] == ' ') offset++;
        if (offset == source.len) break;
        size_t start = offset;
        while (offset < source.len && source.ptr[offset] != ' ') offset++;
        uint8_t rgba[4];
        if (!svg_css_parse_color(source, (svg_xml_span){start, offset - start}, rgba) ||
            rgba[3] != 255U) return false;
        count++;
    }
    static const uint8_t red[4] = {255U, 0U, 0U, 255U};
    static const uint8_t half_blue[4] = {0U, 0U, 255U, 128U};
    static const uint8_t lime[4] = {0U, 255U, 0U, 255U};
    static const uint8_t quarter_blue[4] = {0U, 0U, 255U, 64U};
    static const uint8_t short_alpha[4] = {0U, 255U, 0U, 136U};
    static const uint8_t long_alpha[4] = {51U, 102U, 153U, 128U};
    static const uint8_t clear[4] = {0U, 0U, 0U, 0U};
    return count == 148U && color_is(" ReBeccAPurple ", (uint8_t[4]){102U, 51U, 153U, 255U}) &&
        color_is("rgb(100% 0% 0%)", red) &&
        color_is("rgba(0, 0, 255, 0.5)", half_blue) &&
        color_is("hsl(120,100%,50%)", lime) &&
        color_is("hsla(240 100% 50% / 25%)", quarter_blue) &&
        color_is("#0f08", short_alpha) && color_is("#33669980", long_alpha) &&
        color_is("transparent", clear) &&
        !color_is("currentColor", clear) && !color_is("#12", clear);
}

static bool check_value(base_bytes source, const svg_css_paint *paint, svg_css_property property,
                        const char *expected) {
    if (!paint->values[property].present)
        return false;
    svg_xml_span span = paint->values[property].span;
    size_t n = strlen(expected);
    return span.length == n && span.offset <= source.len && n <= source.len - span.offset &&
           memcmp(source.ptr + span.offset, expected, n) == 0;
}

int main(void) {
    static const char text[] =
        "<svg width=\"8\" height=\"8\"><style>/* cascade */\n"
        "rect, circle { FiLl: red; fill-rule: evenodd; stroke: /* leading */ "
        "blue /* trailing */; "
        "stroke-width: 2; stroke-linecap: round; stroke-linejoin: bevel; "
        "stroke-miterlimit: 5; stroke-dasharray: 2 3; stroke-dashoffset: 4; "
        "clip-rule:evenodd; }"
        "svg g rect.descendant { clip-path:none; stop-color:currentColor; "
        "stop-opacity:1; fill-opacity:50%; stroke-opacity:.25; opacity:.75; "
        "visibility:hidden; display:inline; color:hsl(240,100%,50%); }"
        "#target.paint { fill: green !important; } .other { fill: white; }"
        "</style><g><rect id=\"target\" class=\"paint otherish descendant\" fill=\"white\" "
        "style=\"fill: blue; stroke-dashoffset: 7\" width=\"2\" "
        "height=\"2\"/></g></svg>";
    base_bytes source = {(const uint8_t *)text, sizeof(text) - 1U};
    size_t capacity = source.len / 2U + 2U;
    svg_xml_token *tokens = calloc(capacity, sizeof(*tokens));
    if (!tokens)
        return 1;
    svg_xml_result parsed = svg_xml_tokenize(source, svg_xml_default_options(), tokens, capacity);
    if (parsed.diagnostic.category != SVG_XML_ERROR_NONE) {
        free(tokens);
        return 2;
    }
    svg_css_sheet *sheet = NULL;
    if (svg_css_sheet_create(source, tokens, parsed.token_count, &sheet) != SVG_CSS_OK) {
        free(tokens);
        return 3;
    }
    size_t target = SIZE_MAX;
    for (size_t i = 0; i < parsed.token_count; i++)
        if ((tokens[i].kind == SVG_XML_TOKEN_START_ELEMENT ||
             tokens[i].kind == SVG_XML_TOKEN_EMPTY_ELEMENT) &&
            tokens[i].local_name.length == 4U &&
            !memcmp(source.ptr + tokens[i].local_name.offset, "rect", 4U))
            target = i;
    svg_css_paint computed;
    bool ok = target != SIZE_MAX && svg_css_compute(sheet, source, tokens, parsed.token_count,
                                                    target, NULL, &computed) == SVG_CSS_OK;
    ok = ok && check_value(source, &computed, SVG_CSS_FILL, "green") &&
         check_value(source, &computed, SVG_CSS_FILL_RULE, "evenodd") &&
         check_value(source, &computed, SVG_CSS_STROKE, "blue") &&
         check_value(source, &computed, SVG_CSS_STROKE_WIDTH, "2") &&
         check_value(source, &computed, SVG_CSS_STROKE_LINECAP, "round") &&
         check_value(source, &computed, SVG_CSS_STROKE_LINEJOIN, "bevel") &&
         check_value(source, &computed, SVG_CSS_STROKE_MITERLIMIT, "5") &&
         check_value(source, &computed, SVG_CSS_STROKE_DASHARRAY, "2 3") &&
         check_value(source, &computed, SVG_CSS_STROKE_DASHOFFSET, "7") &&
         check_value(source, &computed, SVG_CSS_CLIP_PATH, "none") &&
         check_value(source, &computed, SVG_CSS_STOP_COLOR, "currentColor") &&
         check_value(source, &computed, SVG_CSS_STOP_OPACITY, "1") &&
         check_value(source, &computed, SVG_CSS_FILL_OPACITY, "50%") &&
         check_value(source, &computed, SVG_CSS_STROKE_OPACITY, ".25") &&
         check_value(source, &computed, SVG_CSS_OPACITY, ".75") &&
         check_value(source, &computed, SVG_CSS_VISIBILITY, "hidden") &&
         check_value(source, &computed, SVG_CSS_DISPLAY, "inline") &&
         check_value(source, &computed, SVG_CSS_COLOR, "hsl(240,100%,50%)");
    ok = ok && check_value(source, &computed, SVG_CSS_CLIP_RULE, "evenodd");
    svg_css_paint initial = svg_css_initial_paint();
    for (size_t i = 0; i < SVG_CSS_PROPERTY_COUNT; i++)
        ok = ok && !initial.values[i].present;
    svg_css_sheet_destroy(sheet);
    free(tokens);
    if (!ok) {
        fprintf(stderr, "CSS cascade/property matrix failed\n");
        return 4;
    }
    if (!color_parser_checks()) {
        fprintf(stderr, "CSS named and functional color checks failed\n");
        return 5;
    }
    puts("CSS bounded cascade, descendant selectors, clip-rule, and colors passed");
    return 0;
}
