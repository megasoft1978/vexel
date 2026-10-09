#include "css.h"

#include <stdlib.h>
#include <string.h>

typedef struct selector {
    svg_xml_span element, id, classes[SVG_CSS_MAX_CLASSES];
    uint8_t class_count;
    bool has_element, has_id;
    uint32_t specificity;
} selector;
typedef struct rule_decl {
    svg_css_property property;
    svg_xml_span value;
    bool important;
} rule_decl;
typedef struct rule {
    selector match;
    rule_decl *decls;
    size_t decl_count;
    uint32_t order;
} rule;
struct svg_css_sheet {
    rule *rules;
    size_t count, capacity, bytes, declarations, match_tests;
    bool hit_limit;
    base_bytes source;
};
typedef struct winner {
    svg_xml_span value;
    bool present, important;
    uint32_t specificity, order;
} winner;

static bool ws(uint8_t c) { return c == ' ' || c == '\t' || c == '\r' || c == '\n' || c == '\f'; }
static void skip_ws(base_bytes s, size_t *i, size_t end) {
    while (*i < end && ws(s.ptr[*i]))
        (*i)++;
}
static bool eq(base_bytes s, svg_xml_span a, const char *b) {
    size_t n = strlen(b);
    return a.length == n && a.offset <= s.len && n <= s.len - a.offset &&
           !memcmp(s.ptr + a.offset, b, n);
}
static bool eq_ci(base_bytes s, svg_xml_span a, const char *b) {
    size_t n = strlen(b);
    if (a.length != n || a.offset > s.len || n > s.len - a.offset)
        return false;
    for (size_t i = 0; i < n; i++) {
        uint8_t x = s.ptr[a.offset + i], y = (uint8_t)b[i];
        if (x >= 'A' && x <= 'Z')
            x = (uint8_t)(x + ('a' - 'A'));
        if (y >= 'A' && y <= 'Z')
            y = (uint8_t)(y + ('a' - 'A'));
        if (x != y)
            return false;
    }
    return true;
}
static bool ident(uint8_t c) {
    return (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || (c >= '0' && c <= '9') || c == '_' ||
           c == '-' || c >= 0x80;
}
bool svg_css_is_style_element(base_bytes source, const svg_xml_token *token) {
    return token &&
           (token->kind == SVG_XML_TOKEN_START_ELEMENT ||
            token->kind == SVG_XML_TOKEN_EMPTY_ELEMENT) &&
           token->prefix.length == 0U && eq(source, token->local_name, "style");
}
static bool span_attr(base_bytes s, const svg_xml_token *t, size_t n, size_t e, const char *name,
                      svg_xml_span *v) {
    for (size_t i = e + 1; i < n && t[i].kind == SVG_XML_TOKEN_ATTRIBUTE; i++)
        if (!t[i].prefix.length && eq(s, t[i].local_name, name)) {
            *v = t[i].value;
            return true;
        }
    return false;
}
static void comments(base_bytes s, size_t *i, size_t end) {
    while (*i + 1 < end && s.ptr[*i] == '/' && s.ptr[*i + 1] == '*') {
        *i += 2;
        while (*i + 1 < end && !(s.ptr[*i] == '*' && s.ptr[*i + 1] == '/'))
            (*i)++;
        if (*i + 1 < end)
            *i += 2;
        else
            *i = end;
        skip_ws(s, i, end);
    }
}
static bool skip_comment(base_bytes s, size_t *i, size_t end) {
    if (*i + 1U >= end || s.ptr[*i] != '/' || s.ptr[*i + 1U] != '*')
        return false;
    *i += 2U;
    while (*i + 1U < end && !(s.ptr[*i] == '*' && s.ptr[*i + 1U] == '/'))
        (*i)++;
    if (*i + 1U < end)
        *i += 2U;
    else
        *i = end;
    return true;
}
static void trim_value(base_bytes s, size_t *a, size_t *z) {
    bool moved = true;
    while (moved && *a < *z) {
        moved = false;
        while (*a < *z && ws(s.ptr[*a])) {
            (*a)++;
            moved = true;
        }
        if (*a + 1U < *z && s.ptr[*a] == '/' && s.ptr[*a + 1U] == '*') {
            size_t i = *a + 2U;
            while (i + 1U < *z && !(s.ptr[i] == '*' && s.ptr[i + 1U] == '/'))
                i++;
            *a = i + 1U < *z ? i + 2U : *z;
            moved = true;
        }
    }
    moved = true;
    while (moved && *a < *z) {
        moved = false;
        while (*z > *a && ws(s.ptr[*z - 1U])) {
            (*z)--;
            moved = true;
        }
        if (*z >= *a + 2U && s.ptr[*z - 2U] == '*' && s.ptr[*z - 1U] == '/') {
            size_t i = *z - 2U;
            while (i > *a && !(s.ptr[i - 1U] == '/' && s.ptr[i] == '*'))
                i--;
            if (i > *a) {
                *z = i - 1U;
                moved = true;
            }
        }
    }
}
static int property(base_bytes s, svg_xml_span n) {
    static const char *names[SVG_CSS_PROPERTY_COUNT] = {"fill",
                                                        "fill-rule",
                                                        "stroke",
                                                        "stroke-width",
                                                        "stroke-linecap",
                                                        "stroke-linejoin",
                                                        "stroke-miterlimit",
                                                        "stroke-dasharray",
                                                        "stroke-dashoffset"};
    for (int i = 0; i < (int)SVG_CSS_PROPERTY_COUNT; i++)
        if (eq_ci(s, n, names[i]))
            return i;
    return -1;
}
static bool parse_selector(base_bytes s, size_t a, size_t z, selector *out) {
    selector q = {0};
    while (z > a && ws(s.ptr[z - 1]))
        z--;
    size_t i = a;
    skip_ws(s, &i, z);
    size_t start = i;
    if (i < z && s.ptr[i] != '#' && s.ptr[i] != '.') {
        while (i < z && ident(s.ptr[i]))
            i++;
        if (i == start)
            return false;
        q.element = (svg_xml_span){start, i - start};
        q.has_element = true;
        q.specificity++;
    }
    while (i < z) {
        uint8_t marker = s.ptr[i++];
        start = i;
        while (i < z && ident(s.ptr[i]))
            i++;
        if (i == start)
            return false;
        svg_xml_span x = {start, i - start};
        if (marker == '#' && !q.has_id) {
            q.id = x;
            q.has_id = true;
            q.specificity += 10000U;
        } else if (marker == '.' && q.class_count < SVG_CSS_MAX_CLASSES) {
            q.classes[q.class_count++] = x;
            q.specificity += 100U;
        } else
            return false;
    }
    *out = q;
    return q.has_element || q.has_id || q.class_count != 0U;
}
static bool parse_decls(svg_css_sheet *sh, size_t a, size_t z, rule_decl **out, size_t *count) {
    size_t cap = 0, n = 0;
    rule_decl *v = NULL;
    size_t i = a;
    while (i < z) {
        skip_ws(sh->source, &i, z);
        comments(sh->source, &i, z);
        if (i >= z)
            break;
        size_t seg = i, colon = SIZE_MAX;
        while (i < z) {
            if (skip_comment(sh->source, &i, z))
                continue;
            if (sh->source.ptr[i] == ';')
                break;
            if (sh->source.ptr[i] == ':' && colon == SIZE_MAX)
                colon = i;
            i++;
        }
        size_t end = i;
        if (i < z)
            i++;
        if (colon == SIZE_MAX || colon >= end)
            continue;
        size_t ns = seg, ne = colon;
        while (ns < ne && ws(sh->source.ptr[ns]))
            ns++;
        while (ne > ns && ws(sh->source.ptr[ne - 1]))
            ne--;
        size_t vs = colon + 1, ve = end;
        trim_value(sh->source, &vs, &ve);
        if (ne == ns || vs == ve)
            continue;
        /* Comments are permitted between value tokens; preserve full value span. */
        svg_xml_span nm = {ns, ne - ns};
        int p = property(sh->source, nm);
        if (p < 0)
            continue;
        bool important = false;
        size_t mark = ve;
        while (mark > vs && ws(sh->source.ptr[mark - 1]))
            mark--;
        size_t bang = mark;
        while (bang > vs && ident(sh->source.ptr[bang - 1]))
            bang--;
        size_t marker = bang;
        while (marker > vs && ws(sh->source.ptr[marker - 1]))
            marker--;
        if (marker > vs && sh->source.ptr[marker - 1] == '!') {
            svg_xml_span im = {bang, mark - bang};
            if (eq_ci(sh->source, im, "important")) {
                important = true;
                ve = marker - 1;
                while (ve > vs && ws(sh->source.ptr[ve - 1]))
                    ve--;
            }
        }
        if (ve <= vs)
            continue;
        if (n >= SVG_CSS_MAX_DECLARATIONS) {
            sh->hit_limit = true;
            free(v);
            return false;
        }
        if (n == cap) {
            size_t nc = cap ? cap * 2 : 8;
            if (nc > SVG_CSS_MAX_DECLARATIONS)
                nc = SVG_CSS_MAX_DECLARATIONS;
            if (nc > SIZE_MAX / sizeof(*v)) {
                free(v);
                return false;
            }
            rule_decl *nv = realloc(v, nc * sizeof(*v));
            if (!nv) {
                free(v);
                return false;
            }
            v = nv;
            cap = nc;
        }
        v[n++] = (rule_decl){(svg_css_property)p, {vs, ve - vs}, important};
    }
    *out = v;
    *count = n;
    return true;
}
static bool append_rule(svg_css_sheet *sh, selector sel, rule_decl *d, size_t n) {
    if (sh->count >= SVG_CSS_MAX_RULES || n > SVG_CSS_MAX_DECLARATIONS - sh->declarations) {
        sh->hit_limit = true;
        return false;
    }
    if (sh->count == sh->capacity) {
        size_t next = sh->capacity == 0U ? 8U : sh->capacity * 2U;
        if (next > SVG_CSS_MAX_RULES)
            next = SVG_CSS_MAX_RULES;
        if (next > SIZE_MAX / sizeof(*sh->rules))
            return false;
        rule *grown = realloc(sh->rules, next * sizeof(*grown));
        if (!grown)
            return false;
        sh->rules = grown;
        sh->capacity = next;
    }
    if (n > SIZE_MAX / sizeof(*d))
        return false;
    sh->rules[sh->count] = (rule){sel, d, n, (uint32_t)sh->count};
    sh->count++;
    sh->declarations += n;
    return true;
}
static bool parse_stylesheet(svg_css_sheet *sh, size_t a, size_t z) {
    size_t i = a;
    while (i < z) {
        skip_ws(sh->source, &i, z);
        comments(sh->source, &i, z);
        if (i >= z)
            break;
        if (sh->source.ptr[i] == '@') {
            while (i < z && sh->source.ptr[i] != ';' && sh->source.ptr[i] != '{')
                if (!skip_comment(sh->source, &i, z))
                    i++;
            if (i < z && sh->source.ptr[i] == '{') {
                unsigned depth = 1;
                i++;
                while (i < z && depth) {
                    if (skip_comment(sh->source, &i, z))
                        continue;
                    if (sh->source.ptr[i] == '{')
                        depth++;
                    else if (sh->source.ptr[i] == '}')
                        depth--;
                    i++;
                }
            } else if (i < z)
                i++;
            continue;
        }
        size_t sel_start = i, brace = i;
        while (brace < z && sh->source.ptr[brace] != '{' && sh->source.ptr[brace] != '}')
            if (!skip_comment(sh->source, &brace, z))
                brace++;
        if (brace >= z || sh->source.ptr[brace] != '{') {
            break;
        }
        size_t end = brace + 1;
        unsigned depth = 1;
        while (end < z && depth) {
            if (skip_comment(sh->source, &end, z))
                continue;
            if (sh->source.ptr[end] == '{')
                depth++;
            else if (sh->source.ptr[end] == '}')
                depth--;
            end++;
        }
        if (depth)
            break;
        size_t body_end = end - 1;
        rule_decl *decls = NULL;
        size_t dn = 0;
        if (!parse_decls(sh, brace + 1, body_end, &decls, &dn))
            return false;
        size_t p = sel_start;
        while (p < brace) {
            size_t comma = p;
            while (comma < brace && sh->source.ptr[comma] != ',')
                comma++;
            selector sel;
            if (parse_selector(sh->source, p, comma, &sel) && dn) {
                if (dn > SIZE_MAX / sizeof(rule_decl)) {
                    free(decls);
                    return false;
                }
                rule_decl *copy = malloc(dn * sizeof(*copy));
                if (!copy) {
                    free(decls);
                    return false;
                }
                memcpy(copy, decls, dn * sizeof(*copy));
                if (!append_rule(sh, sel, copy, dn)) {
                    free(copy);
                    free(decls);
                    return false;
                }
            }
            p = comma < brace ? comma + 1 : brace;
        }
        free(decls);
        i = end;
    }
    return true;
}
svg_css_error svg_css_sheet_create(base_bytes source, const svg_xml_token *tokens, size_t count,
                                   svg_css_sheet **out) {
    if (!out || !base_bytes_valid(source) || (!tokens && count))
        return SVG_CSS_LIMIT;
    *out = NULL;
    svg_css_sheet *sh = calloc(1, sizeof(*sh));
    if (!sh)
        return SVG_CSS_OOM;
    sh->source = source;
    for (size_t i = 0; i < count; i++)
        if (tokens[i].kind == SVG_XML_TOKEN_START_ELEMENT &&
            svg_css_is_style_element(source, &tokens[i])) {
            size_t j = i + 1;
            for (; j < count && tokens[j].kind != SVG_XML_TOKEN_END_ELEMENT; j++)
                if (tokens[j].kind == SVG_XML_TOKEN_TEXT) {
                    svg_xml_span x = tokens[j].value;
                    if (x.length > SVG_CSS_MAX_BYTES - sh->bytes) {
                        svg_css_sheet_destroy(sh);
                        return SVG_CSS_LIMIT;
                    }
                    sh->bytes += x.length;
                    if (!parse_stylesheet(sh, x.offset, x.offset + x.length)) {
                        bool limit = sh->hit_limit;
                        svg_css_sheet_destroy(sh);
                        return limit ? SVG_CSS_LIMIT : SVG_CSS_OOM;
                    }
                }
        }
    *out = sh;
    return SVG_CSS_OK;
}
void svg_css_sheet_destroy(svg_css_sheet *sh) {
    if (!sh)
        return;
    for (size_t i = 0; i < sh->count; i++)
        free(sh->rules[i].decls);
    free(sh->rules);
    free(sh);
}
static bool has_class(base_bytes s, svg_xml_span value, svg_xml_span wanted) {
    size_t i = value.offset, end = i + value.length, count = 0;
    while (i < end) {
        while (i < end && ws(s.ptr[i]))
            i++;
        if (i == end)
            break;
        if (++count > 256U)
            return false;
        size_t a = i;
        while (i < end && !ws(s.ptr[i]))
            i++;
        if (i - a == wanted.length && !memcmp(s.ptr + a, s.ptr + wanted.offset, wanted.length))
            return true;
    }
    return false;
}
static bool matches(base_bytes s, const selector *q, const svg_xml_token *t, size_t n, size_t e) {
    if (q->has_element &&
        (q->element.length != t[e].local_name.length ||
         memcmp(s.ptr + q->element.offset, s.ptr + t[e].local_name.offset, q->element.length)))
        return false;
    svg_xml_span id = {0}, cl = {0};
    bool hi = span_attr(s, t, n, e, "id", &id), hc = span_attr(s, t, n, e, "class", &cl);
    if (q->has_id && (!hi || id.length != q->id.length ||
                      memcmp(s.ptr + id.offset, s.ptr + q->id.offset, id.length)))
        return false;
    for (size_t k = 0; k < q->class_count; k++)
        if (!hc || !has_class(s, cl, q->classes[k]))
            return false;
    return true;
}
static void offer(winner *w, const rule_decl *d, uint32_t specificity, uint32_t order) {
    winner *x = &w[d->property];
    if (!x->present || d->important > x->important ||
        (d->important == x->important &&
         (specificity > x->specificity || (specificity == x->specificity && order >= x->order))))
        *x = (winner){d->value, true, d->important, specificity, order};
}
static bool declarations(svg_css_sheet *sh, size_t a, size_t z, uint32_t spec, uint32_t *order,
                         winner *w) {
    rule_decl *v = NULL;
    size_t n = 0;
    if (!parse_decls(sh, a, z, &v, &n))
        return false;
    for (size_t i = 0; i < n; i++)
        offer(w, &v[i], spec, (*order)++);
    free(v);
    return true;
}
svg_css_error svg_css_compute(svg_css_sheet *sheet, base_bytes source, const svg_xml_token *tokens,
                              size_t count, size_t element, const svg_css_paint *parent,
                              svg_css_paint *out) {
    if (!sheet || !out || element >= count)
        return SVG_CSS_LIMIT;
    *out = parent ? *parent : svg_css_initial_paint();
    winner w[SVG_CSS_PROPERTY_COUNT] = {0};
    const svg_xml_token *t = tokens;
    uint32_t order = 0;
    for (size_t r = 0; r < sheet->count; r++) {
        if (sheet->match_tests >= SVG_CSS_MAX_RULE_MATCHES)
            return SVG_CSS_LIMIT;
        sheet->match_tests++;
        if (matches(source, &sheet->rules[r].match, t, count, element))
            for (size_t d = 0; d < sheet->rules[r].decl_count; d++)
                offer(w, &sheet->rules[r].decls[d], sheet->rules[r].match.specificity,
                      sheet->rules[r].order * SVG_CSS_MAX_DECLARATIONS + (uint32_t)d);
    }
    /* Presentation attributes have zero specificity and source order precedes
     * stylesheet rules. */
    for (size_t p = 0; p < SVG_CSS_PROPERTY_COUNT; p++) {
        static const char *names[] = {"fill",
                                      "fill-rule",
                                      "stroke",
                                      "stroke-width",
                                      "stroke-linecap",
                                      "stroke-linejoin",
                                      "stroke-miterlimit",
                                      "stroke-dasharray",
                                      "stroke-dashoffset"};
        svg_xml_span v;
        if (span_attr(source, t, count, element, names[p], &v)) {
            rule_decl d = {(svg_css_property)p, v, false};
            offer(w, &d, 0U, 0U);
        }
    }
    svg_xml_span style;
    if (span_attr(source, t, count, element, "style", &style)) {
        if (style.length > SVG_CSS_MAX_BYTES)
            return SVG_CSS_LIMIT;
        if (!declarations(sheet, style.offset, style.offset + style.length, 1000000U, &order, w))
            return sheet->hit_limit ? SVG_CSS_LIMIT : SVG_CSS_OOM;
    }
    for (size_t p = 0; p < SVG_CSS_PROPERTY_COUNT; p++)
        if (w[p].present)
            out->values[p] = (svg_css_value){w[p].value, true};
    return SVG_CSS_OK;
}
svg_css_paint svg_css_initial_paint(void) { return (svg_css_paint){0}; }
