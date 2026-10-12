/* WO-CORPUS-01: unknown non-namespaced attributes, <meta>, <!DOCTYPE> and
 * writing-mode are ignored for rendering and reported, never fatal. */
#include "shape.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static int failures;

static void check(bool condition, const char *what)
{
    if (!condition) {
        fprintf(stderr, "FAIL: %s\n", what);
        failures++;
    } else {
        printf("ok: %s\n", what);
    }
}

static svg_shape_error decode(const char *text, svg_shape_document *doc)
{
    svg_shape_diagnostic diagnostic = {0};
    return svg_shape_decode((base_bytes){(const uint8_t *)text, strlen(text)}, doc, &diagnostic);
}

static bool same_render(const svg_shape_document *a, const svg_shape_document *b)
{
    return a->shape_count == b->shape_count && a->edge_count == b->edge_count &&
           a->edge_count != 0U &&
           memcmp(a->edges, b->edges, a->edge_count * sizeof(*a->edges)) == 0 &&
           memcmp(a->colors, b->colors, a->edge_count * 4U) == 0;
}

static size_t offset_of(const char *text, const char *needle)
{
    const char *at = strstr(text, needle);
    return at == NULL ? SIZE_MAX : (size_t)(at - text);
}

/* Decodes `with` and `without`: both succeed, render identically, and `with`
 * reports exactly `count` ignored entries of `kind`, the first at `needle`. */
static void expect_ignored(const char *name, const char *with, const char *without,
                           svg_shape_ignored_kind kind, size_t count, const char *needle)
{
    svg_shape_document a = {0}, b = {0};
    svg_shape_error ea = decode(with, &a), eb = decode(without, &b);
    check(ea == SVG_SHAPE_ERROR_NONE && eb == SVG_SHAPE_ERROR_NONE, name);
    if (ea != SVG_SHAPE_ERROR_NONE || eb != SVG_SHAPE_ERROR_NONE) goto done;
    check(same_render(&a, &b), "  renders as without the construct");
    check(a.ignored[kind].count == count, "  summary count");
    check(a.ignored[kind].first_offset == offset_of(with, needle), "  summary first offset");
    for (unsigned k = 0U; k < SVG_SHAPE_IGNORED_KIND_COUNT; ++k)
        if (k != (unsigned)kind) check(a.ignored[k].count == 0U, "  no other kind reported");
    check(b.ignored[kind].count == 0U, "  clean document reports nothing");
done:
    svg_shape_document_destroy(&a);
    svg_shape_document_destroy(&b);
}

#define RECT "<rect x=\"1\" y=\"1\" width=\"4\" height=\"3\" fill=\"red\""

int main(void)
{
    expect_ignored("unknown attributes",
        "<svg width=\"8\" height=\"8\" data-doc=\"1\">" RECT " data-a=\"1\" foo=\"bar\"/></svg>",
        "<svg width=\"8\" height=\"8\">" RECT "/></svg>",
        SVG_SHAPE_IGNORED_UNKNOWN_ATTRIBUTE, 3U, "data-doc");
    expect_ignored("unknown attribute on a group",
        "<svg width=\"8\" height=\"8\"><g custom=\"x\">" RECT "/></g></svg>",
        "<svg width=\"8\" height=\"8\"><g>" RECT "/></g></svg>",
        SVG_SHAPE_IGNORED_UNKNOWN_ATTRIBUTE, 1U, "custom");
    expect_ignored("meta element",
        "<svg width=\"8\" height=\"8\"><meta charset=\"utf-8\"/>" RECT "/></svg>",
        "<svg width=\"8\" height=\"8\">" RECT "/></svg>",
        SVG_SHAPE_IGNORED_META, 1U, "<meta");
    expect_ignored("doctype with public identifier",
        "<?xml version=\"1.0\"?>\n<!DOCTYPE svg PUBLIC \"-//W3C//DTD SVG 1.1//EN\" \"x.dtd\">\n"
        "<svg width=\"8\" height=\"8\">" RECT "/></svg>",
        "<svg width=\"8\" height=\"8\">" RECT "/></svg>",
        SVG_SHAPE_IGNORED_DOCTYPE, 1U, "<!DOCTYPE");
    expect_ignored("doctype with internal subset",
        "<!DOCTYPE svg [<!-- ]> --><!ENTITY e \"x>y]\"><?p ]>?>]>"
        "<svg width=\"8\" height=\"8\">" RECT "/></svg>",
        "<svg width=\"8\" height=\"8\">" RECT "/></svg>",
        SVG_SHAPE_IGNORED_DOCTYPE, 1U, "<!DOCTYPE");
    expect_ignored("writing-mode attribute, style attribute and stylesheet",
        "<svg width=\"8\" height=\"8\"><style>.a{writing-mode:tb}</style>"
        RECT " writing-mode=\"tb\"/><g style=\"fill:red; writing-mode : tb-rl\"/></svg>",
        "<svg width=\"8\" height=\"8\"><style>.a{}</style>" RECT "/><g/></svg>",
        SVG_SHAPE_IGNORED_WRITING_MODE, 3U, "writing-mode:tb");

    {
        svg_shape_document doc = {0};
        svg_shape_error e = decode(
            "<svg width=\"8\" height=\"8\" xmlns:x=\"urn:x\">" RECT " x:foo=\"1\"/></svg>", &doc);
        check(e == SVG_SHAPE_ERROR_NONE && doc.ignored[0].count == 0U,
              "namespaced foreign attribute stays silently ignored");
        svg_shape_document_destroy(&doc);
    }
    {
        svg_shape_document doc = {0};
        svg_shape_error e = decode(
            "<svg width=\"8\" height=\"8\">" RECT " text-decoration=\"underline\"/></svg>", &doc);
        check(e == SVG_SHAPE_ERROR_UNSUPPORTED,
              "a known rendering attribute that is unsupported still fails closed");
    }
    {
        svg_shape_document doc = {0};
        check(decode("<!DOCTYPE svg [<!ENTITY e \"x\">]><svg width=\"8\" height=\"8\">"
                     "<text>&e;</text></svg>", &doc) == SVG_SHAPE_ERROR_XML,
              "DOCTYPE entity referenced in text still fails as XML, unexpanded");
        check(decode("<!DOCTYPE svg [<!ENTITY e \"1\">]><svg width=\"8\" height=\"8\">"
                     RECT " x=\"&e;\"/></svg>", &doc) == SVG_SHAPE_ERROR_XML,
              "DOCTYPE entity referenced in an attribute still fails as XML");
        check(decode("<svg width=\"8\" height=\"8\"/><!DOCTYPE svg>", &doc) == SVG_SHAPE_ERROR_XML,
              "DOCTYPE after the root element is still an XML error");
    }
    if (failures != 0) {
        fprintf(stderr, "%d failure(s)\n", failures);
        return 1;
    }
    puts("ignored-construct tests passed");
    return 0;
}
