/* Included once by xml_tokenizer.c after the shared scanner helpers. */
static bool doctype_allowed(xml_scanner *s, size_t start)
{
    if (s->depth != 0U || s->root_count != 0U || s->doctype_seen) {
        fail(s, SVG_XML_ERROR_MALFORMED_SYNTAX, start);
        return false;
    }
    return true;
}

static bool emit_doctype(xml_scanner *s, size_t end)
{
    svg_xml_token t = {0};
    t.kind = SVG_XML_TOKEN_DOCTYPE;
    t.depth = s->depth;
    (void)span_at(s->cursor, end, &t.source);
    if (!count_node(s, s->cursor) || !emit(s, t, s->cursor)) return false;
    s->doctype_seen = true;
    s->cursor = end;
    return true;
}

static bool parse_doctype(xml_scanner *s)
{
    size_t start = s->cursor;
    if (!doctype_allowed(s, start)) return false;
    s->doctype = (xml_doctype_state){0};
    for (size_t p = start + 9U; p < s->input.len && !cancel_requested(s); ++p) {
        xml_doctype_result r = xml_doctype_step(&s->doctype, byte_at(s, p));
        if (r == XML_DOCTYPE_BAD) { fail(s, SVG_XML_ERROR_MALFORMED_SYNTAX, p); return false; }
        if (r == XML_DOCTYPE_DONE) return emit_doctype(s, p + 1U);
    }
    if (!s->canceled) fail(s, SVG_XML_ERROR_MALFORMED_SYNTAX, start);
    return false;
}
