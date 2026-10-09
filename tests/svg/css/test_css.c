#include "css.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

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
        "stroke-miterlimit: 5; stroke-dasharray: 2 3; stroke-dashoffset: 4; }"
        "#target.paint { fill: green !important; } .other { fill: white; }"
        "</style><rect id=\"target\" class=\"paint otherish\" fill=\"white\" "
        "style=\"fill: blue; stroke-dashoffset: 7\" width=\"2\" "
        "height=\"2\"/></svg>";
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
         check_value(source, &computed, SVG_CSS_STROKE_DASHOFFSET, "7");
    svg_css_paint initial = svg_css_initial_paint();
    for (size_t i = 0; i < SVG_CSS_PROPERTY_COUNT; i++)
        ok = ok && !initial.values[i].present;
    svg_css_sheet_destroy(sheet);
    free(tokens);
    if (!ok) {
        fprintf(stderr, "CSS cascade/property matrix failed\n");
        return 4;
    }
    puts("CSS bounded cascade and nine-property matrix passed");
    return 0;
}
